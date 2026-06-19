//! Giao diện thiết lập đồ họa cho PinaKey.
//!
//! - [`controller`] chứa toàn bộ logic (nạp/sửa/lưu cấu hình), được unit-test đầy đủ và không phụ
//!   thuộc bất kỳ thư viện GUI nào.
//! - `gui` (sau feature `gui`) là lớp vẽ bằng eframe/egui, mỏng, chỉ điều khiển controller.

pub mod controller;
pub mod fonts;

pub use controller::{settings_flags, SettingsController};

#[cfg(feature = "gui")]
pub mod gui;
