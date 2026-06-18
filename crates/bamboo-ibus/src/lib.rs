//! IBus engine layer for ibus-bamboo.
//!
//! - [`core`] holds the transport-independent Preedit-mode engine logic (fully unit-tested).
//! - [`engine_actor`] runs the non-`Send` engine on its own thread behind a `Send + Sync` handle.
//! - The `dbus` transport (zbus) drives a live IBus daemon. It is behind the default `dbus`
//!   feature; disable it (`--no-default-features`) to build/test just the pure logic.

pub mod constants;
pub mod core;

pub use core::{Action, EngineCore};

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
