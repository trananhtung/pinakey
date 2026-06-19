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
use pinakey_emoji::{load_bundled, MacroTable, TrieNode};

use crate::backspace::{correction_actions, diff_correction};
use crate::constants::*;
use crate::lookup::{EmojiState, EMOJI_PAGE_SIZE};
use crate::props::{self, Prop};
use crate::shortcuts::{match_shortcut, ShortcutAction};

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
    /// Xóa `nchars` ký tự quanh con trỏ (chế độ Surrounding Text). `offset` âm = lùi về trước con trỏ.
    DeleteSurroundingText {
        offset: i32,
        nchars: u32,
    },
    /// Phát `n` phím BackSpace qua `forward_key_event` (chế độ forwarding, chạy được cả Wayland).
    ForwardBackspaces(u32),
    /// Tiêm `n` phím BackSpace qua XTest ở lớp platform (chỉ X11/XWayland).
    FakeBackspaces(u32),
    /// Cập nhật bảng tra cứu (lookup table) emoji/hex; `visible=false` thì ẩn bảng.
    UpdateLookupTable {
        candidates: Vec<String>,
        cursor: u32,
        page_size: u32,
        visible: bool,
    },
    /// Mở giao diện thiết lập đồ họa (chạy binary `pinakey-settings`).
    LaunchSettings,
}

pub struct EngineCore {
    pub preeditor: PinaKeyEngine,
    pub config: Config,
    pub macro_table: MacroTable,
    pub should_restore_key_strokes: bool,
    pub last_key_with_shift: bool,
    wm_class: String,
    /// Chế độ nhập hiện tại (`PREEDIT_IM`, các chế độ sửa lỗi bằng backspace, ...).
    input_mode: i32,
    /// Văn bản (đã mã hóa) hiện đang hiển thị trên màn hình cho từ đang gõ — chỉ dùng ở chế độ
    /// sửa lỗi bằng backspace để tính phần cần xóa lùi giữa hai lần gõ.
    previous_text: String,
    /// Trạng thái bảng tra cứu emoji/hex đang mở (nếu có).
    emoji: EmojiState,
    /// Trie emoji được nạp lười (chỉ khi lần đầu vào chế độ emoji).
    emoji_trie: Option<TrieNode>,
    /// Có đang gõ tiếng Việt không (phím tắt bật/tắt sẽ lật cờ này; tắt = gõ thẳng tiếng Anh).
    vietnamese_enabled: bool,
    /// Từ điển kiểm tra chính tả (chỉ nạp khi bật cờ `IB_SPELL_CHECK_WITH_DICTS`).
    dictionary: Option<core::Dictionary>,
}

const VN_CASE_ALL_SMALL: u8 = 1;
const VN_CASE_ALL_CAPITAL: u8 = 2;
const VN_CASE_NO_CHANGE: u8 = 3;

impl EngineCore {
    pub fn new(config: Config) -> EngineCore {
        let preeditor = build_preeditor(&config);
        let macro_table = MacroTable::new(config.ib_flags & cfg::IB_AUTO_CAPITALIZE_MACRO != 0);
        let input_mode = config.default_input_mode;
        let dictionary = load_dictionary(&config);
        EngineCore {
            preeditor,
            config,
            macro_table,
            should_restore_key_strokes: false,
            last_key_with_shift: false,
            wm_class: String::new(),
            input_mode,
            previous_text: String::new(),
            emoji: EmojiState::new(),
            emoji_trie: None,
            vietnamese_enabled: true,
            dictionary,
        }
    }

    /// Có đang gõ tiếng Việt không.
    pub fn is_vietnamese_enabled(&self) -> bool {
        self.vietnamese_enabled
    }

    /// Đặt từ điển kiểm tra chính tả (cấu hình lại lúc chạy hoặc dùng cho kiểm thử).
    pub fn set_dictionary(&mut self, dict: core::Dictionary) {
        self.dictionary = Some(dict);
    }

    /// Danh sách mục menu thuộc tính phản ánh trạng thái hiện tại.
    pub fn build_props(&self) -> Vec<Prop> {
        props::build_props(&self.config.input_method, self.vietnamese_enabled)
    }

    /// Xử lý khi người dùng kích hoạt một mục trên menu thuộc tính của panel IBus.
    pub fn on_property_activate(&mut self, key: &str, _state: u32) -> Vec<Action> {
        let mut out = Vec::new();
        if key == "vn_toggle" {
            self.toggle_vietnamese(&mut out);
        } else if key == props::OPEN_SETTINGS_KEY {
            out.push(Action::LaunchSettings);
        } else if let Some(im) = key.strip_prefix("im_") {
            if props::INPUT_METHODS.contains(&im) && self.config.input_method != im {
                self.config.input_method = im.to_string();
                self.rebuild_preeditor();
                self.reset_preeditor();
            }
        }
        out
    }

    /// Từ tiếng Việt hiện đang gõ có được coi là hợp lệ không — gồm cả tra từ điển khi bật cờ
    /// `IB_SPELL_CHECK_WITH_DICTS` (từ điển chỉ "giải oan", không loại bỏ từ quy tắc đã chấp nhận).
    pub fn current_word_is_valid(&self) -> bool {
        let vn_seq = self.get_processed_string(mode::VIETNAMESE | mode::LOWER_CASE);
        self.dict_accepts(&vn_seq) || self.preeditor.is_valid(true)
    }

    /// Từ điển có công nhận `word` không (chỉ khi cờ dict bật và từ điển đã nạp).
    fn dict_accepts(&self, word: &str) -> bool {
        self.config.ib_flags & cfg::IB_SPELL_CHECK_WITH_DICTS != 0
            && self.dictionary.as_ref().is_some_and(|d| d.contains(word))
    }

    /// Chế độ nhập hiện tại có phải loại "sửa lỗi bằng backspace" không.
    fn is_backspace_mode(&self) -> bool {
        cfg::IM_BACKSPACE_LIST.contains(&self.input_mode)
    }

    pub fn set_wm_class(&mut self, wm_class: String) {
        self.wm_class = wm_class;
    }

    /// Đặt lại trạng thái soạn thảo bên dưới (tương ứng `Reset` của IBus).
    pub fn reset_preeditor(&mut self) {
        self.preeditor.reset();
        self.previous_text.clear();
        self.emoji.close();
    }

    /// Dựng lại engine biến đổi sau khi cấu hình thay đổi (input method / flags).
    pub fn rebuild_preeditor(&mut self) {
        self.preeditor = build_preeditor(&self.config);
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
        // Kiểm tra chính tả dựa trên từ điển (IBspellCheckWithDicts): nếu từ điển công nhận thì
        // không quay về tiếng Anh, dù bộ quy tắc có thể từ chối.
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
        out.push(Action::CommitText(self.encode_text(s)));
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
        if key_val == IBUS_BACKSPACE || is_word_break_symbol(key_rune) {
            return true;
        }
        if self.get_macro_text().is_some() && key_val == IBUS_TAB {
            return true;
        }
        self.preeditor.can_process_key(key_rune)
    }

    fn update_last_key_with_shift(&mut self, key_val: u32, state: u32) {
        let key_rune = char::from_u32(key_val).unwrap_or('\0');
        if self.preeditor.can_process_key(key_rune) {
            self.last_key_with_shift = state & IBUS_SHIFT_MASK != 0;
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

        if is_printable && self.preeditor.can_process_key(key_rune) {
            if state & IBUS_LOCK_MASK != 0 {
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
        if state & IBUS_RELEASE_MASK != 0 {
            return (false, out);
        }
        // Phím tắt được kiểm trước mọi thứ (kể cả khi đang tắt tiếng Việt) để có thể bật lại.
        if let Some(action) = match_shortcut(&self.config.shortcuts, state, key_val) {
            match action {
                ShortcutAction::ToggleVietnamese => {
                    self.toggle_vietnamese(&mut out);
                    return (true, out);
                }
                ShortcutAction::RestoreWord => {
                    self.should_restore_key_strokes = true;
                    return (true, out);
                }
            }
        }
        // Tắt tiếng Việt: cho mọi phím đi thẳng tới ứng dụng.
        if !self.vietnamese_enabled {
            return (false, out);
        }
        // Bảng tra cứu emoji/hex chiếm quyền xử lý khi đang mở.
        if self.emoji.is_active() {
            let r = self.emoji_process_key(key_val, state, &mut out);
            return (r, out);
        }
        // Kích hoạt chế độ emoji bằng ':' khi buffer rỗng (không phá việc gõ tiếng Việt).
        if key_val == IBUS_COLON && self.get_raw_key_len() == 0 && !self.should_restore_key_strokes
        {
            self.emoji.start();
            self.emit_emoji_ui(&mut out);
            return (true, out);
        }
        let result = if self.is_backspace_mode() {
            self.backspace_process_key_event(key_val, key_code, state, &mut out)
        } else {
            self.preedit_process_key_event(key_val, key_code, state, &mut out)
        };
        self.update_last_key_with_shift(key_val, state);
        (result, out)
    }

    /// Lật trạng thái gõ tiếng Việt: chốt từ đang gõ, dọn trạng thái, rồi hiển thị nhãn VN/EN.
    fn toggle_vietnamese(&mut self, out: &mut Vec<Action>) {
        if self.get_raw_key_len() > 0 {
            let s = self.get_preedit_string();
            self.commit_preedit_and_reset(&s, out);
        } else {
            self.reset_preeditor();
        }
        self.vietnamese_enabled = !self.vietnamese_enabled;
        let label = if self.vietnamese_enabled { "VN" } else { "EN" };
        out.push(Action::UpdateAuxiliary {
            text: label.to_string(),
            visible: true,
        });
    }

    // ----- chế độ tra cứu emoji / hex -----

    fn ensure_emoji_trie(&mut self) {
        if self.emoji_trie.is_none() {
            self.emoji_trie = Some(load_bundled());
        }
    }

    fn emoji_push(&mut self, c: char) {
        self.ensure_emoji_trie();
        let trie = self.emoji_trie.as_ref().unwrap();
        self.emoji.push(c, trie);
    }

    fn emoji_backspace(&mut self) -> bool {
        self.ensure_emoji_trie();
        let trie = self.emoji_trie.as_ref().unwrap();
        self.emoji.backspace(trie)
    }

    fn emoji_raw(&self) -> String {
        format!(":{}", self.emoji.query())
    }

    /// Truy vấn đang ở dạng mã hex (có tiền tố) -> chữ số là một phần mã, không phải nhãn chọn.
    fn query_is_hex(&self) -> bool {
        let q = self.emoji.query();
        q.starts_with("u+") || q.starts_with("U+") || q.starts_with("\\u")
    }

    fn emit_emoji_ui(&self, out: &mut Vec<Action>) {
        out.push(Action::UpdateAuxiliary {
            text: self.emoji_raw(),
            visible: true,
        });
        let candidates = self.emoji.candidates().to_vec();
        let visible = !candidates.is_empty();
        out.push(Action::UpdateLookupTable {
            candidates,
            cursor: self.emoji.cursor() as u32,
            page_size: EMOJI_PAGE_SIZE as u32,
            visible,
        });
    }

    fn finish_emoji(&mut self, commit: &str, out: &mut Vec<Action>) {
        self.emoji.close();
        out.push(Action::HideLookupTable);
        out.push(Action::HideAuxiliary);
        out.push(Action::HidePreedit);
        if !commit.is_empty() {
            out.push(Action::CommitText(self.encode_text(commit)));
        }
    }

    fn emoji_process_key(&mut self, key_val: u32, state: u32, out: &mut Vec<Action>) -> bool {
        let key_rune = char::from_u32(key_val).unwrap_or('\0');
        match key_val {
            IBUS_ESCAPE => {
                let raw = self.emoji_raw();
                self.finish_emoji(&raw, out);
                return true;
            }
            IBUS_RETURN | IBUS_SPACE | IBUS_TAB => {
                if let Some(sel) = self.emoji.selected().map(|s| s.to_string()) {
                    self.finish_emoji(&sel, out);
                    return true;
                }
                // Không có ứng viên: commit nguyên văn ":query". Nếu truy vấn rỗng (chỉ ":"), để
                // phím space/enter đi qua để không nuốt mất ký tự của người dùng.
                let was_empty = self.emoji.query().is_empty();
                let raw = self.emoji_raw();
                self.finish_emoji(&raw, out);
                return !was_empty;
            }
            IBUS_BACKSPACE => {
                if self.emoji_backspace() {
                    self.emit_emoji_ui(out);
                } else {
                    self.finish_emoji("", out);
                }
                return true;
            }
            IBUS_UP | IBUS_LEFT => {
                self.emoji.move_cursor(-1);
                self.emit_emoji_ui(out);
                return true;
            }
            IBUS_DOWN | IBUS_RIGHT => {
                self.emoji.move_cursor(1);
                self.emit_emoji_ui(out);
                return true;
            }
            IBUS_PAGE_UP => {
                self.emoji.page(-1);
                self.emit_emoji_ui(out);
                return true;
            }
            IBUS_PAGE_DOWN => {
                self.emoji.page(1);
                self.emit_emoji_ui(out);
                return true;
            }
            _ => {}
        }
        // Chọn theo nhãn số (1-9) khi không đang gõ mã hex.
        if key_rune.is_ascii_digit() && !self.query_is_hex() {
            let d = (key_rune as u8 - b'0') as usize;
            if let Some(sel) = self.emoji.select_digit(d) {
                self.finish_emoji(&sel, out);
            }
            return true; // nuốt phím số dù có chọn được hay không
        }
        // Ký tự truy vấn: chữ cái, chữ số (khi gõ hex), '+', '\'.
        if is_valid_state(state) && is_emoji_query_char(key_rune) {
            self.emoji_push(key_rune);
            self.emit_emoji_ui(out);
            return true;
        }
        // Phím khác: thoát, commit nguyên văn ":query", để IBus tự chuyển tiếp phím gốc.
        let raw = self.emoji_raw();
        self.finish_emoji(&raw, out);
        false
    }

    /// Vẽ lại văn bản trên màn hình ở chế độ sửa lỗi: tính phần khác biệt so với lần hiển thị
    /// trước rồi phát các action xóa lùi + commit phần đuôi.
    fn redraw_corrected(&mut self, out: &mut Vec<Action>) {
        let new_text = self.encode_text(&self.get_preedit_string());
        let corr = diff_correction(&self.previous_text, &new_text);
        out.extend(correction_actions(self.input_mode, &corr));
        self.previous_text = new_text;
    }

    /// Đường xử lý phím cho các chế độ sửa lỗi bằng backspace (chuyển thể `engine_backspace.go`).
    /// Văn bản được commit thẳng ra ứng dụng; mỗi lần biến đổi thay đổi, ta xóa lùi phần đã sai và
    /// gõ lại phần đuôi thay vì cập nhật vùng preedit.
    fn backspace_process_key_event(
        &mut self,
        key_val: u32,
        _key_code: u32,
        state: u32,
        out: &mut Vec<Action>,
    ) -> bool {
        let key_rune = char::from_u32(key_val).unwrap_or('\0');
        let raw_key_len = self.get_raw_key_len();

        // Phím không xử lý được khi buffer rỗng -> cho đi qua (thanh địa chỉ, phím tắt, ...).
        if !self.preeditor.can_process_key(key_rune)
            && raw_key_len == 0
            && key_val != IBUS_BACKSPACE
        {
            return false;
        }

        if key_val == IBUS_BACKSPACE {
            if raw_key_len > 0 {
                self.preeditor.remove_last_char(true);
                self.redraw_corrected(out);
                return true;
            }
            self.previous_text.clear();
            return false;
        }

        let is_printable = self.is_printable_key(state, key_val);
        if is_printable && self.preeditor.can_process_key(key_rune) {
            let mut kr = key_rune;
            if state & IBUS_LOCK_MASK != 0 {
                kr = self.to_upper(kr);
            }
            let input_mode = self.get_input_mode();
            self.preeditor.process_key(kr, input_mode);
            self.redraw_corrected(out);
            return true;
        }

        // Ký tự ngắt từ / không xử lý được: từ hiện tại đã nằm trên màn hình rồi, chỉ cần reset
        // trạng thái và để IBus tự chuyển tiếp phím (space, dấu câu, ...).
        self.preeditor.reset();
        self.previous_text.clear();
        false
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

        if key_val == IBUS_BACKSPACE {
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

        if key_val == IBUS_TAB {
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

/// Nạp từ điển kiểm tra chính tả khi bật cờ `IB_SPELL_CHECK_WITH_DICTS`: bộ từ khởi đầu đóng kèm,
/// phủ thêm từ điển người dùng (`~/.config/pinakey/dict.txt`) nếu có.
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

fn build_preeditor(config: &Config) -> PinaKeyEngine {
    let pairs: Vec<(String, String)> = config
        .input_method_definitions
        .get(&config.input_method)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    let im = build_input_method_from_pairs(&config.input_method, &pairs);
    core::new_engine(im, config.flags)
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

/// Ký tự được phép trong truy vấn emoji/hex: chữ cái, chữ số, và `+` `\` cho mã hex (`u+`, `\u`).
fn is_emoji_query_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '+' || c == '\\'
}

fn is_valid_state(state: u32) -> bool {
    state & IBUS_CONTROL_MASK == 0
        && state & IBUS_MOD1_MASK == 0
        && state & IBUS_MOD4_MASK == 0
        && state & IBUS_IGNORED_MASK == 0
        && state & IBUS_SUPER_MASK == 0
        && state & IBUS_HYPER_MASK == 0
        && state & IBUS_META_MASK == 0
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
    fn backspace_removes_char() {
        let mut core = EngineCore::new(default_cfg());
        type_keys(&mut core, "vieetj"); // ra "việt"
        let (handled, actions) = core.process_key_event(IBUS_BACKSPACE, 0, 0);
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

    // ----- chế độ sửa lỗi bằng backspace -----

    /// Mô phỏng nội dung trên màn hình ứng dụng: `CommitText` nối thêm, mọi dạng xóa lùi đều bỏ N
    /// ký tự cuối. Nhờ vậy ta kiểm tra được kết quả cuối cùng của một chuỗi correction.
    fn apply_corrections(actions: &[Action]) -> String {
        let mut screen: Vec<char> = Vec::new();
        for a in actions {
            match a {
                Action::CommitText(t) => screen.extend(t.chars()),
                Action::ForwardBackspaces(n) | Action::FakeBackspaces(n) => {
                    for _ in 0..*n {
                        screen.pop();
                    }
                }
                Action::DeleteSurroundingText { nchars, .. } => {
                    for _ in 0..*nchars {
                        screen.pop();
                    }
                }
                _ => {}
            }
        }
        screen.into_iter().collect()
    }

    fn backspace_cfg(mode: i32) -> Config {
        let mut c = default_cfg();
        c.default_input_mode = mode;
        c
    }

    #[test]
    fn backspace_mode_commits_with_corrections() {
        let mut core = EngineCore::new(backspace_cfg(cfg::BACKSPACE_FORWARDING_IM));
        let actions = type_keys(&mut core, "vieetj");
        assert_eq!(apply_corrections(&actions), "việt");
        // Chế độ này không dùng vùng preedit.
        assert!(!actions
            .iter()
            .any(|a| matches!(a, Action::UpdatePreedit { .. })));
    }

    #[test]
    fn backspace_mode_uses_forward_key_events() {
        let mut core = EngineCore::new(backspace_cfg(cfg::BACKSPACE_FORWARDING_IM));
        let actions = type_keys(&mut core, "vieetj");
        // Phải có ít nhất một lần phát phím BackSpace (sửa ee->ê và thêm dấu).
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::ForwardBackspaces(_))));
        assert!(!actions
            .iter()
            .any(|a| matches!(a, Action::FakeBackspaces(_))));
    }

    #[test]
    fn backspace_mode_xtest_uses_fake_backspaces() {
        let mut core = EngineCore::new(backspace_cfg(cfg::XTEST_FAKE_KEY_EVENT_IM));
        let actions = type_keys(&mut core, "vieetj");
        assert_eq!(apply_corrections(&actions), "việt");
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::FakeBackspaces(_))));
    }

    #[test]
    fn backspace_mode_surrounding_uses_delete_surrounding() {
        let mut core = EngineCore::new(backspace_cfg(cfg::SURROUNDING_TEXT_IM));
        let actions = type_keys(&mut core, "vieetj");
        assert_eq!(apply_corrections(&actions), "việt");
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::DeleteSurroundingText { .. })));
    }

    #[test]
    fn backspace_mode_backspace_key_deletes_char() {
        let mut core = EngineCore::new(backspace_cfg(cfg::BACKSPACE_FORWARDING_IM));
        let mut actions = type_keys(&mut core, "vieetj"); // -> việt
        let (handled, bs) = core.process_key_event(IBUS_BACKSPACE, 0, 0);
        assert!(handled);
        actions.extend(bs);
        assert_eq!(apply_corrections(&actions), "việ");
    }

    #[test]
    fn backspace_mode_word_break_passes_through() {
        let mut core = EngineCore::new(backspace_cfg(cfg::BACKSPACE_FORWARDING_IM));
        type_keys(&mut core, "vieetj");
        // space hoàn tất từ; IBus tự chuyển tiếp nên engine không "handle".
        let (handled, _sp) = core.process_key_event(' ' as u32, 0, 0);
        assert!(!handled);
        // Sau ngắt từ, gõ từ mới bắt đầu lại từ chuỗi rỗng.
        let actions = type_keys(&mut core, "as"); // "á"
        assert_eq!(apply_corrections(&actions), "á");
    }

    // ----- bảng tra cứu emoji / hex -----

    fn last_lookup(actions: &[Action]) -> Option<(Vec<String>, bool)> {
        actions.iter().rev().find_map(|a| match a {
            Action::UpdateLookupTable {
                candidates,
                visible,
                ..
            } => Some((candidates.clone(), *visible)),
            _ => None,
        })
    }

    #[test]
    fn colon_enters_emoji_mode() {
        let mut core = EngineCore::new(default_cfg());
        let (handled, actions) = core.process_key_event(IBUS_COLON, 0, 0);
        assert!(handled);
        // Hiển thị auxiliary ":" báo đang ở chế độ emoji.
        assert!(actions
            .iter()
            .any(|a| matches!(a, Action::UpdateAuxiliary { text, .. } if text == ":")));
    }

    #[test]
    fn emoji_keyword_lists_candidates() {
        let mut core = EngineCore::new(default_cfg());
        let actions = type_keys(&mut core, ":grin");
        let (cands, visible) = last_lookup(&actions).expect("phải có lookup table");
        assert!(visible);
        assert!(cands.contains(&"😀".to_string()));
    }

    #[test]
    fn emoji_space_commits_selected() {
        let mut core = EngineCore::new(default_cfg());
        let actions = type_keys(&mut core, ":grin");
        let (cands, _v) = last_lookup(&actions).unwrap();
        let first = cands[0].clone();
        let (handled, sp) = core.process_key_event(IBUS_SPACE, 0, 0);
        assert!(handled);
        assert!(commits(&sp).contains(&first));
        // Bảng tra cứu được ẩn sau khi chọn.
        assert!(sp.iter().any(|a| matches!(a, Action::HideLookupTable)));
    }

    #[test]
    fn hex_commits_decoded_char() {
        let mut core = EngineCore::new(default_cfg());
        type_keys(&mut core, ":u+2764");
        let (handled, sp) = core.process_key_event(IBUS_SPACE, 0, 0);
        assert!(handled);
        assert!(commits(&sp).iter().any(|c| c == "❤"));
    }

    #[test]
    fn emoji_digit_selects_candidate() {
        let mut core = EngineCore::new(default_cfg());
        let actions = type_keys(&mut core, ":grin");
        let (cands, _v) = last_lookup(&actions).unwrap();
        if cands.len() < 2 {
            return; // bộ dữ liệu chỉ có 1 ứng viên: bỏ qua nhánh chọn theo số
        }
        let second = cands[1].clone();
        let (handled, sp) = core.process_key_event('2' as u32, 0, 0);
        assert!(handled);
        assert!(commits(&sp).contains(&second));
    }

    #[test]
    fn emoji_backspace_to_empty_exits() {
        let mut core = EngineCore::new(default_cfg());
        type_keys(&mut core, ":g");
        let (handled, sp) = core.process_key_event(IBUS_BACKSPACE, 0, 0);
        assert!(handled);
        assert!(sp.iter().any(|a| matches!(a, Action::HideLookupTable)));
        // Đã thoát chế độ emoji: ':' tiếp theo lại mở mới (không tích lũy).
        let (_h, a2) = core.process_key_event(IBUS_COLON, 0, 0);
        assert!(a2
            .iter()
            .any(|x| matches!(x, Action::UpdateAuxiliary { text, .. } if text == ":")));
    }

    #[test]
    fn emoji_escape_commits_raw() {
        let mut core = EngineCore::new(default_cfg());
        type_keys(&mut core, ":grin");
        let (handled, sp) = core.process_key_event(IBUS_ESCAPE, 0, 0);
        assert!(handled);
        assert!(commits(&sp).iter().any(|c| c == ":grin"));
    }

    // ----- phím tắt -----

    #[test]
    fn toggle_vietnamese_disables_then_enables() {
        let cfg = default_cfg();
        let mod_mask = crate::shortcuts::decode_modifier(cfg.shortcuts[0]);
        let toggle_key = cfg.shortcuts[1];
        let mut core = EngineCore::new(cfg);
        // Đang bật: gõ tạo preedit.
        let a = type_keys(&mut core, "viet");
        assert!(last_preedit(&a).is_some());
        core.reset_preeditor();
        // Tắt tiếng Việt bằng phím tắt.
        let (h, _) = core.process_key_event(toggle_key, 0, mod_mask);
        assert!(h);
        // Giờ phím thường được cho đi qua, engine không xử lý.
        let (h2, acts) = core.process_key_event('v' as u32, 0, 0);
        assert!(!h2);
        assert!(acts.is_empty());
        // Bật lại -> biến đổi hoạt động trở lại.
        let (h3, _) = core.process_key_event(toggle_key, 0, mod_mask);
        assert!(h3);
        let a2 = type_keys(&mut core, "vieetj");
        assert_eq!(last_preedit(&a2).as_deref(), Some("việt"));
    }

    #[test]
    fn toggle_commits_in_progress_word() {
        let cfg = default_cfg();
        let mod_mask = crate::shortcuts::decode_modifier(cfg.shortcuts[0]);
        let toggle_key = cfg.shortcuts[1];
        let mut core = EngineCore::new(cfg);
        type_keys(&mut core, "vieetj"); // "việt" đang trong preedit
        let (_h, acts) = core.process_key_event(toggle_key, 0, mod_mask);
        // Từ đang gõ được chốt trước khi tắt.
        assert!(commits(&acts).iter().any(|c| c == "việt"));
    }

    #[test]
    fn restore_word_shortcut_sets_flag() {
        let mut cfg = default_cfg();
        cfg.shortcuts[2] = 2; // Shift
        cfg.shortcuts[3] = 0x72; // 'r'
        let mut core = EngineCore::new(cfg);
        let (h, _) = core.process_key_event(0x72, 0, IBUS_SHIFT_MASK);
        assert!(h);
        assert!(core.should_restore_key_strokes);
    }

    // ----- kiểm tra chính tả dựa trên từ điển -----

    #[test]
    fn dictionary_rescues_current_word() {
        let mut cfg = default_cfg();
        cfg.ib_flags |= cfg::IB_SPELL_CHECK_WITH_DICTS;
        let mut core = EngineCore::new(cfg);
        type_keys(&mut core, "loz");
        let vn = core.get_processed_string(mode::VIETNAMESE | mode::LOWER_CASE);
        assert!(!vn.is_empty());
        let mut dict = core::Dictionary::new();
        dict.add(&vn);
        core.set_dictionary(dict);
        // Từ điển công nhận -> hợp lệ, dù bộ quy tắc có thể từ chối.
        assert!(core.current_word_is_valid());
    }

    #[test]
    fn dictionary_gated_by_flag() {
        let cfg = default_cfg(); // không bật IB_SPELL_CHECK_WITH_DICTS
        let mut core = EngineCore::new(cfg);
        type_keys(&mut core, "loz");
        let vn = core.get_processed_string(mode::VIETNAMESE | mode::LOWER_CASE);
        let mut dict = core::Dictionary::new();
        dict.add(&vn);
        core.set_dictionary(dict);
        // Cờ tắt -> từ điển bị bỏ qua: tính hợp lệ theo đúng bộ quy tắc.
        assert_eq!(core.current_word_is_valid(), core.preeditor.is_valid(true));
    }

    // ----- menu thuộc tính -----

    #[test]
    fn property_activate_toggles_vn() {
        let mut core = EngineCore::new(default_cfg());
        assert!(core.is_vietnamese_enabled());
        core.on_property_activate("vn_toggle", 0);
        assert!(!core.is_vietnamese_enabled());
        core.on_property_activate("vn_toggle", 0);
        assert!(core.is_vietnamese_enabled());
    }

    #[test]
    fn property_activate_open_settings_launches() {
        let mut core = EngineCore::new(default_cfg());
        let actions = core.on_property_activate(crate::props::OPEN_SETTINGS_KEY, 0);
        assert_eq!(actions, vec![Action::LaunchSettings]);
    }

    #[test]
    fn property_activate_switches_input_method() {
        let mut core = EngineCore::new(default_cfg());
        assert_eq!(core.config.input_method, "Telex");
        core.on_property_activate("im_VNI", 0);
        assert_eq!(core.config.input_method, "VNI");
        let props = core.build_props();
        assert!(props.iter().any(|p| p.key == "im_VNI" && p.checked));
        assert!(props.iter().any(|p| p.key == "im_Telex" && !p.checked));
    }
}
