//! IBus protocol constants — ported from `ibus_const.go` and goibus `common.go`.

// Modifier-state masks.
pub const IBUS_SHIFT_MASK: u32 = 1 << 0;
pub const IBUS_LOCK_MASK: u32 = 1 << 1;
pub const IBUS_CONTROL_MASK: u32 = 1 << 2;
pub const IBUS_MOD1_MASK: u32 = 1 << 3;
pub const IBUS_MOD4_MASK: u32 = 1 << 6;
pub const IBUS_HANDLED_MASK: u32 = 1 << 24;
pub const IBUS_FORWARD_MASK: u32 = 1 << 25;
pub const IBUS_IGNORED_MASK: u32 = IBUS_FORWARD_MASK;
pub const IBUS_SUPER_MASK: u32 = 1 << 26;
pub const IBUS_HYPER_MASK: u32 = 1 << 27;
pub const IBUS_META_MASK: u32 = 1 << 28;
pub const IBUS_RELEASE_MASK: u32 = 1 << 30;

// Keyvals.
pub const IBUS_TAB: u32 = 0xff09;
pub const IBUS_BACKSPACE: u32 = 0xff08;
pub const IBUS_RETURN: u32 = 0xff0d;
pub const IBUS_ESCAPE: u32 = 0xff1b;
pub const IBUS_SPACE: u32 = 0x020;
pub const IBUS_COLON: u32 = 0x03a;
pub const IBUS_TILDE: u32 = 0x007e;

// Capabilities.
pub const IBUS_CAP_PREEDIT_TEXT: u32 = 1 << 0;
pub const IBUS_CAP_SURROUNDING_TEXT: u32 = 1 << 5;

// Preedit focus mode (passed to UpdatePreeditText).
pub const IBUS_ENGINE_PREEDIT_CLEAR: u32 = 0;
pub const IBUS_ENGINE_PREEDIT_COMMIT: u32 = 1;

// Text attribute types/values.
pub const IBUS_ATTR_TYPE_UNDERLINE: u32 = 1;
pub const IBUS_ATTR_UNDERLINE_SINGLE: u32 = 1;

// D-Bus names / paths / interfaces.
pub const BUS_DAEMON_NAME: &str = "org.freedesktop.DBus";
pub const BUS_PROPERTIES_NAME: &str = "org.freedesktop.DBus.Properties";
pub const IBUS_SERVICE_IBUS: &str = "org.freedesktop.IBus";
pub const IBUS_PATH_IBUS: &str = "/org/freedesktop/IBus";
pub const IBUS_IFACE_SERVICE: &str = "org.freedesktop.IBus.Service";
pub const IBUS_IFACE_ENGINE: &str = "org.freedesktop.IBus.Engine";
pub const IBUS_IFACE_ENGINE_FACTORY: &str = "org.freedesktop.IBus.Factory";

pub const COMPONENT_NAME: &str = "org.freedesktop.IBus.bamboo";
pub const ENGINE_NAME: &str = "Bamboo";
