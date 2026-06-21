//! C-ABI cho lõi PinaKey.
//!
//! Đây là cầu nối để addon **fcitx5 C++** dùng lại nguyên lõi engine Rust (`pinakey-engine`) —
//! đúng mô hình của [`fcitx5-cskk`](https://github.com/fcitx/fcitx5-cskk): C++ giữ một **con trỏ
//! mờ** (`PkEngine*`), bơm sự kiện phím vào, rồi đọc kết quả ra.
//!
//! ## Quy ước bộ nhớ (quan trọng)
//! Mọi chuỗi trả về từ các hàm getter (`pk_engine_commit`, `pk_engine_preedit`, …) đều **được sở
//! hữu bởi `PkEngine`** và chỉ có hiệu lực **tới lần gọi `pk_engine_process_key`/`pk_engine_reset`
//! kế tiếp hoặc tới khi `pk_engine_free`**. Bên C++ phải sao chép ngay (`std::string`) sau khi gọi.
//! Nhờ vậy KHÔNG có chuỗi nào phải free xuyên biên giới FFI — loại bỏ cả một lớp lỗi bộ nhớ.
//!
//! Keysym/modifier dùng giá trị X11 (xem [`pinakey_engine::keysym`]) — trùng với fcitx5, nên C++
//! truyền thẳng `keyEvent.rawKey().sym()` và `states()` mà không cần ánh xạ. Khi phím được *nhả*
//! (release), C++ bật bit [`pinakey_engine::keysym::MOD_RELEASE`] (`1<<30`) trong `state`.

use std::ffi::{c_char, CStr, CString};

use pinakey_config::{default_cfg, load_config, Config};
use pinakey_engine::{Action, EngineCore};

/// Trạng thái engine cho một input context fcitx5. Con trỏ mờ phía C.
pub struct PkEngine {
    core: EngineCore,
    // Bộ đệm kết quả của lần process_key gần nhất (sở hữu chuỗi; con trỏ trao cho C trỏ vào đây).
    commit: CString,
    preedit: CString,
    preedit_cursor: u32,
    preedit_visible: bool,
    preedit_underline: bool,
    // Bản sao NUL-terminated của tên kiểu gõ, để getter trả con trỏ C hợp lệ (String của Rust
    // không kết thúc bằng NUL nên không thể trả thẳng `.as_ptr()`).
    im_name: CString,
}

impl PkEngine {
    fn from_config(config: Config) -> Box<PkEngine> {
        let im_name = to_cstring(&config.input_method);
        Box::new(PkEngine {
            core: EngineCore::new(config),
            commit: CString::default(),
            preedit: CString::default(),
            preedit_cursor: 0,
            preedit_visible: false,
            preedit_underline: false,
            im_name,
        })
    }

    /// Gộp danh sách [`Action`] (sinh ra cho MỘT phím) thành trạng thái commit + preedit phẳng mà
    /// frontend fcitx5 cần. fcitx5 không có khái niệm "auxiliary"/"lookup table" theo kiểu signal
    /// của IBus, nên các action đó được bỏ qua ở MVP (bảng emoji xử lý riêng ở lớp C++).
    fn apply(&mut self, actions: Vec<Action>) {
        let mut commit = String::new();
        // None = phím này không đụng tới preedit (giữ nguyên trạng thái cũ).
        let mut new_preedit: Option<(String, u32, bool)> = None;
        let mut hide = false;
        for action in actions {
            match action {
                Action::CommitText(s) => commit.push_str(&s),
                Action::UpdatePreedit {
                    text,
                    cursor,
                    underline,
                } => {
                    new_preedit = Some((text, cursor, underline));
                    hide = false;
                }
                Action::HidePreedit => {
                    hide = true;
                    new_preedit = None;
                }
                Action::UpdateAuxiliary { .. }
                | Action::HideAuxiliary
                | Action::HideLookupTable => {}
            }
        }
        self.commit = to_cstring(&commit);
        if let Some((text, cursor, underline)) = new_preedit {
            self.preedit = to_cstring(&text);
            self.preedit_cursor = cursor;
            self.preedit_underline = underline;
            self.preedit_visible = !text_is_empty(&self.preedit);
        } else if hide {
            self.preedit = CString::default();
            self.preedit_cursor = 0;
            self.preedit_visible = false;
        }
        // else: giữ nguyên preedit (phím không xử lý / không đổi preedit).
    }
}

fn text_is_empty(c: &CStr) -> bool {
    c.to_bytes().is_empty()
}

/// Chuyển sang `CString`, thay NUL nội bộ bằng rỗng (văn bản tiếng Việt không chứa NUL nên thực tế
/// không xảy ra; đây chỉ là lớp phòng vệ để không bao giờ panic).
fn to_cstring(s: &str) -> CString {
    CString::new(s).unwrap_or_default()
}

/// # Safety
/// `ptr` phải là con trỏ C hợp lệ trỏ tới chuỗi NUL-terminated, hoặc null.
unsafe fn opt_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

// ------------------------------------------------------------------------------------------------
// Vòng đời
// ------------------------------------------------------------------------------------------------

/// Tạo engine với cấu hình mặc định (Telex, Unicode, cờ chuẩn).
#[no_mangle]
pub extern "C" fn pk_engine_new() -> *mut PkEngine {
    Box::into_raw(PkEngine::from_config(default_cfg()))
}

/// Tạo engine, nạp cấu hình người dùng theo `engine_name` (file
/// `~/.config/pinakey/ibus-<name>.config.json`); thiếu file thì dùng mặc định.
///
/// # Safety
/// `name` là chuỗi C NUL-terminated hợp lệ hoặc null.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_new_from_name(name: *const c_char) -> *mut PkEngine {
    let cfg = match opt_str(name) {
        Some(n) if !n.is_empty() => load_config(n),
        _ => default_cfg(),
    };
    Box::into_raw(PkEngine::from_config(cfg))
}

/// Tạo engine từ chuỗi JSON cấu hình; null hoặc JSON sai → cấu hình mặc định.
///
/// # Safety
/// `json` là chuỗi C NUL-terminated hợp lệ hoặc null.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_new_from_json(json: *const c_char) -> *mut PkEngine {
    let cfg = opt_str(json)
        .and_then(|s| serde_json::from_str::<Config>(s).ok())
        .unwrap_or_else(default_cfg);
    Box::into_raw(PkEngine::from_config(cfg))
}

/// Giải phóng engine. Sau lời gọi này, mọi con trỏ chuỗi đã lấy ra đều không còn hợp lệ.
///
/// # Safety
/// `e` phải do `pk_engine_new*` trả về và chưa từng được free; hoặc null (không làm gì).
#[no_mangle]
pub unsafe extern "C" fn pk_engine_free(e: *mut PkEngine) {
    if !e.is_null() {
        drop(Box::from_raw(e));
    }
}

// ------------------------------------------------------------------------------------------------
// Xử lý phím
// ------------------------------------------------------------------------------------------------

/// Xử lý một sự kiện phím. `keyval` là keysym X11/fcitx5, `state` là mặt nạ modifier (bật bit
/// `MOD_RELEASE = 1<<30` nếu là phím nhả). Trả về `true` nếu engine đã "nuốt" phím (C++ gọi
/// `keyEvent.filterAndAccept()`); `false` thì C++ để phím đi tiếp.
///
/// Sau khi gọi, đọc kết quả qua `pk_engine_commit` / `pk_engine_preedit*`.
///
/// # Safety
/// `e` phải là con trỏ engine hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_process_key(
    e: *mut PkEngine,
    keyval: u32,
    state: u32,
) -> bool {
    let Some(engine) = e.as_mut() else {
        return false;
    };
    let (handled, actions) = engine.core.process_key_event(keyval, 0, state);
    engine.apply(actions);
    handled
}

/// Chuỗi cần commit từ lần `process_key` gần nhất (rỗng nếu không có gì để commit).
///
/// # Safety
/// `e` hợp lệ; con trỏ trả về chỉ dùng được tới lần `process_key`/`reset`/`free` kế tiếp.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_commit(e: *const PkEngine) -> *const c_char {
    match e.as_ref() {
        Some(engine) => engine.commit.as_ptr(),
        None => c"".as_ptr(),
    }
}

/// Văn bản preedit hiện tại (rỗng nếu không hiển thị preedit).
///
/// # Safety
/// Như `pk_engine_commit`.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_preedit(e: *const PkEngine) -> *const c_char {
    match e.as_ref() {
        Some(engine) => engine.preedit.as_ptr(),
        None => c"".as_ptr(),
    }
}

/// Vị trí con trỏ trong preedit (số ký tự).
///
/// # Safety
/// `e` hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_preedit_cursor(e: *const PkEngine) -> u32 {
    e.as_ref().map(|x| x.preedit_cursor).unwrap_or(0)
}

/// Preedit có nên hiển thị không.
///
/// # Safety
/// `e` hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_preedit_visible(e: *const PkEngine) -> bool {
    e.as_ref().map(|x| x.preedit_visible).unwrap_or(false)
}

/// Preedit có nên gạch chân không (theo cờ IB_NO_UNDERLINE của người dùng).
///
/// # Safety
/// `e` hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_preedit_underline(e: *const PkEngine) -> bool {
    e.as_ref().map(|x| x.preedit_underline).unwrap_or(false)
}

// ------------------------------------------------------------------------------------------------
// Điều khiển trạng thái
// ------------------------------------------------------------------------------------------------

/// Đặt lại buffer soạn thảo (tương ứng `reset()` của fcitx5 khi đổi focus/huỷ).
///
/// # Safety
/// `e` hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_reset(e: *mut PkEngine) {
    if let Some(engine) = e.as_mut() {
        engine.core.reset_preeditor();
        engine.commit = CString::default();
        engine.preedit = CString::default();
        engine.preedit_cursor = 0;
        engine.preedit_visible = false;
    }
}

/// Đặt tên chương trình của input context (vd `firefox`) để bật cách khắc phục theo ứng dụng.
///
/// # Safety
/// `e` hợp lệ; `program` là chuỗi C hợp lệ hoặc null.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_set_program(e: *mut PkEngine, program: *const c_char) {
    if let Some(engine) = e.as_mut() {
        engine.core.set_wm_class(opt_str(program).unwrap_or("").to_string());
    }
}

/// Đổi kiểu gõ ("Telex" / "VNI" / "VIQR" …) và dựng lại engine biến đổi.
///
/// # Safety
/// `e` hợp lệ; `name` là chuỗi C hợp lệ hoặc null (null = không đổi).
#[no_mangle]
pub unsafe extern "C" fn pk_engine_set_input_method(e: *mut PkEngine, name: *const c_char) {
    if let (Some(engine), Some(n)) = (e.as_mut(), opt_str(name)) {
        engine.core.config.input_method = n.to_string();
        engine.im_name = to_cstring(n);
        engine.core.rebuild_preeditor();
        engine.core.reset_preeditor();
    }
}

/// Đổi bảng mã đầu ra ("Unicode", "TCVN3", …).
///
/// # Safety
/// `e` hợp lệ; `name` là chuỗi C hợp lệ hoặc null (null = không đổi).
#[no_mangle]
pub unsafe extern "C" fn pk_engine_set_charset(e: *mut PkEngine, name: *const c_char) {
    if let (Some(engine), Some(n)) = (e.as_mut(), opt_str(name)) {
        engine.core.config.output_charset = n.to_string();
    }
}

/// Tên kiểu gõ hiện tại.
///
/// # Safety
/// `e` hợp lệ; con trỏ trả về hợp lệ tới lần đổi cấu hình kế tiếp.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_input_method(e: *const PkEngine) -> *const c_char {
    match e.as_ref() {
        Some(engine) => engine.im_name.as_ptr(),
        None => c"".as_ptr(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gõ một chuỗi phím in được qua C-ABI; trả về (commit gộp, preedit cuối).
    unsafe fn type_str(e: *mut PkEngine, s: &str) -> (String, String) {
        let mut commit = String::new();
        for c in s.chars() {
            pk_engine_process_key(e, c as u32, 0);
            let cm = CStr::from_ptr(pk_engine_commit(e)).to_str().unwrap();
            commit.push_str(cm);
        }
        let pe = CStr::from_ptr(pk_engine_preedit(e)).to_str().unwrap().to_string();
        (commit, pe)
    }

    #[test]
    fn telex_preedit_via_ffi() {
        unsafe {
            let e = pk_engine_new();
            let (_commit, preedit) = type_str(e, "vieetj");
            assert_eq!(preedit, "việt");
            assert!(pk_engine_preedit_visible(e));
            pk_engine_free(e);
        }
    }

    #[test]
    fn word_break_commits_via_ffi() {
        unsafe {
            let e = pk_engine_new();
            let (mut commit, _p) = type_str(e, "tieengs");
            // space commit từ
            pk_engine_process_key(e, 0x20, 0);
            commit.push_str(CStr::from_ptr(pk_engine_commit(e)).to_str().unwrap());
            assert!(commit.contains("tiếng"), "commit was {commit:?}");
            // preedit phải trống sau khi commit
            assert!(!pk_engine_preedit_visible(e));
            pk_engine_free(e);
        }
    }

    #[test]
    fn reset_clears_buffer() {
        unsafe {
            let e = pk_engine_new();
            type_str(e, "vieet");
            pk_engine_reset(e);
            assert_eq!(CStr::from_ptr(pk_engine_preedit(e)).to_bytes(), b"");
            pk_engine_free(e);
        }
    }

    #[test]
    fn switch_to_vni() {
        unsafe {
            let e = pk_engine_new();
            let name = CString::new("VNI").unwrap();
            pk_engine_set_input_method(e, name.as_ptr());
            // VNI: "viet65" -> "việt" (6=mũ? ) dùng chuỗi VNI cho "việt": v i e6 t -> ... an toàn:
            // chỉ kiểm tra getter trả đúng tên và engine vẫn gõ ra tiếng Việt.
            assert_eq!(CStr::from_ptr(pk_engine_input_method(e)).to_str().unwrap(), "VNI");
            let (_c, preedit) = type_str(e, "a1"); // VNI: 1 = dấu sắc -> "á"
            assert_eq!(preedit, "á", "VNI preedit was {preedit:?}");
            pk_engine_free(e);
        }
    }
}
