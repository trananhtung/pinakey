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

use crate::constants::*;

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
}

const VN_CASE_ALL_SMALL: u8 = 1;
const VN_CASE_ALL_CAPITAL: u8 = 2;
const VN_CASE_NO_CHANGE: u8 = 3;

impl EngineCore {
    pub fn new(config: Config) -> EngineCore {
        let preeditor = build_preeditor(&config);
        let macro_table = MacroTable::new(config.ib_flags & cfg::IB_AUTO_CAPITALIZE_MACRO != 0);
        EngineCore {
            preeditor,
            config,
            macro_table,
            should_restore_key_strokes: false,
            last_key_with_shift: false,
            wm_class: String::new(),
        }
    }

    pub fn set_wm_class(&mut self, wm_class: String) {
        self.wm_class = wm_class;
    }

    /// Đặt lại trạng thái soạn thảo bên dưới (tương ứng `Reset` của IBus).
    pub fn reset_preeditor(&mut self) {
        self.preeditor.reset();
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
        // Kiểm tra chính tả dựa trên từ điển (IBspellCheckWithDicts) chưa được chuyển thể; quay về
        // dùng kiểm tra tính hợp lệ theo quy tắc, khớp với tập flag mặc định.
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
}
