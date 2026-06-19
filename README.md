# PinaKey

**PinaKey** là một bộ gõ tiếng Việt (IME) cho Linux/IBus, viết hoàn toàn bằng Rust thuần —
gõ Telex / VNI / VIQR mà không cần cgo. Giao thức IBus được hiện thực trên
[`zbus`](https://crates.io/crates/zbus) và tích hợp X11 qua
[`x11rb`](https://crates.io/crates/x11rb).

## Về cái tên

<img src="docs/assets/francisco-de-pina.jpg" alt="Francisco de Pina (trong tranh khắc cùng Alexandre de Rhodes)" align="right" width="200">

**PinaKey** tri ân **Francisco de Pina** (1585–1625), giáo sĩ Dòng Tên người Bồ Đào Nha, người
đầu tiên La-tinh hóa tiếng Việt một cách có hệ thống tại Thanh Chiêm – Hội An và đặt nền móng cho
**chữ Quốc Ngữ** — thứ chữ mà mọi bàn phím tiếng Việt ngày nay đều gõ. Ông là thầy dạy tiếng Việt
cho Alexandre de Rhodes và thường bị lãng quên sau cái bóng của học trò; bộ gõ này là một lời tri
ân nhỏ. Hậu tố **"Key"** đánh dấu nó là một bộ gõ (keyboard / input method).

> PinaKey tham khảo ý tưởng từ **Bamboo** (bộ gõ ibus-bamboo).

## Bố cục workspace

| Crate | Trách nhiệm | Trạng thái |
|-------|-------------|------------|
| `pinakey-core` | Biến đổi Telex/VNI/VIQR, kiểm tra chính tả, mã hóa charset. | ✅ Hoàn chỉnh — 47 test biến đổi đều pass. |
| `pinakey-config` | Cấu hình JSON, feature flag, đường dẫn cấu hình. | ✅ Hoàn chỉnh. |
| `pinakey-emoji` | Trie emoji + bảng macro. | ✅ Hoàn chỉnh. |
| `pinakey-ibus` | Logic engine chế độ Preedit + lớp truyền tải D-Bus IBus đầy đủ (zbus). | ✅ Hoàn chỉnh. |
| `pinakey-platform` | Nhận diện class của cửa sổ đang focus trên X11 (XWayland). | ◐ Wayland thuần + tiêm phím XTest là phần làm tiếp. |
| `pinakey` (bin) | Binary của engine: chế độ `--version` và `--ibus` nhúng. | ✅ |

Engine biến đổi (`pinakey-core`) là trái tim của dự án và được bao phủ bởi một bộ test hành vi,
ánh xạ các con trỏ `*Transformation` được alias sang `Rc<RefCell<Transformation>>`
(định danh con trỏ → `Rc::ptr_eq`, đột biến → `borrow_mut`).

## Biên dịch

```sh
cargo build --workspace          # tất cả crate + binary
cargo test --workspace           # 62 test
cargo fmt --all --check          # cổng kiểm định dạng (CI bắt buộc)
cargo clippy --workspace --all-targets -- -D warnings   # cổng lint
./target/debug/pinakey --version
```

Xem [ARCHITECTURE.md](ARCHITECTURE.md) để biết đồ thị phụ thuộc giữa các crate và lý do thiết kế,
và [CONTRIBUTING.md](CONTRIBUTING.md) để biết quy trình phát triển cũng như cách tạo lại các bảng dữ liệu.

## Cài đặt (Linux / IBus)

### Cách nhanh nhất — một dòng lệnh

```sh
curl -fsSL https://raw.githubusercontent.com/trananhtung/pinakey/main/tools/install-online.sh | bash
```

Lệnh này tự nhận diện CPU (**x86_64** hoặc **aarch64**), tải binary của bản release mới nhất,
đăng ký engine với IBus và thêm PinaKey vào nguồn nhập GNOME. Sau khi cài, nhấn **Super+Space**
để chuyển sang *PinaKey — Bộ gõ tiếng Việt* và gõ Telex (ví dụ `vieetj` → `việt`).

**Yêu cầu:** một bản Linux có **IBus** (GNOME mặc định đã có), lệnh `curl`, và quyền `sudo`
(chỉ dùng để đặt file component vào `/usr/share/ibus/component` — nơi duy nhất IBus quét).

Gỡ cài đặt bất cứ lúc nào:

```sh
curl -fsSL https://raw.githubusercontent.com/trananhtung/pinakey/main/tools/install-online.sh | bash -s -- --uninstall
```

> Thích tự tay hơn? Mỗi bản [Releases](https://github.com/trananhtung/pinakey/releases) đính kèm
> sẵn binary `pinakey-x86_64` / `pinakey-aarch64` kèm file `.sha256` để bạn tải và kiểm tra thủ công.

### Cài từ mã nguồn (cho người phát triển)

```sh
cargo build --release -p pinakey
! bash tools/install.sh      # cài binary + icon vào ~/.local/lib/pinakey, chép component XML
                             # vào /usr/share/ibus/component (cần sudo), làm mới IBus, và thêm
                             # PinaKey vào danh sách nguồn nhập GNOME
```

Gỡ bằng `bash tools/uninstall.sh`. Một bài kiểm tra đầu-cuối trực tiếp nằm ở
`cargo run -p pinakey-ibus --example smoketest`.

## Ghi chú kiến trúc

- `pinakey-core` dựa trên `Rc` (đơn luồng). Vì interface zbus bắt buộc phải `Send + Sync`, engine
  được chạy trên một thread riêng phía sau một actor giao tiếp qua channel (`pinakey-ibus::EngineHandle`).
- Logic xử lý phím ở chế độ Preedit (`pinakey-ibus::core`) độc lập với lớp truyền tải: nó trả về
  một danh sách `Action` (commit / cập-nhật-preedit / ẩn), nhờ vậy toàn bộ hành vi IME được
  unit-test mà không cần một daemon IBus đang chạy. Lớp D-Bus dịch các `Action` thành tín hiệu IBus.

## Chưa hiện thực (phần làm tiếp)

Chế độ nhập Preedit mặc định đã hoạt động đầu-cuối. Những tính năng sau vẫn còn lại (mỗi tính năng
cần một daemon IBus đang chạy + màn hình để kiểm tra đầy đủ):

- Các chế độ nhập sửa-lỗi-bằng-Backspace và tiêm phím XTest / Wayland.
- Bảng tra cứu emoji và hexadecimal.
- Phím tắt, menu thuộc tính, kiểm tra chính tả dựa trên từ điển.
- Giao diện thiết lập đồ họa.

Xem `docs/superpowers/specs/2026-06-18-pinakey-design.md` để biết toàn bộ thiết kế.
