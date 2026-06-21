//! Hằng keysym/modifier **trung lập với transport**.
//!
//! Trước đây engine tham chiếu trực tiếp các hằng `IBUS_*`. Vì keyval của IBus, keysym của fcitx5
//! và keysym của X11/XKB **dùng chung một bảng giá trị** (đều bắt nguồn từ `<X11/keysymdef.h>` và
//! `<X11/X.h>`), ta đặt tên trung lập ở đây để engine không phụ thuộc vào một frontend cụ thể.
//! Mỗi frontend (IBus qua `zbus`, hay addon fcitx5 qua C-ABI) chỉ việc nạp keysym/modifier của nó
//! vào — giá trị số khớp nhau nên không cần ánh xạ.
//!
//! Tham chiếu: `FcitxKey_BackSpace == IBUS_BackSpace == XK_BackSpace == 0xff08`, và bố cục bit của
//! `KeyState`/`IBusModifierType`/`XKB modmask` trùng nhau (Shift=bit0, Lock=bit1, Control=bit2,
//! Mod1=bit3, …).

// ----- Mặt nạ modifier (trùng bố cục bit X11/XKB; IBus và fcitx5 đều dùng) -----
pub const MOD_SHIFT: u32 = 1 << 0;
pub const MOD_LOCK: u32 = 1 << 1; // Caps Lock
pub const MOD_CONTROL: u32 = 1 << 2;
pub const MOD_MOD1: u32 = 1 << 3; // thường là Alt
pub const MOD_MOD4: u32 = 1 << 6; // thường là Super/Win
pub const MOD_HANDLED: u32 = 1 << 24; // chỉ IBus đặt bit này; vô hại với fcitx5
pub const MOD_FORWARD: u32 = 1 << 25;
pub const MOD_IGNORED: u32 = MOD_FORWARD;
pub const MOD_SUPER: u32 = 1 << 26;
pub const MOD_HYPER: u32 = 1 << 27;
pub const MOD_META: u32 = 1 << 28;
/// Phím được nhả (key release). IBus mã hoá thành bit của `state`; fcitx5 báo qua `isRelease()` —
/// lớp C-ABI sẽ bật bit này khi đó để engine xử lý đồng nhất.
pub const MOD_RELEASE: u32 = 1 << 30;

// ----- Mã phím (keysym) -----
pub const KEY_TAB: u32 = 0xff09;
pub const KEY_BACKSPACE: u32 = 0xff08;
pub const KEY_RETURN: u32 = 0xff0d;
pub const KEY_ESCAPE: u32 = 0xff1b;
pub const KEY_SPACE: u32 = 0x020;
pub const KEY_COLON: u32 = 0x03a;
pub const KEY_TILDE: u32 = 0x007e;
