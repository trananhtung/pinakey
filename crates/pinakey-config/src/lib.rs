//! Nạp/lưu cấu hình — chuyển từ `config/config.go`.

pub mod flags;

use pinakey_core::{flag as core_flag, input_method_definitions_owned};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(format!("{}/.config", home_dir())))
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
    let mut c = default_cfg();
    if let Ok(data) = std::fs::read_to_string(get_config_path(engine_name)) {
        if let Ok(parsed) = serde_json::from_str::<Config>(&data) {
            c = parsed;
        }
    }
    c
}

/// Tương đương `SaveConfig` trong Go.
pub fn save_config(c: &Config, engine_name: &str) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(c).map_err(std::io::Error::other)?;
    let dir = get_config_dir();
    std::fs::create_dir_all(&dir)?;
    std::fs::write(get_config_path(engine_name), data)
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
    fn config_dir_honors_xdg_config_home() {
        // Khi đặt XDG_CONFIG_HOME, thư mục config phải nằm dưới đó (chuẩn XDG),
        // không phải hardcode ~/.config.
        let tmp = std::env::temp_dir().join("pk_xdg_cfg_test");
        // SAFETY: edition 2021, test đơn luồng cho biến env này.
        std::env::set_var("XDG_CONFIG_HOME", &tmp);
        let dir = get_config_dir();
        std::env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(dir, tmp.join("pinakey"));
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
