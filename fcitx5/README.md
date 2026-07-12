# Addon fcitx5 cho PinaKey

Frontend **fcitx5** của PinaKey: một addon C++ mỏng bọc lõi engine Rust qua C-ABI
(`crates/pinakey-ffi`), theo mô hình [fcitx5-cskk](https://github.com/fcitx/fcitx5-cskk). Toàn bộ
logic tiếng Việt nằm ở Rust; thư mục này chỉ là lớp tích hợp với fcitx5.

## Thành phần

| File | Vai trò |
|------|---------|
| `src/pinakey.{h,cpp}` | `PinaKeyEngine : InputMethodEngineV2` + `PinaKeyState : InputContextProperty`. |
| `src/pinakey.conf.in` | Đăng ký input method (hiện trong fcitx5-configtool). |
| `src/pinakey-addon.conf.in` | Metadata addon (Library, Category…). |
| `test/` | Test tích hợp qua testfrontend của fcitx5 + ô văn bản giả lập (gõ không gạch chân). |
| `server/` | Daemon uinput bơm Backspace (opt-in, `-DPINAKEY_BUILD_UINPUT_SERVER=ON`) + systemd service + udev rule. |
| `CMakeLists.txt` | Build lõi Rust (staticlib) bằng cargo rồi link addon. |

## Build, test, cài

```sh
# Phụ thuộc (Debian/Ubuntu):
sudo apt install fcitx5 libfcitx5core-dev libfcitx5utils-dev fcitx5-modules-dev \
                 extra-cmake-modules cmake g++

cmake -S fcitx5 -B fcitx5/build -DCMAKE_INSTALL_PREFIX=/usr
cmake --build fcitx5/build
ctest --test-dir fcitx5/build --output-on-failure   # chạy toàn bộ test tích hợp + unit
sudo cmake --install fcitx5/build
fcitx5 -r -d                                         # khởi động lại fcitx5
```

Tiện hơn: `bash tools/install-fcitx5.sh` (build + test, rồi `sudo cmake --install`).

Sau khi cài: mở **fcitx5-configtool** → thêm input method **PinaKey** → chuyển sang nó (Ctrl+Space)
→ gõ Telex (`vieetj` → `việt`).

## Gõ không gạch chân

Mặc định bật (cờ `IB_NO_UNDERLINE`). Với ứng dụng hỗ trợ *Surrounding Text*, addon commit thẳng và
sửa tại chỗ bằng `deleteSurroundingText` (không preedit). App không hỗ trợ → tự lùi về preedit.
Với app không hỗ trợ Surrounding Text vẫn có thể gõ không gạch chân qua **daemon uinput** (`server/`,
opt-in): build với `-DPINAKEY_BUILD_UINPUT_SERVER=ON` và bật `PINAKEY_UINPUT=1` — xem
[USAGE.md mục 9](../USAGE.md) và [SECURITY.md](../SECURITY.md).

Ngoại lệ (#66): LibreOffice/OpenOffice (`soffice`) quảng cáo Surrounding Text nhưng báo cáo không
đáng tin khi gõ nhanh (lạc hậu, thiếu dấu cách) → addon tự dùng preedit cho các app này.

## Tạo lại C-ABI header

Header `crates/pinakey-ffi/include/pinakey_ffi.h` được sinh bằng cbindgen và commit kèm repo
(để build C++ không cần cbindgen). Khi đổi C-ABI: `tools/gen-ffi-header.sh`.
