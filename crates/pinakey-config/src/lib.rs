//! Nạp/lưu cấu hình — chuyển từ `config/config.go`.

pub mod flags;

use pinakey_core::{flag as core_flag, input_method_definitions_owned};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Cấu hình engine được lưu trữ (`Config` trong Go). Tên các trường JSON khớp chính xác với tên
/// trường của struct Go, nên các file cấu hình cũ vẫn tương thích.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(rename = "InputMethod")]
    pub input_method: String,
    #[serde(rename = "InputMethodDefinitions")]
    pub input_method_definitions: HashMap<String, HashMap<String, String>>,
    #[serde(rename = "OutputCharset")]
    pub output_charset: String,
    #[serde(rename = "Flags")]
    pub flags: u32,
    #[serde(rename = "IBflags")]
    pub ib_flags: u32,
    #[serde(rename = "Shortcuts")]
    pub shortcuts: [u32; 10],
    #[serde(rename = "DefaultInputMode")]
    pub default_input_mode: i32,
    #[serde(rename = "InputModeMapping")]
    pub input_mode_mapping: HashMap<String, i32>,
    /// Danh sách tên chương trình (wm_class/program) mà PinaKey KHÔNG xử lý tiếng Việt — gõ thẳng
    /// tiếng Anh (issue #9). So khớp không phân biệt hoa/thường: khớp khi bằng đúng hoặc là chuỗi con.
    #[serde(rename = "EnglishExclude", default)]
    pub english_exclude: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        default_cfg()
    }
}

/// Tương đương `DefaultCfg()` trong Go.
pub fn default_cfg() -> Config {
    Config {
        input_method: "Telex".to_string(),
        output_charset: "Unicode".to_string(),
        input_method_definitions: input_method_definitions_owned(),
        flags: core_flag::STD_FLAGS,
        ib_flags: flags::IB_STD_FLAGS,
        shortcuts: [1, 126, 0, 0, 0, 0, 0, 0, 5, 117],
        default_input_mode: flags::PREEDIT_IM,
        input_mode_mapping: HashMap::new(),
        english_exclude: Vec::new(),
    }
}

fn home_dir() -> String {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "~".to_string())
}

/// `$XDG_CONFIG_HOME/pinakey` (mặc định `~/.config/pinakey`) — thư mục cấu hình riêng cho
/// từng người dùng của PinaKey. Tôn trọng chuẩn XDG; chỉ về `~/.config` khi không xác định được.
pub fn get_config_dir() -> PathBuf {
    config_dir_in(dirs::config_dir(), &home_dir())
}

/// Logic thuần (không đọc môi trường) để test được trực tiếp: ưu tiên thư mục config base của
/// XDG (`dirs::config_dir()` = `$XDG_CONFIG_HOME` hoặc `~/.config`), fallback `{home}/.config`.
fn config_dir_in(xdg_config_base: Option<PathBuf>, home: &str) -> PathBuf {
    xdg_config_base
        .unwrap_or_else(|| PathBuf::from(format!("{}/.config", home)))
        .join("pinakey")
}

pub fn get_macro_path(engine_name: &str) -> PathBuf {
    get_config_dir().join(format!("ibus-{}.macro.text", engine_name))
}

/// `~/.config/pinakey/dict.txt` — từ điển chính tả do người dùng bổ sung (issue #18).
pub fn get_dict_path() -> PathBuf {
    get_config_dir().join("dict.txt")
}

pub fn get_config_path(engine_name: &str) -> PathBuf {
    get_config_dir().join(format!("ibus-{}.config.json", engine_name))
}

/// Nạp cấu hình: bắt đầu từ giá trị mặc định, sau đó phủ lên bằng file JSON của người dùng (nếu có).
pub fn load_config(engine_name: &str) -> Config {
    load_config_from(&get_config_path(engine_name))
}

/// Nạp config từ một đường dẫn cụ thể. Phân biệt rõ:
/// - file không tồn tại → trả mặc định (bình thường, lần chạy đầu);
/// - file tồn tại nhưng JSON hỏng → **backup sang `*.corrupt`** (không để `save_config` sau ghi đè
///   mất dữ liệu người dùng) rồi mới trả mặc định, kèm cảnh báo ra stderr.
fn load_config_from(path: &Path) -> Config {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) => {
            // NotFound = bình thường (lần chạy đầu). Lỗi đọc thật (quyền, I/O...) thì cảnh báo —
            // nếu nuốt im lặng, save_config sau sẽ ghi đè mất config mà người dùng không hay.
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("pinakey: không đọc được config ({e}); dùng mặc định");
            }
            return default_cfg();
        }
    };
    match serde_json::from_str::<Config>(&data) {
        Ok(parsed) => parsed,
        Err(e) => {
            let backup = path.with_extension("json.corrupt");
            if let Err(re) = std::fs::rename(path, &backup) {
                eprintln!(
                    "pinakey: config hỏng ({e}) nhưng không backup được ({re}); giữ nguyên file, dùng mặc định"
                );
            } else {
                eprintln!(
                    "pinakey: config hỏng ({e}); đã backup sang {} và dùng mặc định",
                    backup.display()
                );
            }
            default_cfg()
        }
    }
}

/// Tương đương `SaveConfig` trong Go. Ghi **atomic** để mất điện / bị kill giữa chừng không làm
/// hỏng file config: ghi ra file tạm cùng thư mục rồi `rename` (đổi tên là thao tác atomic trên
/// cùng filesystem).
pub fn save_config(c: &Config, engine_name: &str) -> std::io::Result<()> {
    save_config_to(c, &get_config_path(engine_name))
}

fn save_config_to(c: &Config, path: &Path) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(c).map_err(std::io::Error::other)?;
    if let Some(dir) = path.parent() {
        // parent() của đường dẫn tương đối không có thư mục cha trả về Some("") → create_dir_all("")
        // lỗi trên một số OS; bỏ qua khi rỗng.
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, data)?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp); // không để lại file .tmp rác khi rename lỗi
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_telex_and_definitions() {
        let c = default_cfg();
        assert_eq!(c.input_method, "Telex");
        assert_eq!(c.output_charset, "Unicode");
        assert!(c.input_method_definitions.contains_key("Telex"));
        assert!(c.input_method_definitions.contains_key("VNI"));
        assert_eq!(c.default_input_mode, flags::PREEDIT_IM);
        assert_eq!(c.shortcuts, [1, 126, 0, 0, 0, 0, 0, 0, 5, 117]);
    }

    #[test]
    fn partial_json_overlays_defaults() {
        // Chỉ có InputMethod: mọi thứ còn lại phải quay về giá trị mặc định.
        let json = r#"{"InputMethod":"VNI"}"#;
        let c: Config = serde_json::from_str(json).unwrap();
        assert_eq!(c.input_method, "VNI");
        assert_eq!(c.output_charset, "Unicode");
        assert_eq!(c.flags, core_flag::STD_FLAGS);
        assert!(c.input_method_definitions.contains_key("Telex"));
    }

    #[test]
    fn config_dir_uses_xdg_base_when_present() {
        // Khi có thư mục config base của XDG ($XDG_CONFIG_HOME), config nằm dưới đó.
        // Không mutate biến môi trường nên không có data race khi test chạy song song.
        let xdg = PathBuf::from("/tmp/xdgbase");
        assert_eq!(
            config_dir_in(Some(xdg.clone()), "/home/u"),
            xdg.join("pinakey")
        );
    }

    #[test]
    fn config_dir_falls_back_to_home_config_when_xdg_absent() {
        assert_eq!(
            config_dir_in(None, "/home/u"),
            PathBuf::from("/home/u/.config/pinakey")
        );
    }

    fn unique_tmp(tag: &str) -> PathBuf {
        // Thư mục tạm riêng cho mỗi test (tránh đụng nhau khi chạy song song).
        let dir = std::env::temp_dir().join(format!("pk_cfg_test_{}_{}", tag, std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("ibus-Telex.config.json")
    }

    #[test]
    fn load_from_missing_file_returns_default() {
        let path = unique_tmp("missing");
        let _ = std::fs::remove_file(&path);
        let c = load_config_from(&path);
        assert_eq!(c.input_method, "Telex");
    }

    #[test]
    fn load_from_corrupt_backs_up_and_does_not_lose_data() {
        let path = unique_tmp("corrupt");
        std::fs::write(&path, "{ this is not valid json ]").unwrap();
        let c = load_config_from(&path);
        // Trả mặc định để engine vẫn chạy...
        assert_eq!(c.input_method, "Telex");
        // ...nhưng KHÔNG được mất dữ liệu người dùng: file hỏng phải được backup.
        let backup = path.with_extension("json.corrupt");
        assert!(
            backup.exists(),
            "file config hỏng phải được backup sang .corrupt thay vì mất"
        );
        assert!(
            !path.exists(),
            "file hỏng phải được dời đi để save sau không ghi đè lên nó"
        );
        std::fs::remove_file(&backup).ok();
    }

    #[test]
    fn save_to_is_atomic_no_tmp_left_and_roundtrips() {
        let path = unique_tmp("atomic");
        let _ = std::fs::remove_file(&path);
        let mut cfg = default_cfg();
        cfg.input_method = "VNI".to_string();
        save_config_to(&cfg, &path).unwrap();
        // Không để lại file .tmp rác.
        assert!(
            !path.with_extension("json.tmp").exists(),
            "không được để lại file .tmp sau khi ghi atomic"
        );
        // Đọc lại đúng nội dung.
        let back = load_config_from(&path);
        assert_eq!(back.input_method, "VNI");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn save_to_cleans_up_tmp_when_rename_fails() {
        let dir = std::env::temp_dir().join(format!("pk_renfail_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ibus-Telex.config.json");
        // Tạo một THƯ MỤC ngay tại path để rename(file -> dir-không-rỗng) thất bại.
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("keep"), b"x").unwrap();
        let r = save_config_to(&default_cfg(), &path);
        assert!(r.is_err(), "rename đè lên thư mục phải lỗi");
        assert!(
            !path.with_extension("json.tmp").exists(),
            "phải dọn file .tmp khi rename lỗi, không để lại rác"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn roundtrip_json() {
        let c = default_cfg();
        let data = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&data).unwrap();
        assert_eq!(back.input_method, c.input_method);
        assert_eq!(back.ib_flags, c.ib_flags);
    }
}
