//! Logic của giao diện thiết lập, tách hoàn toàn khỏi phần vẽ GUI.
//!
//! [`SettingsController`] giữ một bản nháp [`Config`], cung cấp các thao tác chỉnh sửa (đổi kiểu gõ,
//! charset, chế độ nhập, bật/tắt cờ), theo dõi trạng thái "đã sửa" (dirty) và nạp/lưu xuống file.
//! Toàn bộ là logic thuần nên được unit-test mà không cần mở cửa sổ nào.

use pinakey_config::{default_cfg, flags, load_config, save_config, Config};

/// Danh sách cờ (ib_flags) hiển thị dạng bật/tắt trong giao diện, kèm nhãn tiếng Việt.
pub fn settings_flags() -> Vec<(u32, &'static str)> {
    vec![
        (flags::IB_SPELL_CHECK_ENABLED, "Kiểm tra chính tả"),
        (flags::IB_SPELL_CHECK_WITH_RULES, "Chính tả theo quy tắc"),
        (flags::IB_SPELL_CHECK_WITH_DICTS, "Chính tả theo từ điển"),
        (
            flags::IB_AUTO_NON_VN_RESTORE,
            "Tự khôi phục từ không phải tiếng Việt",
        ),
        (flags::IB_DD_FREE_STYLE, "Cho phép gõ dd tự do"),
        (flags::IB_MACRO_ENABLED, "Bật macro"),
        (flags::IB_AUTO_CAPITALIZE_MACRO, "Tự viết hoa macro"),
        (flags::IB_NO_UNDERLINE, "Không gạch chân preedit"),
    ]
}

/// Bộ điều khiển cho màn hình thiết lập.
pub struct SettingsController {
    config: Config,
    engine_name: String,
    dirty: bool,
}

impl SettingsController {
    /// Nạp cấu hình hiện có của `engine_name` (hoặc mặc định nếu chưa có file).
    pub fn load(engine_name: &str) -> Self {
        SettingsController {
            config: load_config(engine_name),
            engine_name: engine_name.to_string(),
            dirty: false,
        }
    }

    /// Khởi tạo trực tiếp từ một `Config` (dùng cho kiểm thử).
    pub fn from_config(config: Config, engine_name: &str) -> Self {
        SettingsController {
            config,
            engine_name: engine_name.to_string(),
            dirty: false,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Các kiểu gõ khả dụng (khóa của input_method_definitions), đã sắp xếp.
    pub fn input_methods(&self) -> Vec<String> {
        let mut ims: Vec<String> = self
            .config
            .input_method_definitions
            .keys()
            .cloned()
            .collect();
        ims.sort();
        ims
    }

    /// Các charset đầu ra khả dụng.
    pub fn charsets(&self) -> Vec<String> {
        pinakey_core::get_charset_names()
    }

    /// Các chế độ nhập khả dụng (giá trị + nhãn).
    pub fn input_modes(&self) -> Vec<(i32, &'static str)> {
        flags::im_lookup_table()
    }

    pub fn input_method(&self) -> &str {
        &self.config.input_method
    }

    /// Đổi kiểu gõ; bỏ qua nếu không nằm trong danh sách khả dụng.
    pub fn set_input_method(&mut self, im: &str) {
        if im == self.config.input_method {
            return;
        }
        if self.config.input_method_definitions.contains_key(im) {
            self.config.input_method = im.to_string();
            self.dirty = true;
        }
    }

    pub fn output_charset(&self) -> &str {
        &self.config.output_charset
    }

    pub fn set_output_charset(&mut self, charset: &str) {
        if charset != self.config.output_charset {
            self.config.output_charset = charset.to_string();
            self.dirty = true;
        }
    }

    pub fn input_mode(&self) -> i32 {
        self.config.default_input_mode
    }

    pub fn set_input_mode(&mut self, mode: i32) {
        if mode != self.config.default_input_mode {
            self.config.default_input_mode = mode;
            self.dirty = true;
        }
    }

    pub fn flag_enabled(&self, flag: u32) -> bool {
        self.config.ib_flags & flag != 0
    }

    /// Bật/tắt một cờ ib_flags.
    pub fn set_flag(&mut self, flag: u32, on: bool) {
        let new = if on {
            self.config.ib_flags | flag
        } else {
            self.config.ib_flags & !flag
        };
        if new != self.config.ib_flags {
            self.config.ib_flags = new;
            self.dirty = true;
        }
    }

    pub fn toggle_flag(&mut self, flag: u32) {
        self.set_flag(flag, !self.flag_enabled(flag));
    }

    /// Khôi phục về cấu hình mặc định.
    pub fn reset_to_default(&mut self) {
        self.config = default_cfg();
        self.dirty = true;
    }

    /// Lưu cấu hình xuống file và xóa cờ dirty.
    pub fn save(&mut self) -> std::io::Result<()> {
        save_config(&self.config, &self.engine_name)?;
        self.dirty = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl() -> SettingsController {
        SettingsController::from_config(default_cfg(), "pinakey_settings_test")
    }

    #[test]
    fn load_default_is_clean() {
        let c = ctrl();
        assert!(!c.is_dirty());
        assert_eq!(c.input_method(), "Telex");
    }

    #[test]
    fn input_methods_sorted_and_nonempty() {
        let c = ctrl();
        let ims = c.input_methods();
        assert!(ims.contains(&"Telex".to_string()));
        assert!(ims.contains(&"VNI".to_string()));
        let mut sorted = ims.clone();
        sorted.sort();
        assert_eq!(ims, sorted);
    }

    #[test]
    fn set_input_method_marks_dirty() {
        let mut c = ctrl();
        c.set_input_method("VNI");
        assert_eq!(c.input_method(), "VNI");
        assert!(c.is_dirty());
    }

    #[test]
    fn set_unknown_input_method_ignored() {
        let mut c = ctrl();
        c.set_input_method("KhongCoThat");
        assert_eq!(c.input_method(), "Telex");
        assert!(!c.is_dirty());
    }

    #[test]
    fn set_charset_marks_dirty() {
        let mut c = ctrl();
        let other = c
            .charsets()
            .into_iter()
            .find(|cs| cs != c.output_charset())
            .expect("phải có charset khác");
        c.set_output_charset(&other);
        assert_eq!(c.output_charset(), other);
        assert!(c.is_dirty());
    }

    #[test]
    fn set_input_mode_marks_dirty() {
        let mut c = ctrl();
        c.set_input_mode(flags::XTEST_FAKE_KEY_EVENT_IM);
        assert_eq!(c.input_mode(), flags::XTEST_FAKE_KEY_EVENT_IM);
        assert!(c.is_dirty());
    }

    #[test]
    fn toggle_flag_flips_and_marks_dirty() {
        let mut c = ctrl();
        let was = c.flag_enabled(flags::IB_SPELL_CHECK_WITH_DICTS);
        c.toggle_flag(flags::IB_SPELL_CHECK_WITH_DICTS);
        assert_eq!(c.flag_enabled(flags::IB_SPELL_CHECK_WITH_DICTS), !was);
        assert!(c.is_dirty());
    }

    #[test]
    fn reset_to_default_restores() {
        let mut c = ctrl();
        c.set_input_method("VNI");
        c.reset_to_default();
        assert_eq!(c.input_method(), "Telex");
    }

    #[test]
    fn save_and_reload_roundtrip() {
        let name = "pinakey_settings_roundtrip_test";
        let mut c = SettingsController::from_config(default_cfg(), name);
        c.set_input_method("VNI");
        c.toggle_flag(flags::IB_MACRO_ENABLED);
        c.save().expect("lưu được");
        assert!(!c.is_dirty());

        let reloaded = SettingsController::load(name);
        assert_eq!(reloaded.input_method(), "VNI");
        assert_eq!(
            reloaded.flag_enabled(flags::IB_MACRO_ENABLED),
            c.flag_enabled(flags::IB_MACRO_ENABLED)
        );

        // dọn file cấu hình tạm
        let _ = std::fs::remove_file(pinakey_config::get_config_path(name));
    }
}
