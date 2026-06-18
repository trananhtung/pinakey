//! IBus engine layer for ibus-bamboo.
//!
//! `core` holds the transport-independent Preedit-mode engine logic (fully unit-tested).
//! The D-Bus/zbus transport that drives a live IBus daemon is added in `dbus` (feature-gated so
//! the pure logic builds and tests without system dependencies).

pub mod constants;
pub mod core;

pub use core::{Action, EngineCore};
