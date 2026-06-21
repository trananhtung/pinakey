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

use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use std::sync::OnceLock;

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
    // Bản sao NUL-terminated của tên kiểu gõ + bảng mã, để getter trả con trỏ C hợp lệ (String của
    // Rust không kết thúc bằng NUL nên không thể trả thẳng `.as_ptr()`).
    im_name: CString,
    charset_name: CString,

    // ----- trạng thái cho chế độ "gõ không gạch chân" (diff-and-replace) -----
    // Chuỗi mà segment hiện tại ĐANG hiển thị trong tài liệu (phần đang soạn, chưa cố định).
    prev_displayed: String,
    // Kết quả của lần process_key_replace gần nhất: xoá `replace_delete` ký tự cuối rồi chèn
    // `replace_insert`. C++ thực hiện qua deleteSurroundingText(-n, n) + commitString.
    replace_delete: u32,
    replace_insert: CString,
}

impl PkEngine {
    fn from_config(config: Config) -> Box<PkEngine> {
        let im_name = to_cstring(&config.input_method);
        let charset_name = to_cstring(&config.output_charset);
        Box::new(PkEngine {
            core: EngineCore::new(config),
            commit: CString::default(),
            preedit: CString::default(),
            preedit_cursor: 0,
            preedit_visible: false,
            preedit_underline: false,
            im_name,
            charset_name,
            prev_displayed: String::new(),
            replace_delete: 0,
            replace_insert: CString::default(),
        })
    }

    /// Chế độ preedit cổ điển: gộp [`Action`] thành trạng thái commit + preedit phẳng.
    fn apply(&mut self, actions: Vec<Action>) {
        let folded = fold_actions(actions);
        self.commit = to_cstring(&folded.commit);
        if let Some((text, cursor, underline)) = folded.preedit {
            self.preedit = to_cstring(&text);
            self.preedit_cursor = cursor;
            self.preedit_underline = underline;
            self.preedit_visible = !text_is_empty(&self.preedit);
        } else if folded.hide {
            self.preedit = CString::default();
            self.preedit_cursor = 0;
            self.preedit_visible = false;
        }
        // else: giữ nguyên preedit (phím không xử lý / không đổi preedit).
    }

    /// Chế độ "gõ không gạch chân": tính lệnh thay thế (xoá N ký tự cuối + chèn chuỗi mới) bằng
    /// cách so phần tiền tố chung giữa chuỗi đang hiển thị và chuỗi mong muốn mới. Đây chính là cốt
    /// lõi của việc commit trực tiếp thay vì hiện preedit (đối chiếu fcitx5-lotus `compareAndSplit`).
    fn apply_replace(&mut self, actions: Vec<Action>) {
        let folded = fold_actions(actions);
        let preedit = folded.preedit.map(|(t, _, _)| t).unwrap_or_default();
        // Chuỗi mong muốn cho segment = (phần vừa cố định) + (phần còn đang soạn).
        // commit đã là văn bản hoàn tất (gồm cả ký tự ngắt từ); preedit là phần còn soạn dở.
        let new_displayed = format!("{}{}", folded.commit, preedit);
        let (delete, insert) = diff_replace(&self.prev_displayed, &new_displayed);
        self.replace_delete = delete;
        self.replace_insert = to_cstring(&insert);
        // Phần commit nay là cố định trong tài liệu; chỉ còn phần preedit là segment đang theo dõi.
        self.prev_displayed = preedit;
    }
}

/// Kết quả gộp các [`Action`] của một phím.
struct Folded {
    commit: String,
    /// `Some` nếu phím cập nhật preedit; `None` nghĩa là không đụng preedit.
    preedit: Option<(String, u32, bool)>,
    /// `true` nếu phím yêu cầu ẩn preedit.
    hide: bool,
}

fn fold_actions(actions: Vec<Action>) -> Folded {
    let mut commit = String::new();
    let mut preedit: Option<(String, u32, bool)> = None;
    let mut hide = false;
    for action in actions {
        match action {
            Action::CommitText(s) => commit.push_str(&s),
            Action::UpdatePreedit {
                text,
                cursor,
                underline,
            } => {
                preedit = Some((text, cursor, underline));
                hide = false;
            }
            Action::HidePreedit => {
                hide = true;
                preedit = None;
            }
            Action::UpdateAuxiliary { .. } | Action::HideAuxiliary | Action::HideLookupTable => {}
        }
    }
    Folded {
        commit,
        preedit,
        hide,
    }
}

/// So tiền tố chung (theo ký tự Unicode) giữa chuỗi cũ và mới; trả về `(số ký tự cuối cần xoá,
/// chuỗi cần chèn)`. Ví dụ `("vie", "viê") -> (1, "ê")`, `("tiếng", "tiếng ") -> (0, " ")`.
fn diff_replace(old: &str, new: &str) -> (u32, String) {
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    let mut i = 0;
    while i < old_chars.len() && i < new_chars.len() && old_chars[i] == new_chars[i] {
        i += 1;
    }
    let delete = (old_chars.len() - i) as u32;
    let insert: String = new_chars[i..].iter().collect();
    (delete, insert)
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
pub unsafe extern "C" fn pk_engine_process_key(e: *mut PkEngine, keyval: u32, state: u32) -> bool {
    let Some(engine) = e.as_mut() else {
        return false;
    };
    let (handled, actions) = engine.core.process_key_event(keyval, 0, state);
    engine.apply(actions);
    handled
}

/// Xử lý phím cho chế độ **gõ không gạch chân**: thay vì preedit, trả về một lệnh thay thế. Sau khi
/// gọi, C++ đọc `pk_engine_replace_delete` (số ký tự cuối cần xoá) và `pk_engine_replace_insert`
/// (chuỗi cần chèn) rồi áp bằng `deleteSurroundingText(-n, n)` + `commitString`. Trả về `handled`.
///
/// # Safety
/// `e` phải là con trỏ engine hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_process_key_replace(
    e: *mut PkEngine,
    keyval: u32,
    state: u32,
) -> bool {
    let Some(engine) = e.as_mut() else {
        return false;
    };
    let (handled, actions) = engine.core.process_key_event(keyval, 0, state);
    engine.apply_replace(actions);
    handled
}

/// Số ký tự (Unicode) ở cuối cần xoá khỏi tài liệu cho lần `process_key_replace` gần nhất.
///
/// # Safety
/// `e` hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_replace_delete(e: *const PkEngine) -> u32 {
    e.as_ref().map(|x| x.replace_delete).unwrap_or(0)
}

/// Chuỗi cần chèn (commit) cho lần `process_key_replace` gần nhất.
///
/// # Safety
/// `e` hợp lệ; con trỏ trả về dùng được tới lần gọi kế tiếp.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_replace_insert(e: *const PkEngine) -> *const c_char {
    match e.as_ref() {
        Some(engine) => engine.replace_insert.as_ptr(),
        None => c"".as_ptr(),
    }
}

/// Người dùng có bật chế độ "gõ không gạch chân" không (cờ IB_NO_UNDERLINE). C++ dùng cờ này (cùng
/// với khả năng SurroundingText của ứng dụng) để chọn giữa chế độ replace và preedit.
///
/// # Safety
/// `e` hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_no_underline(e: *const PkEngine) -> bool {
    e.as_ref()
        .map(|x| x.core.config.ib_flags & pinakey_config::flags::IB_NO_UNDERLINE != 0)
        .unwrap_or(false)
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

/// Engine có đang soạn dở một segment không (preedit hiển thị, hoặc đang theo dõi đoạn ở chế độ
/// không-gạch-chân). C++ dùng để biết có nên kích hoạt tra emoji bằng `:` hay không (issue #11/#26).
///
/// # Safety
/// `e` hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_is_composing(e: *const PkEngine) -> bool {
    e.as_ref()
        .map(|x| x.preedit_visible || !x.prev_displayed.is_empty())
        .unwrap_or(false)
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

/// Nạp lại file macro + từ điển từ đĩa (issue #20, live-reload) mà không đổi cấu hình đang chạy.
///
/// # Safety
/// `e` hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_reload(e: *mut PkEngine) {
    if let Some(engine) = e.as_mut() {
        engine.core.reload_data();
    }
}

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
        // Quên segment đang theo dõi: sau reset (đổi focus, click chuột…) tài liệu không còn liên
        // quan tới buffer cũ, nên chế độ không-gạch-chân phải bắt đầu lại từ rỗng.
        engine.prev_displayed.clear();
        engine.replace_delete = 0;
        engine.replace_insert = CString::default();
    }
}

/// Kết thúc phiên soạn khi mất focus (issue #6): trả về phần preedit đang hiển thị để C++ commit
/// (tránh kẹt/mất chữ), rồi reset engine. Dùng cho chế độ preedit; ở chế độ gõ-không-gạch-chân
/// văn bản đã nằm sẵn trong tài liệu nên C++ chỉ gọi `pk_engine_reset`.
///
/// # Safety
/// `e` hợp lệ; con trỏ trả về dùng được tới lần gọi kế tiếp.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_flush_preedit(e: *mut PkEngine) -> *const c_char {
    let Some(engine) = e.as_mut() else {
        return c"".as_ptr();
    };
    // Dùng lại ô `commit` làm nơi giữ chuỗi trả về (hợp lệ tới lần gọi kế tiếp).
    engine.commit = engine.preedit.clone();
    let ptr = engine.commit.as_ptr();
    engine.core.reset_preeditor();
    engine.preedit = CString::default();
    engine.preedit_cursor = 0;
    engine.preedit_visible = false;
    engine.prev_displayed.clear();
    engine.replace_delete = 0;
    engine.replace_insert = CString::default();
    ptr
}

/// Đặt tên chương trình của input context (vd `firefox`) để bật cách khắc phục theo ứng dụng.
///
/// # Safety
/// `e` hợp lệ; `program` là chuỗi C hợp lệ hoặc null.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_set_program(e: *mut PkEngine, program: *const c_char) {
    if let Some(engine) = e.as_mut() {
        engine
            .core
            .set_wm_class(opt_str(program).unwrap_or("").to_string());
    }
}

/// Chương trình đang focus (đặt qua `pk_engine_set_program`) có nằm trong danh sách loại trừ tiếng
/// Anh không (issue #9). C++ dùng cờ này để cho phím đi thẳng (pass-through), không gõ tiếng Việt.
///
/// # Safety
/// `e` hợp lệ.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_program_excluded(e: *const PkEngine) -> bool {
    e.as_ref()
        .map(|x| x.core.is_program_excluded())
        .unwrap_or(false)
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
        engine.charset_name = to_cstring(n);
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

/// Tên bảng mã hiện tại.
///
/// # Safety
/// `e` hợp lệ; con trỏ trả về hợp lệ tới lần đổi cấu hình kế tiếp.
#[no_mangle]
pub unsafe extern "C" fn pk_engine_charset(e: *const PkEngine) -> *const c_char {
    match e.as_ref() {
        Some(engine) => engine.charset_name.as_ptr(),
        None => c"".as_ptr(),
    }
}

// ------------------------------------------------------------------------------------------------
// Liệt kê kiểu gõ / bảng mã (cho menu fcitx5, issue #12/#17). Danh sách tĩnh, cache một lần;
// con trỏ trả về sống suốt vòng đời tiến trình.
// ------------------------------------------------------------------------------------------------

static IM_NAMES: OnceLock<Vec<CString>> = OnceLock::new();
static CHARSET_NAMES: OnceLock<Vec<CString>> = OnceLock::new();

fn im_names() -> &'static [CString] {
    IM_NAMES.get_or_init(|| {
        pinakey_core::input_method_definitions()
            .iter()
            .map(|(n, _)| to_cstring(n))
            .collect()
    })
}

fn charset_names() -> &'static [CString] {
    CHARSET_NAMES.get_or_init(|| {
        pinakey_core::get_charset_names()
            .iter()
            .map(|s| to_cstring(s))
            .collect()
    })
}

/// Số kiểu gõ dựng sẵn.
#[no_mangle]
pub extern "C" fn pk_input_method_count() -> u32 {
    im_names().len() as u32
}

/// Tên kiểu gõ thứ `i` (rỗng nếu ngoài phạm vi).
///
/// # Safety
/// Con trỏ trả về sống suốt vòng đời tiến trình.
#[no_mangle]
pub unsafe extern "C" fn pk_input_method_name_at(i: u32) -> *const c_char {
    im_names()
        .get(i as usize)
        .map(|c| c.as_ptr())
        .unwrap_or(c"".as_ptr())
}

/// Số bảng mã đầu ra.
#[no_mangle]
pub extern "C" fn pk_charset_count() -> u32 {
    charset_names().len() as u32
}

/// Tên bảng mã thứ `i` (rỗng nếu ngoài phạm vi).
///
/// # Safety
/// Con trỏ trả về sống suốt vòng đời tiến trình.
#[no_mangle]
pub unsafe extern "C" fn pk_charset_name_at(i: u32) -> *const c_char {
    charset_names()
        .get(i as usize)
        .map(|c| c.as_ptr())
        .unwrap_or(c"".as_ptr())
}

// ------------------------------------------------------------------------------------------------
// Tra cứu emoji (issue #11/#26). Trie EmojiOne đóng kèm binary; truy vấn theo tiền tố keyword/ascii.
// ------------------------------------------------------------------------------------------------

static EMOJI_TRIE: OnceLock<pinakey_emoji::TrieNode> = OnceLock::new();

fn emoji_trie() -> &'static pinakey_emoji::TrieNode {
    EMOJI_TRIE.get_or_init(|| {
        pinakey_emoji::load_emojione_from_str(include_str!(
            "../../pinakey-emoji/data/emojione.json"
        ))
        .unwrap_or_default()
    })
}

thread_local! {
    static EMOJI_RESULT: RefCell<CString> = RefCell::new(CString::default());
}

/// Tra emoji theo `query` (tiền tố keyword như "smile" hoặc ascii như ":)"). Trả về danh sách emoji
/// khớp, mỗi dòng một emoji, phân tách bằng `\n` (tối đa 60). Con trỏ trả về hợp lệ tới lần gọi
/// `pk_emoji_query` kế tiếp TRÊN CÙNG THREAD; C++ phải sao chép ngay.
///
/// # Safety
/// `query` là chuỗi C hợp lệ hoặc null.
#[no_mangle]
pub unsafe extern "C" fn pk_emoji_query(query: *const c_char) -> *const c_char {
    let q = opt_str(query).unwrap_or("");
    let engine = pinakey_emoji::EmojiEngine::new(emoji_trie());
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for emoji in engine.filter(q) {
        if seen.insert(emoji.clone()) {
            out.push(emoji);
            if out.len() >= 60 {
                break;
            }
        }
    }
    let joined = to_cstring(&out.join("\n"));
    let ptr = joined.as_ptr();
    EMOJI_RESULT.with(|cell| *cell.borrow_mut() = joined);
    ptr
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
        let pe = CStr::from_ptr(pk_engine_preedit(e))
            .to_str()
            .unwrap()
            .to_string();
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
            assert_eq!(
                CStr::from_ptr(pk_engine_input_method(e)).to_str().unwrap(),
                "VNI"
            );
            let (_c, preedit) = type_str(e, "a1"); // VNI: 1 = dấu sắc -> "á"
            assert_eq!(preedit, "á", "VNI preedit was {preedit:?}");
            pk_engine_free(e);
        }
    }

    // ----- chế độ "gõ không gạch chân" (diff-and-replace) -----

    /// Gõ qua chế độ replace, mô phỏng tài liệu: mỗi phím xoá `delete` ký tự cuối rồi chèn chuỗi.
    /// Trả về nội dung tài liệu cuối cùng — đúng thứ người dùng nhìn thấy.
    unsafe fn type_replace(e: *mut PkEngine, s: &str) -> String {
        let mut doc = String::new();
        for c in s.chars() {
            pk_engine_process_key_replace(e, c as u32, 0);
            let del = pk_engine_replace_delete(e);
            for _ in 0..del {
                doc.pop();
            }
            let ins = CStr::from_ptr(pk_engine_replace_insert(e))
                .to_str()
                .unwrap();
            doc.push_str(ins);
        }
        doc
    }

    #[test]
    fn diff_replace_basics() {
        assert_eq!(diff_replace("vie", "viê"), (1, "ê".to_string()));
        assert_eq!(diff_replace("tiếng", "tiếng "), (0, " ".to_string()));
        assert_eq!(diff_replace("", "abc"), (0, "abc".to_string()));
        assert_eq!(diff_replace("abc", "ab"), (1, String::new()));
        assert_eq!(diff_replace("viêt", "việt"), (2, "ệt".to_string())); // ê t -> ệ t
        assert_eq!(diff_replace("xin", "xin"), (0, String::new()));
    }

    #[test]
    fn no_underline_types_vietnamese_directly() {
        unsafe {
            let e = pk_engine_new();
            // Không preedit: tài liệu nhận thẳng "việt" qua chuỗi xoá+chèn.
            assert_eq!(type_replace(e, "vieetj"), "việt");
            pk_engine_free(e);
        }
    }

    #[test]
    fn no_underline_word_break_and_multiword() {
        unsafe {
            let e = pk_engine_new();
            // "tieengs" -> "tiếng", space cố định + thêm dấu cách; rồi "vieetj" -> "việt".
            assert_eq!(type_replace(e, "tieengs vieetj"), "tiếng việt");
            pk_engine_free(e);
        }
    }

    #[test]
    fn no_underline_non_vietnamese_fallback() {
        unsafe {
            let e = pk_engine_new();
            assert_eq!(type_replace(e, "loz "), "loz ");
            pk_engine_free(e);
        }
    }

    #[test]
    fn program_excluded_via_config() {
        unsafe {
            let cfg = CString::new(r#"{"EnglishExclude":["konsole","code"]}"#).unwrap();
            let e = pk_engine_new_from_json(cfg.as_ptr());
            // chưa đặt program → không loại trừ
            assert!(!pk_engine_program_excluded(e));
            let p1 = CString::new("konsole").unwrap();
            pk_engine_set_program(e, p1.as_ptr());
            assert!(pk_engine_program_excluded(e));
            let p2 = CString::new("firefox").unwrap();
            pk_engine_set_program(e, p2.as_ptr());
            assert!(!pk_engine_program_excluded(e));
            // khớp chuỗi con: "code - oss" chứa "code"
            let p3 = CString::new("code - oss").unwrap();
            pk_engine_set_program(e, p3.as_ptr());
            assert!(pk_engine_program_excluded(e));
            pk_engine_free(e);
        }
    }

    #[test]
    fn emoji_query_and_enumeration() {
        unsafe {
            // emoji theo keyword
            let q = CString::new("grin").unwrap();
            let res = CStr::from_ptr(pk_emoji_query(q.as_ptr())).to_str().unwrap();
            assert!(res.contains('😀'), "kết quả grin: {res:?}");
            // liệt kê kiểu gõ + bảng mã có phần tử
            assert!(pk_input_method_count() >= 3);
            assert!(pk_charset_count() >= 1);
            let first_im = CStr::from_ptr(pk_input_method_name_at(0)).to_str().unwrap();
            assert!(!first_im.is_empty());
            let first_cs = CStr::from_ptr(pk_charset_name_at(0)).to_str().unwrap();
            assert_eq!(first_cs, "Unicode");
        }
    }

    #[test]
    fn flush_preedit_commits_and_resets() {
        unsafe {
            let e = pk_engine_new();
            let (_c, preedit) = type_str(e, "vieetj");
            assert_eq!(preedit, "việt");
            let flushed = CStr::from_ptr(pk_engine_flush_preedit(e)).to_str().unwrap();
            assert_eq!(flushed, "việt"); // trả về để C++ commit
                                         // sau flush, preedit trống (đã reset)
            assert!(!pk_engine_preedit_visible(e));
            assert_eq!(CStr::from_ptr(pk_engine_preedit(e)).to_bytes(), b"");
            pk_engine_free(e);
        }
    }

    #[test]
    fn no_underline_reset_forgets_segment() {
        unsafe {
            let e = pk_engine_new();
            type_replace(e, "vieet"); // đang soạn "viêt"
            pk_engine_reset(e);
            // Sau reset, gõ tiếp bắt đầu từ rỗng (không xoá nhầm chữ cũ trong tài liệu).
            pk_engine_process_key_replace(e, 'a' as u32, 0);
            assert_eq!(pk_engine_replace_delete(e), 0);
            assert_eq!(
                CStr::from_ptr(pk_engine_replace_insert(e))
                    .to_str()
                    .unwrap(),
                "a"
            );
            pk_engine_free(e);
        }
    }
}
