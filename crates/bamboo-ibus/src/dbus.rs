//! D-Bus transport for the IBus engine — ported from goibus (`bus.go`, `factory.go`, `engine.go`)
//! and `main.go`, implemented over zbus.
//!
//! Note: this layer requires a live IBus daemon + D-Bus and therefore cannot be exercised by the
//! crate's unit tests; the pure behaviour is tested in `core`. It is a faithful, compiling port of
//! the protocol surface goibus exposes for the Preedit input mode.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use zbus::object_server::{ObjectServer, SignalEmitter};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::{connection, interface};

use bamboo_config::load_config;

use crate::constants::*;
use crate::core::Action;
use crate::engine_actor::EngineHandle;
use crate::serialize::IBusText;

/// The IBus engine object, exported on `org.freedesktop.IBus.Engine`.
pub struct BambooEngine {
    handle: EngineHandle,
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl BambooEngine {
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

    async fn focus_in(&self) {
        // Detect the focused window's class for per-application workarounds (X11 only; no-op
        // elsewhere), mirroring the Go engine's FocusIn -> checkWmClass.
        if let Some(class) = bamboo_platform::get_focus_window_class() {
            self.handle.set_wm_class(class);
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
    async fn property_activate(&self, _name: String, _state: u32) {}
    async fn property_show(&self, _name: String) {}
    async fn property_hide(&self, _name: String) {}
    async fn page_up(&self) {}
    async fn page_down(&self) {}
    async fn cursor_up(&self) {}
    async fn cursor_down(&self) {}
    async fn candidate_clicked(&self, _index: u32, _button: u32, _state: u32) {}
    async fn destroy(&self) {}

    // ----- signals (org.freedesktop.IBus.Engine) -----

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
    async fn forward_key_event(
        emitter: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn require_surrounding_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

async fn apply_actions(emitter: &SignalEmitter<'_>, actions: &[Action]) -> zbus::Result<()> {
    for action in actions {
        match action {
            Action::CommitText(s) => {
                let v = make_text_value(s, None)?;
                BambooEngine::commit_text(emitter, v.into()).await?;
            }
            Action::UpdatePreedit {
                text,
                cursor,
                underline,
            } => {
                let v = make_text_value(text, if *underline { Some(*cursor) } else { None })?;
                BambooEngine::update_preedit_text(
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
                BambooEngine::update_auxiliary_text(emitter, v.into(), *visible).await?;
            }
            Action::HidePreedit => BambooEngine::hide_preedit_text(emitter).await?,
            Action::HideAuxiliary => BambooEngine::hide_auxiliary_text(emitter).await?,
            Action::HideLookupTable => BambooEngine::hide_lookup_table(emitter).await?,
        }
    }
    Ok(())
}

fn make_text_value(text: &str, underline_len: Option<u32>) -> zbus::Result<OwnedValue> {
    let t = match underline_len {
        Some(len) => IBusText::with_underline(text, len),
        None => IBusText::new(text),
    }
    .map_err(zbus::Error::from)?;
    t.into_value().map_err(zbus::Error::from)
}

/// The IBus factory, exported on `org.freedesktop.IBus.Factory`. Creates engine objects on demand.
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
        let engine = BambooEngine { handle };
        let opath = OwnedObjectPath::try_from(path)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        server.at(&opath, engine).await?;
        Ok(opath)
    }
}

/// Run the engine as an embedded IBus component (`ibus-bamboo --ibus`): connect to the IBus bus,
/// claim the component name, and export the factory, then serve forever.
pub async fn run_embedded() -> zbus::Result<()> {
    let address = crate::address::ibus_address()
        .map_err(|e| zbus::Error::Address(format!("cannot find IBus address: {e}")))?;
    let conn = connection::Builder::address(address.as_str())?.build().await?;
    conn.request_name(COMPONENT_NAME).await?;
    conn.object_server()
        .at("/org/freedesktop/IBus/Factory", Factory::new())
        .await?;
    std::future::pending::<()>().await;
    Ok(())
}
