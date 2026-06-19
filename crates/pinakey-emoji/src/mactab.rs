//! Bảng macro — chuyển từ `mactab.go`.
//!
//! Bản Go khởi tạo một goroutine để liên tục kiểm tra mtime của file macro; ở đây `load_from_file`
//! được gọi tường minh và chính sách nạp (lại) do bên gọi quyết định (lớp engine IBus).

use std::collections::HashMap;

#[derive(Default)]
pub struct MacroTable {
    enable: bool,
    auto_capitalize_macro: bool,
    m_table: HashMap<String, String>,
}

impl MacroTable {
    pub fn new(auto_capitalize_macro: bool) -> MacroTable {
        MacroTable {
            enable: false,
            auto_capitalize_macro,
            m_table: HashMap::new(),
        }
    }

    pub fn load_from_file(&mut self, macro_file_name: &str) -> std::io::Result<()> {
        let content = std::fs::read_to_string(macro_file_name)?;
        self.m_table = HashMap::new();
        for line in content.lines() {
            let s = line.trim();
            if s.is_empty() || s.starts_with(';') || s.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = s.splitn(2, ':').collect();
            if parts.len() == 2 {
                let mut key = parts[0].trim().to_string();
                if self.auto_capitalize_macro {
                    key = key.to_lowercase();
                }
                self.m_table.insert(key, parts[1].trim().to_string());
            }
        }
        Ok(())
    }

    pub fn get_text(&self, key: &str) -> String {
        let k = if self.auto_capitalize_macro {
            key.to_lowercase()
        } else {
            key.to_string()
        };
        self.m_table.get(&k).cloned().unwrap_or_default()
    }

    pub fn has_key(&self, key: &str) -> bool {
        let k = if self.auto_capitalize_macro {
            key.to_lowercase()
        } else {
            key.to_string()
        };
        self.m_table.get(&k).is_some_and(|v| !v.is_empty())
    }

    pub fn has_prefix(&self, key: &str) -> bool {
        if self.m_table.get(key).is_some_and(|v| !v.is_empty()) {
            return true;
        }
        self.m_table.keys().any(|k| k.starts_with(key))
    }

    pub fn is_enabled(&self) -> bool {
        self.enable
    }

    pub fn set_enabled(&mut self, enable: bool) {
        self.enable = enable;
        if !enable {
            self.m_table = HashMap::new();
        }
    }

    pub fn set_auto_capitalize(&mut self, v: bool) {
        self.auto_capitalize_macro = v;
    }
}
