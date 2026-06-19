//! Tra cứu emoji bằng trie và bảng macro — bản chuyển sang Rust của `emoji.go`, `trie.go`, `mactab.go`.

mod emoji;
mod mactab;
mod trie;

pub use emoji::{load_bundled, load_emojione, load_emojione_from_str, EmojiEngine};
pub use mactab::MacroTable;
pub use trie::TrieNode;

#[cfg(test)]
mod tests {
    //! Chuyển từ `emoji_test.go`.
    use super::*;

    fn load() -> TrieNode {
        load_emojione_from_str(include_str!("../data/emojione.json")).unwrap()
    }

    #[test]
    fn test_emoji_find_result() {
        let trie = load();
        let be = EmojiEngine::new(&trie);
        assert!(be.match_string(":'"));
        assert!(be.match_string(":')"));
        assert!(be.match_string("gri"));
        assert!(be.match_string("grin"));
    }

    #[test]
    fn test_load_bundled() {
        let trie = load_bundled();
        let be = EmojiEngine::new(&trie);
        // Bộ dữ liệu nhúng phải tra được emoji quen thuộc.
        assert!(be.match_string("grin"));
        assert!(be.filter("grin").contains(&"😀".to_string()));
    }

    #[test]
    fn test_filter_emoji() {
        let trie = load();
        let be = EmojiEngine::new(&trie);
        let grinnings = be.filter(":')");
        assert!(grinnings.contains(&"😂".to_string()));
        let grinnings2 = be.filter(":");
        assert!(grinnings2.contains(&"😂".to_string()));
        let grinnings3 = be.filter("grin");
        assert!(grinnings3.contains(&"😀".to_string()));
    }

    #[test]
    fn test_trie_basic() {
        let mut t = TrieNode::new();
        t.insert("abc", "X");
        t.insert("abc", "Y");
        t.insert("abd", "Z");
        let lookup = t.find_prefix("ab").unwrap();
        assert_eq!(lookup.get("abc").unwrap(), "X:Y");
        assert_eq!(lookup.get("abd").unwrap(), "Z");
        assert!(t.find_prefix("zzz").is_none());
    }

    #[test]
    fn test_macro_table() {
        // Ghi một file macro tạm rồi nạp nó.
        let dir = std::env::temp_dir();
        let path = dir.join("pinakey_macro_test.txt");
        std::fs::write(&path, "; comment\nVN : Việt Nam\nHELLO:xin chào\n").unwrap();
        let mut m = MacroTable::new(true);
        m.load_from_file(path.to_str().unwrap()).unwrap();
        assert!(m.has_key("vn"));
        assert!(m.has_key("VN")); // auto-capitalize chuyển khóa tra cứu về chữ thường
        assert_eq!(m.get_text("hello"), "xin chào");
        assert!(m.has_prefix("he"));
        assert!(!m.has_prefix("zzz"));
    }
}
