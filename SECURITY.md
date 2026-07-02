# Chính sách bảo mật — PinaKey

## Báo lỗ hổng

Dùng **GitHub private vulnerability reporting**: trang
[Security → Report a vulnerability](https://github.com/trananhtung/pinakey/security/advisories/new)
của repo. Vui lòng **không** mở issue công khai cho lỗ hổng chưa vá. Chúng tôi phản hồi trong
vòng 7 ngày và phối hợp công bố sau khi có bản vá.

Phiên bản được hỗ trợ: bản release mới nhất.

## Mô hình an toàn

Bộ gõ là phần mềm đọc **mọi phím bạn gõ** — thiết kế của PinaKey tối thiểu hoá đặc quyền và
minh bạch về từng thành phần:

| Thành phần | Chạy với quyền | Ghi chú |
|---|---|---|
| Addon fcitx5 (`pinakey.so`) | tiến trình fcitx5 của user (không đặc quyền) | Toàn bộ xử lý phím trong tiến trình; **không mạng, không telemetry**. |
| Lõi Rust (`pinakey-ffi`) | như addon | Logic thuần; I/O duy nhất: đọc/ghi file cấu hình trong `~/.config/pinakey/` (ghi atomic). |
| GUI thiết lập | user | Ghi config + gọi D-Bus session `org.fcitx.Fcitx5` để áp config (#69). |
| Daemon uinput (TÙY CHỌN, mặc định KHÔNG cài/bật) | user tại seat (ACL `uaccess` trên `/dev/uinput`) | Xem chi tiết dưới. |

Không thành phần nào ghi lại nội dung gõ: engine chỉ giữ buffer từ đang soạn trong RAM;
log (stderr/journal) không chứa văn bản người dùng.

## Daemon uinput (thử nghiệm, opt-in)

Thành phần "đặc quyền" duy nhất, và chỉ tồn tại khi bạn tự build với
`-DPINAKEY_BUILD_UINPUT_SERVER=ON` **và** bật service + `PINAKEY_UINPUT=1`:

- **Khả năng duy nhất**: tạo một thiết bị bàn phím ảo **chỉ khai báo phím Backspace**
  (`UI_SET_KEYBIT KEY_BACKSPACE`) và phát tối đa 999 Backspace mỗi yêu cầu. Không đọc thiết bị
  nhập nào, không đổi cấu hình thiết bị của bạn.
- **Xác thực client 2 lớp**: `SO_PEERCRED` (UID phải trùng user phục vụ) **và**
  `readlink /proc/<pid>/exe` phải là binary fcitx5 ở prefix chuẩn (`/usr/bin`,
  `/usr/local/bin`) — không tin `argv[0]`/cmdline vì giả được.
- **Quyền hệ thống tối thiểu**: udev rule chỉ gắn tag `uaccess` cho đúng `/dev/uinput`
  (ACL theo seat đang hoạt động — user không ngồi máy không mở được); systemd unit chạy quyền
  user với `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome=read-only`, `PrivateTmp`.

### Hạn chế đã biết (theo dõi ở issue #72)

- Socket đang dùng **abstract namespace** (`\0pinakeysocket-<user>-kb`) — không có quyền
  filesystem nên tiến trình khác user có thể *thử* kết nối (bị từ chối bởi 2 lớp xác thực,
  nhưng về nguyên tắc nên chuyển sang socket filesystem `0600` trong `$XDG_RUNTIME_DIR`).
  Việc chuyển đổi thay đổi protocol client (addon) nên sẽ làm trong một PR riêng.
- `readlink /proc/<pid>/exe` có cửa sổ TOCTOU hẹp (pid tái sử dụng) — chấp nhận được trong
  threat model desktop (kẻ tấn công cùng UID vốn đã thao túng được phiên của bạn).

## Phạm vi KHÔNG thuộc threat model

- Tiến trình độc hại **cùng UID** với bạn: trên desktop Linux, nó vốn đã đọc được `~/.config`,
  `ptrace` được fcitx5, và ghi âm bàn phím qua nhiều đường khác — PinaKey không (và không thể)
  bảo vệ chống lại nó, chỉ bảo đảm không mở rộng thêm bề mặt (daemon uinput chỉ cho nó bơm…
  Backspace).
- Nội dung clipboard / bộ nhớ các ứng dụng khác.
