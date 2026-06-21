//! Từ điển kiểm tra chính tả dựa trên danh sách từ — tương ứng `IBspellCheckWithDicts` của
//! ibus-bamboo.
//!
//! Khác với kiểm tra theo quy tắc CVC (luôn bật), từ điển cho phép "giải oan" cho những từ hợp lệ
//! mà bộ quy tắc đơn giản từ chối (từ mượn, tên riêng, …). Tra cứu không phân biệt hoa/thường.

use std::collections::HashSet;
use std::io;

/// Tập các từ tiếng Việt hợp lệ.
#[derive(Debug, Clone, Default)]
pub struct Dictionary {
    words: HashSet<String>,
}

impl Dictionary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Nạp từ một chuỗi nhiều dòng: mỗi dòng một từ; bỏ qua dòng trống và dòng chú thích (`#`).
    pub fn load_str(text: &str) -> Self {
        let mut words = HashSet::new();
        for line in text.lines() {
            let w = line.trim();
            if w.is_empty() || w.starts_with('#') {
                continue;
            }
            words.insert(w.to_lowercase());
        }
        Dictionary { words }
    }

    /// Nạp từ một file (UTF-8). Trả lỗi nếu không đọc được.
    pub fn load_file(path: &str) -> io::Result<Self> {
        Ok(Self::load_str(&std::fs::read_to_string(path)?))
    }

    /// Từ điển khởi đầu được đóng kèm trong binary (bộ từ thông dụng).
    pub fn bundled() -> Self {
        Self::load_str(include_str!("../data/words.txt"))
    }

    /// `word` có nằm trong từ điển không (không phân biệt hoa/thường). Chuỗi rỗng luôn `false`.
    pub fn contains(&self, word: &str) -> bool {
        !word.is_empty() && self.words.contains(&word.to_lowercase())
    }

    /// Thêm một từ.
    pub fn add(&mut self, word: &str) {
        self.words.insert(word.to_lowercase());
    }

    /// Gộp thêm các từ từ một từ điển khác (ví dụ phủ từ điển người dùng lên bộ khởi đầu).
    pub fn merge(&mut self, other: &Dictionary) {
        for w in &other.words {
            self.words.insert(w.clone());
        }
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_str_skips_blanks_and_comments() {
        let d = Dictionary::load_str("# chú thích\nviệt\n\n  nam  \n#còn nữa\nchào\n");
        assert_eq!(d.len(), 3);
        assert!(d.contains("việt"));
        assert!(d.contains("nam"));
        assert!(d.contains("chào"));
    }

    #[test]
    fn contains_is_case_insensitive() {
        let d = Dictionary::load_str("Việt\n");
        assert!(d.contains("việt"));
        assert!(d.contains("VIỆT"));
        assert!(d.contains("Việt"));
        assert!(!d.contains("viet"));
        assert!(!d.contains(""));
        assert!(!d.contains("xyz"));
    }

    #[test]
    fn add_and_merge() {
        let mut a = Dictionary::new();
        a.add("Một");
        assert!(a.contains("một"));
        let mut b = Dictionary::new();
        b.add("hai");
        a.merge(&b);
        assert!(a.contains("hai"));
        assert!(a.contains("một"));
    }

    #[test]
    fn bundled_is_nonempty_and_has_common_words() {
        let d = Dictionary::bundled();
        assert!(!d.is_empty());
        assert!(d.contains("việt"));
        assert!(d.contains("chào"));
    }
}
