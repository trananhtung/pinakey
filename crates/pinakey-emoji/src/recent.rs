//! Lịch sử emoji gần dùng (issue #63): giữ tối đa N emoji dùng gần nhất, persist ra file văn bản
//! (mỗi dòng một emoji) trong thư mục cấu hình. Ghi **atomic + bền vững** (file tạm → `sync_all()`
//! → rename → fsync thư mục cha) như config — cả kill giữa chừng lẫn mất điện đều không để lại
//! file rỗng/cụt (xem `write_file_durable`, #163). Module này không tự quyết định đường dẫn: tầng
//! FFI đưa đường dẫn từ `pinakey-config` vào, nhờ đó test được bằng file tạm.

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
        if let Err(e) = write_file_durable(&tmp, path, data.as_bytes()) {
            let _ = std::fs::remove_file(&tmp); // không để lại file .tmp rác khi ghi/rename lỗi
            return Err(e);
        }
        Ok(())
    }
}

/// Ghi `data` ra `tmp` rồi `rename` sang `path` bền vững với mất điện: `sync_all()` file tạm
/// TRƯỚC rename, fsync thư mục cha SAU rename (best-effort). `fs::write`+`rename` chỉ atomic với
/// crash/kill tiến trình, không với mất điện — khi đó file đích có thể rỗng/cụt sau reboot. (#163)
fn write_file_durable(tmp: &Path, path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    {
        let mut f = std::fs::File::create(tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    std::fs::rename(tmp, path)?;
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            if let Ok(d) = std::fs::File::open(dir) {
                let _ = d.sync_all();
            }
        }
    }
    Ok(())
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
    fn save_overwrites_existing_durably() {
        // #163: ghi đè file lịch sử đã có phải cho nội dung mới nguyên vẹn (sync_all → rename →
        // fsync thư mục cha), không lẫn phần đuôi của bản cũ.
        let path = tmp("overwrite");
        let mut r = RecentEmoji::new(9);
        r.record("😀");
        r.record("👍");
        r.record("🎉");
        r.save_to_file(&path).unwrap();
        let mut r2 = RecentEmoji::new(9);
        r2.record("🚀");
        r2.save_to_file(&path).unwrap();
        let loaded = RecentEmoji::load_from_file(&path, 9);
        assert_eq!(loaded.items(), ["🚀"]);
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
