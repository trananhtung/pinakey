# CLAUDE.md

PinaKey — bộ gõ tiếng Việt (Telex/VNI/VIQR) cho fcitx5 trên Linux. **Lõi xử lý bằng Rust thuần**
(Cargo workspace) + **addon C++ mỏng** tích hợp fcitx5 dùng lại lõi Rust qua C-ABI.

Tài liệu nền (đọc khi cần chiều sâu): [ARCHITECTURE.md](ARCHITECTURE.md) (đồ thị phụ thuộc + quyết
định thiết kế), [CONTRIBUTING.md](CONTRIBUTING.md) (quy trình + cổng CI), [USAGE.md](USAGE.md)
(hướng dẫn người dùng), [docs/BENCHMARK.md](docs/BENCHMARK.md).

## Lệnh thường dùng (lõi Rust)

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all --check                                # cổng định dạng (CI bắt buộc)
cargo clippy --workspace --all-targets -- -D warnings  # cổng lint (CI, warning = lỗi)
```

Chạy cả 3 cổng trên **trước khi commit** — CI (`.github/workflows/ci.yml`) coi warning là lỗi.

Build/test addon fcitx5 (C++):

```sh
cmake -S fcitx5 -B fcitx5/build -DCMAKE_INSTALL_PREFIX=/usr
cmake --build fcitx5/build
ctest --test-dir fcitx5/build --output-on-failure
```

## Bố cục workspace

Phụ thuộc chảy từ dưới lên: `core/config/emoji` → `engine` → `ffi`/`settings` → addon C++.

| Crate | Trách nhiệm |
|-------|-------------|
| `pinakey-core` | Biến đổi Telex/VNI/VIQR, chính tả, từ điển, charset. **Logic thuần, đơn luồng, không I/O**, không phụ thuộc crate anh em. |
| `pinakey-config` | Cấu hình JSON, feature flag, đường dẫn. |
| `pinakey-emoji` | Tra emoji (fuzzy + trie), lịch sử gần dùng, macro. |
| `pinakey-engine` | Lõi engine trung lập transport: `process_key → (handled, Vec<Action>)`, không I/O. |
| `pinakey-ffi` | C-ABI (cbindgen) bọc `pinakey-engine` cho addon C++. |
| `pinakey-settings` | GUI thiết lập (egui, feature `gui`); controller luôn có test. |
| `fcitx5/` (C++) | Addon `InputMethodEngineV2` gọi `pinakey-ffi` + daemon uinput bơm Backspace. |

## Nơi đặt code

- Logic biến đổi **thuần** → `pinakey-core` (không I/O).
- Hành vi engine → `pinakey-engine`, biểu diễn dưới dạng `Action` để unit-test **không cần daemon**.
- Giữ addon `fcitx5/` là lớp dịch mỏng — **không viết lại logic tiếng Việt bằng C++**.
- Hành vi mới = thêm `Action` + test. Bug tái hiện được → sửa kèm test chống tái diễn.

## Gotcha (dễ vi phạm)

- **File sinh tự động — đừng sửa tay:**
  - `crates/pinakey-core/src/charset_def.rs` ← `tools/gen_charset.py` (cần `BAMBOO_GO_SRC`, chi tiết trong CONTRIBUTING).
  - `crates/pinakey-ffi/include/pinakey_ffi.h` ← `tools/gen-ffi-header.sh` (cần `cargo install cbindgen`).
  Sau khi sinh: chạy `cargo fmt --all` (bộ sinh có tính tất định, fmt chuẩn hóa lại).
- **`pinakey-core` phải đơn luồng**: thuật toán biến đổi dùng con trỏ alias mô hình bằng
  `Rc<RefCell<Transformation>>` (`Rc::ptr_eq` so định danh) — không thêm `Send`/`Sync`.
- **`panic = "abort"` ở profile release** (Cargo.toml): panic băng qua biên FFI vào C++ là UB;
  staticlib phát hành liên kết tĩnh vào addon. Đừng dùng `catch_unwind`/`should_panic` ở mã release.
- **Bit feature-flag phải giữ nguyên vị trí** (`pinakey-config/src/flags.rs`): giá trị `1 << n` được
  port khớp từ bản gốc Go; các bit lỗi thời vẫn giữ chỗ để giá trị số không đổi. Thêm flag mới =
  dùng bit trống kế tiếp, không tái dùng bit cũ.
- **Version — nguồn duy nhất là `Cargo.toml`** (`workspace.package.version`). `packaging/PKGBUILD` và
  `packaging/flake.nix` là file độc lập phải cập nhật khớp tay; CI chạy `tools/check-versions.sh` để
  bắt lệch. `fcitx5/CMakeLists.txt` đọc thẳng từ Cargo.toml nên tự khớp.
- **Lệch lint clippy**: chỉ dùng `#[allow(clippy::...)]` **khoanh hẹp + comment lý do** khi cố ý
  giữ dạng khớp thuật toán tham chiếu (khuôn mẫu: `flattener.rs`, `spelling.rs`, `transform_utils.rs`).
  Đừng tắt lint bao trùm ở cấp crate. Lint chung ở `[workspace.lints]` trong `Cargo.toml` gốc.

Nhiều module là **bản port từ Go** (Bamboo) — đó là lý do đôi chỗ giữ dạng không idiomatic để dễ
đối chiếu với thuật toán tham chiếu. Ưu tiên khớp hành vi gốc hơn viết lại "sạch".

## Test

- Ưu tiên test **logic thuần ở Rust** (`pinakey-engine`/`pinakey-ffi` qua C-ABI) — không cần daemon.
- Addon fcitx5: test tích hợp `fcitx5/test/` chạy qua `ctest --test-dir fcitx5/build`.
- E2E (fcitx5 thật + dbusfrontend): `bash tools/run-e2e.sh` (CI: `.github/workflows/e2e.yml`).
  Thêm ca: sửa danh sách `CASES` trong `fcitx5/test/e2e/pinakey_e2e.py`.

## Quy ước commit & PR

- **Conventional Commits bằng tiếng Việt**: `feat(engine): ...`, `fix(fcitx5): ...`,
  `test(fcitx5): ...`, `refactor(...)`, `docs(...)`, `chore(release): X.Y.Z`.
- Tham chiếu issue trong tiêu đề: `(#NN)`. Commit sửa theo review ghi `(góp ý review)`.
- Làm trên nhánh đặt tên `feat/NN-...` / `fix/NN-...` rồi mở PR (merge qua "Merge pull request").

## Lưu ý

- Ngôn ngữ dự án là **tiếng Việt** (README, tài liệu, comment, commit message) — giữ nhất quán.
- License GPL-3.0-or-later, Rust edition 2021, cần rustup >= 1.85.
- Landing page ở repo tách biệt `trananhtung/pinakey-web` (không phải repo này).
