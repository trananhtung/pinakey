//! Tìm một font phủ đầy đủ chữ tiếng Việt để nạp vào egui.
//!
//! Font mặc định của egui không có glyph cho các nguyên âm mang dấu chồng (ộ, ế, ệ, ậ, ữ…) nên
//! chúng hiện thành ô vuông. Ta tìm một font hệ thống có phủ tiếng Việt (Noto Sans / DejaVu Sans /
//! Liberation Sans — gần như luôn có trên Linux desktop), ưu tiên danh sách đường dẫn quen thuộc rồi
//! mới hỏi fontconfig (`fc-match`). Phần tìm đường dẫn là logic thuần nên được unit-test.

use std::path::Path;
use std::process::Command;

/// Các đường dẫn font phủ tiếng Việt thường gặp trên nhiều bản phân phối Linux.
pub const FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
];

/// Trả về đường dẫn đầu tiên trong `candidates` thực sự là một file.
pub fn first_existing(candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|p| Path::new(p).is_file())
        .map(|p| (*p).to_string())
}

/// Hỏi fontconfig (`fc-match`) một font phủ tiếng Việt; `None` nếu không có fontconfig.
fn fc_match_vietnamese() -> Option<String> {
    let out = Command::new("fc-match")
        .args(["-f", "%{file}", ":lang=vi"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !path.is_empty() && Path::new(&path).is_file() {
        Some(path)
    } else {
        None
    }
}

/// Tìm một font phủ tiếng Việt: ưu tiên đường dẫn quen thuộc, sau đó tới fontconfig.
pub fn find_vietnamese_font() -> Option<String> {
    first_existing(FONT_CANDIDATES).or_else(fc_match_vietnamese)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_existing_picks_first_real_file() {
        let dir = std::env::temp_dir();
        let f = dir.join("pinakey_font_probe.ttf");
        std::fs::write(&f, b"x").unwrap();
        let fp = f.to_str().unwrap();

        assert_eq!(
            first_existing(&["/khong/ton/tai/a.ttf", fp]).as_deref(),
            Some(fp)
        );
        assert_eq!(first_existing(&["/khong/ton/tai/a.ttf"]), None);
        assert_eq!(first_existing(&[]), None);

        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn candidates_are_absolute_ttf_paths() {
        for p in FONT_CANDIDATES {
            assert!(p.starts_with('/'), "đường dẫn font phải tuyệt đối: {p}");
            assert!(p.ends_with(".ttf"), "kỳ vọng file .ttf: {p}");
        }
    }
}
