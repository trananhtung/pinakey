//! Trạng thái bảng tra cứu (lookup table) cho **emoji** và **hexadecimal** — chuyển thể ý tưởng từ
//! `engine_emoji.go` của ibus-bamboo.
//!
//! Người dùng gõ `:` ở đầu từ để vào chế độ emoji, sau đó gõ từ khóa (`:grin`) để lọc emoji, hoặc
//! gõ mã codepoint Unicode có tiền tố (`:u+1f600`, `:❤`) để chèn ký tự theo mã hex. Phần state
//! machine này thuần (không phụ thuộc D-Bus) nên được unit-test đầy đủ; `EngineCore` lái nó và dịch
//! trạng thái thành các `Action`.

use pinakey_emoji::{EmojiEngine, TrieNode};

/// Số ứng viên hiển thị mỗi trang của lookup table.
pub const EMOJI_PAGE_SIZE: usize = 9;
/// Giới hạn tổng số ứng viên để tránh dựng bảng quá lớn.
const MAX_CANDIDATES: usize = 90;

/// Phân tích một chuỗi mã hex Unicode **có tiền tố** (`u+`, `U+`, hoặc `\u`) thành ký tự tương ứng.
///
/// Bắt buộc có tiền tố để không nuốt mất các từ khóa emoji vốn cũng toàn chữ số hex (ví dụ
/// `face`, `dead`, `cafe`). Trả về `None` nếu thiếu tiền tố, phần còn lại không phải hex thuần,
/// hoặc không phải codepoint hợp lệ (vượt `0x10FFFF` hoặc nằm trong vùng surrogate).
pub fn hex_to_char(s: &str) -> Option<char> {
    let t = s.trim();
    let hex = t
        .strip_prefix("\\u")
        .or_else(|| t.strip_prefix("U+"))
        .or_else(|| t.strip_prefix("u+"))?;
    if hex.is_empty() || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
}

/// Dựng danh sách ứng viên cho `query`: nếu là mã hex (có tiền tố) thì ký tự giải mã đứng đầu, kế
/// đến là các emoji khớp tiền tố `query` (đã khử trùng lặp), cắt còn tối đa `max`.
pub fn compute_candidates(query: &str, trie: &TrieNode, max: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if query.is_empty() {
        return out;
    }
    if let Some(c) = hex_to_char(query) {
        out.push(c.to_string());
    }
    let engine = EmojiEngine::new(trie);
    for e in engine.filter(query) {
        if out.len() >= max {
            break;
        }
        if !out.contains(&e) {
            out.push(e);
        }
    }
    out.truncate(max);
    out
}

/// Trạng thái phiên tra cứu emoji/hex đang mở.
#[derive(Default)]
pub struct EmojiState {
    active: bool,
    query: String,
    candidates: Vec<String>,
    cursor: usize,
}

impl EmojiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn candidates(&self) -> &[String] {
        &self.candidates
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Mở phiên tra cứu với truy vấn rỗng.
    pub fn start(&mut self) {
        self.active = true;
        self.query.clear();
        self.candidates.clear();
        self.cursor = 0;
    }

    /// Thêm một ký tự vào truy vấn rồi tính lại ứng viên (đưa con trỏ về đầu).
    pub fn push(&mut self, c: char, trie: &TrieNode) {
        self.query.push(c);
        self.recompute(trie);
    }

    /// Xóa ký tự cuối của truy vấn. Trả về `false` nếu truy vấn đã rỗng (người gọi nên thoát chế độ).
    pub fn backspace(&mut self, trie: &TrieNode) -> bool {
        if self.query.pop().is_none() {
            return false;
        }
        self.recompute(trie);
        !self.query.is_empty()
    }

    fn recompute(&mut self, trie: &TrieNode) {
        self.candidates = compute_candidates(&self.query, trie, MAX_CANDIDATES);
        self.cursor = 0;
    }

    /// Vị trí bắt đầu của trang hiện tại.
    pub fn page_start(&self) -> usize {
        (self.cursor / EMOJI_PAGE_SIZE) * EMOJI_PAGE_SIZE
    }

    /// Dời con trỏ `delta` bước (âm = về trước), kẹp trong [0, len).
    pub fn move_cursor(&mut self, delta: isize) {
        if self.candidates.is_empty() {
            self.cursor = 0;
            return;
        }
        let max = self.candidates.len() as isize - 1;
        let next = (self.cursor as isize + delta).clamp(0, max);
        self.cursor = next as usize;
    }

    /// Lật `delta` trang (âm = trang trước).
    pub fn page(&mut self, delta: isize) {
        self.move_cursor(delta * EMOJI_PAGE_SIZE as isize);
    }

    /// Emoji đang được chọn (tại con trỏ).
    pub fn selected(&self) -> Option<&str> {
        self.candidates.get(self.cursor).map(|s| s.as_str())
    }

    /// Chọn theo nhãn số `1..=EMOJI_PAGE_SIZE` trên trang hiện tại; trả về ứng viên nếu hợp lệ.
    pub fn select_digit(&self, digit: usize) -> Option<String> {
        if !(1..=EMOJI_PAGE_SIZE).contains(&digit) {
            return None;
        }
        let idx = self.page_start() + (digit - 1);
        self.candidates.get(idx).cloned()
    }

    /// Đóng phiên tra cứu.
    pub fn close(&mut self) {
        self.active = false;
        self.query.clear();
        self.candidates.clear();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_trie() -> TrieNode {
        let mut t = TrieNode::new();
        t.insert("grin", "😀");
        t.insert("grinning", "😀");
        t.insert("joy", "😂");
        t.insert("heart", "❤");
        t
    }

    #[test]
    fn hex_requires_prefix() {
        assert_eq!(hex_to_char("u+1f600"), Some('😀'));
        assert_eq!(hex_to_char("U+2764"), Some('❤'));
        assert_eq!(hex_to_char("\\u00e9"), Some('é'));
        // không tiền tố -> None (giữ nguyên là từ khóa emoji)
        assert_eq!(hex_to_char("1f600"), None);
        assert_eq!(hex_to_char("face"), None);
    }

    #[test]
    fn hex_rejects_invalid() {
        assert_eq!(hex_to_char("u+zzzz"), None);
        assert_eq!(hex_to_char("u+"), None);
        assert_eq!(hex_to_char("u+110000"), None); // vượt 0x10FFFF
        assert_eq!(hex_to_char("u+d800"), None); // surrogate
    }

    #[test]
    fn candidates_emoji_dedup() {
        let trie = test_trie();
        let c = compute_candidates("grin", &trie, MAX_CANDIDATES);
        // "grin" và "grinning" cùng map 😀 -> chỉ còn một.
        assert_eq!(c, vec!["😀".to_string()]);
    }

    #[test]
    fn candidates_hex_first() {
        let trie = test_trie();
        let c = compute_candidates("u+2764", &trie, MAX_CANDIDATES);
        assert_eq!(c.first().map(|s| s.as_str()), Some("❤"));
    }

    #[test]
    fn candidates_none_for_unknown() {
        let trie = test_trie();
        assert!(compute_candidates("zzz", &trie, MAX_CANDIDATES).is_empty());
    }

    #[test]
    fn candidates_respects_max() {
        let trie = test_trie();
        let c = compute_candidates("", &trie, MAX_CANDIDATES);
        // truy vấn rỗng -> không ứng viên (tránh bung toàn bộ trie).
        assert!(c.is_empty());
    }

    #[test]
    fn push_builds_query_and_candidates() {
        let trie = test_trie();
        let mut s = EmojiState::new();
        s.start();
        for c in "grin".chars() {
            s.push(c, &trie);
        }
        assert_eq!(s.query(), "grin");
        assert_eq!(s.candidates(), &["😀".to_string()]);
        assert_eq!(s.cursor(), 0);
    }

    #[test]
    fn backspace_shrinks_query() {
        let trie = test_trie();
        let mut s = EmojiState::new();
        s.start();
        for c in "joy".chars() {
            s.push(c, &trie);
        }
        assert!(s.backspace(&trie));
        assert_eq!(s.query(), "jo");
        // backspace tới rỗng -> trả false (báo người gọi thoát).
        assert!(s.backspace(&trie));
        assert!(!s.backspace(&trie));
        assert_eq!(s.query(), "");
    }

    #[test]
    fn cursor_clamps() {
        let trie = test_trie();
        let mut s = EmojiState::new();
        s.start();
        for c in "grin".chars() {
            s.push(c, &trie);
        }
        s.move_cursor(-1);
        assert_eq!(s.cursor(), 0);
        s.move_cursor(5);
        assert_eq!(s.cursor(), 0); // chỉ có 1 ứng viên
    }

    #[test]
    fn paging_moves_by_page_size() {
        // dựng nhiều ứng viên giả qua trie để kiểm tra phân trang.
        let mut t = TrieNode::new();
        for i in 0..20 {
            t.insert(&format!("e{i:02}"), &format!("E{i}"));
        }
        let mut s = EmojiState::new();
        s.start();
        s.push('e', &t);
        assert!(s.candidates().len() >= 20);
        assert_eq!(s.page_start(), 0);
        s.page(1);
        assert_eq!(s.page_start(), EMOJI_PAGE_SIZE);
        s.page(-1);
        assert_eq!(s.page_start(), 0);
    }

    #[test]
    fn select_digit_within_page() {
        let mut t = TrieNode::new();
        for i in 0..20 {
            t.insert(&format!("e{i:02}"), &format!("E{i}"));
        }
        let mut s = EmojiState::new();
        s.start();
        s.push('e', &t);
        let first = s.candidates()[0].clone();
        assert_eq!(s.select_digit(1), Some(first));
        // số vượt số ứng viên trên trang -> None
        assert_eq!(s.select_digit(0), None);
    }
}
