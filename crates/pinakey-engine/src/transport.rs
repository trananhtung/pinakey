//! Quy tắc chọn transport theo ứng dụng (issue #67): bảng `(mẫu tên chương trình, transport)`
//! ba lớp — nhúng sẵn trong binary → file hệ thống `/usr/share/pinakey/transport-rules.conf`
//! (đóng gói cập nhật được không cần rebuild) → file người dùng
//! `~/.config/pinakey/transport-rules.conf` (thắng tất cả). Tổng quát hoá cơ chế "LibreOffice
//! → preedit" của #66 và chuẩn hoá nhóm terminal thành rule dữ liệu.

/// Transport ép buộc cho một app. Giá trị số ổn định vì đi qua FFI sang C++.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TransportPref {
    /// Không ép: theo capability của app (SurroundingText → replace, không → preedit/uinput).
    Auto = 0,
    /// Luôn dùng preedit (ổn định 100%) dù app quảng cáo SurroundingText.
    Preedit = 1,
    /// Luôn dùng diff-replace khi có SurroundingText, bỏ qua danh sách chặn built-in.
    Replace = 2,
}

/// Bảng rule đã gộp các lớp; dòng khớp SAU thắng (lớp sau append sau → tự nhiên thắng lớp trước).
pub struct TransportRules {
    rules: Vec<(String, TransportPref)>,
}

/// Nội dung rule built-in nhúng kèm binary.
pub const EMBEDDED_RULES: &str = include_str!("../data/transport-rules.conf");

/// Đường dẫn file rule hệ thống (gói cài đặt có thể cập nhật mà không cần rebuild).
pub const SYSTEM_RULES_PATH: &str = "/usr/share/pinakey/transport-rules.conf";

impl TransportRules {
    /// Bảng rỗng (không rule nào — mọi app đều Auto).
    pub fn empty() -> Self {
        TransportRules { rules: Vec::new() }
    }

    /// Parse một lớp rule và nối vào SAU các rule hiện có (lớp nối sau thắng lớp trước).
    /// Dòng rỗng / bắt đầu `#` `;` bị bỏ qua; dòng sai cú pháp bị bỏ qua (không phá phiên gõ).
    pub fn append_layer(&mut self, text: &str) {
        for line in text.lines() {
            let s = line.trim();
            if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
                continue;
            }
            let mut it = s.split_whitespace();
            let (Some(verb), Some(pattern)) = (it.next(), it.next()) else {
                continue;
            };
            let pref = match verb.to_ascii_lowercase().as_str() {
                "preedit" => TransportPref::Preedit,
                "replace" => TransportPref::Replace,
                "auto" => TransportPref::Auto,
                _ => continue,
            };
            self.rules.push((pattern.to_ascii_lowercase(), pref));
        }
    }

    /// Transport cho `wm_class` (khớp bằng đúng hoặc chuỗi con, không phân biệt hoa/thường).
    /// Không rule nào khớp → `Auto`. Duyệt NGƯỢC để dòng/lớp sau thắng.
    pub fn lookup(&self, wm_class: &str) -> TransportPref {
        if wm_class.is_empty() {
            return TransportPref::Auto;
        }
        let w = wm_class.to_ascii_lowercase();
        for (pattern, pref) in self.rules.iter().rev() {
            if w == *pattern || w.contains(pattern.as_str()) {
                return *pref;
            }
        }
        TransportPref::Auto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_rules_cover_libreoffice_and_terminals() {
        let mut r = TransportRules::empty();
        r.append_layer(EMBEDDED_RULES);
        for p in [
            "soffice.bin",
            "libreoffice-writer",
            "org.libreoffice.LibreOffice",
            "gnome-terminal-server",
            "kitty",
            "Alacritty",
        ] {
            assert_eq!(r.lookup(p), TransportPref::Preedit, "{p} phải là preedit");
        }
        for p in ["firefox", "google-chrome", "gedit", ""] {
            assert_eq!(r.lookup(p), TransportPref::Auto, "{p:?} phải là auto");
        }
    }

    #[test]
    fn later_layer_wins() {
        // Rule người dùng (lớp nối sau) thắng rule built-in: ép LibreOffice về replace.
        let mut r = TransportRules::empty();
        r.append_layer(EMBEDDED_RULES);
        r.append_layer("replace soffice\npreedit gedit\n");
        assert_eq!(r.lookup("soffice.bin"), TransportPref::Replace);
        assert_eq!(r.lookup("gedit"), TransportPref::Preedit);
        assert_eq!(r.lookup("libreoffice-writer"), TransportPref::Preedit); // không bị đụng
    }

    #[test]
    fn parse_skips_comments_and_garbage() {
        let mut r = TransportRules::empty();
        r.append_layer("# chú thích\n; nữa\n\npreedit\nbay bổng gì đó\nPREEDIT Foo\nauto foo\n");
        // "preedit" thiếu mẫu và "bay bổng" sai verb → bỏ qua; PREEDIT Foo hợp lệ (không phân
        // biệt hoa/thường); "auto foo" ghi đè lại thành Auto (dòng sau thắng).
        assert_eq!(r.lookup("foo"), TransportPref::Auto);
        assert_eq!(r.rules.len(), 2);
    }
}
