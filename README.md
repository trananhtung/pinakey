# PinaKey

[![All Contributors](https://img.shields.io/github/all-contributors/trananhtung/pinakey?color=ee8449&style=flat-square)](#người-đóng-góp)

Trên phần lớn bộ gõ Linux, gõ `vieetj` sẽ hiện ra một đoạn `vieetj` gạch chân nằm chờ ở đó, đến khi
bạn gõ dấu cách nó mới bật thành `việt`.

PinaKey bỏ luôn đoạn gạch chân đó. Chữ hiện thẳng ra màn hình, giống hệt lúc bạn gõ trên Windows hay
macOS. Đây là một bộ gõ tiếng Việt cho Linux, chạy trên fcitx5.

Đổi lại, mỗi lần bạn thêm một dấu, PinaKey phải xoá chữ vừa hiện rồi viết lại chữ mới, ngay giữa lúc
bạn đang gõ tiếp. Việc đó phải vừa nhanh vừa không được sai một ký tự nào, nên lõi xử lý viết bằng
Rust thuần: khoảng 14 µs cho mỗi phím, không GC chen ngang, 165 test đơn vị. Phần nối vào fcitx5 chỉ
là một addon C++ mỏng.

Trang giới thiệu có sân chơi gõ thử Telex/VNI ngay trong trình duyệt:
[trananhtung.github.io/pinakey-web](https://trananhtung.github.io/pinakey-web/)
(mã nguồn: [`trananhtung/pinakey-web`](https://github.com/trananhtung/pinakey-web)).
Hướng dẫn đầy đủ cho người dùng nằm ở [USAGE.md](USAGE.md).

## Cài đặt

Ubuntu và Debian, cách nhanh nhất:

```sh
curl -fsSL https://raw.githubusercontent.com/trananhtung/pinakey/main/tools/install-deb.sh | bash
```

Hoặc tải gói `.deb` mới nhất ở [Releases](https://github.com/trananhtung/pinakey/releases/latest)
rồi `sudo apt install ./fcitx5-pinakey_*.deb`.

Cài xong còn ba bước nữa mới gõ được:

1. `im-config -n fcitx5`, rồi **đăng xuất và đăng nhập lại**. Chỉ cần làm một lần, và chỉ khi
   fcitx5 chưa phải bộ gõ của hệ thống. Bỏ qua bước này hay gặp lỗi "PinaKey (Not available)".
2. `fcitx5 -r -d` để fcitx5 nạp addon mới.
3. Mở `fcitx5-configtool`, bỏ tick "Only Show Current Language" ở góc dưới, tìm PinaKey rồi bấm mũi tên sang phải.

Xong. Nhấn Ctrl+Space để chuyển sang PinaKey và gõ thử `vieetj`, chữ ra phải là `việt`.

Cách gỡ cài đặt, cách xử lý khi app snap/flatpak không gõ được, và các lỗi hay gặp khác:
[USAGE.md](USAGE.md#1-cài-đặt).

## Gõ được những gì

Telex, VNI, VIQR, kèm vài biến thể dựng sẵn, trong đó có Telex đơn giản (gõ dấu chặt).

### Chỗ nào không gạch chân, chỗ nào vẫn còn

| Ứng dụng | Lúc gõ |
|----------|--------|
| Firefox, Chrome, VS Code, Telegram, phần lớn app GTK và Qt | Không gạch chân |
| Terminal: gnome-terminal, konsole, kitty, foot, ghostty… | Còn gạch chân (preedit) |
| LibreOffice | Còn gạch chân (preedit) |

PinaKey chỉ bỏ được gạch chân khi app cho phép đọc và xoá chữ đã hiện ra (Surrounding Text).
Terminal thì không cho thật: VTE nhận lệnh xoá rồi bỏ qua luôn, nên nếu cứ cố sẽ ra chữ rối như
`tieêngếng`. LibreOffice có cho, nhưng báo cáo sai vị trí khi gõ nhanh. Gặp hai nhóm này PinaKey tự
lùi về preedit, chữ vẫn ra đúng, chỉ là có gạch chân trong lúc soạn. Bạn không phải cấu hình gì cả;
danh sách nằm sẵn trong bộ gõ và sửa được ở `~/.config/pinakey/transport-rules.conf`.

Có thêm chế độ uinput để bỏ gạch chân ở cả terminal, nhưng còn thử nghiệm và tắt sẵn
(xem mục 9 của USAGE).

### Emoji

Gõ `:tên` là ra, tìm gần đúng trên khoảng 11 nghìn khóa nên `:heye` vẫn tìm được `heart_eyes`.
Vừa mở `:` đã thấy 9 emoji dùng gần nhất, chọn bằng phím số. Cần ký tự Unicode lạ thì gõ `:u<hex>`.

### Còn lại

- Menu ở khay trạng thái để đổi kiểu gõ và bảng mã.
- Từ điển chính tả "giải oan" cho từ mượn, cộng từ điển riêng của bạn ở `~/.config/pinakey/dict.txt`.
- Gõ tắt (macro), có cả `$DATE` và `$TIME` tự điền.
- Tự bỏ qua app tiếng Anh (terminal, IDE) và ô mật khẩu.
- Vài tiện ích nhỏ, mặc định tắt: w thành ư theo 3 mức, tự viết hoa đầu câu, hai dấu cách thành một dấu chấm rồi một dấu cách.
- Sửa file macro hay từ điển là có hiệu lực ngay, không phải khởi động lại.
- Giao diện thiết lập đồ họa viết bằng egui, bấm lưu là áp dụng luôn.

Con số 14 µs ở trên tự chạy lại được, cách đo nằm ở [docs/BENCHMARK.md](docs/BENCHMARK.md).

## Về cái tên

<img src="docs/assets/francisco-de-pina.jpg" alt="Francisco de Pina (trong tranh khắc cùng Alexandre de Rhodes)" align="right" width="200">

PinaKey tri ân Francisco de Pina (1585–1625), giáo sĩ Dòng Tên người Bồ Đào Nha. Ông là người đầu
tiên La-tinh hóa tiếng Việt một cách có hệ thống, ở Thanh Chiêm và Hội An, đặt nền móng cho chữ
Quốc Ngữ, thứ chữ mà mọi bàn phím tiếng Việt ngày nay đều gõ. Ông cũng là thầy dạy tiếng Việt của
Alexandre de Rhodes, và thường bị lãng quên sau cái bóng của học trò. Bộ gõ này là một lời tri ân nhỏ.
Hậu tố "Key" đánh dấu nó là một bộ gõ.

## Dành cho người phát triển

PinaKey tham khảo ý tưởng từ Bamboo (ibus-bamboo),
[fcitx5-lotus](https://github.com/LotusInputMethod/fcitx5-lotus) cho phần gõ không gạch chân, và
[fcitx5-cskk](https://github.com/fcitx/fcitx5-cskk) cho cách bọc một lõi không phải C++ qua C-ABI.

### Các crate trong workspace

| Crate | Trách nhiệm |
|-------|-------------|
| `pinakey-core` | Biến đổi Telex/VNI/VIQR, kiểm tra chính tả, từ điển, mã hóa charset. Logic thuần, không I/O. |
| `pinakey-config` | Cấu hình JSON, feature flag, đường dẫn cấu hình. |
| `pinakey-emoji` | Tra emoji (fuzzy + trie), lịch sử gần dùng, bảng macro. |
| `pinakey-engine` | Lõi engine trung lập transport: `process_key → (handled, Vec<Action>)`, không I/O. |
| `pinakey-ffi` | C-ABI (cbindgen) bọc `pinakey-engine` để addon C++ dùng lại lõi Rust. |
| `pinakey-settings` | Giao diện thiết lập đồ họa (egui, feature `gui`). |
| `fcitx5/` (C++) | Addon fcitx5 (`InputMethodEngineV2`) và daemon uinput bơm Backspace. |

### Build và test lõi Rust

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all --check                                  # cổng định dạng (CI)
cargo clippy --workspace --all-targets -- -D warnings    # cổng lint (CI)
```

[ARCHITECTURE.md](ARCHITECTURE.md) có đồ thị phụ thuộc và lý do đằng sau từng quyết định thiết kế.
[CONTRIBUTING.md](CONTRIBUTING.md) có quy trình phát triển.

### Build addon fcitx5 từ nguồn

Phụ thuộc trên Debian và Ubuntu:

```sh
sudo apt install fcitx5 fcitx5-configtool libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev \
                 fcitx5-modules-dev extra-cmake-modules cmake g++ pkg-config
# + Rust (rustup) >= 1.85
```

Một lệnh là xong (build, ctest, `sudo cmake --install`, khởi động lại fcitx5):

```sh
bash tools/install-fcitx5.sh
```

Hoặc làm từng bước:

```sh
cmake -S fcitx5 -B fcitx5/build -DCMAKE_INSTALL_PREFIX=/usr   # cargo tự build lõi Rust (staticlib)
cmake --build fcitx5/build
ctest --test-dir fcitx5/build --output-on-failure            # test tích hợp qua fcitx5 thật
sudo cmake --install fcitx5/build
fcitx5 -r -d
```

Sau đó làm ba bước ở phần [Cài đặt](#cài-đặt) bên trên để fcitx5 nhận PinaKey.

Muốn tự đóng gói cho deb, rpm, AUR hay Nix thì xem [packaging/](packaging/).

### Giao diện thiết lập

```sh
cargo build --release -p pinakey-settings --features gui
./target/release/pinakey-settings
```

### Chế độ uinput, để bỏ gạch chân ở terminal

Cảnh báo trước: chế độ này tắt mặc định và không ổn định trên GNOME Wayland, vì frontend D-Bus
không bảo đảm thứ tự xoá và commit nên chữ dễ bị rối. Terminal mặc định vẫn dùng preedit.
Chi tiết ở mục 9 của USAGE.

Cần đủ cả ba: build kèm `-DPINAKEY_BUILD_UINPUT_SERVER=ON` (mặc định OFF), bật daemon, rồi đặt biến
môi trường `PINAKEY_UINPUT=1` và đăng nhập lại.

```sh
cmake -S fcitx5 -B fcitx5/build -DPINAKEY_BUILD_UINPUT_SERVER=ON && cmake --build fcitx5/build && sudo cmake --install fcitx5/build
sudo udevadm control --reload && sudo udevadm trigger
systemctl --user enable --now pinakey-uinput-server
echo 'PINAKEY_UINPUT=1' >> ~/.config/environment.d/fcitx5.conf   # rồi đăng xuất và đăng nhập lại
```

### Các mảnh ghép lại với nhau thế nào

Toàn bộ logic xử lý phím nằm ở `pinakey-engine`. Nó không biết gì về transport, chỉ trả về một danh
sách `Action` (commit, cập nhật preedit, ẩn), nên unit-test được mà không cần dựng daemon. Keysym và
modifier dùng thẳng giá trị X11, trùng với fcitx5 nên không phải ánh xạ lại.

Addon trong `fcitx5/` gọi `pinakey-ffi` qua C-ABI rồi dịch từng `Action` thành lệnh fcitx5:
`commitString`, cập nhật preedit, hoặc `deleteSurroundingText`. Gõ không gạch chân chính là so tiền
tố chung giữa chuỗi đang hiển thị và chuỗi mới, ra được cặp (số ký tự cần xoá, chuỗi cần chèn).

`pinakey-core` dùng `Rc` nên đơn luồng. Mỗi input context giữ một engine riêng.

## Lịch sử

PinaKey khởi đầu là bộ gõ IBus thuần Rust. Từ EPIC #22 dự án chuyển hẳn sang fcitx5 để có gõ không
gạch chân mượt hơn và chạy vững trên Wayland, nhờ khả năng bơm Backspace mà IBus không cấp.
Frontend IBus cũ đã gỡ bỏ.

## Đóng góp

Mọi đóng góp đều được hoan nghênh: sửa lỗi, thêm bảng phím hoặc bảng mã, cải thiện tài liệu, báo lỗi,
góp ý. [CONTRIBUTING.md](CONTRIBUTING.md) mô tả quy trình làm việc cục bộ và các cổng chất lượng
(fmt, clippy, test, e2e) mà CI bắt buộc. Mọi PR đều qua CI trước khi merge.

Vài hướng dễ bắt đầu: thêm test gõ Telex/VNI/VIQR, mở rộng từ điển chính tả, viết tài liệu, hoặc
đóng gói cho distro khác. Cứ mở issue hoặc PR.

Lỗ hổng bảo mật thì báo theo kênh riêng trong [SECURITY.md](SECURITY.md), đừng mở issue công khai.
Mô hình an toàn của từng thành phần cũng nằm ở đó.

## Người đóng góp

Cảm ơn những người đã đóng góp cho PinaKey ([bảng emoji](https://allcontributors.org/docs/en/emoji-key)):

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

Dự án theo chuẩn [all-contributors](https://github.com/all-contributors/all-contributors): mọi loại
đóng góp đều được ghi nhận, không riêng code. Để thêm người đóng góp, comment trong issue hoặc PR:

```text
@all-contributors please add @username for code, doc
```

(cần cài [all-contributors bot](https://allcontributors.org/docs/en/bot/installation) cho repo, hoặc
dùng CLI: `npx all-contributors-cli add @username code,doc`).
