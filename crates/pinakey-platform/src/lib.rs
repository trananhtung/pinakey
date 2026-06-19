//! Tích hợp nền tảng cho pinakey.
//!
//! Hiện cung cấp việc phát hiện class của cửa sổ đang focus trên X11 (dùng cho các workaround
//! riêng theo từng ứng dụng). Việc đọc window-class trên Wayland (`wl_introspector.go`) và bơm
//! phím qua XTest (`x11_keyboard.c`, chỉ dùng cho các chế độ nhập sửa lỗi bằng backspace) sẽ được
//! bổ sung sau; chế độ Preedit mặc định không cần đến chúng.

pub mod x11;

pub use x11::{get_focus_window_class, parse_wm_class};
