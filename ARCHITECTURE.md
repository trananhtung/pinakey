# Kiến trúc

PinaKey là một Cargo workspace gồm sáu crate. Phụ thuộc chảy nghiêm ngặt từ dưới lên; không crate
nào phụ thuộc vào crate phía trên nó, nhờ vậy mỗi tầng có thể được hiểu và kiểm thử một cách độc lập.

```
                 ┌─────────────────────┐
                 │       pinakey       │  binary: phân tích tham số, runtime tokio
                 │    (src/main.rs)    │
                 └──────────┬──────────┘
                            │
                 ┌──────────▼──────────┐
                 │    pinakey-ibus     │  engine IBus: truyền tải D-Bus + logic xử lý phím
                 └───┬──────────┬────┬─┘
          ┌─────────┘           │    └──────────────┐
          │                     │                   │
┌─────────▼────────┐ ┌──────────▼─────┐ ┌───────────▼───────┐
│  pinakey-config  │ │  pinakey-emoji │ │  pinakey-platform │  tích hợp X11/Wayland
└─────────┬────────┘ └──────────┬─────┘ └───────────────────┘
          │                     │
          └──────────┬──────────┘
                     │
          ┌──────────▼──────────┐
          │    pinakey-core     │  engine biến đổi (không I/O, không phụ thuộc crate anh em)
          └─────────────────────┘
```

## Các crate

| Crate | Trách nhiệm | Phụ thuộc chính |
|-------|-------------|-----------------|
| `pinakey-core` | Biến đổi Telex/VNI/VIQR, kiểm tra chính tả, mã hóa charset. Logic thuần túy, đơn luồng, không I/O. | `once_cell`, `regex` |
| `pinakey-config` | Đọc/ghi cấu hình JSON, feature flag, đường dẫn cấu hình. | `pinakey-core`, `serde`, `dirs` |
| `pinakey-emoji` | Tra cứu trie emoji và bảng macro. | `serde` |
| `pinakey-ibus` | Engine IBus: phần xử lý phím độc lập với lớp truyền tải (`core`) cộng với bề mặt giao thức D-Bus (`dbus`, nằm sau feature `dbus`). | ba crate phía trên, `pinakey-platform`, `zbus` |
| `pinakey-platform` | Nhận diện class của cửa sổ đang focus (X11). Phần introspection Wayland và tiêm phím XTest là phần làm tiếp. | `x11rb` |
| `pinakey` | Binary. Phân tích tham số (`--version`, `--ibus`) và khởi động engine nhúng. | `pinakey-ibus`, `tokio` |

## Hai quyết định thiết kế đáng biết

### 1. Alias con trỏ → `Rc<RefCell<Transformation>>`

Thuật toán biến đổi giữ một danh sách các `Transformation`, trong đó `target` của mỗi phần tử là
một con trỏ alias trỏ tới một phần tử khác trong cùng danh sách, dựa vào **định danh con trỏ** và
**đột biến tại chỗ**. PinaKey mô hình hóa điều này bằng:

- `Rc<RefCell<Transformation>>` (được alias là: `TransRef`) — các node chia sẻ, khả biến.
- `Rc::ptr_eq` — so sánh định danh con trỏ.
- `Rc::as_ptr(..) as usize` — một khóa ổn định cho map nối thêm.
- `borrow_mut()` — đột biến tại chỗ.

Đây là lý do `pinakey-core` đơn luồng: `Rc`/`RefCell` không phải `Send`/`Sync`.

### 2. Engine không `Send` đối đầu D-Bus `Send + Sync` → thread actor

`zbus` đòi hỏi các đối tượng interface phải `Send + Sync`, nhưng `pinakey-core` dựa trên `Rc` và
không thể vượt qua ranh giới thread. Vì vậy engine chạy trên **thread riêng của nó** phía sau một
actor giao tiếp qua channel, `pinakey-ibus::EngineHandle`. Handle này `Send + Sync` và chuyển tiếp
sự kiện phím / reset / cập nhật window-class qua một channel `mpsc`; thread engine sở hữu phần
trạng thái không `Send`.

## Luồng dữ liệu cho một lần gõ phím

```
IBus daemon ──ProcessKeyEvent──▶ dbus::PinaKeyEngine
                                      │ EngineHandle.process_key(keyval, keycode, state)
                                      ▼
                              thread engine: core::EngineCore.process_key_event
                                      │ trả về (handled: bool, Vec<Action>)
                                      ▼
                              dbus::apply_actions  ──phát tín hiệu IBus──▶ IBus daemon
```

`core::Action` (`CommitText`, `UpdatePreedit`, `HidePreedit`, …) độc lập với lớp truyền tải, nhờ
vậy toàn bộ hành vi chế độ Preedit được unit-test trong `pinakey-ibus` **mà không cần** một daemon
IBus đang chạy. Việc duy nhất của lớp D-Bus là dịch các `Action` thành tín hiệu IBus.

## Chiến lược kiểm thử

- `pinakey-core` được bao phủ bởi một bộ test hành vi trong `crates/pinakey-core/tests/`
  (`transformation.rs`, `utils.rs`, `rules_parser.rs`), chạy trên public API như một consumer bên ngoài.
- `pinakey-ibus::core`, `pinakey-config`, `pinakey-emoji`, và `pinakey-platform::parse_wm_class`
  đều có unit test cho phần logic thuần túy của chúng.
- Các đường D-Bus và màn hình trực tiếp không thể chạy trong CI (không có daemon IBus / màn hình);
  chúng chỉ được kiểm tra biên dịch và giữ mỏng để phần `core` đã được test gánh hành vi.

## Dữ liệu được sinh tự động

`crates/pinakey-core/src/charset_def.rs` (~2.100 mục charset) là file **được sinh tự động** bởi
`tools/gen_charset.py`. Đừng sửa tay. Xem
[CONTRIBUTING.md](CONTRIBUTING.md#tạo-lại-các-bảng-charset).

Xem [README.md](README.md) để biết hướng dẫn biên dịch và danh sách các tính năng chưa hiện thực.
