//! Phím tắt của engine — chuyển thể ý tưởng `Shortcuts` trong cấu hình ibus-bamboo.
//!
//! Cấu hình lưu `shortcuts: [u32; 10]` là 5 cặp `(modifier, keyval)`. Modifier được mã hóa gọn:
//! bit `1`=Control, `2`=Shift, `4`=Alt (Mod1), `8`=Super (Mod4). PinaKey dùng:
//!  - cặp 0 (`shortcuts[0..2]`) → **bật/tắt tiếng Việt** (chuyển sang gõ thẳng tiếng Anh);
//!  - cặp 1 (`shortcuts[2..4]`) → **khôi phục** các phím gốc của từ đang gõ.
//!
//! `keyval == 0` nghĩa là chưa gán và không bao giờ khớp.

use crate::constants::*;

/// Hành động ứng với một phím tắt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    ToggleVietnamese,
    RestoreWord,
}

/// Giải mã modifier dạng gọn của cấu hình thành mặt nạ trạng thái IBus.
pub fn decode_modifier(code: u32) -> u32 {
    let mut mask = 0;
    if code & 1 != 0 {
        mask |= IBUS_CONTROL_MASK;
    }
    if code & 2 != 0 {
        mask |= IBUS_SHIFT_MASK;
    }
    if code & 4 != 0 {
        mask |= IBUS_MOD1_MASK;
    }
    if code & 8 != 0 {
        mask |= IBUS_SUPER_MASK;
    }
    mask
}

/// Các modifier được tính khi so khớp phím tắt (bỏ qua Lock/Release/...).
const RELEVANT_MODS: u32 = IBUS_SHIFT_MASK | IBUS_CONTROL_MASK | IBUS_MOD1_MASK | IBUS_SUPER_MASK;

fn pair_matches(mod_code: u32, target_key: u32, state: u32, keyval: u32) -> bool {
    target_key != 0 && keyval == target_key && (state & RELEVANT_MODS) == decode_modifier(mod_code)
}

/// Trả về hành động phím tắt khớp với `(state, keyval)`, nếu có.
pub fn match_shortcut(shortcuts: &[u32; 10], state: u32, keyval: u32) -> Option<ShortcutAction> {
    if pair_matches(shortcuts[0], shortcuts[1], state, keyval) {
        return Some(ShortcutAction::ToggleVietnamese);
    }
    if pair_matches(shortcuts[2], shortcuts[3], state, keyval) {
        return Some(ShortcutAction::RestoreWord);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_modifier_maps_bits() {
        assert_eq!(decode_modifier(0), 0);
        assert_eq!(decode_modifier(1), IBUS_CONTROL_MASK);
        assert_eq!(decode_modifier(2), IBUS_SHIFT_MASK);
        assert_eq!(decode_modifier(4), IBUS_MOD1_MASK);
        assert_eq!(decode_modifier(8), IBUS_SUPER_MASK);
        assert_eq!(decode_modifier(3), IBUS_CONTROL_MASK | IBUS_SHIFT_MASK);
    }

    #[test]
    fn matches_toggle_pair() {
        // cặp 0 = (Control, keyval 0x76 'v'); cặp 1 = (Shift, keyval 0x72 'r').
        let sc = [1, 0x76, 2, 0x72, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            match_shortcut(&sc, IBUS_CONTROL_MASK, 0x76),
            Some(ShortcutAction::ToggleVietnamese)
        );
        assert_eq!(
            match_shortcut(&sc, IBUS_SHIFT_MASK, 0x72),
            Some(ShortcutAction::RestoreWord)
        );
    }

    #[test]
    fn modifier_must_match_exactly() {
        let sc = [1, 0x76, 0, 0, 0, 0, 0, 0, 0, 0];
        // đúng keyval nhưng thiếu Control -> không khớp
        assert_eq!(match_shortcut(&sc, 0, 0x76), None);
        // thừa Shift -> không khớp (so khớp chính xác các modifier liên quan)
        assert_eq!(
            match_shortcut(&sc, IBUS_CONTROL_MASK | IBUS_SHIFT_MASK, 0x76),
            None
        );
        // Lock/Release không tính vào modifier liên quan -> vẫn khớp
        assert_eq!(
            match_shortcut(&sc, IBUS_CONTROL_MASK | IBUS_LOCK_MASK, 0x76),
            Some(ShortcutAction::ToggleVietnamese)
        );
    }

    #[test]
    fn unassigned_never_matches() {
        let sc = [0; 10];
        assert_eq!(match_shortcut(&sc, 0, 0), None);
        assert_eq!(match_shortcut(&sc, IBUS_CONTROL_MASK, 0), None);
    }
}
