//! Engine PinaKey trung lập với transport.
//!
//! Đây là "trái tim không I/O" của bộ gõ: [`EngineCore::process_key_event`] nhận `(keyval, keycode,
//! state)` và trả về `(handled, Vec<Action>)`. Không có D-Bus, không có fcitx5, không có mạng — nhờ
//! vậy mọi hành vi kiểm thử được bằng unit test, và mọi frontend (IBus, fcitx5, …) đều dùng lại
//! đúng một lõi này.
//!
//! - [`keysym`] — hằng keysym/modifier trung lập (giá trị X11 dùng chung cho IBus và fcitx5).
//! - [`engine`] — logic engine chế độ Preedit (chuyển từ `engine_preedit.go`).

pub mod engine;
pub mod keysym;

pub use engine::{Action, EngineCore};
