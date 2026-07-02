//! Logic engine thuần cho chế độ Preedit — chuyển thể từ `engine_preedit.go` và các hàm hỗ trợ
//! liên quan trong `engine_utils.go`.
//!
//! Phần này được tách hoàn toàn khỏi D-Bus: `process_key_event` trả về danh sách các [`Action`]
//! mà lớp transport cần thực hiện, nhờ vậy toàn bộ hành vi của bộ gõ có thể kiểm thử bằng unit
//! test mà không cần IBus daemon. Chế độ nhập mặc định (Preedit) đã được hiện thực; các chế độ sửa
//! lỗi bằng backspace, bảng emoji và hexa, phím tắt cùng lookup table được xây thêm ở các module
//! cấp cao hơn.

use pinakey_config::{flags as cfg, Config};
use pinakey_core::{
    self as core, build_input_method_from_pairs, has_any_vietnamese_rune, has_any_vietnamese_vowel,
    is_word_break_symbol, mode, IEngine, PinaKeyEngine,
};
use pinakey_emoji::MacroTable;

use crate::keysym::*;

/// Một tác động phụ (side effect) mà lớp transport phải thực hiện (tương ứng với các signal của
/// engine goibus dùng trong luồng preedit).
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    CommitText(String),
    UpdatePreedit {
        text: String,
        cursor: u32,
        underline: bool,
    },
    UpdateAuxiliary {
        text: String,
        visible: bool,
    },
    HidePreedit,
    HideAuxiliary,
    HideLookupTable,
}

pub struct EngineCore {
    pub preeditor: PinaKeyEngine,
    pub config: Config,
    pub macro_table: MacroTable,
    pub should_restore_key_strokes: bool,
    pub last_key_with_shift: bool,
    wm_class: String,
    /// Từ điển chính tả (chỉ nạp khi bật cờ `IB_SPELL_CHECK_WITH_DICTS`) — issue #18.
    dictionary: Option<core::Dictionary>,
    /// #65: trạng thái theo dõi kết câu cho tự-viết-hoa-đầu-câu (chỉ dùng khi bật cờ).
    sentence: SentenceState,
    /// #65: cửa sổ double-space→". " đang mở (vừa commit "từ " xong). Single-shot: phím kế
    /// tiếp bất kỳ (không bị addon tiêu thụ) sẽ đóng lại.
    double_space_armed: bool,
}

/// #65: máy trạng thái viết hoa đầu câu. Engine thấy MỌI phím (kể cả phím forward khi buffer
/// rỗng) nên có thể theo dõi "kết câu rồi + đã có khoảng trắng" thuần túy từ luồng phím/commit.
#[derive(Clone, Copy, PartialEq)]
enum SentenceState {
    /// Bình thường.
    Idle,
    /// Vừa commit chuỗi kết thúc bằng `.` `!` `?` — chờ khoảng trắng.
    AfterPunct,
    /// Đã có khoảng trắng sau dấu kết câu — chữ cái tiếp theo sẽ được viết hoa (one-shot).
    ReadyToCapitalize,
}

const VN_CASE_ALL_SMALL: u8 = 1;
const VN_CASE_ALL_CAPITAL: u8 = 2;
const VN_CASE_NO_CHANGE: u8 = 3;

/// Tên engine quy ước, dùng cho đường dẫn file macro `ibus-<name>.macro.text`.
const ENGINE_NAME: &str = "PinaKey";

impl EngineCore {
    pub fn new(config: Config) -> EngineCore {
        let preeditor = build_preeditor(&config);
        let mut macro_table = MacroTable::new(config.ib_flags & cfg::IB_AUTO_CAPITALIZE_MACRO != 0);
        // #7: nạp file gõ tắt (macro) khi người dùng bật IB_MACRO_ENABLED; thiếu file thì bỏ qua.
        if config.ib_flags & cfg::IB_MACRO_ENABLED != 0 {
            if let Some(path) = pinakey_config::get_macro_path(ENGINE_NAME).to_str() {
                let _ = macro_table.load_from_file(path);
            }
            macro_table.set_enabled(true);
        }
        let dictionary = load_dictionary(&config);
        EngineCore {
            preeditor,
            config,
            macro_table,
            should_restore_key_strokes: false,
            last_key_with_shift: false,
            wm_class: String::new(),
            dictionary,
            sentence: SentenceState::Idle,
            double_space_armed: false,
        }
    }

    /// #65: dấu cách kế tiếp có nên biến thành ". " không. Addon C++ hỏi TRƯỚC khi đưa phím
    /// space vào engine; nếu đúng và app cho phép xoá surrounding text, addon tự xoá dấu cách
    /// cũ + commit ". " rồi gọi [`Self::double_space_consume`].
    pub fn double_space_armed(&self) -> bool {
        self.double_space_armed
    }

    /// #65: addon đã thực hiện double-space→". ": đóng cửa sổ; văn bản giờ kết thúc ". " nên
    /// nếu bật viết-hoa-đầu-câu thì chữ cái kế tiếp được viết hoa luôn.
    pub fn double_space_consume(&mut self) {
        self.double_space_armed = false;
        if self.config.ib_flags & cfg::IB_CAPITALIZE_SENTENCE != 0 {
            self.sentence = SentenceState::ReadyToCapitalize;
        }
    }

    /// Từ điển có công nhận `word` không (chỉ khi cờ `IB_SPELL_CHECK_WITH_DICTS` bật và đã nạp).
    /// Từ điển chỉ "giải oan" cho từ hợp lệ mà quy tắc CVC từ chối; không loại bỏ từ đã chấp nhận.
    fn dict_accepts(&self, word: &str) -> bool {
        self.config.ib_flags & cfg::IB_SPELL_CHECK_WITH_DICTS != 0
            && self.dictionary.as_ref().is_some_and(|d| d.contains(word))
    }

    /// Truy vấn công khai: từ điển có công nhận `word` không (theo cờ + từ điển đã nạp). Issue #18.
    pub fn accepts_word(&self, word: &str) -> bool {
        self.dict_accepts(word)
    }

    pub fn set_wm_class(&mut self, wm_class: String) {
        self.wm_class = wm_class;
    }

    /// Chương trình đang focus có nằm trong danh sách loại trừ tiếng Anh không (issue #9).
    /// Khớp không phân biệt hoa/thường: bằng đúng hoặc là chuỗi con của wm_class.
    pub fn is_program_excluded(&self) -> bool {
        if self.wm_class.is_empty() {
            return false;
        }
        let w = self.wm_class.to_ascii_lowercase();
        self.config.english_exclude.iter().any(|p| {
            let p = p.trim().to_ascii_lowercase();
            !p.is_empty() && (w == p || w.contains(&p))
        })
    }

    /// Surrounding text của chương trình đang focus có KHÔNG đáng tin không (issue #66).
    /// LibreOffice/OpenOffice (soffice) báo surrounding text lạc hậu/thiếu dấu cách khi gõ nhanh,
    /// làm diff xoá-chèn sai vùng — với các app này addon phải rơi về preedit dù chúng quảng cáo
    /// khả năng SurroundingText. Khớp không phân biệt hoa/thường, theo chuỗi con (như danh sách
    /// loại trừ #9) để phủ mọi biến thể: soffice.bin, libreoffice-writer, org.libreoffice.…
    pub fn is_surrounding_text_unreliable(&self) -> bool {
        const BROKEN_SURROUNDING: [&str; 2] = ["soffice", "libreoffice"];
        if self.wm_class.is_empty() {
            return false;
        }
        let w = self.wm_class.to_ascii_lowercase();
        BROKEN_SURROUNDING.iter().any(|p| w.contains(p))
    }

    /// Đặt lại trạng thái soạn thảo bên dưới (tương ứng `Reset` của IBus).
    pub fn reset_preeditor(&mut self) {
        self.preeditor.reset();
        // Mất focus / con trỏ nhảy → không còn biết văn cảnh, huỷ viết hoa chờ + double-space.
        self.sentence = SentenceState::Idle;
        self.double_space_armed = false;
    }

    /// Dựng lại engine biến đổi sau khi cấu hình thay đổi (input method / flags).
    pub fn rebuild_preeditor(&mut self) {
        self.preeditor = build_preeditor(&self.config);
    }

    /// Nạp lại file macro + từ điển từ đĩa (issue #20, live-reload) — KHÔNG đụng tới cấu hình đang
    /// chạy (kiểu gõ/bảng mã/flags giữ nguyên). Gọi khi phát hiện file thay đổi.
    pub fn reload_data(&mut self) {
        if self.config.ib_flags & cfg::IB_MACRO_ENABLED != 0 {
            let mut mt = MacroTable::new(self.config.ib_flags & cfg::IB_AUTO_CAPITALIZE_MACRO != 0);
            if let Some(path) = pinakey_config::get_macro_path(ENGINE_NAME).to_str() {
                let _ = mt.load_from_file(path);
            }
            mt.set_enabled(true);
            self.macro_table = mt;
        }
        self.dictionary = load_dictionary(&self.config);
    }

    // ----- các hàm đọc (không có side effect) -----

    fn get_processed_string(&self, mode_flags: u32) -> String {
        self.preeditor.get_processed_string(mode_flags)
    }

    fn macro_enabled(&self) -> bool {
        self.config.ib_flags & cfg::IB_MACRO_ENABLED != 0
    }

    fn get_macro_text(&self) -> Option<String> {
        if !self.macro_enabled() {
            return None;
        }
        let text = self.get_processed_string(mode::PUNCTUATION);
        if self.macro_table.has_key(&text) {
            Some(self.expand_macro(&text))
        } else {
            None
        }
    }

    fn expand_macro(&self, s: &str) -> String {
        let macro_text = self.macro_table.get_text(s);
        // #64: thay $DATE/$TIME bằng ngày/giờ TẠI THỜI ĐIỂM KÍCH HOẠT, theo format trong config.
        // Mở rộng TRƯỚC biến đổi hoa/thường: nếu sau, ALL_SMALL hạ "$TIME" thành "$time" và
        // placeholder không bao giờ khớp.
        let macro_text = pinakey_emoji::expand_placeholders_now(
            &macro_text,
            &self.config.macro_date_format,
            &self.config.macro_time_format,
        );
        if self.config.ib_flags & cfg::IB_AUTO_CAPITALIZE_MACRO != 0 {
            match determine_macro_case(s) {
                VN_CASE_ALL_SMALL => return macro_text.to_lowercase(),
                VN_CASE_ALL_CAPITAL => return macro_text.to_uppercase(),
                _ => {}
            }
        }
        macro_text
    }

    fn get_input_mode(&self) -> u32 {
        if self.should_fallback_to_english(false) {
            mode::ENGLISH
        } else {
            mode::VIETNAMESE
        }
    }

    fn should_fallback_to_english(&self, check_vn_rune: bool) -> bool {
        if self.config.ib_flags & cfg::IB_AUTO_NON_VN_RESTORE == 0 {
            return false;
        }
        let vn_seq = self.get_processed_string(mode::VIETNAMESE | mode::LOWER_CASE);
        let vn_runes: Vec<char> = vn_seq.chars().collect();
        if vn_runes.is_empty() {
            return false;
        }
        if self.get_macro_text().is_some() {
            return false;
        }
        // Cho phép dd ngay cả trong chuỗi không phải tiếng Việt (dd hay gặp trong từ viết tắt)
        if self.config.ib_flags & cfg::IB_DD_FREE_STYLE != 0
            && !has_any_vietnamese_vowel(&vn_seq)
            && (*vn_runes.last().unwrap() == 'd' || vn_seq.contains('đ'))
        {
            return false;
        }
        if check_vn_rune && !has_any_vietnamese_rune(&vn_seq) {
            return false;
        }
        !self.preeditor.is_valid(false)
    }

    fn must_fallback_to_english(&self) -> bool {
        if self.config.ib_flags & cfg::IB_AUTO_NON_VN_RESTORE == 0 {
            return false;
        }
        let vn_seq = self.get_processed_string(mode::VIETNAMESE | mode::LOWER_CASE);
        if vn_seq.is_empty() {
            return false;
        }
        if self.config.ib_flags & cfg::IB_DD_FREE_STYLE != 0 && vn_seq.contains('đ') {
            return false;
        }
        // #18: nếu từ điển công nhận chuỗi này thì coi là hợp lệ (không khôi phục tiếng Anh) —
        // "giải oan" cho từ mượn / tên riêng mà bộ quy tắc CVC đơn giản từ chối.
        if self.dict_accepts(&vn_seq) {
            return false;
        }
        !self.preeditor.is_valid(true)
    }

    fn get_composed_string(&self, old_text: &str) -> String {
        if has_any_vietnamese_rune(old_text) && self.must_fallback_to_english() {
            self.get_processed_string(mode::ENGLISH)
        } else {
            old_text.to_string()
        }
    }

    fn encode_text(&self, text: &str) -> String {
        let bytes = core::encode(&self.config.output_charset, text);
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn get_preedit_string(&self) -> String {
        if self.macro_enabled() {
            return self.get_processed_string(mode::PUNCTUATION);
        }
        if self.should_fallback_to_english(true) {
            return self.get_processed_string(mode::ENGLISH);
        }
        self.get_processed_string(mode::VIETNAMESE)
    }

    fn get_raw_key_len(&self) -> usize {
        self.get_processed_string(mode::ENGLISH | mode::FULL_TEXT)
            .len()
    }

    fn rune_count(&self) -> usize {
        self.get_preedit_string().chars().count()
    }

    fn to_upper(&self, key_rune: char) -> char {
        let mapped = match key_rune {
            '[' => Some('{'),
            ']' => Some('}'),
            '{' => Some('['),
            '}' => Some(']'),
            _ => None,
        };
        if let Some(m) = mapped {
            if self
                .preeditor
                .get_input_method()
                .appending_keys
                .contains(&key_rune)
            {
                return m;
            }
        }
        key_rune
    }

    // ----- các hàm có side effect (đẩy Action vào danh sách) -----

    fn update_preedit(&self, processed_str: &str, out: &mut Vec<Action>) {
        let encoded = self.encode_text(processed_str);
        let preedit_len = encoded.chars().count() as u32;
        if preedit_len == 0 {
            out.push(Action::HidePreedit);
            out.push(Action::HideAuxiliary);
            out.push(Action::CommitText(String::new()));
            return;
        }
        // Cách khắc phục cho WPS (auxiliary text) trong Go dựa vào danh sách WM_CLASS cố định; bỏ
        // qua ở đây vì danh sách WM_CLASS nằm ở lớp platform.
        let underline = self.config.ib_flags & cfg::IB_NO_UNDERLINE == 0;
        out.push(Action::UpdatePreedit {
            text: encoded,
            cursor: preedit_len,
            underline,
        });
    }

    fn commit_text(&mut self, s: &str, out: &mut Vec<Action>) {
        if s.is_empty() {
            return;
        }
        self.track_sentence_commit(s);
        self.track_double_space_commit(s);
        out.push(Action::CommitText(self.encode_text(s)));
    }

    /// #65: mở cửa sổ double-space khi commit là "từ + một dấu cách" — ký tự ngay trước dấu
    /// cách phải là chữ/số (sau dấu câu như "xong. " thì "xong.. " là vô nghĩa → không mở).
    fn track_double_space_commit(&mut self, s: &str) {
        if self.config.ib_flags & cfg::IB_DOUBLE_SPACE_PERIOD == 0 {
            return;
        }
        let mut rev = s.chars().rev();
        self.double_space_armed =
            rev.next() == Some(' ') && rev.next().is_some_and(|c| c.is_alphanumeric());
    }

    /// #65: cập nhật máy trạng thái viết-hoa-đầu-câu theo chuỗi vừa commit. Chuỗi kết thúc bằng
    /// `.` `!` `?` → chờ khoảng trắng; kết thúc bằng khoảng trắng ngay sau dấu kết câu ("xong. ")
    /// → sẵn sàng viết hoa; còn lại → về Idle.
    fn track_sentence_commit(&mut self, s: &str) {
        if self.config.ib_flags & cfg::IB_CAPITALIZE_SENTENCE == 0 {
            return;
        }
        let mut rev = s.chars().rev().peekable();
        let mut saw_ws = false;
        while rev.peek().is_some_and(|c| c.is_whitespace()) {
            rev.next();
            saw_ws = true;
        }
        self.sentence = match rev.next() {
            Some('.' | '!' | '?') if saw_ws => SentenceState::ReadyToCapitalize,
            Some('.' | '!' | '?') => SentenceState::AfterPunct,
            _ => SentenceState::Idle,
        };
    }

    /// #65: xử lý phím cho viết-hoa-đầu-câu, TRƯỚC khi phím vào luồng chính. Trả về `key_val`
    /// (đã đổi thành chữ hoa nếu đúng lúc). Khoảng trắng sau dấu kết câu — dù bị forward vì
    /// buffer rỗng — vẫn đi qua đây nên máy trạng thái nhìn thấy đủ.
    fn apply_sentence_capitalize(&mut self, key_val: u32, state: u32) -> u32 {
        if self.config.ib_flags & cfg::IB_CAPITALIZE_SENTENCE == 0 {
            return key_val;
        }
        let key_rune = char::from_u32(key_val).unwrap_or('\0');
        match self.sentence {
            SentenceState::Idle => {}
            SentenceState::AfterPunct => {
                self.sentence = if key_rune == ' ' || key_val == KEY_RETURN {
                    SentenceState::ReadyToCapitalize
                } else {
                    SentenceState::Idle
                };
            }
            SentenceState::ReadyToCapitalize => {
                if key_rune == ' ' || key_val == KEY_RETURN {
                    // thêm khoảng trắng/xuống dòng → vẫn chờ chữ cái đầu.
                } else if key_rune.is_ascii_lowercase() && is_valid_state(state) {
                    self.sentence = SentenceState::Idle;
                    return key_val - 32; // 'a'..'z' → 'A'..'Z'
                } else {
                    // Chữ hoa sẵn, số, Backspace, phím điều hướng… → người dùng tự quyết.
                    self.sentence = SentenceState::Idle;
                }
            }
        }
        key_val
    }

    fn commit_preedit_and_reset(&mut self, s: &str, out: &mut Vec<Action>) {
        out.push(Action::HidePreedit);
        out.push(Action::HideAuxiliary);
        out.push(Action::HideLookupTable);
        self.commit_text(s, out);
        self.preeditor.reset();
    }

    fn commit_preedit_and_reset_for_wbs(&mut self, s: &str, is_wbs: bool, out: &mut Vec<Action>) {
        if self.config.ib_flags & cfg::IB_WORKAROUND_FOR_FB_MESSENGER != 0 || is_wbs {
            self.commit_text(s, out);
            out.push(Action::HidePreedit);
        } else {
            out.push(Action::HidePreedit);
            self.commit_text(s, out);
        }
        out.push(Action::HideAuxiliary);
        out.push(Action::HideLookupTable);
        self.preeditor.reset();
    }

    // ----- xử lý phím -----

    fn is_printable_key(&self, state: u32, key_val: u32) -> bool {
        is_valid_state(state) && self.is_valid_key_val(key_val)
    }

    fn is_valid_key_val(&self, key_val: u32) -> bool {
        let key_rune = char::from_u32(key_val).unwrap_or('\0');
        if key_val == KEY_BACKSPACE || is_word_break_symbol(key_rune) {
            return true;
        }
        if self.get_macro_text().is_some() && key_val == KEY_TAB {
            return true;
        }
        self.preeditor.can_process_key(key_rune)
    }

    /// #65 mức 1 (w→ư "không áp dụng ở đầu từ"): chặn w/W tạo `ư` khi từ hiện tại còn rỗng —
    /// "www", "word", "web"… giữ nguyên; giữa từ ("tw" → "tư") vẫn hoạt động. Phím bị chặn rơi
    /// xuống nhánh xử lý tiếng Anh (append thô).
    fn w_suppressed(&self, key_rune: char) -> bool {
        self.config.w_shortcut == 1
            && matches!(key_rune, 'w' | 'W')
            && self.get_processed_string(mode::ENGLISH).is_empty()
    }

    fn update_last_key_with_shift(&mut self, key_val: u32, state: u32) {
        let key_rune = char::from_u32(key_val).unwrap_or('\0');
        if self.preeditor.can_process_key(key_rune) {
            self.last_key_with_shift = state & MOD_SHIFT != 0;
        } else {
            self.last_key_with_shift = false;
        }
    }

    /// Trả về `(commit_text, is_word_break)`.
    fn get_commit_text(&mut self, key_val: u32, _key_code: u32, state: u32) -> (String, bool) {
        let mut key_rune = char::from_u32(key_val).unwrap_or('\0');
        let is_printable = self.is_printable_key(state, key_val);
        let old_text = self.get_preedit_string();

        if self.should_restore_key_strokes {
            self.should_restore_key_strokes = false;
            self.preeditor
                .restore_last_word(!has_any_vietnamese_rune(&old_text));
            return (self.get_preedit_string(), false);
        }

        let key_s = if is_printable {
            key_rune.to_string()
        } else {
            String::new()
        };

        // #65 mức 1: w ở đầu từ append thô (giữ "w" trong preedit, KHÔNG commit ngay — nếu rơi
        // xuống handle_non_vn_word thì "word" bị cắt thành "w" + "ord" và 'r' thành dấu hỏi).
        if is_printable && self.w_suppressed(key_rune) {
            self.preeditor.process_key(key_rune, mode::ENGLISH);
            return (format!("{}{}", old_text, key_s), false);
        }

        if is_printable && self.preeditor.can_process_key(key_rune) {
            if state & MOD_LOCK != 0 {
                key_rune = self.to_upper(key_rune);
            }
            let input_mode = self.get_input_mode();
            self.preeditor.process_key(key_rune, input_mode);
            if self
                .preeditor
                .get_input_method()
                .appending_keys
                .contains(&key_rune)
            {
                let new_text = if self.should_fallback_to_english(true) {
                    self.get_processed_string(mode::ENGLISH)
                } else {
                    self.get_processed_string(mode::VIETNAMESE)
                };
                let full_seq = self.get_processed_string(mode::VIETNAMESE);
                if !full_seq.is_empty()
                    && full_seq.as_bytes().last().map(|b| *b as char) == Some(key_rune)
                {
                    // [[ => [
                    let ret = self.get_preedit_string();
                    let last_rune = ret.as_bytes().last().map(|b| *b as char).unwrap_or('\0');
                    let is_wbs = is_word_break_symbol(last_rune);
                    if is_wbs {
                        self.preeditor.remove_last_char(false);
                        self.preeditor.process_key(' ', mode::ENGLISH);
                    }
                    return (ret, is_wbs);
                } else if new_text.ends_with(key_rune) {
                    // f] => f]
                    let is_wbs = is_word_break_symbol(key_rune);
                    if is_wbs {
                        self.preeditor.remove_last_char(false);
                        self.preeditor.process_key(' ', mode::ENGLISH);
                    }
                    return (format!("{}{}", old_text, key_rune), is_wbs);
                } else {
                    // ] => o?
                    return (self.get_preedit_string(), false);
                }
            } else if self.macro_enabled() {
                return (self.get_processed_string(mode::PUNCTUATION), false);
            } else {
                return (self.get_preedit_string(), false);
            }
        } else if self.macro_enabled() {
            if is_printable
                && self
                    .macro_table
                    .has_prefix(&format!("{}{}", old_text, key_s))
            {
                self.preeditor.process_key(key_rune, mode::ENGLISH);
                return (format!("{}{}", old_text, key_s), false);
            }
            if self.macro_table.has_key(&old_text) {
                if is_printable {
                    return (format!("{}{}", self.expand_macro(&old_text), key_s), true);
                }
                return (self.expand_macro(&old_text), true);
            }
        }
        (self.handle_non_vn_word(key_val, _key_code, state), true)
    }

    fn handle_non_vn_word(&mut self, key_val: u32, _key_code: u32, state: u32) -> String {
        let key_rune = char::from_u32(key_val).unwrap_or('\0');
        let is_printable = self.is_printable_key(state, key_val);
        let old_text = self.get_preedit_string();
        let key_s = if is_printable {
            key_rune.to_string()
        } else {
            String::new()
        };
        if has_any_vietnamese_rune(&old_text) && self.must_fallback_to_english() {
            self.preeditor.restore_last_word(false);
            let new_text = format!(
                "{}{}",
                self.preeditor
                    .get_processed_string(mode::PUNCTUATION | mode::ENGLISH),
                key_s
            );
            if is_printable {
                self.preeditor.process_key(key_rune, mode::ENGLISH);
            }
            return new_text;
        }
        if is_printable {
            self.preeditor.process_key(key_rune, mode::ENGLISH);
            return format!("{}{}", old_text, key_s);
        }
        format!("{}{}", old_text, key_s)
    }

    /// Điểm vào chính — trả về `(handled, actions)`.
    pub fn process_key_event(
        &mut self,
        key_val: u32,
        key_code: u32,
        state: u32,
    ) -> (bool, Vec<Action>) {
        let mut out = Vec::new();
        if state & MOD_RELEASE != 0 {
            return (false, out);
        }
        if is_modifier_keysym(key_val) {
            return (false, out);
        }
        // #65: phím này tới engine nghĩa là addon KHÔNG tiêu thụ nó cho double-space → cửa sổ
        // khép lại (commit sinh ra trong chính sự kiện này có thể mở lại ở commit_text).
        self.double_space_armed = false;
        // #65: viết hoa đầu câu — có thể đổi key_val thành chữ hoa; chạy TRƯỚC luồng chính để
        // cả phím forward (space khi buffer rỗng) cũng đi qua máy trạng thái.
        let key_val = self.apply_sentence_capitalize(key_val, state);
        let result = self.preedit_process_key_event(key_val, key_code, state, &mut out);
        self.update_last_key_with_shift(key_val, state);
        (result, out)
    }

    fn preedit_process_key_event(
        &mut self,
        key_val: u32,
        key_code: u32,
        state: u32,
        out: &mut Vec<Action>,
    ) -> bool {
        let raw_key_len = self.get_raw_key_len();
        let key_rune = char::from_u32(key_val).unwrap_or('\0');
        let old_text = self.get_preedit_string();

        if !self.should_restore_key_strokes
            && !self.preeditor.can_process_key(key_rune)
            && raw_key_len == 0
            && !self.macro_enabled()
        {
            // Không xử lý ký tự đặc biệt khi buffer rỗng (thanh địa chỉ Chrome, v.v.)
            return false;
        }

        if key_val == KEY_BACKSPACE {
            if self.rune_count() == 1 {
                self.commit_preedit_and_reset("", out);
                return true;
            }
            if raw_key_len > 0 {
                self.preeditor.remove_last_char(true);
                let s = self.get_preedit_string();
                self.update_preedit(&s, out);
                return true;
            }
            return false;
        }

        if key_val == KEY_TAB {
            if let Some(mac_text) = self.get_macro_text() {
                self.commit_preedit_and_reset(&mac_text, out);
            } else {
                let composed = self.get_composed_string(&old_text);
                self.commit_preedit_and_reset(&composed, out);
                return false;
            }
            return true;
        }

        let (new_text, is_word_break_rune) = self.get_commit_text(key_val, key_code, state);
        let is_printable_key = self.is_printable_key(state, key_val);
        if is_word_break_rune {
            self.commit_preedit_and_reset_for_wbs(&new_text, is_printable_key, out);
            return is_printable_key;
        }
        self.update_preedit(&new_text, out);
        is_printable_key
    }
}

fn build_preeditor(config: &Config) -> PinaKeyEngine {
    let mut pairs: Vec<(String, String)> = config
        .input_method_definitions
        .get(&config.input_method)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    // #65: w→ư — thêm rule appending `__Ư` vào phím w (biến Telex thành Telex W). Chỉ áp cho
    // định nghĩa có phím w chưa có phần appending; VNI/VIQR không có "w" nên không đổi.
    if config.w_shortcut > 0 {
        for (k, v) in pairs.iter_mut() {
            if k == "w" && !v.contains("__") {
                v.push_str("__Ư");
            }
        }
    }
    let im = build_input_method_from_pairs(&config.input_method, &pairs);
    let mut flags = config.flags;
    // Telex đơn giản (#16): tắt gõ-dấu-tự-do để dấu áp ngay, không tự dời.
    if core::is_simple_telex(&config.input_method) {
        flags &= !core::flag::FREE_TONE_MARKING;
    }
    core::new_engine(im, flags)
}

/// Nạp từ điển chính tả khi bật `IB_SPELL_CHECK_WITH_DICTS` (issue #18): bộ từ khởi đầu đóng kèm
/// binary, phủ thêm từ điển người dùng `~/.config/pinakey/dict.txt` nếu có.
fn load_dictionary(config: &Config) -> Option<core::Dictionary> {
    if config.ib_flags & cfg::IB_SPELL_CHECK_WITH_DICTS == 0 {
        return None;
    }
    let mut dict = core::Dictionary::bundled();
    if let Some(path) = pinakey_config::get_dict_path().to_str() {
        if let Ok(user) = core::Dictionary::load_file(path) {
            dict.merge(&user);
        }
    }
    Some(dict)
}

fn determine_macro_case(s: &str) -> u8 {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return VN_CASE_NO_CHANGE;
    }
    if chars[0].is_lowercase() {
        return VN_CASE_ALL_SMALL;
    }
    for &c in &chars[1..] {
        if c.is_lowercase() {
            return VN_CASE_NO_CHANGE;
        }
        if is_word_break_symbol(c) {
            return VN_CASE_NO_CHANGE;
        }
    }
    VN_CASE_ALL_CAPITAL
}

fn is_valid_state(state: u32) -> bool {
    state & MOD_CONTROL == 0
        && state & MOD_MOD1 == 0
        && state & MOD_MOD4 == 0
        && state & MOD_IGNORED == 0
        && state & MOD_SUPER == 0
        && state & MOD_HYPER == 0
        && state & MOD_META == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinakey_config::default_cfg;

    /// Gõ một chuỗi (mỗi ký tự là một phím in được) và trả về các action sinh ra bởi phím cuối
    /// cùng, cùng với danh sách tích lũy mọi commit đã phát ra.
    fn type_keys(core: &mut EngineCore, s: &str) -> Vec<Action> {
        let mut all = Vec::new();
        for c in s.chars() {
            let (_handled, actions) = core.process_key_event(c as u32, 0, 0);
            all.extend(actions);
        }
        all
    }

    fn last_preedit(actions: &[Action]) -> Option<String> {
        actions.iter().rev().find_map(|a| match a {
            Action::UpdatePreedit { text, .. } => Some(text.clone()),
            _ => None,
        })
    }

    fn commits(actions: &[Action]) -> Vec<String> {
        actions
            .iter()
            .filter_map(|a| match a {
                Action::CommitText(t) if !t.is_empty() => Some(t.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn bare_modifier_press_mid_word_does_not_commit() {
        // Nhấn Shift/Ctrl đơn lẻ giữa từ không được ép commit từ đang gõ
        // (trước đây "vieet" + Shift + "j" cho ra "viêtj" thay vì "việt").
        let mut core = EngineCore::new(default_cfg());
        type_keys(&mut core, "vieet"); // preedit "viêt"
        for sym in [0xffe1_u32, 0xffe3, 0xfe03] {
            // Shift_L, Control_L, ISO_Level3_Shift (AltGr)
            let (handled, actions) = core.process_key_event(sym, 0, 0);
            assert!(!handled, "keysym modifier {sym:#x} phải được cho đi qua");
            assert!(
                commits(&actions).is_empty(),
                "keysym modifier {sym:#x} không được commit preedit"
            );
        }
        let actions = type_keys(&mut core, "j");
        assert_eq!(last_preedit(&actions).as_deref(), Some("việt"));
    }

    #[test]
    fn telex_preedit_builds_vietnamese() {
        let mut core = EngineCore::new(default_cfg()); // mặc định = Telex
        let actions = type_keys(&mut core, "vieetj");
        assert_eq!(last_preedit(&actions).as_deref(), Some("việt"));
    }

    #[test]
    fn word_break_commits_word() {
        let mut core = EngineCore::new(default_cfg());
        // gõ "tieengs" -> "tiếng", rồi phím space commit nó
        let mut actions = type_keys(&mut core, "tieengs");
        assert_eq!(last_preedit(&actions).as_deref(), Some("tiếng"));
        let (_h, sp) = core.process_key_event(' ' as u32, 0, 0);
        actions.extend(sp);
        // Phím ngắt từ (space) được commit cùng với từ.
        assert!(commits(&actions).iter().any(|c| c.trim() == "tiếng"));
    }

    #[test]
    fn w_shortcut_off_keeps_w() {
        // #65 mức 0 (mặc định): "w" đứng riêng vẫn là "w".
        let mut core = EngineCore::new(default_cfg());
        let actions = type_keys(&mut core, "w");
        assert_eq!(last_preedit(&actions).as_deref(), Some("w"));
    }

    #[test]
    fn w_shortcut_everywhere() {
        // #65 mức 2: "w" ra "ư" ở mọi vị trí; gõ đúp "ww" trả lại "w".
        let mut cfg = default_cfg();
        cfg.w_shortcut = 2;
        let mut core = EngineCore::new(cfg);
        let actions = type_keys(&mut core, "w");
        assert_eq!(last_preedit(&actions).as_deref(), Some("ư"), "w → ư");
        core.reset_preeditor();
        let actions = type_keys(&mut core, "ww");
        assert_eq!(last_preedit(&actions).as_deref(), Some("w"), "ww → w");
        core.reset_preeditor();
        let actions = type_keys(&mut core, "tw");
        assert_eq!(last_preedit(&actions).as_deref(), Some("tư"), "tw → tư");
    }

    #[test]
    fn w_shortcut_not_at_word_start() {
        // #65 mức 1: "w" ở ĐẦU TỪ giữ nguyên (www, word…); giữa từ vẫn ra "ư".
        let mut cfg = default_cfg();
        cfg.w_shortcut = 1;
        let mut core = EngineCore::new(cfg);
        let actions = type_keys(&mut core, "w");
        assert_eq!(last_preedit(&actions).as_deref(), Some("w"), "đầu từ giữ w");
        core.reset_preeditor();
        let actions = type_keys(&mut core, "www");
        assert_eq!(
            last_preedit(&actions).as_deref(),
            Some("www"),
            "www giữ nguyên"
        );
        core.reset_preeditor();
        let actions = type_keys(&mut core, "tw");
        assert_eq!(
            last_preedit(&actions).as_deref(),
            Some("tư"),
            "giữa từ tw → tư"
        );
        core.reset_preeditor();
        // Từ tiếng Anh bắt đầu bằng w phải nguyên vẹn — đặc biệt 'r' không được thành dấu hỏi
        // (nếu w bị commit rời thay vì append, "word" thành "w" + "ỏd").
        let actions = type_keys(&mut core, "word");
        assert_eq!(
            last_preedit(&actions).as_deref(),
            Some("word"),
            "word giữ nguyên"
        );
    }

    #[test]
    fn w_shortcut_does_not_break_uw() {
        // #65: cả 3 mức không được phá "uw" → "ư" chuẩn Telex.
        for level in [0u8, 1, 2] {
            let mut cfg = default_cfg();
            cfg.w_shortcut = level;
            let mut core = EngineCore::new(cfg);
            let actions = type_keys(&mut core, "tuw");
            assert_eq!(
                last_preedit(&actions).as_deref(),
                Some("tư"),
                "mức {level}: tuw → tư"
            );
        }
    }

    #[test]
    fn capitalize_sentence_after_period_and_space() {
        // #65: sau "xong." + space (space đi qua engine dù buffer rỗng), chữ cái đầu tiên của
        // từ kế tiếp tự viết hoa — kể cả khi từ đó có dấu Telex.
        let mut cfg = default_cfg();
        cfg.ib_flags |= cfg::IB_CAPITALIZE_SENTENCE;
        let mut core = EngineCore::new(cfg);
        type_keys(&mut core, "xong");
        core.process_key_event('.' as u32, 0, 0); // commit "xong."
        core.process_key_event(' ' as u32, 0, 0); // buffer rỗng → space forward, engine vẫn thấy
        let actions = type_keys(&mut core, "vieetj");
        assert_eq!(last_preedit(&actions).as_deref(), Some("Việt"));
    }

    #[test]
    fn capitalize_sentence_off_by_default() {
        let mut core = EngineCore::new(default_cfg());
        type_keys(&mut core, "xong");
        core.process_key_event('.' as u32, 0, 0);
        core.process_key_event(' ' as u32, 0, 0);
        let actions = type_keys(&mut core, "vieetj");
        assert_eq!(last_preedit(&actions).as_deref(), Some("việt"));
    }

    #[test]
    fn capitalize_sentence_not_after_comma_or_without_space() {
        let mut cfg = default_cfg();
        cfg.ib_flags |= cfg::IB_CAPITALIZE_SENTENCE;
        let mut core = EngineCore::new(cfg);
        // Dấu phẩy không kết câu.
        type_keys(&mut core, "xong");
        core.process_key_event(',' as u32, 0, 0);
        core.process_key_event(' ' as u32, 0, 0);
        let actions = type_keys(&mut core, "va");
        assert_eq!(
            last_preedit(&actions).as_deref(),
            Some("va"),
            "sau dấu phẩy"
        );
        // "." nhưng CHƯA có khoảng trắng → chưa viết hoa (viết tắt, số thập phân…).
        let mut core = EngineCore::new({
            let mut c = default_cfg();
            c.ib_flags |= cfg::IB_CAPITALIZE_SENTENCE;
            c
        });
        type_keys(&mut core, "xong");
        core.process_key_event('.' as u32, 0, 0);
        let actions = type_keys(&mut core, "va");
        assert_eq!(
            last_preedit(&actions).as_deref(),
            Some("va"),
            "chưa có space"
        );
    }

    #[test]
    fn capitalize_sentence_single_shot_and_cleared_by_backspace() {
        let mut cfg = default_cfg();
        cfg.ib_flags |= cfg::IB_CAPITALIZE_SENTENCE;
        let mut core = EngineCore::new(cfg);
        // Backspace sau ". " (người dùng sửa tay) → huỷ viết hoa chờ.
        type_keys(&mut core, "xong");
        core.process_key_event('.' as u32, 0, 0);
        core.process_key_event(' ' as u32, 0, 0);
        core.process_key_event(KEY_BACKSPACE, 0, 0);
        let actions = type_keys(&mut core, "va");
        assert_eq!(
            last_preedit(&actions).as_deref(),
            Some("va"),
            "backspace huỷ"
        );
        // Chỉ viết hoa MỘT chữ đầu: từ thứ hai của câu không bị hoa.
        let mut core = EngineCore::new({
            let mut c = default_cfg();
            c.ib_flags |= cfg::IB_CAPITALIZE_SENTENCE;
            c
        });
        type_keys(&mut core, "xong");
        core.process_key_event('.' as u32, 0, 0);
        core.process_key_event(' ' as u32, 0, 0);
        type_keys(&mut core, "hai");
        core.process_key_event(' ' as u32, 0, 0); // commit "Hai "
        let actions = type_keys(&mut core, "ba");
        assert_eq!(last_preedit(&actions).as_deref(), Some("ba"), "one-shot");
    }

    #[test]
    fn double_space_arms_after_word_space_commit() {
        // #65: sau khi commit "tiếng " (từ + dấu cách), engine "lên đạn" cho double-space.
        let mut cfg = default_cfg();
        cfg.ib_flags |= cfg::IB_DOUBLE_SPACE_PERIOD;
        let mut core = EngineCore::new(cfg);
        type_keys(&mut core, "tieengs");
        assert!(!core.double_space_armed(), "chưa commit thì chưa arm");
        core.process_key_event(' ' as u32, 0, 0);
        assert!(core.double_space_armed(), "commit 'tiếng ' phải arm");
    }

    #[test]
    fn double_space_not_armed_when_off_or_after_punct() {
        // Cờ tắt (mặc định) → không bao giờ arm.
        let mut core = EngineCore::new(default_cfg());
        type_keys(&mut core, "tieengs");
        core.process_key_event(' ' as u32, 0, 0);
        assert!(!core.double_space_armed(), "mặc định tắt");
        // Commit kết thúc ". " (đã có dấu câu) → không arm ("xong.  " thành "xong.. " là vô nghĩa).
        let mut cfg = default_cfg();
        cfg.ib_flags |= cfg::IB_DOUBLE_SPACE_PERIOD;
        let mut core = EngineCore::new(cfg);
        type_keys(&mut core, "xong");
        core.process_key_event('.' as u32, 0, 0);
        core.process_key_event(' ' as u32, 0, 0);
        assert!(!core.double_space_armed(), "sau dấu câu không arm");
    }

    #[test]
    fn double_space_disarmed_by_any_following_key() {
        let mut cfg = default_cfg();
        cfg.ib_flags |= cfg::IB_DOUBLE_SPACE_PERIOD;
        let mut core = EngineCore::new(cfg);
        type_keys(&mut core, "tieengs");
        core.process_key_event(' ' as u32, 0, 0);
        assert!(core.double_space_armed());
        type_keys(&mut core, "a"); // gõ tiếp → cửa sổ double-space khép lại
        assert!(!core.double_space_armed());
    }

    #[test]
    fn double_space_consume_disarms_and_chains_capitalize() {
        // Consume (C++ đã xoá space + commit ". ") → hết arm; nếu bật viết-hoa-đầu-câu thì
        // chữ kế tiếp được hoa luôn (văn bản giờ kết thúc ". ").
        let mut cfg = default_cfg();
        cfg.ib_flags |= cfg::IB_DOUBLE_SPACE_PERIOD | cfg::IB_CAPITALIZE_SENTENCE;
        let mut core = EngineCore::new(cfg);
        type_keys(&mut core, "tieengs");
        core.process_key_event(' ' as u32, 0, 0);
        assert!(core.double_space_armed());
        core.double_space_consume();
        assert!(!core.double_space_armed());
        let actions = type_keys(&mut core, "vieetj");
        assert_eq!(last_preedit(&actions).as_deref(), Some("Việt"));
    }

    /// Dựng engine bật macro với nội dung bảng macro cho trước (ghi file tạm rồi nạp).
    /// `name` phải là duy nhất cho mỗi test — các test chạy song song, trùng file tạm sẽ race.
    fn core_with_macro(name: &str, macro_lines: &str, cfg: pinakey_config::Config) -> EngineCore {
        let path = std::env::temp_dir().join(format!(
            "pinakey_engine_macro_test_{}_{}.txt",
            std::process::id(),
            name
        ));
        std::fs::write(&path, macro_lines).unwrap();
        let mut cfg = cfg;
        cfg.ib_flags |= cfg::IB_MACRO_ENABLED;
        let mut core = EngineCore::new(cfg);
        let mut mt = MacroTable::new(false);
        mt.load_from_file(path.to_str().unwrap()).unwrap();
        mt.set_enabled(true);
        core.macro_table = mt;
        std::fs::remove_file(&path).ok();
        core
    }

    #[test]
    fn macro_expands_date_placeholder_at_activation() {
        // #64: "$DATE" trong giá trị macro → ngày hiện tại theo format mặc định dd/mm/yyyy.
        let mut core = core_with_macro("date_mac_dinh", "hnay : hôm nay $DATE\n", default_cfg());
        let mut actions = type_keys(&mut core, "hnay");
        let (_h, sp) = core.process_key_event(' ' as u32, 0, 0);
        actions.extend(sp);
        let all = commits(&actions).join("");
        let rest = all
            .strip_prefix("hôm nay ")
            .unwrap_or_else(|| panic!("commit phải bắt đầu bằng 'hôm nay ': {all:?}"));
        let date: Vec<char> = rest.trim_end().chars().collect();
        assert_eq!(date.len(), 10, "dd/mm/yyyy phải 10 ký tự: {rest:?}");
        for (i, c) in date.iter().enumerate() {
            if i == 2 || i == 5 {
                assert_eq!(*c, '/', "vị trí {i} phải là '/': {rest:?}");
            } else {
                assert!(c.is_ascii_digit(), "vị trí {i} phải là chữ số: {rest:?}");
            }
        }
    }

    #[test]
    fn macro_custom_format_from_config() {
        // #64: format lấy từ config tại thời điểm kích hoạt. Format toàn ký tự literal
        // (không có %) hợp lệ với strftime → kết quả xác định, test không phụ thuộc đồng hồ.
        let mut cfg = default_cfg();
        cfg.macro_date_format = "ngày".to_string();
        let mut core = core_with_macro("date_tuy_chinh", "hnay : hôm nay $DATE\n", cfg);
        let mut actions = type_keys(&mut core, "hnay");
        let (_h, sp) = core.process_key_event(' ' as u32, 0, 0);
        actions.extend(sp);
        let all = commits(&actions).join("");
        assert_eq!(all.trim_end(), "hôm nay ngày", "commit: {all:?}");
    }

    #[test]
    fn macro_double_dollar_stays_literal() {
        // #64: "$$TIME" phải ra literal "$TIME", không bị thay bằng giờ. Tắt auto-capitalize
        // macro: tính năng đó (có sẵn) hạ/nâng chữ TOÀN BỘ kết quả theo cách gõ key, sẽ biến
        // "$TIME" thành "$time" — không liên quan tới cơ chế escape đang test.
        let mut cfg = default_cfg();
        cfg.ib_flags &= !cfg::IB_AUTO_CAPITALIZE_MACRO;
        let mut core = core_with_macro("literal", "sig : $$TIME\n", cfg);
        let mut actions = type_keys(&mut core, "sig");
        let (_h, sp) = core.process_key_event(' ' as u32, 0, 0);
        actions.extend(sp);
        let all = commits(&actions).join("");
        assert_eq!(all.trim_end(), "$TIME", "commit: {all:?}");
    }

    #[test]
    fn surrounding_text_unreliable_for_libreoffice() {
        // #66: LibreOffice/OpenOffice (soffice) báo surrounding text lạc hậu/thiếu dấu cách khi
        // gõ nhanh → phải bị đánh dấu "không đáng tin" để addon rơi về preedit.
        let mut core = EngineCore::new(default_cfg());
        assert!(
            !core.is_surrounding_text_unreliable(),
            "chưa đặt program → coi như đáng tin"
        );
        for p in [
            "soffice",
            "soffice.bin",
            "libreoffice-writer",
            "LibreOffice-calc",
            "org.libreoffice.LibreOffice",
        ] {
            core.set_wm_class(p.to_string());
            assert!(
                core.is_surrounding_text_unreliable(),
                "{p} phải bị coi là không đáng tin"
            );
        }
        for p in ["firefox", "google-chrome", "onlyoffice-desktopeditors"] {
            core.set_wm_class(p.to_string());
            assert!(!core.is_surrounding_text_unreliable(), "{p} phải đáng tin");
        }
    }

    #[test]
    fn backspace_removes_char() {
        let mut core = EngineCore::new(default_cfg());
        type_keys(&mut core, "vieetj"); // ra "việt"
        let (handled, actions) = core.process_key_event(KEY_BACKSPACE, 0, 0);
        assert!(handled);
        assert_eq!(last_preedit(&actions).as_deref(), Some("việ"));
    }

    #[test]
    fn non_vietnamese_falls_back() {
        // "xin" hợp lệ, nhưng một chuỗi rõ ràng không hợp lệ thì khi commit phải khôi phục lại các
        // phím gốc đã gõ.
        let mut core = EngineCore::new(default_cfg());
        let mut actions = type_keys(&mut core, "loz"); // không phải tiếng Việt -> giữ nguyên "loz"
        let (_h, sp) = core.process_key_event(' ' as u32, 0, 0);
        actions.extend(sp);
        assert!(commits(&actions).iter().any(|c| c.trim() == "loz"));
    }

    #[test]
    fn empty_buffer_passes_through_punctuation() {
        let mut core = EngineCore::new(default_cfg());
        // '.' với buffer rỗng không xử lý được, buffer lại rỗng -> không handle (cho đi qua)
        let (handled, _actions) = core.process_key_event('.' as u32, 0, 0);
        assert!(!handled);
    }

    fn simple_telex_cfg() -> pinakey_config::Config {
        let mut c = default_cfg();
        c.input_method = "Telex (đơn giản)".to_string();
        c
    }

    #[test]
    fn simple_telex_differs_from_standard_free_marking() {
        // Telex chuẩn (free marking): "anhs" -> dấu sắc tìm đúng nguyên âm 'a' -> "ánh".
        let mut std = EngineCore::new(default_cfg());
        let std_out = last_preedit(&type_keys(&mut std, "anhs"));
        // Telex đơn giản (free marking tắt): 's' sau phụ âm 'h' -> không áp dấu -> "anhs".
        let mut simple = EngineCore::new(simple_telex_cfg());
        let simple_out = last_preedit(&type_keys(&mut simple, "anhs"));
        assert_eq!(std_out.as_deref(), Some("ánh"));
        assert_ne!(
            std_out, simple_out,
            "free marking phải đổi hành vi của 'anhs'"
        );
    }

    #[test]
    fn dict_spellcheck_wiring() {
        use pinakey_config::flags as cfg;
        // Mặc định KHÔNG bật cờ dict → không công nhận theo từ điển.
        let off = EngineCore::new(default_cfg());
        assert!(!off.accepts_word("việt"));
        // Bật cờ dict → nạp bộ từ khởi đầu (có "việt", "chào").
        let mut c = default_cfg();
        c.ib_flags |= cfg::IB_SPELL_CHECK_WITH_DICTS;
        let on = EngineCore::new(c);
        assert!(on.accepts_word("việt"));
        assert!(on.accepts_word("chào"));
        assert!(!on.accepts_word("zzqx"));
    }

    #[test]
    fn simple_telex_still_types_basic_vietnamese() {
        let mut e1 = EngineCore::new(simple_telex_cfg());
        assert_eq!(
            last_preedit(&type_keys(&mut e1, "as")).as_deref(),
            Some("á")
        );
        let mut e2 = EngineCore::new(simple_telex_cfg());
        assert_eq!(
            last_preedit(&type_keys(&mut e2, "aa")).as_deref(),
            Some("â")
        );
    }
}
