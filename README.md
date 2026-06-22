# PinaKey

[![All Contributors](https://img.shields.io/github/all-contributors/trananhtung/pinakey?color=ee8449&style=flat-square)](#người-đóng-góp)

**PinaKey** là một bộ gõ tiếng Việt (IME) cho Linux trên nền **fcitx5**, với **lõi xử lý viết hoàn
toàn bằng Rust thuần** (gõ Telex / VNI / VIQR, không cgo) và một **addon C++ mỏng** tích hợp vào
fcitx5. Trải nghiệm mặc định là **gõ không gạch chân** — chữ hiện thẳng như gõ thường, không có
preedit gạch chân.

🌐 **Trang giới thiệu:** **[trananhtung.github.io/pinakey-web](https://trananhtung.github.io/pinakey-web/)**
— landing page song ngữ Việt/Anh, có **sân chơi gõ thử Telex/VNI ngay trong trình duyệt**.
Mã nguồn trang web: [`trananhtung/pinakey-web`](https://github.com/trananhtung/pinakey-web).

📖 **Hướng dẫn sử dụng cho người dùng:** [USAGE.md](USAGE.md) — cài đặt, bật bộ gõ, bảng phím
Telex/VNI/VIQR, gõ không gạch chân, emoji, từ điển, gõ tắt, khắc phục sự cố.

> PinaKey tham khảo ý tưởng từ **Bamboo** (ibus-bamboo), **[fcitx5-lotus](https://github.com/LotusInputMethod/fcitx5-lotus)**
> (gõ không gạch chân) và **[fcitx5-cskk](https://github.com/fcitx/fcitx5-cskk)** (addon C++ bọc lõi
> không-C++ qua C-ABI).

## Về cái tên

<img src="docs/assets/francisco-de-pina.jpg" alt="Francisco de Pina (trong tranh khắc cùng Alexandre de Rhodes)" align="right" width="200">

**PinaKey** tri ân **Francisco de Pina** (1585–1625), giáo sĩ Dòng Tên người Bồ Đào Nha, người
đầu tiên La-tinh hóa tiếng Việt một cách có hệ thống tại Thanh Chiêm – Hội An và đặt nền móng cho
**chữ Quốc Ngữ** — thứ chữ mà mọi bàn phím tiếng Việt ngày nay đều gõ. Ông là thầy dạy tiếng Việt
cho Alexandre de Rhodes và thường bị lãng quên sau cái bóng của học trò; bộ gõ này là một lời tri
ân nhỏ. Hậu tố **"Key"** đánh dấu nó là một bộ gõ (keyboard / input method).

## Tính năng

- **Telex / VNI / VIQR** + nhiều biến thể dựng sẵn, kể cả **Telex đơn giản** (gõ dấu chặt).
- **Gõ không gạch chân**: với app hỗ trợ *Surrounding Text* (đa số GTK/Qt) commit thẳng + sửa tại
  chỗ; với app khác (terminal…) tự lùi về **preedit** (ổn định). Có chế độ **uinput thử nghiệm**
  (opt-in, không ổn định trên GNOME Wayland) — xem USAGE mục 9.
- **Bảng tra emoji** (`:tên`) và **nhập Unicode hex** (`:u<hex>`), chọn bằng số/Enter.
- **Menu** trên khay trạng thái: đổi kiểu gõ + bảng mã.
- **Từ điển chính tả** "giải oan" cho từ mượn (+ từ điển người dùng `~/.config/pinakey/dict.txt`).
- **Gõ tắt (macro)**, **loại trừ app tiếng Anh** (terminal/IDE…), **tự bỏ qua ô mật khẩu**.
- **Live-reload** file macro/dict khi sửa (không cần khởi động lại).
- **Giao diện thiết lập** đồ họa thuần Rust (egui).

## Bố cục workspace

| Crate | Trách nhiệm |
|-------|-------------|
| `pinakey-core` | Biến đổi Telex/VNI/VIQR, kiểm tra chính tả, từ điển, mã hóa charset. Logic thuần, không I/O. |
| `pinakey-config` | Cấu hình JSON, feature flag, đường dẫn cấu hình. |
| `pinakey-emoji` | Trie emoji + bảng macro. |
| `pinakey-engine` | **Lõi engine trung lập transport**: `process_key → (handled, Vec<Action>)`, không I/O. |
| `pinakey-ffi` | **C-ABI** (cbindgen) bọc `pinakey-engine` để addon fcitx5 C++ dùng lại lõi Rust. |
| `pinakey-settings` | Giao diện thiết lập đồ họa (egui, feature `gui`). |
| `fcitx5/` (C++) | **Addon fcitx5** (`InputMethodEngineV2`) + **daemon uinput** bơm Backspace. |

## Biên dịch & test (lõi Rust)

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all --check                                  # cổng định dạng (CI)
cargo clippy --workspace --all-targets -- -D warnings    # cổng lint (CI)
```

Xem [ARCHITECTURE.md](ARCHITECTURE.md) để biết đồ thị phụ thuộc và lý do thiết kế,
và [CONTRIBUTING.md](CONTRIBUTING.md) để biết quy trình phát triển.

## Cài đặt (fcitx5)

### Cách dễ nhất — gói `.deb` dựng sẵn (Ubuntu/Debian, khuyến nghị)

```sh
curl -fsSL https://raw.githubusercontent.com/trananhtung/pinakey/main/tools/install-deb.sh | bash
```

hoặc tải `.deb` mới nhất tại [Releases](https://github.com/trananhtung/pinakey/releases/latest) rồi
`sudo apt install ./fcitx5-pinakey_*.deb`. Sau đó xem **[3 bước bắt đầu gõ](USAGE.md#1-cài-đặt)**.

### Build từ nguồn

#### Phụ thuộc build (Debian/Ubuntu)

```sh
sudo apt install fcitx5 fcitx5-configtool libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev \
                 fcitx5-modules-dev extra-cmake-modules cmake g++ pkg-config
# + Rust (rustup) >= 1.85
```

#### Build & cài

```sh
bash tools/install-fcitx5.sh     # build + ctest + sudo cmake --install + restart fcitx5
```

hoặc thủ công:

```sh
cmake -S fcitx5 -B fcitx5/build -DCMAKE_INSTALL_PREFIX=/usr   # cargo tự build lõi Rust (staticlib)
cmake --build fcitx5/build
ctest --test-dir fcitx5/build --output-on-failure            # test tích hợp qua fcitx5 thật
sudo cmake --install fcitx5/build
fcitx5 -r -d
```

Sau đó: nếu fcitx5 chưa bật ở mức phiên, chạy `im-config -n fcitx5` rồi **đăng nhập lại** (tránh lỗi
“Not available”); mở **fcitx5-configtool** → thêm input method **PinaKey** (Tiếng Việt) → Ctrl+Space
để chuyển → gõ Telex, ví dụ `vieetj` → `việt`. Chi tiết: [USAGE.md](USAGE.md#1-cài-đặt).

> **Tự đóng gói** (deb/rpm/AUR/Nix) cho người phân phối: xem [packaging/](packaging/).

### Giao diện thiết lập (tùy chọn)

```sh
cargo build --release -p pinakey-settings --features gui
./target/release/pinakey-settings
```

### (Thử nghiệm) Gõ không gạch chân ở terminal — daemon uinput

> ⚠️ Tắt mặc định và **không ổn định trên GNOME Wayland** (frontend D-Bus không bảo đảm thứ tự
> xoá/commit → rối ký tự). Terminal mặc định dùng preedit. Chi tiết + cảnh báo: USAGE mục 9.

Cần cả 3: (1) build kèm `-DPINAKEY_BUILD_UINPUT_SERVER=ON` (mặc định OFF), (2) bật daemon, (3) đặt
env `PINAKEY_UINPUT=1` rồi đăng nhập lại.

```sh
cmake -S fcitx5 -B fcitx5/build -DPINAKEY_BUILD_UINPUT_SERVER=ON && cmake --build fcitx5/build && sudo cmake --install fcitx5/build
sudo udevadm control --reload && sudo udevadm trigger
systemctl --user enable --now pinakey-uinput-server
echo 'PINAKEY_UINPUT=1' >> ~/.config/environment.d/fcitx5.conf   # rồi đăng xuất/đăng nhập lại
```

## Ghi chú kiến trúc

- Logic xử lý phím nằm ở **`pinakey-engine`** (lõi trung lập transport): trả về danh sách `Action`
  (commit / cập-nhật-preedit / ẩn), unit-test được mà không cần daemon. Keysym/modifier dùng giá trị
  X11 — trùng với fcitx5 nên không cần ánh xạ.
- Addon fcitx5 (`fcitx5/`) gọi `pinakey-ffi` (C-ABI), rồi dịch `Action` thành lệnh fcitx5
  (`commitString` / preedit / `deleteSurroundingText`). Gõ không gạch chân = so tiền tố chung giữa
  chuỗi đang hiển thị và chuỗi mới → `(số ký tự xoá, chuỗi chèn)`.
- `pinakey-core` dùng `Rc` (đơn luồng); mỗi input context giữ một thực thể engine riêng.

## Lịch sử

PinaKey khởi đầu là bộ gõ IBus thuần Rust; từ EPIC #22 đã **chuyển hẳn sang fcitx5** để có gõ không
gạch chân mượt + Wayland vững (bơm Backspace mà IBus không cấp). Frontend IBus cũ đã được gỡ bỏ.

## Đóng góp

PinaKey rất hoan nghênh đóng góp — từ sửa lỗi, thêm bảng phím/bảng mã, cải thiện tài liệu, tới báo
lỗi và góp ý. Xem [CONTRIBUTING.md](CONTRIBUTING.md) cho quy trình làm việc cục bộ và các cổng chất
lượng (fmt/clippy/test/e2e) mà CI bắt buộc. Mọi PR đều được CI kiểm tự động trước khi merge.

Một vài hướng dễ bắt đầu: thêm/đối chiếu test gõ Telex/VNI/VIQR, mở rộng từ điển chính tả, viết tài
liệu, hoặc đóng gói cho thêm distro. Cứ mở issue/PR — chúng tôi sẵn lòng hỗ trợ!

## Người đóng góp

Cảm ơn những người tuyệt vời ✨ đã đóng góp cho PinaKey ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/trananhtung"><img src="https://avatars.githubusercontent.com/u/30992229?s=100" width="100px;" alt="Tung Tran"/><br /><sub><b>Tung Tran</b></sub></a><br /><a href="https://github.com/trananhtung/pinakey/commits?author=trananhtung" title="Code">💻</a> <a href="https://github.com/trananhtung/pinakey/commits?author=trananhtung" title="Documentation">📖</a> <a href="#maintenance-trananhtung" title="Maintenance">🚧</a> <a href="#infra-trananhtung" title="Infrastructure (Hosting, Build-Tools, etc)">🚇</a></td>
    </tr>
  </tbody>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

Dự án theo chuẩn [all-contributors](https://github.com/all-contributors/all-contributors) — **mọi
loại đóng góp** đều được ghi nhận, không chỉ code. Để thêm người đóng góp, comment trong issue/PR:

```
@all-contributors please add @username for code, doc
```

(cần cài [all-contributors bot](https://allcontributors.org/docs/en/bot/installation) cho repo;
hoặc dùng CLI: `npx all-contributors-cli add @username code,doc`).
