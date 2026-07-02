//! Lịch sử emoji gần dùng (issue #63): giữ tối đa N emoji dùng gần nhất, persist ra file văn bản
//! (mỗi dòng một emoji) trong thư mục cấu hình. Ghi **atomic** (file tạm + rename) như config —
//! mất điện giữa chừng không làm hỏng file. Module này không tự quyết định đường dẫn: tầng FFI
//! đưa đường dẫn từ `pinakey-config` vào, nhờ đó test được bằng file tạm.

use std::path::Path;

/// Danh sách emoji dùng gần nhất, mới nhất đứng trước, tối đa `cap` phần tử.
pub struct RecentEmoji {
    cap: usize,
    items: Vec<String>,
}

impl RecentEmoji {
    pub fn new(cap: usize) -> Self {
        RecentEmoji {
            cap,
            items: Vec::new(),
        }
    }

    /// Nạp từ file (mỗi dòng một emoji, dòng đầu = mới nhất). File thiếu / không đọc được →
    /// danh sách rỗng (bình thường ở lần chạy đầu).
    pub fn load_from_file(path: &Path, cap: usize) -> Self {
        let mut r = RecentEmoji::new(cap);
        if let Ok(data) = std::fs::read_to_string(path) {
            r.items = data
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .take(cap)
                .map(str::to_string)
                .collect();
        }
        r
    }

    /// Ghi nhận một lần dùng: đưa lên đầu, loại bản trùng cũ, cắt về `cap`.
    pub fn record(&mut self, emoji: &str) {
        if emoji.is_empty() {
            return;
        }
        self.items.retain(|e| e != emoji);
        self.items.insert(0, emoji.to_string());
        self.items.truncate(self.cap);
    }

    /// Danh sách hiện tại, mới nhất trước.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Ghi atomic: file tạm cùng thư mục (mang PID để hai tiến trình không giẫm nhau) rồi
    /// `rename` — cùng cơ chế với `save_config` bên pinakey-config.
    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                std::fs::create_dir_all(dir)?;
            }
        }
        let mut data = self.items.join("\n");
        if !data.is_empty() {
            data.push('\n');
        }
        let tmp = path.with_extension(format!("txt.tmp.{}", std::process::id()));
        std::fs::write(&tmp, data)?;
        if let Err(e) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp); // không để lại file .tmp rác khi rename lỗi
            return Err(e);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "pinakey_recent_test_{}_{}",
            std::process::id(),
            name
        ))
    }

    #[test]
    fn record_moves_front_dedups_and_caps() {
        let mut r = RecentEmoji::new(3);
        r.record("😀");
        r.record("😂");
        r.record("😍");
        assert_eq!(r.items(), ["😍", "😂", "😀"]); // mới nhất trước
        r.record("😀"); // dùng lại → nhảy lên đầu, không nhân đôi
        assert_eq!(r.items(), ["😀", "😍", "😂"]);
        r.record("🎉"); // vượt cap → rơi phần tử cũ nhất
        assert_eq!(r.items(), ["🎉", "😀", "😍"]);
    }

    #[test]
    fn save_load_roundtrip() {
        let path = tmp("roundtrip");
        let mut r = RecentEmoji::new(9);
        r.record("😀");
        r.record("👍");
        r.save_to_file(&path).unwrap();
        let loaded = RecentEmoji::load_from_file(&path, 9);
        assert_eq!(loaded.items(), ["👍", "😀"]);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_missing_file_is_empty() {
        let r = RecentEmoji::load_from_file(&tmp("khong_ton_tai"), 9);
        assert!(r.items().is_empty());
    }

    #[test]
    fn load_truncates_to_cap_and_skips_blank_lines() {
        let path = tmp("cap");
        std::fs::write(&path, "😀\n\n😂\n😍\n🎉\n").unwrap();
        let r = RecentEmoji::load_from_file(&path, 2);
        assert_eq!(r.items(), ["😀", "😂"]);
        std::fs::remove_file(&path).ok();
    }
}
