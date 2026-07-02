# Hướng dẫn sử dụng PinaKey

**PinaKey** là bộ gõ tiếng Việt cho Linux (nền **fcitx5**). Trải nghiệm mặc định là **gõ không gạch
chân** — chữ tiếng Việt hiện thẳng như gõ thường, không có dòng gạch chân chờ xác nhận.

---

## 1. Cài đặt

### Cách dễ nhất — gói `.deb` dựng sẵn (khuyến nghị, Ubuntu/Debian)

Không cần build, không cần Rust. Một lệnh:

```sh
curl -fsSL https://raw.githubusercontent.com/trananhtung/pinakey/main/tools/install-deb.sh | bash
```

Hoặc tải thủ công gói `.deb` mới nhất tại **<https://github.com/trananhtung/pinakey/releases/latest>**
rồi cài:

```sh
sudo apt install ./fcitx5-pinakey_*.deb
```

> Gói tự kéo theo `fcitx5-configtool` + `im-config` (Recommends) và in hướng dẫn sau khi cài.

### Cách build từ nguồn (cho người tự build / nhà phát triển)

```sh
git clone https://github.com/trananhtung/pinakey.git
cd pinakey
bash tools/install-fcitx5.sh        # thêm --clean nếu cache CMake hỏng
```

Script tự kiểm dependency, build, cài vào `/usr`, và (hỏi rồi) tự thêm PinaKey vào fcitx5.
**Yêu cầu** (Debian/Ubuntu):

```sh
sudo apt install fcitx5 fcitx5-configtool libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev \
                 fcitx5-modules-dev extra-cmake-modules cmake g++ pkg-config
# + Rust (rustup) >= 1.85
```

### Sau khi cài — 3 bước để bắt đầu gõ

1. **Bật fcitx5 ở mức phiên (chỉ làm 1 lần)** — nếu fcitx5 chưa phải bộ gõ của hệ thống:
   ```sh
   im-config -n fcitx5
   ```
   rồi **ĐĂNG XUẤT và đăng nhập lại** (hoặc khởi động lại máy). *Bước này tránh lỗi
   “PinaKey (Not available)”.*
2. **Khởi động lại fcitx5** để nó nhận addon mới: `fcitx5 -r -d`.
3. **Thêm PinaKey**: mở `fcitx5-configtool` → bỏ tick **“Only Show Current Language”** (góc dưới)
   → tìm **PinaKey** → bấm **→**. (Nếu cài bằng `install-fcitx5.sh` và đã chọn tự thêm thì bỏ qua.)

Xong — nhấn **Ctrl+Space** để chuyển sang PinaKey và gõ thử: `vieetj` → **việt**.

---

## 2. Bật/tắt gõ tiếng Việt

- Nhấn **Ctrl + Space** để chuyển qua lại giữa **PinaKey (tiếng Việt)** và bàn phím thường (tiếng Anh).
- Biểu tượng trên khay hệ thống cho biết đang ở chế độ nào.

---

## 3. Gõ tiếng Việt

PinaKey hỗ trợ **Telex / VNI / VIQR**. Mặc định là **Telex**.

### Telex (mặc định)

| Gõ | Ra | | Gõ | Ra |
|----|----|----|----|----|
| `aa` | â | | `s` | dấu sắc (a → á) |
| `ee` | ê | | `f` | dấu huyền (a → à) |
| `oo` | ô | | `r` | dấu hỏi (a → ả) |
| `aw` | ă | | `x` | dấu ngã (a → ã) |
| `ow` | ơ | | `j` | dấu nặng (a → ạ) |
| `uw` / `w` | ư | | `z` | xoá dấu |
| `dd` | đ | | | |

**Ví dụ:** `Vieejt Nam` → **Việt Nam** · `tieengs Vieejt` → **tiếng Việt** · `dduwowngf` →
**đường** · `ddi hocj` → **đi học**.

### VNI

| Gõ | Ra | | Gõ | Ra |
|----|----|----|----|----|
| `6` | â/ê/ô (a6, e6, o6) | | `1` | dấu sắc |
| `7` | ư/ơ (u7, o7) | | `2` | dấu huyền |
| `8` | ă (a8) | | `3` | dấu hỏi |
| `9` | đ (d9) | | `4` | dấu ngã |
| `0` | xoá dấu | | `5` | dấu nặng |

**Ví dụ:** `a1` → **á** · `a6` → **â** · `a8` → **ă** · `o7` → **ơ** · `d9` → **đ** ·
`viet65` → **việt** (gõ chữ, rồi `6` cho ê, `5` cho dấu nặng — dấu có thể gõ ở cuối từ).

### VIQR

`'` sắc · `` ` `` huyền · `?` hỏi · `~` ngã · `.` nặng · `^` â/ê/ô · `+` ư/ơ · `(` ă · `dd` đ.

**Đổi kiểu gõ:** xem mục [6. Menu khay](#6-menu-khay-đổi-kiểu-gõ--bảng-mã).

### Tiện ích gõ tuỳ chọn (mặc định tắt)

Bật trong [Giao diện thiết lập](#10-giao-diện-thiết-lập-tuỳ-chọn):

- **Gõ w ra ư (Telex):** 3 mức — *Tắt* / *Không áp dụng ở đầu từ* (gõ `tw` → `tư` nhưng `www`,
  `word` giữ nguyên) / *Mọi nơi* (`w` → `ư`, gõ đúp `ww` trả lại `w`).
- **Tự viết hoa đầu câu:** sau `.` `!` `?` và khoảng trắng (hoặc Enter), chữ cái đầu tiên tự
  thành hoa.
- **Hai dấu cách liên tiếp → `. `:** kết câu nhanh kiểu bàn phím điện thoại — gõ từ + 2 dấu cách
  ra `từ. `. (Cần app hỗ trợ *Surrounding Text*; kết hợp được với tự viết hoa đầu câu.)

---

## 4. Gõ không gạch chân (mặc định)

Khác bộ gõ truyền thống (hiện chữ gạch chân rồi mới “chốt”), PinaKey **ghi thẳng** chữ tiếng Việt vào
ô văn bản và tự sửa tại chỗ khi bạn gõ thêm dấu. Bạn thấy chữ như gõ bình thường, không có gạch chân.

- Hoạt động tốt nhất với app hỗ trợ *Surrounding Text* (đa số app GTK/Qt: trình duyệt, soạn thảo…) — gõ không gạch chân, ổn định.
- App không hỗ trợ *Surrounding Text* (terminal, vài app Electron) → **tự lùi về chế độ preedit** (có dòng tạm/gạch chân nhưng gõ luôn đúng). Đây là hành vi mặc định, tin cậy.
- Một số app **có** Surrounding Text nhưng dùng không đáng tin (LibreOffice, terminal) → PinaKey
  **tự nhận diện theo tên app** và dùng preedit, không cần chỉnh gì.
- **Tự chỉnh per-app:** tạo `~/.config/pinakey/transport-rules.conf`, mỗi dòng
  `preedit|replace|auto <tên-app>` (ví dụ `preedit slack`) — rule của bạn **thắng** rule có sẵn.
  Danh sách có sẵn xem `/usr/share/pinakey/transport-rules.conf`.
- Có một chế độ **thử nghiệm** dùng daemon uinput để bỏ gạch chân ở cả terminal, nhưng **không ổn định trên GNOME Wayland** — xem [mục 9](#9-thử-nghiệm-gõ-không-gạch-chân-ở-terminal-uinput).

---

## 5. Emoji và ký tự Unicode

Khi không đang gõ dở một từ, gõ dấu **`:`** để mở bảng tra:

- **Emoji gần dùng:** vừa mở `:` (chưa gõ gì) → hiện tối đa **9 emoji dùng gần nhất**, chọn bằng
  phím **số 1–9** hoặc click. (Enter/Space lúc này vẫn ra dấu `:` bình thường — không sợ chọn nhầm
  emoji khi gõ `: ` trong câu.)
- **Emoji theo tên (tìm fuzzy):** `:grin`, `:heart_eyes` … — không cần gõ đủ hay gõ đúng liền
  mạch, `:heye` vẫn ra `heart_eyes`. Chọn bằng phím **số 1–9** hoặc **Enter**.
- **Ký tự Unicode theo mã hex:** `:u1f600` → 😀, `:u00e9` → é. (Gõ `:u` rồi mã hex, Enter.)
- **Esc** để huỷ, **Backspace** để xoá bớt.

Lịch sử gần dùng lưu ở `~/.config/pinakey/emoji-recent.txt` (xoá file này để xoá lịch sử).

---

## 6. Menu khay (đổi kiểu gõ / bảng mã)

Bấm vào biểu tượng PinaKey trên khay (hoặc menu trạng thái của fcitx5) → có 2 menu con:

- **Kiểu gõ:** Telex / VNI / VIQR / Telex (đơn giản) / …
- **Bảng mã:** Unicode (mặc định) / TCVN3 / …

---

## 7. Từ điển & gõ tắt (macro)

- **Từ điển riêng:** thêm các từ (mượn, tên riêng) vào `~/.config/pinakey/dict.txt` (mỗi dòng một từ)
  để PinaKey không “sửa nhầm” chúng về tiếng Anh.
- **Gõ tắt (macro):** tạo `~/.config/pinakey/ibus-PinaKey.macro.text`, mỗi dòng `khoá:nội dung`
  (ví dụ `vn:Việt Nam`). Gõ `vn` rồi phím chốt → bung thành “Việt Nam”.
- **Ngày/giờ động trong macro:** nội dung chứa `$DATE` / `$TIME` được thay bằng ngày/giờ **tại lúc
  gõ** — ví dụ `hnay:hôm nay $DATE` → “hôm nay 02/07/2026”. Format đổi được trong GUI thiết lập
  (chuẩn strftime; mặc định `%d/%m/%Y` và `%H:%M`). Muốn ra chữ `$TIME` thật, viết `$$TIME`.
- **Sửa nóng:** PinaKey **tự nạp lại** file dict/macro khi bạn sửa — không cần khởi động lại.

---

## 8. Loại trừ ứng dụng & ô mật khẩu

- **Ô mật khẩu:** PinaKey tự động không xử lý tiếng Việt trong ô mật khẩu.
- **Loại trừ app (gõ thẳng tiếng Anh):** thêm tên chương trình vào `EnglishExclude` trong
  `~/.config/pinakey/ibus-PinaKey.config.json` (ví dụ `"EnglishExclude": ["konsole", "code"]`).
- **Nhớ chế độ theo app:** dùng tuỳ chọn *Global Config → Share Input State = Program* của fcitx5 để
  mỗi app nhớ riêng đang bật tiếng Việt hay không.

---

## 9. (Thử nghiệm) Gõ không gạch chân ở terminal — uinput

> ⚠️ **Thử nghiệm, tắt mặc định.** Một số app (terminal, vài app Electron) không hỗ trợ
> *Surrounding Text*, nên mặc định PinaKey dùng **preedit** (có gạch chân nhưng gõ luôn đúng) ở đó.
> Chế độ uinput dưới đây cố bỏ gạch chân bằng cách bơm phím Backspace, **nhưng KHÔNG ổn định trên
> GNOME Wayland** (frontend D-Bus của GNOME không bảo đảm thứ tự xoá/commit → dễ rối/nhân đôi ký
> tự). Chỉ nên thử nếu bạn hiểu rủi ro; trên môi trường khác (KDE/X11) có thể khá hơn.

Cần **3 bước** (thiếu bước nào cũng không có tác dụng):

1. **Build kèm daemon** (mặc định không build):
   ```sh
   bash tools/install-fcitx5.sh --clean   # cấu hình lại; xem thêm bên dưới
   # hoặc thủ công:
   cmake -S fcitx5 -B fcitx5/build -DPINAKEY_BUILD_UINPUT_SERVER=ON && cmake --build fcitx5/build && sudo cmake --install fcitx5/build
   ```
2. **Bật daemon** (chạy dưới quyền bạn, cấp quyền /dev/uinput):
   ```sh
   sudo udevadm control --reload && sudo udevadm trigger
   systemctl --user enable --now pinakey-uinput-server
   ```
3. **Bật cờ ở addon** — thêm vào `~/.config/environment.d/fcitx5.conf`:
   ```sh
   PINAKEY_UINPUT=1
   ```
   rồi **đăng xuất / đăng nhập lại**.

Muốn tắt: bỏ `PINAKEY_UINPUT=1` (đăng nhập lại) hoặc `systemctl --user disable --now pinakey-uinput-server`.

---

## 10. Giao diện thiết lập (tuỳ chọn)

```sh
cargo build --release -p pinakey-settings --features gui
./target/release/pinakey-settings
```

Bấm **Lưu** là setting **áp dụng ngay** cho fcitx5 đang chạy (qua D-Bus; nếu không gọi được,
addon tự nhận file config đổi trong ~2 giây) — không cần khởi động lại. Sửa file config bằng
tay cũng được áp tự động như vậy.

---

## 11. Gỡ cài đặt

- Cài bằng `.deb`: `sudo apt remove fcitx5-pinakey`
- Cài từ nguồn: `sudo cmake --build fcitx5/build --target uninstall`

Rồi `fcitx5 -r -d` (hoặc đăng nhập lại).

---

## 12. Khắc phục sự cố

| Triệu chứng | Cách xử lý |
|---|---|
| **PinaKey hiện “(Not available)”** | Addon chưa nạp được. (1) Bật fcitx5 ở mức phiên: `im-config -n fcitx5` rồi **đăng xuất/đăng nhập lại**. (2) Cài vào `/usr` (gói `.deb` hoặc `cmake --install`) — **đừng** cài kiểu user-local `~/.local` + `FCITX_ADDON_DIRS` vì không bền vững. |
| Không thấy PinaKey trong configtool | Bỏ tick “Only Show Current Language”; chạy `fcitx5 -r -d` rồi mở lại. |
| Gõ ra tiếng Anh | Nhấn **Ctrl+Space** để chuyển sang PinaKey; kiểm tra biểu tượng khay. |
| Terminal hiện gạch chân | Bình thường: terminal không hỗ trợ Surrounding Text nên dùng preedit (gõ vẫn đúng). Muốn thử bỏ gạch chân: chế độ uinput thử nghiệm (mục 9) — lưu ý không ổn định trên GNOME Wayland. |
| Trình duyệt/editor hiện gạch chân | App đó chưa cấp Surrounding Text, hoặc cờ "Không gạch chân preedit" đang tắt → bật lại trong công cụ thiết lập PinaKey. |
| **Không gõ được tiếng Việt trong Slack / Discord / VS Code… (app cài bằng snap/flatpak)** | App chạy trong sandbox **thiếu GTK immodule `fcitx5`** → mọi bộ gõ đều chết, không riêng PinaKey. Cài bản **`.deb`/native** thay cho snap/flatpak (xem mục **“App snap/flatpak”** bên dưới). |
| Gõ tắt/từ điển không ăn | Kiểm tra đường dẫn file trong `~/.config/pinakey/`. |
| Không gõ được ở ô mật khẩu | **Cố ý**: PinaKey tự tắt ở ô mật khẩu (an toàn). Gõ trực tiếp không qua bộ gõ. |
| Thanh địa chỉ trình duyệt thi thoảng sai ký tự đầu (`đ` → `dđ`) | Lỗi đã biết với **autocomplete/autofill** của Chromium (issue #60): gợi ý được chọn sẵn làm lệch vùng sửa chữ. Tạm thời: gõ chậm ký tự đầu, hoặc tắt inline autocomplete của trình duyệt. |
| Chẩn đoán chung | `fcitx5-diagnose` để xem fcitx5 có nhận PinaKey không. |

### Thu log để báo lỗi

Khi [mở issue](https://github.com/trananhtung/pinakey/issues/new/choose), đính kèm:

1. **`fcitx5-diagnose`** — chạy trong terminal, dán toàn bộ output (form báo lỗi có ô riêng).
2. **Log chi tiết của addon** khi tái hiện được lỗi:
   ```sh
   fcitx5 -r -d --verbose 'pinakey=4'   # khởi động lại fcitx5 với log mức 4 cho pinakey
   journalctl --user -f | grep -i pinakey   # hoặc xem log qua journal
   ```
   Tái hiện lỗi rồi copy phần log quanh thời điểm đó. **Lưu ý:** log không chứa nội dung bạn
   gõ, nhưng vẫn nên rà lại trước khi dán.
3. Môi trường: distro, DE, X11/Wayland (`echo $XDG_SESSION_TYPE`), app xảy ra lỗi **kèm cách
   cài** (deb/snap/flatpak — hành xử rất khác nhau).

### App snap/flatpak không gõ được tiếng Việt (Slack, Discord, VS Code…)

Đây **không phải lỗi PinaKey**. App đóng gói bằng **snap** hoặc **flatpak** chạy trong môi trường
sandbox riêng, thường **không kèm GTK immodule `im-fcitx5.so`**, nên fcitx5 (PinaKey, Unikey…)
hay cả ibus đều **không nạp được** trong app đó — gõ ra chữ không dấu.

Kiểm tra app có phải bản snap/flatpak không:

```sh
snap list 2>/dev/null | grep -i slack      # hoặc tên app khác
flatpak list 2>/dev/null | grep -i slack
```

**Cách khắc phục: cài bản `.deb`/native** (dùng GTK hệ thống có sẵn `im-fcitx5.so`), ví dụ Slack:

```sh
# Gỡ bản snap rồi cài .deb chính thức từ slack.com
sudo snap remove slack
sudo apt install ./slack-desktop-*.deb
```

Bản `.deb` tự kế thừa `GTK_IM_MODULE=fcitx` của phiên (như Google Chrome) nên gõ tiếng Việt
bình thường. Mẹo kiểm chứng nhanh: nếu **Google Chrome (bản `.deb`)** gõ được tiếng Việt mà
app kia thì không, gần như chắc chắn app kia là bản snap/flatpak.

> Lưu ý: đặt `GTK_IM_MODULE=xim` cho app snap **không cứu được** các app nền Electron/Chromium
> (Slack, Discord, VS Code) — phải dùng bản `.deb`/native.

---

Báo lỗi / góp ý: <https://github.com/trananhtung/pinakey/issues>
