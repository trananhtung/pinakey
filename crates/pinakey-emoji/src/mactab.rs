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
        let k = if self.auto_capitalize_macro {
            key.to_lowercase()
        } else {
            key.to_string()
        };
        if self.m_table.get(&k).is_some_and(|v| !v.is_empty()) {
            return true;
        }
        self.m_table.keys().any(|t| t.starts_with(&k))
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

/// Format strftime mặc định cho `$DATE` — kiểu Việt Nam `02/07/2026`.
pub const DEFAULT_DATE_FORMAT: &str = "%d/%m/%Y";
/// Format strftime mặc định cho `$TIME` — 24 giờ `14:30`.
pub const DEFAULT_TIME_FORMAT: &str = "%H:%M";

/// Thay placeholder động trong giá trị macro (issue #64) **tại thời điểm kích hoạt**:
/// - `$DATE` / `$TIME` → ngày/giờ `now` theo format strftime tương ứng (format hỏng → dùng mặc
///   định, không bao giờ panic giữa phiên gõ);
/// - `$$DATE` / `$$TIME` → literal `$DATE` / `$TIME` (escape); `$$` khác cũng còn một `$`;
/// - `$` khác (`$5`, `$FOO`…) giữ nguyên.
///
/// Hàm thuần: `now` do bên gọi đưa vào nên test được với clock giả lập; bên gọi thật dùng
/// [`expand_placeholders_now`].
pub fn expand_placeholders(
    text: &str,
    now: chrono::NaiveDateTime,
    date_fmt: &str,
    time_fmt: &str,
) -> String {
    if !text.contains('$') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find('$') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        if let Some(stripped) = after.strip_prefix('$') {
            // "$$" → một "$" literal; phần sau (kể cả "TIME"/"DATE") là văn bản thường.
            out.push('$');
            rest = stripped;
        } else if let Some(stripped) = after.strip_prefix("DATE") {
            out.push_str(&format_checked(now, date_fmt, DEFAULT_DATE_FORMAT));
            rest = stripped;
        } else if let Some(stripped) = after.strip_prefix("TIME") {
            out.push_str(&format_checked(now, time_fmt, DEFAULT_TIME_FORMAT));
            rest = stripped;
        } else {
            // "$" đơn lẻ ($5, $FOO, cuối chuỗi…) → giữ nguyên.
            out.push('$');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// Như [`expand_placeholders`] nhưng dùng giờ hệ thống hiện tại (giờ địa phương).
pub fn expand_placeholders_now(text: &str, date_fmt: &str, time_fmt: &str) -> String {
    expand_placeholders(text, chrono::Local::now().naive_local(), date_fmt, time_fmt)
}

/// Format `now` theo `fmt`; nếu `fmt` chứa chỉ định strftime không hợp lệ thì dùng `fallback`
/// (hằng của PinaKey, luôn hợp lệ). `DelayedFormat` sẽ panic khi Display gặp `Item::Error`,
/// nên phải kiểm tra trước bằng `StrftimeItems`.
fn format_checked(now: chrono::NaiveDateTime, fmt: &str, fallback: &str) -> String {
    use chrono::format::{Item, StrftimeItems};
    let valid = !StrftimeItems::new(fmt).any(|i| matches!(i, Item::Error));
    let fmt = if valid { fmt } else { fallback };
    now.format(fmt).to_string()
}

#[cfg(test)]
mod placeholder_tests {
    use super::*;
    use chrono::NaiveDate;

    fn now() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 7, 2)
            .unwrap()
            .and_hms_opt(14, 30, 5)
            .unwrap()
    }

    #[test]
    fn expands_date_and_time_with_default_formats() {
        assert_eq!(
            expand_placeholders(
                "hôm nay $DATE lúc $TIME",
                now(),
                DEFAULT_DATE_FORMAT,
                DEFAULT_TIME_FORMAT
            ),
            "hôm nay 02/07/2026 lúc 14:30"
        );
    }

    #[test]
    fn expands_with_custom_strftime_formats() {
        assert_eq!(
            expand_placeholders("$DATE $TIME", now(), "%Y-%m-%d", "%H:%M:%S"),
            "2026-07-02 14:30:05"
        );
    }

    #[test]
    fn double_dollar_escapes_to_literal() {
        // $$TIME → literal "$TIME" (không thay); $$ trước chữ thường chỉ còn một "$".
        assert_eq!(
            expand_placeholders(
                "$$TIME còn $TIME",
                now(),
                DEFAULT_DATE_FORMAT,
                DEFAULT_TIME_FORMAT
            ),
            "$TIME còn 14:30"
        );
        assert_eq!(
            expand_placeholders("$$DATE", now(), DEFAULT_DATE_FORMAT, DEFAULT_TIME_FORMAT),
            "$DATE"
        );
    }

    #[test]
    fn unrelated_dollars_kept_verbatim() {
        assert_eq!(
            expand_placeholders(
                "giá $5, $FOO, cuối câu $",
                now(),
                DEFAULT_DATE_FORMAT,
                DEFAULT_TIME_FORMAT
            ),
            "giá $5, $FOO, cuối câu $"
        );
        // Liền kề không dấu cách vẫn tách đúng.
        assert_eq!(
            expand_placeholders(
                "$TIME$DATE",
                now(),
                DEFAULT_DATE_FORMAT,
                DEFAULT_TIME_FORMAT
            ),
            "14:3002/07/2026"
        );
    }

    #[test]
    fn invalid_format_falls_back_to_default() {
        // "%!" không phải chỉ định strftime hợp lệ → dùng format mặc định thay vì panic/rác.
        assert_eq!(
            expand_placeholders("$DATE", now(), "%!", DEFAULT_TIME_FORMAT),
            "02/07/2026"
        );
        assert_eq!(
            expand_placeholders("$TIME", now(), DEFAULT_DATE_FORMAT, "%!"),
            "14:30"
        );
    }

    #[test]
    fn text_without_placeholders_unchanged() {
        assert_eq!(
            expand_placeholders("xin chào", now(), DEFAULT_DATE_FORMAT, DEFAULT_TIME_FORMAT),
            "xin chào"
        );
    }
}
