//! Trie tiền tố — chuyển từ `trie.go`.

use std::collections::HashMap;

#[derive(Default)]
pub struct TrieNode {
    is_word: bool,
    value: String,
    children: HashMap<char, TrieNode>,
}

impl TrieNode {
    pub fn new() -> TrieNode {
        TrieNode::default()
    }

    /// Chèn `word` ánh xạ tới `value`. Nhiều value cho cùng một word được nối với nhau bằng ':'
    /// (giống `InsertTrie` trong Go).
    pub fn insert(&mut self, word: &str, value: &str) {
        let mut node = self;
        for c in word.chars() {
            node = node.children.entry(c).or_default();
        }
        if node.value.is_empty() {
            node.value = value.to_string();
        } else {
            node.value.push(':');
            node.value.push_str(value);
        }
        node.is_word = true;
    }

    fn dfs(&self, lookup: &mut HashMap<String, String>, s: &str) {
        if self.is_word {
            lookup.insert(s.to_string(), self.value.clone());
        }
        for (chr, t) in &self.children {
            let mut key = s.to_string();
            key.push(*chr);
            t.dfs(lookup, &key);
        }
    }

    /// Tất cả các cặp `word -> value` nằm dưới `prefix`, hoặc `None` nếu prefix không tồn tại
    /// (giống `FindPrefix` trong Go trả về map nil).
    pub fn find_prefix(&self, prefix: &str) -> Option<HashMap<String, String>> {
        let mut node = self;
        for c in prefix.chars() {
            match node.children.get(&c) {
                Some(n) => node = n,
                None => return None,
            }
        }
        let mut lookup = HashMap::new();
        node.dfs(&mut lookup, prefix);
        Some(lookup)
    }
}
