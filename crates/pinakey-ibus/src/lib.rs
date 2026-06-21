//! Lớp engine IBus cho pinakey.
//!
//! - [`core`] chứa logic engine chế độ Preedit độc lập với transport (được unit test đầy đủ).
//! - [`engine_actor`] chạy engine (vốn không `Send`) trên thread riêng, ẩn sau một handle
//!   `Send + Sync`.
//! - Lớp transport `dbus` (zbus) điều khiển IBus daemon đang chạy. Nó nằm sau feature `dbus` mặc
//!   định; tắt nó (`--no-default-features`) để chỉ build/test phần logic thuần.

pub mod constants;

// Lõi engine nay nằm ở crate trung lập `pinakey-engine`; tái xuất để các bên dùng IBus
// (và transport zbus bên dưới) vẫn import từ `pinakey_ibus` như trước.
pub use pinakey_engine::{Action, EngineCore};

#[cfg(feature = "dbus")]
pub mod address;
#[cfg(feature = "dbus")]
pub mod dbus;
#[cfg(feature = "dbus")]
pub mod engine_actor;
#[cfg(feature = "dbus")]
pub mod serialize;

#[cfg(feature = "dbus")]
pub use engine_actor::EngineHandle;
