//! Lớp engine IBus cho pinakey.
//!
//! - [`core`] chứa logic engine chế độ Preedit độc lập với transport (được unit test đầy đủ).
//! - [`engine_actor`] chạy engine (vốn không `Send`) trên thread riêng, ẩn sau một handle
//!   `Send + Sync`.
//! - Lớp transport `dbus` (zbus) điều khiển IBus daemon đang chạy. Nó nằm sau feature `dbus` mặc
//!   định; tắt nó (`--no-default-features`) để chỉ build/test phần logic thuần.

pub mod backspace;
pub mod constants;
pub mod core;
pub mod lookup;
pub mod props;
pub mod shortcuts;

pub use backspace::{correction_actions, diff_correction, Correction};
pub use core::{Action, EngineCore};
pub use lookup::{compute_candidates, hex_to_char, EmojiState};
pub use props::{build_props, Prop, PropKind};
pub use shortcuts::{decode_modifier, match_shortcut, ShortcutAction};

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
