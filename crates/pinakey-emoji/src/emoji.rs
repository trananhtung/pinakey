//! Emoji lookup engine — ported from `emoji.go`.

use crate::trie::TrieNode;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Default)]
struct EmojiOne {
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    ascii: Vec<String>,
}

/// Build the emoji trie from EmojiOne JSON text (the map key is a `-`-separated hex codepoint
/// sequence; ascii forms and keywords become trie keys mapping to the rendered emoji).
pub fn load_emojione_from_str(json: &str) -> Result<TrieNode, serde_json::Error> {
    let map: HashMap<String, EmojiOne> = serde_json::from_str(json)?;
    let mut trie = TrieNode::new();
    for (k, v) in map {
        let mut code_point_str = String::new();
        for code_point in k.split('-') {
            if let Ok(code) = u32::from_str_radix(code_point, 16) {
                if let Some(c) = char::from_u32(code) {
                    code_point_str.push(c);
                }
            }
        }
        for ascii in &v.ascii {
            trie.insert(ascii, &code_point_str);
        }
        for keyword in &v.keywords {
            trie.insert(keyword, &code_point_str);
        }
    }
    Ok(trie)
}

pub fn load_emojione(path: &str) -> std::io::Result<TrieNode> {
    let data = std::fs::read_to_string(path)?;
    load_emojione_from_str(&data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Tracks the keys typed during an emoji query (`EmojiEngine` in Go).
pub struct EmojiEngine<'a> {
    trie: &'a TrieNode,
    keys: Vec<char>,
}

impl<'a> EmojiEngine<'a> {
    pub fn new(trie: &'a TrieNode) -> Self {
        EmojiEngine {
            trie,
            keys: Vec::new(),
        }
    }

    pub fn match_string(&self, s: &str) -> bool {
        self.trie.find_prefix(s).is_some()
    }

    pub fn filter(&self, s: &str) -> Vec<String> {
        let mut code_points = Vec::new();
        let lookup = match self.trie.find_prefix(s) {
            Some(l) => l,
            None => return code_points,
        };
        let mut keys: Vec<String> = lookup.keys().cloned().collect();
        keys.sort();
        for name in &keys {
            let mut cps: Vec<String> = lookup[name].split(':').map(|x| x.to_string()).collect();
            cps.sort();
            for cp in cps {
                code_points.push(cp);
            }
        }
        code_points
    }

    pub fn process_key(&mut self, key: char) {
        self.keys.push(key);
    }

    pub fn get_raw_string(&self) -> String {
        self.keys.iter().collect()
    }

    pub fn reset(&mut self) {
        self.keys.clear();
    }

    pub fn query(&self) -> Vec<String> {
        self.filter(&self.get_raw_string())
    }

    pub fn remove_last_key(&mut self) {
        self.keys.pop();
    }
}
