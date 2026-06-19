//! Các hằng số của giao thức IBus — chuyển thể từ `ibus_const.go` và `common.go` của goibus.

// Mặt nạ (mask) cho trạng thái phím bổ trợ (modifier).
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

// Mã phím (keyval).
pub const IBUS_TAB: u32 = 0xff09;
pub const IBUS_BACKSPACE: u32 = 0xff08;
pub const IBUS_RETURN: u32 = 0xff0d;
pub const IBUS_ESCAPE: u32 = 0xff1b;
pub const IBUS_SPACE: u32 = 0x020;
pub const IBUS_COLON: u32 = 0x03a;
pub const IBUS_TILDE: u32 = 0x007e;

// Phím điều hướng (dùng cho bảng tra cứu emoji/hex).
pub const IBUS_LEFT: u32 = 0xff51;
pub const IBUS_UP: u32 = 0xff52;
pub const IBUS_RIGHT: u32 = 0xff53;
pub const IBUS_DOWN: u32 = 0xff54;
pub const IBUS_PAGE_UP: u32 = 0xff55;
pub const IBUS_PAGE_DOWN: u32 = 0xff56;

// Keycode phần cứng của phím BackSpace (KEY_BACKSPACE của evdev = 14) dùng khi forward key event,
// khớp với giá trị bản gốc ibus-bamboo dùng cho chế độ sửa lỗi bằng backspace.
pub const BACKSPACE_KEYCODE: u32 = 14;

// Khả năng (capability).
pub const IBUS_CAP_PREEDIT_TEXT: u32 = 1 << 0;
pub const IBUS_CAP_SURROUNDING_TEXT: u32 = 1 << 5;

// Chế độ xử lý preedit khi mất focus (truyền cho UpdatePreeditText).
pub const IBUS_ENGINE_PREEDIT_CLEAR: u32 = 0;
pub const IBUS_ENGINE_PREEDIT_COMMIT: u32 = 1;

// Loại và giá trị thuộc tính của văn bản.
pub const IBUS_ATTR_TYPE_UNDERLINE: u32 = 1;
pub const IBUS_ATTR_UNDERLINE_SINGLE: u32 = 1;

// Tên / đường dẫn / interface của D-Bus.
pub const BUS_DAEMON_NAME: &str = "org.freedesktop.DBus";
pub const BUS_PROPERTIES_NAME: &str = "org.freedesktop.DBus.Properties";
pub const IBUS_SERVICE_IBUS: &str = "org.freedesktop.IBus";
pub const IBUS_PATH_IBUS: &str = "/org/freedesktop/IBus";
pub const IBUS_IFACE_SERVICE: &str = "org.freedesktop.IBus.Service";
pub const IBUS_IFACE_ENGINE: &str = "org.freedesktop.IBus.Engine";
pub const IBUS_IFACE_ENGINE_FACTORY: &str = "org.freedesktop.IBus.Factory";

// "PinaKey" tri ân Francisco de Pina (1585–1625), người đặt nền móng cho chữ Quốc Ngữ.
// Tên IBus riêng cho ứng dụng để PinaKey cài đặt và chạy song song với các bộ gõ tiếng Việt
// khác mà không xung đột tên trên bus.
pub const COMPONENT_NAME: &str = "org.freedesktop.IBus.pinakey";
pub const ENGINE_NAME: &str = "PinaKey";
