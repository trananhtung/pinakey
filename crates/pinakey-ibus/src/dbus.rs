//! Lớp transport D-Bus cho engine IBus — chuyển thể từ goibus (`bus.go`, `factory.go`, `engine.go`)
//! và `main.go`, hiện thực trên nền zbus.
//!
//! Lưu ý: lớp này cần IBus daemon và D-Bus đang chạy thực sự nên không thể kiểm thử bằng unit test
//! của crate; phần logic thuần được kiểm thử trong `core`. Đây là bản chuyển thể trung thực, biên
//! dịch được, của bề mặt giao thức mà goibus cung cấp cho chế độ nhập Preedit.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use zbus::object_server::{ObjectServer, SignalEmitter};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::{connection, interface};

use pinakey_config::load_config;

use crate::constants::*;
use crate::core::Action;
use crate::engine_actor::EngineHandle;
use crate::serialize::{IBusLookupTable, IBusPropList, IBusText};

/// Đối tượng engine IBus, được export trên `org.freedesktop.IBus.Engine`.
pub struct PinaKeyEngine {
    handle: EngineHandle,
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl PinaKeyEngine {
    async fn process_key_event(
        &self,
        keyval: u32,
        keycode: u32,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let (handled, actions) = self.handle.process_key(keyval, keycode, state);
        if let Err(e) = apply_actions(&emitter, &actions).await {
            eprintln!("failed to emit signal: {e}");
        }
        handled
    }

    async fn focus_in(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        // Phát hiện class của cửa sổ đang focus để áp dụng cách khắc phục theo từng ứng dụng (chỉ
        // X11; nơi khác không làm gì), tương ứng FocusIn -> checkWmClass của engine Go.
        if let Some(class) = pinakey_platform::get_focus_window_class() {
            self.handle.set_wm_class(class);
        }
        // Đăng ký menu thuộc tính lên panel IBus.
        if let Err(e) = self.register_props(&emitter).await {
            eprintln!("failed to register properties: {e}");
        }
    }
    async fn focus_out(&self) {}

    async fn reset(&self) {
        self.handle.reset();
    }

    async fn enable(&self) {}
    async fn disable(&self) {}
    async fn set_capabilities(&self, _cap: u32) {}
    async fn set_cursor_location(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}
    async fn set_content_type(&self, _purpose: u32, _hints: u32) {}
    async fn set_surrounding_text(&self, _text: Value<'_>, _cursor: u32, _anchor: u32) {}
    async fn property_activate(
        &self,
        name: String,
        state: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let actions = self.handle.property_activate(name, state);
        if let Err(e) = apply_actions(&emitter, &actions).await {
            eprintln!("failed to emit signal: {e}");
        }
        // Trạng thái có thể đổi (bật/tắt VN, đổi kiểu gõ) -> cập nhật lại menu.
        if let Err(e) = self.register_props(&emitter).await {
            eprintln!("failed to refresh properties: {e}");
        }
    }
    async fn property_show(&self, _name: String) {}
    async fn property_hide(&self, _name: String) {}
    async fn page_up(&self) {}
    async fn page_down(&self) {}
    async fn cursor_up(&self) {}
    async fn cursor_down(&self) {}
    async fn candidate_clicked(&self, _index: u32, _button: u32, _state: u32) {}
    async fn destroy(&self) {}

    // ----- các signal (org.freedesktop.IBus.Engine) -----

    #[zbus(signal)]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_auxiliary_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_auxiliary_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_lookup_table(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_lookup_table(
        emitter: &SignalEmitter<'_>,
        table: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_lookup_table(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn forward_key_event(
        emitter: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn require_surrounding_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn delete_surrounding_text(
        emitter: &SignalEmitter<'_>,
        offset: i32,
        nchars: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn register_properties(emitter: &SignalEmitter<'_>, props: Value<'_>)
        -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_property(emitter: &SignalEmitter<'_>, prop: Value<'_>) -> zbus::Result<()>;
}

impl PinaKeyEngine {
    /// Dựng menu thuộc tính hiện tại và phát `register_properties` lên panel.
    async fn register_props(&self, emitter: &SignalEmitter<'_>) -> zbus::Result<()> {
        let props = self.handle.props();
        let list = IBusPropList::from_props(&props)
            .and_then(|l| l.into_value())
            .map_err(zbus::Error::from)?;
        PinaKeyEngine::register_properties(emitter, list.into()).await
    }
}

async fn apply_actions(emitter: &SignalEmitter<'_>, actions: &[Action]) -> zbus::Result<()> {
    for action in actions {
        match action {
            Action::CommitText(s) => {
                let v = make_text_value(s, None)?;
                PinaKeyEngine::commit_text(emitter, v.into()).await?;
            }
            Action::UpdatePreedit {
                text,
                cursor,
                underline,
            } => {
                let v = make_text_value(text, if *underline { Some(*cursor) } else { None })?;
                PinaKeyEngine::update_preedit_text(
                    emitter,
                    v.into(),
                    *cursor,
                    true,
                    IBUS_ENGINE_PREEDIT_COMMIT,
                )
                .await?;
            }
            Action::UpdateAuxiliary { text, visible } => {
                let v = make_text_value(text, None)?;
                PinaKeyEngine::update_auxiliary_text(emitter, v.into(), *visible).await?;
            }
            Action::HidePreedit => PinaKeyEngine::hide_preedit_text(emitter).await?,
            Action::HideAuxiliary => PinaKeyEngine::hide_auxiliary_text(emitter).await?,
            Action::HideLookupTable => PinaKeyEngine::hide_lookup_table(emitter).await?,
            Action::DeleteSurroundingText { offset, nchars } => {
                PinaKeyEngine::delete_surrounding_text(emitter, *offset, *nchars).await?;
            }
            Action::ForwardBackspaces(n) => {
                // Phát từng cặp press + release phím BackSpace; chạy được cả trên Wayland.
                for _ in 0..*n {
                    PinaKeyEngine::forward_key_event(emitter, IBUS_BACKSPACE, BACKSPACE_KEYCODE, 0)
                        .await?;
                    PinaKeyEngine::forward_key_event(
                        emitter,
                        IBUS_BACKSPACE,
                        BACKSPACE_KEYCODE,
                        IBUS_RELEASE_MASK,
                    )
                    .await?;
                }
            }
            Action::FakeBackspaces(n) => {
                // Tiêm phím qua XTest là một lệnh X11 đồng bộ — chạy trên blocking pool để không
                // chặn reactor async, đồng thời giữ đúng thứ tự (xóa xong mới commit phần đuôi).
                let n = *n;
                let _ =
                    tokio::task::spawn_blocking(move || pinakey_platform::fake_backspaces(n)).await;
            }
            Action::UpdateLookupTable {
                candidates,
                cursor,
                page_size,
                visible,
            } => {
                let table = IBusLookupTable::new(candidates, *cursor, *page_size)
                    .and_then(|t| t.into_value())
                    .map_err(zbus::Error::from)?;
                PinaKeyEngine::update_lookup_table(emitter, table.into(), *visible).await?;
                if *visible {
                    PinaKeyEngine::show_lookup_table(emitter).await?;
                } else {
                    PinaKeyEngine::hide_lookup_table(emitter).await?;
                }
            }
            Action::LaunchSettings => launch_settings(),
        }
    }
    Ok(())
}

/// Mở giao diện thiết lập đồ họa. IBus có thể chạy engine với `PATH` hạn chế nên thử cả `PATH` lẫn
/// đường dẫn cài đặt mặc định.
fn launch_settings() {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        "pinakey-settings".to_string(),
        format!("{home}/.local/bin/pinakey-settings"),
    ];
    for c in &candidates {
        if std::process::Command::new(c).spawn().is_ok() {
            return;
        }
    }
    eprintln!("pinakey: không tìm thấy 'pinakey-settings' để mở giao diện thiết lập");
}

fn make_text_value(text: &str, underline_len: Option<u32>) -> zbus::Result<OwnedValue> {
    let t = match underline_len {
        Some(len) => IBusText::with_underline(text, len),
        None => IBusText::new(text),
    }
    .map_err(zbus::Error::from)?;
    t.into_value().map_err(zbus::Error::from)
}

/// Factory của IBus, được export trên `org.freedesktop.IBus.Factory`. Tạo đối tượng engine khi cần.
pub struct Factory {
    counter: Arc<AtomicU32>,
}

impl Factory {
    pub fn new() -> Factory {
        Factory {
            counter: Arc::new(AtomicU32::new(0)),
        }
    }
}

impl Default for Factory {
    fn default() -> Self {
        Factory::new()
    }
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl Factory {
    async fn create_engine(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        engine_name: String,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        let path = format!("/org/freedesktop/IBus/Engine/{n}");
        let config = load_config(&engine_name);
        let handle = EngineHandle::spawn(config);
        let engine = PinaKeyEngine { handle };
        let opath =
            OwnedObjectPath::try_from(path).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        server.at(&opath, engine).await?;
        Ok(opath)
    }
}

/// Chạy engine như một IBus component nhúng (`pinakey --ibus`): kết nối tới bus IBus, đăng ký tên
/// component, export factory, rồi phục vụ vô thời hạn.
pub async fn run_embedded() -> zbus::Result<()> {
    let address = crate::address::ibus_address()
        .map_err(|e| zbus::Error::Address(format!("cannot find IBus address: {e}")))?;
    let conn = connection::Builder::address(address.as_str())?
        .build()
        .await?;
    conn.request_name(COMPONENT_NAME).await?;
    conn.object_server()
        .at("/org/freedesktop/IBus/Factory", Factory::new())
        .await?;
    std::future::pending::<()>().await;
    Ok(())
}
