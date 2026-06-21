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
        let w = self.wm_class.to_lowercase();
        self.config.english_exclude.iter().any(|p| {
            let p = p.trim().to_lowercase();
            !p.is_empty() && (w == p || w.contains(&p))
        })
    }

    /// Đặt lại trạng thái soạn thảo bên dưới (tương ứng `Reset` của IBus).
    pub fn reset_preeditor(&mut self) {
        self.preeditor.reset();
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
        if key_val == KEY_BACKSPACE || is_word_break_symbol(key_rune) {
            return true;
        }
        if self.get_macro_text().is_some() && key_val == KEY_TAB {
            return true;
        }
        self.preeditor.can_process_key(key_rune)
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
    let pairs: Vec<(String, String)> = config
        .input_method_definitions
        .get(&config.input_method)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
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
