//! Configuration load/save — ported from `config/config.go`.

pub mod flags;

use bamboo_core::{flag as core_flag, input_method_definitions_owned};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// The persisted engine configuration (`Config` in Go). JSON field names match the Go struct
/// field names exactly, so existing config files remain compatible.
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
}

impl Default for Config {
    fn default() -> Self {
        default_cfg()
    }
}

/// Equivalent of Go `DefaultCfg()`.
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
    }
}

fn home_dir() -> String {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "~".to_string())
}

/// `~/.config/ibus-bamboo` (the engine name is always "bamboo" here, matching Go).
pub fn get_config_dir() -> PathBuf {
    PathBuf::from(format!("{}/.config/ibus-bamboo", home_dir()))
}

pub fn get_macro_path(engine_name: &str) -> PathBuf {
    get_config_dir().join(format!("ibus-{}.macro.text", engine_name))
}

pub fn get_config_path(engine_name: &str) -> PathBuf {
    get_config_dir().join(format!("ibus-{}.config.json", engine_name))
}

/// Equivalent of Go `LoadConfig`: starts from defaults, then overlays the JSON file (if any).
pub fn load_config(engine_name: &str) -> Config {
    let mut c = default_cfg();
    if engine_name == "bamboous" {
        c.default_input_mode = flags::US_IM;
        c.ib_flags = flags::IB_US_STD_FLAGS;
        return c;
    }
    if let Ok(data) = std::fs::read_to_string(get_config_path(engine_name)) {
        if let Ok(parsed) = serde_json::from_str::<Config>(&data) {
            c = parsed;
        }
    }
    c
}

/// Equivalent of Go `SaveConfig`.
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
        // Only InputMethod present: everything else must fall back to defaults.
        let json = r#"{"InputMethod":"VNI"}"#;
        let c: Config = serde_json::from_str(json).unwrap();
        assert_eq!(c.input_method, "VNI");
        assert_eq!(c.output_charset, "Unicode");
        assert_eq!(c.flags, core_flag::STD_FLAGS);
        assert!(c.input_method_definitions.contains_key("Telex"));
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
