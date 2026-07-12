# Kiến trúc

PinaKey là một Cargo workspace (lõi Rust thuần) + một addon C++ cho fcitx5. Phụ thuộc chảy nghiêm
ngặt từ dưới lên; mỗi tầng hiểu và kiểm thử được độc lập. **Toàn bộ logic tiếng Việt nằm ở Rust**;
addon C++ chỉ là lớp tích hợp mỏng (mô hình fcitx5-cskk).

```
        ┌──────────────────────────────┐     ┌────────────────────────────┐
        │   fcitx5/ addon (C++ mỏng)   │     │  fcitx5/ daemon uinput (C++)│
        │   PinaKeyEngine : IMEngineV2 │     │  bơm Backspace cho app khó  │
        └───────────────┬──────────────┘     └────────────────────────────┘
                        │ C-ABI (cbindgen)
             ┌──────────▼──────────┐        ┌──────────────────────────┐
             │     pinakey-ffi     │        │  pinakey-settings (egui) │  GUI thiết lập
             │  staticlib + cdylib │        └─┬─────────────────────┬──┘
             └──────────┬──────────┘          │                     │
             ┌──────────▼──────────┐          │                     │
             │   pinakey-engine    │          │                     │
             │ lõi engine trung lập│          │                     │
             │  (Action, keysym)   │          │                     │
             └──────────┬──────────┘          │                     │
              ┌─────────┴────────┬────────────┼─────────┐           │
   ┌──────────▼────────┐   ┌─────▼────────────▼─┐   ┌───▼───────────▼─────┐
   │   pinakey-emoji   │   │   pinakey-config   │   │    pinakey-core     │  biến đổi + từ điển
   └───────────────────┘   └────────────────────┘   └─────────────────────┘
```

## Các crate

| Crate | Trách nhiệm | Phụ thuộc chính |
|-------|-------------|-----------------|
| `pinakey-core` | Biến đổi Telex/VNI/VIQR, kiểm tra chính tả, từ điển, charset. Logic thuần, đơn luồng, không I/O. | `once_cell`, `regex` |
| `pinakey-config` | Đọc/ghi cấu hình JSON, feature flag, đường dẫn. | `pinakey-core`, `serde`, `dirs` |
| `pinakey-emoji` | Tra emoji (fuzzy + trie), lịch sử gần dùng + bảng macro. | `serde` |
| `pinakey-engine` | **Lõi engine trung lập transport**: `EngineCore::process_key_event → (handled, Vec<Action>)`, không I/O. Keysym/modifier X11 trung lập (`keysym`). | core, config, emoji |
| `pinakey-ffi` | **C-ABI** (con trỏ mờ + con trỏ mượn) bọc `pinakey-engine`; header sinh bằng cbindgen. | `pinakey-engine`, `pinakey-emoji`, `serde_json` |
| `pinakey-settings` | Giao diện thiết lập đồ họa (egui, feature `gui`); controller logic luôn được test. | `pinakey-config`, `pinakey-core`, `eframe?` |
| `fcitx5/` (C++) | Addon `InputMethodEngineV2` gọi `pinakey-ffi`; daemon uinput bơm Backspace. | `pinakey-ffi`, Fcitx5::Core |

## Quyết định thiết kế đáng biết

### 1. Alias con trỏ → `Rc<RefCell<Transformation>>`

Thuật toán biến đổi giữ danh sách `Transformation` mà `target` của mỗi phần tử là con trỏ alias trỏ
tới phần tử khác trong cùng danh sách (dựa vào **định danh con trỏ** + **đột biến tại chỗ**). PinaKey
mô hình hóa bằng `Rc<RefCell<Transformation>>` (`Rc::ptr_eq` để so định danh, `borrow_mut` để đột
biến). Đây là lý do `pinakey-core` đơn luồng.

### 2. Lõi không-C++ dùng lại qua C-ABI (mô hình fcitx5-cskk)

Toàn bộ logic tiếng Việt ở Rust (`pinakey-engine`). Addon fcitx5 (C++) giữ một con trỏ mờ `PkEngine*`
cho mỗi input context, bơm `(keysym, state)` vào và đọc kết quả ra qua `pinakey-ffi`. Vì keyval của
fcitx5 và keysym X11 dùng chung bảng giá trị nên truyền thẳng, không cần ánh xạ. **Không** viết lại
logic tiếng Việt bằng C++.

## Luồng dữ liệu cho một lần gõ phím

```
fcitx5 ──keyEvent──▶ PinaKeyState (C++) ──pk_engine_process_key(sym,state)──▶ pinakey-ffi
                                  │ (EngineCore, trả (handled, Vec<Action>))
                                  ▼
   gõ thường:  commitString / setPreedit
   không gạch chân (SurroundingText): deleteSurroundingText(-n,n) + commitString
   không gạch chân (uinput): bơm n Backspace qua daemon + commitString
```

`Action` (`CommitText`, `UpdatePreedit`, `HidePreedit`, …) độc lập transport, nhờ vậy toàn bộ hành vi
được unit-test trong `pinakey-engine`/`pinakey-ffi` mà không cần daemon.

**Gõ không gạch chân** (`pinakey-ffi::process_key_replace`): so tiền tố chung giữa chuỗi đang hiển
thị và chuỗi mới rồi trả `(số ký tự xoá, chuỗi chèn)`; addon áp bằng `deleteSurroundingText` hoặc
bơm Backspace qua daemon uinput.

## Chiến lược kiểm thử

- `pinakey-core`, `pinakey-engine`, `pinakey-config`, `pinakey-emoji`: unit test cho logic thuần.
- `pinakey-ffi`: test chạy qua chính C-ABI (Telex/VNI, reset, đổi kiểu gõ, diff-and-replace của gõ
  không gạch chân, emoji, loại trừ app).
- `pinakey-settings`: test controller (đọc/ghi config) — không cần GUI.
- Addon **fcitx5**: test tích hợp chạy qua fcitx5 thật (`fcitx5/test/`): một dùng `testfrontend`; một
  dựng `InputContext` giả lập ô văn bản có *Surrounding Text* để kiểm tra gõ không gạch chân đầu-cuối.
  Chạy bằng `ctest --test-dir fcitx5/build`.

## Dữ liệu sinh tự động

- `crates/pinakey-core/src/charset_def.rs` (~2.100 mục) sinh bởi `tools/gen_charset.py`.
- `crates/pinakey-ffi/include/pinakey_ffi.h` sinh bởi cbindgen (`tools/gen-ffi-header.sh`).
Đừng sửa tay; xem [CONTRIBUTING.md](CONTRIBUTING.md).
