# Đóng góp

Cảm ơn bạn đã chung tay với PinaKey. Hướng dẫn này trình bày quy trình làm việc cục bộ, các cổng
chất lượng mà CI bắt buộc, và cách tạo lại dữ liệu được sinh tự động.

## Yêu cầu tiên quyết

- Một bộ toolchain Rust stable gần đây (khuyến nghị `rustup`) kèm `rustfmt` và `clippy`:
  ```sh
  rustup component add rustfmt clippy
  ```
- Lõi Rust thuần (`cargo build/test --workspace`) **không cần** thư viện hệ thống đặc biệt
  (frontend IBus dùng x11rb/zbus đã được gỡ).
- Để build **addon fcitx5**, cài các gói fcitx5 dev — xem mục [Build addon fcitx5](#build-addon-fcitx5).
- Python 3 (để tạo lại bảng charset, và chạy [test E2E](#test-đầu-cuối-e2e)).

## Quy trình hằng ngày

```sh
cargo build --workspace          # build tất cả crate + binary
cargo test --workspace           # chạy toàn bộ test
cargo fmt --all                  # định dạng
cargo clippy --workspace --all-targets   # lint
```

## Cổng chất lượng (phải pass trước khi merge)

CI (`.github/workflows/ci.yml`) chạy các lệnh này với chế độ coi warning là lỗi. Hãy chạy cục bộ trước:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Định dạng được ghim bởi `rustfmt.toml`. Các lint áp dụng cho toàn workspace nằm dưới
`[workspace.lints]` trong `Cargo.toml` gốc; mỗi crate bật theo bằng `[lints] workspace = true`.

### Khi cần đi chệch khỏi một lint của clippy

Vài chỗ cố ý giữ một hình thức không "idiomatic" để phản chiếu đúng thuật toán tham chiếu (nhờ vậy
hành vi dễ đối chiếu với nó). Trong những trường hợp đó, hãy thêm một `#[allow(clippy::...)]`
**có chủ đích, khoanh vùng hẹp** kèm comment giải thích lý do — xem `flattener.rs`, `spelling.rs`,
`transform_utils.rs` để biết khuôn mẫu. Đừng tắt lint kiểu bao trùm ở cấp crate.

## Bố cục dự án

Xem [ARCHITECTURE.md](ARCHITECTURE.md) để biết đồ thị phụ thuộc giữa các crate và quyết định thiết
kế then chốt (alias con trỏ → `Rc<RefCell>`, lõi non-C++ dùng lại qua C-ABI). Tóm lại:

- Đặt **logic biến đổi thuần túy** trong `pinakey-core`. Nó không có I/O và không phụ thuộc các crate
  anh em.
- Đặt **hành vi engine độc lập transport** trong `pinakey-engine`, trả về các `Action` để unit-test
  được mà không cần daemon. Giữ addon fcitx5 (`fcitx5/`) là lớp dịch mỏng gọi qua `pinakey-ffi`.
- Giữ mỗi file tập trung; nếu một module bắt đầu làm vài việc không liên quan, hãy tách nó ra.

## Test

- `pinakey-core` / `pinakey-engine` được bao phủ bởi bộ test hành vi. Khi thêm một hành vi biến đổi,
  hãy thêm test đi kèm.
- Hãy test **logic thuần túy** ở Rust (`pinakey-engine`/`pinakey-ffi`). Hành vi mới nên biểu diễn
  dưới dạng `Action` và test chúng. Addon fcitx5 có test tích hợp trong `fcitx5/test/`
  (`ctest --test-dir fcitx5/build`).

## Tạo lại các bảng charset

`crates/pinakey-core/src/charset_def.rs` là file được sinh tự động — đừng bao giờ sửa tay. Dữ liệu
charset cũ bắt nguồn từ dự án tham chiếu thượng nguồn; để tạo lại sau khi nó thay đổi:

```sh
git clone https://github.com/BambooEngine/bamboo-core /tmp/bamboo-src
BAMBOO_GO_SRC=/tmp/bamboo-src python3 tools/gen_charset.py
cargo fmt --all          # bộ sinh xuất ra dạng gọn; fmt sẽ chuẩn hóa lại
git diff                 # rà soát thay đổi
```

Bộ sinh có tính tất định: tạo lại từ cùng một nguồn rồi chạy `cargo fmt` sẽ cho ra file giống hệt
từng byte.

## Build addon fcitx5

```sh
cmake -S fcitx5 -B fcitx5/build -DCMAKE_INSTALL_PREFIX=/usr
cmake --build fcitx5/build
ctest --test-dir fcitx5/build --output-on-failure
```

Tạo lại header C-ABI sau khi đổi `pinakey-ffi`: `tools/gen-ffi-header.sh` (cần `cargo install cbindgen`).

## Test đầu-cuối (E2E)

`fcitx5/test/e2e/pinakey_e2e.py` chạy fcitx5 THẬT + dbusfrontend, bơm phím qua D-Bus và kiểm chuỗi
ra (Telex, gõ không gạch chân qua SurroundingText, emoji…). CI (`.github/workflows/e2e.yml`) đóng
gói `.deb`, cài vào Ubuntu rồi chạy E2E ở mỗi PR. Chạy thủ công với bản đã cài hệ thống:

```sh
sudo cmake --install fcitx5/build      # hoặc cài .deb
bash tools/run-e2e.sh
```

Với bản cài ở `~/.local` (không sudo):

```sh
PINAKEY_E2E_ADDON_DIRS="$HOME/.local/lib/fcitx5:/usr/lib/x86_64-linux-gnu/fcitx5" \
PINAKEY_E2E_DATA_HOME="$HOME/.local/share" bash tools/run-e2e.sh
```

Thêm ca kiểm thử: sửa danh sách `CASES` trong `pinakey_e2e.py` (mỗi ca: nhãn, có SurroundingText?,
chuỗi phím, chuỗi mong đợi).
