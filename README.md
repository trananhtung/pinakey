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
| `pinakey-core` | Biến đổi Telex/VNI/VIQR, kiểm tra chính tả (quy tắc + từ điển), mã hóa charset. | ✅ Hoàn chỉnh. |
| `pinakey-config` | Cấu hình JSON, feature flag, đường dẫn cấu hình. | ✅ Hoàn chỉnh. |
| `pinakey-emoji` | Trie emoji + bảng macro. | ✅ Hoàn chỉnh. |
| `pinakey-ibus` | Engine: Preedit + sửa-lỗi-Backspace, emoji/hex, phím tắt, menu thuộc tính, lớp D-Bus (zbus). | ✅ Hoàn chỉnh. |
| `pinakey-platform` | Nhận diện class của cửa sổ focus trên X11 (XWayland) + tiêm phím XTest. | ◐ Tiêm phím XTest ✅; đọc window-class trên Wayland thuần là phần làm tiếp. |
| `pinakey-settings` | Giao diện thiết lập đồ họa (egui/eframe thuần Rust) + controller logic. | ✅ Hoàn chỉnh. |
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
cargo build --release -p pinakey-settings --features gui   # giao diện thiết lập (tùy chọn)
! bash tools/install.sh      # cài engine + icon vào ~/.local/lib/pinakey, chép component XML
                             # vào /usr/share/ibus/component (cần sudo), làm mới IBus, thêm PinaKey
                             # vào nguồn nhập GNOME, và cài GUI thiết lập + mục menu (nếu đã build)
```

Mở giao diện thiết lập bằng `pinakey-settings`, mục **"PinaKey — Thiết lập"** trong menu ứng dụng,
hoặc bấm **"Mở bảng thiết lập…"** ngay trong menu IBus (biểu tượng **vi** trên thanh trên cùng) khi
đang chọn PinaKey.

Gỡ bằng `bash tools/uninstall.sh`. Các bài kiểm tra đầu-cuối trực tiếp (cần IBus daemon đang chạy):

```sh
cargo run -p pinakey-ibus --example smoketest             # chế độ Preedit
cargo run -p pinakey-ibus --example backspace_smoketest   # chế độ sửa lỗi bằng backspace
cargo run -p pinakey-ibus --example emoji_smoketest       # bảng tra cứu emoji + hex
cargo run -p pinakey-ibus --example shortcut_props_smoketest  # phím tắt + menu thuộc tính
```

## Ghi chú kiến trúc

- `pinakey-core` dựa trên `Rc` (đơn luồng). Vì interface zbus bắt buộc phải `Send + Sync`, engine
  được chạy trên một thread riêng phía sau một actor giao tiếp qua channel (`pinakey-ibus::EngineHandle`).
- Logic xử lý phím ở chế độ Preedit (`pinakey-ibus::core`) độc lập với lớp truyền tải: nó trả về
  một danh sách `Action` (commit / cập-nhật-preedit / ẩn), nhờ vậy toàn bộ hành vi IME được
  unit-test mà không cần một daemon IBus đang chạy. Lớp D-Bus dịch các `Action` thành tín hiệu IBus.

## Tính năng

- **Chế độ nhập**: Preedit (mặc định) và các chế độ **sửa-lỗi-bằng-Backspace** (Surrounding Text,
  ForwardKeyEvent — chạy cả Wayland, XTest — X11/XWayland), chọn bằng `DefaultInputMode`.
- **Bảng tra cứu emoji + hexadecimal**: gõ `:` ở đầu từ để mở (`:grin` → 😀, `:u+2764` → ❤);
  mũi tên/PageUp/PageDown để di chuyển, số `1`–`9` hoặc Space/Enter để chọn, Esc để hủy.
- **Phím tắt** (`Shortcuts` trong cấu hình): bật/tắt tiếng Việt, khôi phục phím gốc.
- **Menu thuộc tính** trên panel IBus: đổi nhanh kiểu gõ, bật/tắt tiếng Việt.
- **Kiểm tra chính tả**: theo quy tắc (mặc định) và theo **từ điển** (`IB_SPELL_CHECK_WITH_DICTS`;
  bộ từ khởi đầu đóng kèm + `~/.config/pinakey/dict.txt`).
- **Giao diện thiết lập đồ họa** (`pinakey-settings`, egui/eframe thuần Rust).

### Phần làm tiếp

- Đọc window-class trên **Wayland thuần** (hiện chỉ qua X11/XWayland); chế độ ForwardKeyEvent đã phủ
  phần lớn nhu cầu xóa lùi trên Wayland.

Xem `docs/superpowers/specs/2026-06-18-pinakey-design.md` để biết toàn bộ thiết kế.
