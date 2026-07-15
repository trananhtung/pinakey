//! Nạp/lưu cấu hình — chuyển từ `config/config.go`.

pub mod flags;

use pinakey_core::{flag as core_flag, input_method_definitions_owned};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Cảnh báo ra stderr nhưng NUỐT lỗi ghi (#99): `eprintln!` panic khi ghi stderr thất bại,
/// mà profile release đặt `panic = "abort"` và staticlib nhúng vào tiến trình fcitx5 —
/// một lời cảnh báo không được phép giết cả bộ gõ.
#[macro_export]
macro_rules! warn_stderr {
    ($($arg:tt)*) => {{
        use ::std::io::Write;
        let _ = writeln!(::std::io::stderr(), $($arg)*);
    }};
}

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
    /// #161: GIỮ-CHỖ tương thích JSON. Bên Bamboo Go dùng để gán phím tắt "khôi phục phím gõ"
    /// (restore key strokes), nhưng cơ chế đó chưa từng được đấu dây trong bản Rust — không crate
    /// nào tiêu thụ mảng này. Giữ trường để round-trip config cũ (file có `"Shortcuts": [...]` vẫn
    /// nạp/ghi lại nguyên vẹn), KHÔNG có tác dụng chức năng. Tương tự cách giữ chỗ bit flag lỗi thời.
    #[serde(rename = "Shortcuts")]
    pub shortcuts: [u32; 10],
    /// #110: chỉ frontend IBus (dùng chung file config) tiêu thụ; addon fcitx5 chọn transport
    /// bằng `IB_NO_UNDERLINE` + transport-rules. Giữ trường để round-trip config, KHÔNG bày ra
    /// GUI thiết lập fcitx5.
    #[serde(rename = "DefaultInputMode")]
    pub default_input_mode: i32,
    #[serde(rename = "InputModeMapping")]
    pub input_mode_mapping: HashMap<String, i32>,
    /// Danh sách tên chương trình (wm_class/program) mà PinaKey KHÔNG xử lý tiếng Việt — gõ thẳng
    /// tiếng Anh (issue #9). So khớp không phân biệt hoa/thường: khớp khi bằng đúng hoặc là chuỗi con.
    #[serde(rename = "EnglishExclude", default)]
    pub english_exclude: Vec<String>,
    /// #65: gõ `w` ra `ư` (Telex). 0 = tắt (mặc định), 1 = không áp dụng ở đầu từ, 2 = mọi nơi.
    #[serde(rename = "WShortcut", default)]
    pub w_shortcut: u8,
    /// Format strftime cho placeholder `$DATE` trong macro (issue #64).
    #[serde(rename = "MacroDateFormat", default = "default_macro_date_format")]
    pub macro_date_format: String,
    /// Format strftime cho placeholder `$TIME` trong macro (issue #64).
    #[serde(rename = "MacroTimeFormat", default = "default_macro_time_format")]
    pub macro_time_format: String,
}

fn default_macro_date_format() -> String {
    "%d/%m/%Y".to_string()
}

fn default_macro_time_format() -> String {
    "%H:%M".to_string()
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
        w_shortcut: 0,
        macro_date_format: default_macro_date_format(),
        macro_time_format: default_macro_time_format(),
    }
}

/// `$XDG_CONFIG_HOME/pinakey` (mặc định `~/.config/pinakey`) — thư mục cấu hình riêng cho từng
/// người dùng. `None` khi cả XDG lẫn `$HOME` đều không xác định (systemd unit, `env -i`…) —
/// tuyệt đối không fallback về đường dẫn tương đối.
pub fn try_config_dir() -> Option<PathBuf> {
    config_dir_in(dirs::config_dir(), dirs::home_dir())
}

/// Logic thuần (không đọc môi trường) để test được trực tiếp: ưu tiên thư mục config base của
/// XDG (`dirs::config_dir()` = `$XDG_CONFIG_HOME` hoặc `~/.config`), fallback `{home}/.config`.
fn config_dir_in(xdg_config_base: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    xdg_config_base
        .or_else(|| home.map(|h| h.join(".config")))
        .map(|base| base.join("pinakey"))
}

// #162: mọi getter đường dẫn trả `Option<PathBuf>` — `None` khi thư mục config không xác định
// được (`try_config_dir()` = None). Trước đây các getter join lên một `PathBuf` RỖNG (từ
// `get_config_dir()` đã gỡ), cho ra đường dẫn TƯƠNG ĐỐI phân giải theo CWD của tiến trình fcitx5:
// đọc phải file macro/dict/config lạ đặt sẵn trong CWD, và GHI emoji-recent.txt vào CWD. `None`
// buộc mọi caller coi như "không có file" — đọc bỏ qua, ghi bỏ qua — như `save_config` vốn đã làm.

/// `~/.config/pinakey/ibus-<name>.macro.text`. `None` khi không xác định được thư mục config.
pub fn get_macro_path(engine_name: &str) -> Option<PathBuf> {
    Some(try_config_dir()?.join(format!("ibus-{}.macro.text", engine_name)))
}

/// `~/.config/pinakey/dict.txt` — từ điển chính tả do người dùng bổ sung (issue #18).
pub fn get_dict_path() -> Option<PathBuf> {
    Some(try_config_dir()?.join("dict.txt"))
}

/// `~/.config/pinakey/emoji-recent.txt` — lịch sử emoji gần dùng (issue #63), mỗi dòng một emoji.
pub fn get_emoji_recent_path() -> Option<PathBuf> {
    Some(try_config_dir()?.join("emoji-recent.txt"))
}

/// `~/.config/pinakey/transport-rules.conf` — rule transport theo app của NGƯỜI DÙNG (issue #67),
/// thắng rule built-in/hệ thống. Cú pháp: mỗi dòng `preedit|replace|auto <mẫu-tên-chương-trình>`.
pub fn get_transport_rules_path() -> Option<PathBuf> {
    Some(try_config_dir()?.join("transport-rules.conf"))
}

/// `~/.config/pinakey/ibus-<name>.config.json`. `None` khi không xác định được thư mục config.
pub fn get_config_path(engine_name: &str) -> Option<PathBuf> {
    Some(try_config_dir()?.join(format!("ibus-{}.config.json", engine_name)))
}

/// Nạp cấu hình: bắt đầu từ giá trị mặc định, sau đó phủ lên bằng file JSON của người dùng (nếu có).
/// Không xác định được thư mục config → trả mặc định (không đọc đường dẫn tương đối theo CWD, #162).
pub fn load_config(engine_name: &str) -> Config {
    match get_config_path(engine_name) {
        Some(path) => load_config_from(&path),
        None => default_cfg(),
    }
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
                warn_stderr!("pinakey: không đọc được config ({e}); dùng mặc định");
            }
            return default_cfg();
        }
    };
    match serde_json::from_str::<Config>(&data) {
        Ok(parsed) => parsed,
        Err(e) => {
            let backup = path.with_extension("json.corrupt");
            if let Err(re) = std::fs::rename(path, &backup) {
                warn_stderr!(
                    "pinakey: config hỏng ({e}) nhưng không backup được ({re}); giữ nguyên file, dùng mặc định"
                );
            } else {
                warn_stderr!(
                    "pinakey: config hỏng ({e}); đã backup sang {} và dùng mặc định",
                    backup.display()
                );
            }
            default_cfg()
        }
    }
}

/// Tương đương `SaveConfig` trong Go. Ghi **atomic + bền vững**: ghi ra file tạm cùng thư mục,
/// `sync_all()` rồi `rename` (đổi tên là thao tác atomic trên cùng filesystem), sau đó fsync thư
/// mục cha. Nhờ vậy cả bị kill giữa chừng lẫn mất điện đều không để lại file config rỗng/cụt
/// (xem `write_file_durable`, #163).
pub fn save_config(c: &Config, engine_name: &str) -> std::io::Result<()> {
    let path = get_config_path(engine_name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "không xác định được thư mục cấu hình ($XDG_CONFIG_HOME và $HOME đều thiếu)",
        )
    })?;
    save_config_to(c, &path)
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
    let tmp = tmp_path_for(path, std::process::id());
    if let Err(e) = write_file_durable(&tmp, path, data.as_bytes()) {
        let _ = std::fs::remove_file(&tmp); // không để lại file .tmp rác khi ghi/rename lỗi
        return Err(e);
    }
    Ok(())
}

/// Ghi `data` ra `tmp` rồi `rename` sang `path` một cách **bền vững với mất điện**.
///
/// `fs::write`+`rename` chỉ atomic với crash/kill tiến trình; khi MẤT ĐIỆN, metadata của rename
/// có thể chạm đĩa trước dữ liệu file tạm → sau reboot file đích rỗng/cụt, `load_config_from` coi
/// là JSON hỏng và reset toàn bộ thiết lập. Vì vậy `sync_all()` file tạm TRƯỚC rename, và fsync
/// thư mục cha SAU rename để entry đổi tên bền vững. Fsync thư mục là best-effort (một số
/// filesystem không hỗ trợ). (#163)
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

/// Đường dẫn file tạm cho ghi atomic: cùng thư mục với `path` (để `rename` là atomic) nhưng mang
/// PID để hai tiến trình ghi cùng lúc không giẫm lên file tạm của nhau.
fn tmp_path_for(path: &Path, pid: u32) -> PathBuf {
    path.with_extension(format!("json.tmp.{pid}"))
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
    fn typing_convenience_defaults_off() {
        // #65: 3 tiện ích gõ mặc định TẮT — file config cũ không có trường/bit mới vẫn giữ
        // nguyên hành vi hiện tại.
        let c: Config = serde_json::from_str(r#"{"InputMethod":"VNI"}"#).unwrap();
        assert_eq!(c.w_shortcut, 0);
        assert_eq!(default_cfg().ib_flags & flags::IB_CAPITALIZE_SENTENCE, 0);
        assert_eq!(default_cfg().ib_flags & flags::IB_DOUBLE_SPACE_PERIOD, 0);
        // Giá trị tuỳ chỉnh round-trip được.
        let mut c = default_cfg();
        c.w_shortcut = 2;
        let back: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back.w_shortcut, 2);
    }

    #[test]
    fn macro_formats_default_and_roundtrip() {
        // #64: file config cũ (không có 2 trường format) nạp ra format mặc định…
        let c: Config = serde_json::from_str(r#"{"InputMethod":"VNI"}"#).unwrap();
        assert_eq!(c.macro_date_format, "%d/%m/%Y");
        assert_eq!(c.macro_time_format, "%H:%M");
        // …và giá trị tuỳ chỉnh sống sót qua serialize/deserialize.
        let mut c = default_cfg();
        c.macro_date_format = "%Y-%m-%d".to_string();
        c.macro_time_format = "%H:%M:%S".to_string();
        let json = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.macro_date_format, "%Y-%m-%d");
        assert_eq!(back.macro_time_format, "%H:%M:%S");
    }

    #[test]
    fn config_dir_uses_xdg_base_when_present() {
        // Khi có thư mục config base của XDG ($XDG_CONFIG_HOME), config nằm dưới đó.
        // Không mutate biến môi trường nên không có data race khi test chạy song song.
        let xdg = PathBuf::from("/tmp/xdgbase");
        assert_eq!(
            config_dir_in(Some(xdg.clone()), Some(PathBuf::from("/home/u"))),
            Some(xdg.join("pinakey"))
        );
    }

    #[test]
    fn config_dir_falls_back_to_home_config_when_xdg_absent() {
        assert_eq!(
            config_dir_in(None, Some(PathBuf::from("/home/u"))),
            Some(PathBuf::from("/home/u/.config/pinakey"))
        );
    }

    #[test]
    fn config_dir_is_none_without_xdg_and_home() {
        // Thiếu cả XDG lẫn HOME: KHÔNG được fallback về đường dẫn tương đối kiểu "~/.config"
        // (từng tạo thư mục tên "~" thật trong CWD — nguy cơ `rm -rf ~`).
        assert_eq!(config_dir_in(None, None), None);
    }

    #[test]
    fn path_getters_compose_on_config_dir_and_are_absolute() {
        // #162: mọi getter phải build trên try_config_dir() — đường dẫn TUYỆT ĐỐI dưới thư mục
        // config, KHÔNG BAO GIỜ tương đối theo CWD. (Khi try_config_dir() = None thì kiểu Option
        // buộc trả None — không còn nhánh "PathBuf rỗng → đường dẫn tương đối" như trước.)
        let Some(base) = try_config_dir() else {
            return; // môi trường không có XDG lẫn HOME — None đã được test riêng ở trên.
        };
        assert!(base.is_absolute(), "thư mục config phải tuyệt đối");
        assert_eq!(
            get_config_path("Telex"),
            Some(base.join("ibus-Telex.config.json"))
        );
        assert_eq!(get_emoji_recent_path(), Some(base.join("emoji-recent.txt")));
        assert_eq!(get_dict_path(), Some(base.join("dict.txt")));
        assert_eq!(
            get_macro_path("Telex"),
            Some(base.join("ibus-Telex.macro.text"))
        );
        assert_eq!(
            get_transport_rules_path(),
            Some(base.join("transport-rules.conf"))
        );
    }

    #[test]
    fn tmp_path_unique_per_process_and_same_dir() {
        // Hai tiến trình ghi cùng config phải dùng file tạm khác nhau (tránh publish file dở),
        // nhưng vẫn cùng thư mục để rename là atomic trên cùng filesystem.
        let p = Path::new("/x/ibus-Telex.config.json");
        assert_ne!(tmp_path_for(p, 1234), tmp_path_for(p, 5678));
        assert_eq!(tmp_path_for(p, 1).parent(), p.parent());
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
            !tmp_path_for(&path, std::process::id()).exists(),
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
            !tmp_path_for(&path, std::process::id()).exists(),
            "phải dọn file .tmp khi rename lỗi, không để lại rác"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_to_overwrites_existing_durably() {
        // #163: ghi đè file config đã có phải cho nội dung mới nguyên vẹn (đường ghi bền vững:
        // sync_all file tạm → rename → fsync thư mục cha).
        let path = unique_tmp("overwrite");
        let _ = std::fs::remove_file(&path);
        let mut cfg = default_cfg();
        cfg.input_method = "Telex".to_string();
        save_config_to(&cfg, &path).unwrap();
        cfg.input_method = "VIQR".to_string();
        save_config_to(&cfg, &path).unwrap();
        let back = load_config_from(&path);
        assert_eq!(back.input_method, "VIQR");
        assert!(!tmp_path_for(&path, std::process::id()).exists());
        std::fs::remove_file(&path).ok();
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
