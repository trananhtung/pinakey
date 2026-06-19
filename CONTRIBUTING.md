# Đóng góp

Cảm ơn bạn đã chung tay với PinaKey. Hướng dẫn này trình bày quy trình làm việc cục bộ, các cổng
chất lượng mà CI bắt buộc, và cách tạo lại dữ liệu được sinh tự động.

## Yêu cầu tiên quyết

- Một bộ toolchain Rust stable gần đây (khuyến nghị `rustup`) kèm `rustfmt` và `clippy`:
  ```sh
  rustup component add rustfmt clippy
  ```
- Các thư viện hệ thống cho phần X11 / D-Bus (tên gói trên Debian/Ubuntu):
  ```sh
  sudo apt-get install -y libxcb1-dev libdbus-1-dev pkg-config
  ```
- Python 3 (chỉ để tạo lại các bảng charset).

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

Xem [ARCHITECTURE.md](ARCHITECTURE.md) để biết đồ thị phụ thuộc giữa các crate và hai quyết định
thiết kế then chốt (alias con trỏ → `Rc<RefCell>`, và thread actor của engine). Tóm lại:

- Đặt **logic biến đổi thuần túy** trong `pinakey-core`. Nó không có I/O và không phụ thuộc vào các
  crate anh em.
- Đặt **hành vi IBus độc lập với lớp truyền tải** trong `pinakey-ibus::core`, trả về các `Action`
  để nó vẫn unit-test được mà không cần daemon đang chạy. Giữ `pinakey-ibus::dbus` là một lớp dịch mỏng.
- Giữ mỗi file tập trung; nếu một module bắt đầu làm vài việc không liên quan, hãy tách nó ra.

## Test

- `pinakey-core` được bao phủ bởi bộ test hành vi trong `crates/pinakey-core/tests/`. Khi bạn thêm
  một hành vi biến đổi, hãy thêm test đi kèm.
- Hãy test **logic thuần túy**, không phải phần đường ống D-Bus/màn hình (mà CI không chạy được).
  Nếu bạn thêm hành vi IBus, hãy biểu diễn nó dưới dạng các `core::Action` và test chúng.

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

## Chưa hiện thực

Chế độ nhập Preedit mặc định đã hoạt động đầu-cuối. Các phần làm tiếp lớn hơn (chế độ sửa-lỗi-bằng-
Backspace + tiêm phím, bảng tra cứu emoji/hex, phím tắt, menu thuộc tính, kiểm tra chính tả bằng từ
điển, giao diện thiết lập đồ họa) được liệt kê trong [README.md](README.md#chưa-hiện-thực-phần-làm-tiếp).
Mỗi phần cần một daemon IBus đang chạy + màn hình để kiểm tra đầy đủ.
