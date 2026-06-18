//! Platform integration for pinakey.
//!
//! Currently provides X11 focused-window class detection (used for per-application workarounds).
//! Wayland window-class introspection (`wl_introspector.go`) and XTest key injection
//! (`x11_keyboard.c`, used only by the backspace-correction input modes) are planned follow-ups;
//! the default Preedit mode does not require them.

pub mod x11;

pub use x11::{get_focus_window_class, parse_wm_class};
