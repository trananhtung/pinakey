//! Input-mode and feature-flag constants — ported from `config/flags.go`.

// Input modes (Go `iota + 1`).
pub const PREEDIT_IM: i32 = 1;
pub const SURROUNDING_TEXT_IM: i32 = 2;
pub const BACKSPACE_FORWARDING_IM: i32 = 3;
pub const SHIFT_LEFT_FORWARDING_IM: i32 = 4;
pub const FORWARD_AS_COMMIT_IM: i32 = 5;
pub const XTEST_FAKE_KEY_EVENT_IM: i32 = 6;
pub const US_IM: i32 = 7;

/// Human-readable (Vietnamese) labels for each input mode.
pub fn im_lookup_table() -> Vec<(i32, &'static str)> {
    vec![
        (PREEDIT_IM, "Cấu hình mặc định (Pre-edit)"),
        (SURROUNDING_TEXT_IM, "Sửa lỗi gạch chân (Surrounding Text)"),
        (
            BACKSPACE_FORWARDING_IM,
            "Sửa lỗi gạch chân (ForwardKeyEvent I)",
        ),
        (
            SHIFT_LEFT_FORWARDING_IM,
            "Sửa lỗi gạch chân (ForwardKeyEvent II)",
        ),
        (
            FORWARD_AS_COMMIT_IM,
            "Sửa lỗi gạch chân (Forward as commit)",
        ),
        (
            XTEST_FAKE_KEY_EVENT_IM,
            "Sửa lỗi gạch chân (XTestFakeKeyEvent)",
        ),
        (US_IM, "Thêm vào danh sách loại trừ"),
    ]
}

pub const IM_BACKSPACE_LIST: &[i32] = &[
    SURROUNDING_TEXT_IM,
    BACKSPACE_FORWARDING_IM,
    SHIFT_LEFT_FORWARDING_IM,
    FORWARD_AS_COMMIT_IM,
    XTEST_FAKE_KEY_EVENT_IM,
];

// IBus engine feature flags (Go `uint`, `1 << iota`). `_`-prefixed bits are deprecated/unused but
// kept positionally so the numeric values match the original.
pub const IB_AUTO_COMMIT_WITH_VN_NOT_MATCH: u32 = 1 << 0;
pub const IB_MACRO_ENABLED: u32 = 1 << 1;
// 1<<2, 1<<3 deprecated
pub const IB_SPELL_CHECK_ENABLED: u32 = 1 << 4;
pub const IB_AUTO_NON_VN_RESTORE: u32 = 1 << 5;
pub const IB_DD_FREE_STYLE: u32 = 1 << 6;
pub const IB_NO_UNDERLINE: u32 = 1 << 7;
pub const IB_SPELL_CHECK_WITH_RULES: u32 = 1 << 8;
pub const IB_SPELL_CHECK_WITH_DICTS: u32 = 1 << 9;
pub const IB_AUTO_COMMIT_WITH_DELAY: u32 = 1 << 10;
// 1<<11, 1<<12 deprecated
pub const IB_PREEDIT_ELIMINATION: u32 = 1 << 13;
// 1<<14 deprecated
pub const IB_AUTO_CAPITALIZE_MACRO: u32 = 1 << 15;
// 1<<16, 1<<17, 1<<18 deprecated
pub const IB_WORKAROUND_FOR_FB_MESSENGER: u32 = 1 << 19;
pub const IB_WORKAROUND_FOR_WPS: u32 = 1 << 20;

pub const IB_STD_FLAGS: u32 = IB_SPELL_CHECK_ENABLED
    | IB_SPELL_CHECK_WITH_RULES
    | IB_AUTO_NON_VN_RESTORE
    | IB_DD_FREE_STYLE
    | IB_AUTO_CAPITALIZE_MACRO
    | IB_NO_UNDERLINE
    | IB_WORKAROUND_FOR_WPS;

pub const IB_US_STD_FLAGS: u32 = 0;
