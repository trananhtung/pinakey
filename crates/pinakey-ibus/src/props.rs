//! Menu thuộc tính (property menu) hiển thị trên panel IBus — chuyển thể ý tưởng `prop.go`.
//!
//! Phần biểu diễn ở đây thuần (không phụ thuộc D-Bus) để unit-test: [`build_props`] dựng danh sách
//! mục menu phản ánh trạng thái hiện tại (bật/tắt tiếng Việt, kiểu gõ đang chọn). Lớp `dbus` dịch
//! các [`Prop`] thành `IBusProperty`/`IBusPropList` và phát qua `register_properties`.

/// Loại mục menu (ánh xạ tới `PropType` của IBus).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    /// Mục bật/tắt (checkbox).
    Toggle,
    /// Mục chọn một trong nhiều (radio).
    Radio,
    /// Mục hành động bấm-một-lần (ví dụ "Mở bảng thiết lập…").
    Action,
}

/// Khóa của mục menu mở giao diện thiết lập.
pub const OPEN_SETTINGS_KEY: &str = "open_settings";

/// Một mục trong menu thuộc tính.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prop {
    pub key: String,
    pub label: String,
    pub kind: PropKind,
    pub checked: bool,
}

/// Các kiểu gõ hiển thị trong menu (theo thứ tự).
pub const INPUT_METHODS: [&str; 3] = ["Telex", "VNI", "VIQR"];

/// Dựng danh sách mục menu: một mục bật/tắt tiếng Việt + các mục chọn kiểu gõ.
pub fn build_props(input_method: &str, vietnamese_enabled: bool) -> Vec<Prop> {
    let mut props = vec![Prop {
        key: "vn_toggle".to_string(),
        label: if vietnamese_enabled {
            "Tiếng Việt"
        } else {
            "English"
        }
        .to_string(),
        kind: PropKind::Toggle,
        checked: vietnamese_enabled,
    }];
    for im in INPUT_METHODS {
        props.push(Prop {
            key: format!("im_{im}"),
            label: im.to_string(),
            kind: PropKind::Radio,
            checked: input_method == im,
        });
    }
    props.push(Prop {
        key: OPEN_SETTINGS_KEY.to_string(),
        label: "Mở bảng thiết lập…".to_string(),
        kind: PropKind::Action,
        checked: false,
    });
    props
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_prop_is_vn_toggle() {
        let p = build_props("Telex", true);
        assert_eq!(p[0].key, "vn_toggle");
        assert_eq!(p[0].kind, PropKind::Toggle);
        assert!(p[0].checked);
        assert_eq!(p[0].label, "Tiếng Việt");
    }

    #[test]
    fn vn_toggle_reflects_disabled() {
        let p = build_props("Telex", false);
        assert!(!p[0].checked);
        assert_eq!(p[0].label, "English");
    }

    #[test]
    fn input_methods_are_radios_with_current_checked() {
        let p = build_props("VNI", true);
        let telex = p.iter().find(|x| x.key == "im_Telex").unwrap();
        let vni = p.iter().find(|x| x.key == "im_VNI").unwrap();
        assert_eq!(telex.kind, PropKind::Radio);
        assert!(!telex.checked);
        assert!(vni.checked);
        // có đủ ba kiểu gõ
        for im in INPUT_METHODS {
            assert!(p.iter().any(|x| x.key == format!("im_{im}")));
        }
    }

    #[test]
    fn has_open_settings_action() {
        let p = build_props("Telex", true);
        let s = p
            .iter()
            .find(|x| x.key == OPEN_SETTINGS_KEY)
            .expect("phải có mục mở thiết lập");
        assert_eq!(s.kind, PropKind::Action);
        assert!(!s.checked);
    }
}
