# Kiến trúc

PinaKey là một Cargo workspace gồm bảy crate Rust + một addon C++ (`fcitx5/`). Phụ thuộc chảy
nghiêm ngặt từ dưới lên; không crate nào phụ thuộc vào crate phía trên nó, nhờ vậy mỗi tầng có thể
được hiểu và kiểm thử một cách độc lập. **Hai frontend** (IBus và fcitx5) dùng chung một lõi
`pinakey-engine`.

```
   ┌────────────────────┐        ┌─────────────────────────────┐
   │      pinakey       │        │   fcitx5/ (addon C++ mỏng)  │ frontend fcitx5
   │   (src/main.rs)    │        │   PinaKeyEngine : IMEngineV2 │
   └─────────┬──────────┘        └──────────────┬──────────────┘
             │                                  │ C-ABI
   ┌─────────▼──────────┐        ┌──────────────▼──────────────┐
   │    pinakey-ibus    │        │         pinakey-ffi         │ cbindgen header
   │ truyền tải D-Bus   │        │   (staticlib/cdylib C-ABI)  │
   └─────────┬──────────┘        └──────────────┬──────────────┘
             └───────────────┬──────────────────┘
                  ┌──────────▼──────────┐
                  │   pinakey-engine    │  lõi engine trung lập transport (Action, keysym)
                  └──────────┬──────────┘   (+ pinakey-platform cho IBus)
          ┌─────────────────┼─────────────────┐
┌─────────▼────────┐ ┌──────▼─────────┐ ┌──────▼────────────┐
│  pinakey-config  │ │  pinakey-emoji │ │  pinakey-platform │  tích hợp X11/Wayland
└─────────┬────────┘ └──────┬─────────┘ └───────────────────┘
          └─────────┬───────┘
          ┌─────────▼──────────┐
          │    pinakey-core    │  engine biến đổi (không I/O, không phụ thuộc crate anh em)
          └────────────────────┘
```

## Các crate

| Crate | Trách nhiệm | Phụ thuộc chính |
|-------|-------------|-----------------|
| `pinakey-core` | Biến đổi Telex/VNI/VIQR, kiểm tra chính tả, mã hóa charset. Logic thuần túy, đơn luồng, không I/O. | `once_cell`, `regex` |
| `pinakey-config` | Đọc/ghi cấu hình JSON, feature flag, đường dẫn cấu hình. | `pinakey-core`, `serde`, `dirs` |
| `pinakey-emoji` | Tra cứu trie emoji và bảng macro. | `serde` |
| `pinakey-engine` | **Lõi engine trung lập transport**: `EngineCore::process_key_event → (handled, Vec<Action>)`, không I/O. Hằng keysym/modifier X11 trung lập (`keysym`). | `pinakey-core`, `pinakey-config`, `pinakey-emoji` |
| `pinakey-ibus` | Frontend IBus: bề mặt giao thức D-Bus (`dbus`) dịch `Action` thành tín hiệu IBus. | `pinakey-engine`, `pinakey-platform`, `zbus` |
| `pinakey-ffi` | **C-ABI** (con trỏ mờ + con trỏ mượn) bọc `pinakey-engine`; header sinh bằng cbindgen. Dùng cho addon fcitx5. | `pinakey-engine`, `serde_json` |
| `pinakey-platform` | Nhận diện class của cửa sổ đang focus (X11). Phần introspection Wayland và tiêm phím XTest là phần làm tiếp. | `x11rb` |
| `pinakey` | Binary. Phân tích tham số (`--version`, `--ibus`) và khởi động engine nhúng. | `pinakey-ibus`, `tokio` |
| `fcitx5/` (C++) | Frontend fcitx5: addon `InputMethodEngineV2` mỏng gọi `pinakey-ffi`; dịch `Action` thành `commitString`/preedit/`deleteSurroundingText`. Hỗ trợ gõ không gạch chân. | `pinakey-ffi`, Fcitx5::Core |

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
                       thread engine: pinakey_engine::EngineCore.process_key_event
                                      │ trả về (handled: bool, Vec<Action>)
                                      ▼
                              dbus::apply_actions  ──phát tín hiệu IBus──▶ IBus daemon

fcitx5 ──keyEvent──▶ PinaKeyState (C++) ──pk_engine_process_key(sym, state)──▶ pinakey-ffi
                                      │ (cùng EngineCore, qua C-ABI)
                                      ▼
                  commitString / setPreedit  (hoặc deleteSurroundingText+commitString
                  cho chế độ "gõ không gạch chân")  ──▶ ứng dụng
```

`Action` (`CommitText`, `UpdatePreedit`, `HidePreedit`, …) độc lập với lớp truyền tải, nhờ vậy toàn
bộ hành vi chế độ Preedit được unit-test trong `pinakey-engine`/`pinakey-ffi` **mà không cần** một
daemon. Mỗi frontend chỉ dịch `Action` sang API của nó.

**Gõ không gạch chân** (`pinakey-ffi::process_key_replace`): thay vì hiện preedit, lõi so tiền tố
chung giữa chuỗi đang hiển thị và chuỗi mới rồi trả về `(số ký tự xoá, chuỗi chèn)`; addon fcitx5
áp bằng `deleteSurroundingText(-n, n)` + `commitString`. Logic này được unit-test hoàn toàn trong
`pinakey-ffi`.

## Chiến lược kiểm thử

- `pinakey-core` được bao phủ bởi một bộ test hành vi trong `crates/pinakey-core/tests/`
  (`transformation.rs`, `utils.rs`, `rules_parser.rs`), chạy trên public API như một consumer bên ngoài.
- `pinakey-engine`, `pinakey-config`, `pinakey-emoji`, và `pinakey-platform::parse_wm_class`
  đều có unit test cho phần logic thuần túy của chúng.
- `pinakey-ffi` có unit test chạy qua chính C-ABI (Telex/VNI, reset, chuyển kiểu gõ, và toàn bộ
  chuỗi diff-and-replace của chế độ gõ không gạch chân).
- Addon **fcitx5** có test tích hợp chạy qua fcitx5 thật (`fcitx5/test/`): một dùng `testfrontend`
  của fcitx5 để kiểm tra commit; một dựng `InputContext` giả lập ô văn bản có *Surrounding Text* để
  kiểm tra gõ không gạch chân đầu-cuối. Chạy bằng `ctest --test-dir fcitx5/build`.
- Các đường D-Bus và màn hình trực tiếp không thể chạy trong CI (không có daemon IBus / màn hình);
  chúng chỉ được kiểm tra biên dịch và giữ mỏng để phần `engine` đã được test gánh hành vi.

## Dữ liệu được sinh tự động

`crates/pinakey-core/src/charset_def.rs` (~2.100 mục charset) là file **được sinh tự động** bởi
`tools/gen_charset.py`. Đừng sửa tay. Xem
[CONTRIBUTING.md](CONTRIBUTING.md#tạo-lại-các-bảng-charset).

Xem [README.md](README.md) để biết hướng dẫn biên dịch và danh sách các tính năng chưa hiện thực.
