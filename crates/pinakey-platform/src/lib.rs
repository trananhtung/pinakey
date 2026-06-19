//! Tích hợp nền tảng cho pinakey.
//!
//! Cung cấp: (1) phát hiện class của cửa sổ đang focus trên X11 ([`x11`], dùng cho các workaround
//! riêng theo từng ứng dụng) và (2) tiêm phím giả lập qua XTest ([`inject`], cho các chế độ nhập
//! sửa lỗi bằng backspace). Việc đọc window-class trên Wayland thuần (`wl_introspector.go`) sẽ được
//! bổ sung sau; chế độ Preedit mặc định không cần đến lớp này.

pub mod inject;
pub mod x11;

pub use inject::{fake_backspaces, fake_key_presses, find_keycode};
pub use x11::{get_focus_window_class, parse_wm_class};
