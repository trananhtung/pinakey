//! Tiêm phím giả lập qua **XTest** — bản chuyển thuần Rust của `x11_keyboard.c`
//! (`XTestFakeKeyEvent`), dùng x11rb thay cho Xlib/cgo.
//!
//! Chỉ các chế độ nhập "sửa lỗi bằng backspace" dùng tới đây để xóa lùi ký tự trực tiếp ở tầng X
//! server (hoạt động trên X11 và XWayland). Trên Wayland thuần, chế độ forward-key-event của IBus
//! mới là đường đi tương ứng nên không cần XTest.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt as _, KEY_PRESS_EVENT, KEY_RELEASE_EVENT};
use x11rb::protocol::xtest::ConnectionExt as _;

/// Keysym X11 của phím BackSpace.
pub const KEYSYM_BACKSPACE: u32 = 0xff08;

/// Tìm keycode đầu tiên ánh xạ tới `target` trong bảng keysym phẳng do `GetKeyboardMapping` trả về.
///
/// `keysyms` là mảng phẳng `keysyms_per_keycode` keysym mỗi keycode, bắt đầu từ keycode `min`.
/// Đây là phần logic thuần (không cần X server) nên được unit-test trực tiếp.
pub fn find_keycode(keysyms: &[u32], per: usize, min: u8, target: u32) -> Option<u8> {
    if per == 0 {
        return None;
    }
    for (i, chunk) in keysyms.chunks(per).enumerate() {
        if chunk.contains(&target) {
            return Some(min.wrapping_add(i as u8));
        }
    }
    None
}

/// Tra keycode cho một keysym qua bảng phím hiện tại của X server.
fn keysym_to_keycode<C: Connection>(conn: &C, keysym: u32) -> Option<u8> {
    let setup = conn.setup();
    let min = setup.min_keycode;
    let max = setup.max_keycode;
    let count = max.saturating_sub(min).saturating_add(1);
    let mapping = conn.get_keyboard_mapping(min, count).ok()?.reply().ok()?;
    let per = mapping.keysyms_per_keycode as usize;
    find_keycode(&mapping.keysyms, per, min, keysym)
}

/// Tiêm `n` phím BackSpace qua XTest. Trả về `false` nếu không kết nối được X server hoặc không tìm
/// thấy keycode (ví dụ Wayland thuần không có XWayland). Mỗi backspace là một cặp press + release.
pub fn fake_backspaces(n: u32) -> bool {
    fake_key_presses(KEYSYM_BACKSPACE, n)
}

/// Tiêm `n` lần nhấn (press + release) phím ứng với `keysym` qua XTest.
pub fn fake_key_presses(keysym: u32, n: u32) -> bool {
    if n == 0 {
        return true;
    }
    let Ok((conn, _screen)) = x11rb::connect(None) else {
        return false;
    };
    let Some(keycode) = keysym_to_keycode(&conn, keysym) else {
        return false;
    };
    let root = conn.setup().roots.first().map(|s| s.root).unwrap_or(0);
    for _ in 0..n {
        if conn
            .xtest_fake_input(KEY_PRESS_EVENT, keycode, 0, root, 0, 0, 0)
            .is_err()
        {
            return false;
        }
        if conn
            .xtest_fake_input(KEY_RELEASE_EVENT, keycode, 0, root, 0, 0, 0)
            .is_err()
        {
            return false;
        }
    }
    conn.flush().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_keycode_locates_target_row() {
        // 3 keysym mỗi keycode; keycode bắt đầu từ 8 (giá trị min điển hình của X).
        // hàng 0 (kc 8): [a, A, 0]; hàng 1 (kc 9): [BackSpace, 0, 0]
        let keysyms = vec![0x61, 0x41, 0, 0xff08, 0, 0];
        assert_eq!(find_keycode(&keysyms, 3, 8, 0xff08), Some(9));
        assert_eq!(find_keycode(&keysyms, 3, 8, 0x61), Some(8));
    }

    #[test]
    fn find_keycode_missing_is_none() {
        let keysyms = vec![0x61, 0x41, 0, 0xff08, 0, 0];
        assert_eq!(find_keycode(&keysyms, 3, 8, 0xdead), None);
    }

    #[test]
    fn find_keycode_zero_per_is_none() {
        assert_eq!(find_keycode(&[], 0, 8, 0xff08), None);
    }
}
