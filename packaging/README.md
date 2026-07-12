# Đóng gói PinaKey (issue #21)

Bộ gõ là **addon fcitx5** (build từ nguồn bằng CMake; lõi Rust biên dịch qua cargo).

## .deb / .rpm (CPack)

```sh
cmake -S fcitx5 -B fcitx5/build -DCMAKE_INSTALL_PREFIX=/usr -DPINAKEY_BUILD_TESTS=OFF
cmake --build fcitx5/build
( cd fcitx5/build && cpack -G DEB )    # -> fcitx5-pinakey_<ver>_<arch>.deb (DEB-DEFAULT)
( cd fcitx5/build && cpack -G RPM )    # cần rpmbuild
```

## Arch (AUR)

Dùng [`PKGBUILD`](PKGBUILD): `makepkg -si`.

## Nix

Dùng [`flake.nix`](flake.nix): `nix build ./packaging#default` (khung mẫu — lõi Rust cần mạng/cargo
vendor; có thể cần điều chỉnh `fetchCargoTarball` cho build thuần Nix offline).

## Thông báo chuyển Việt/Anh

fcitx5 hiển thị sẵn popup khi chuyển input method (Ctrl+Space). Menu khay trạng thái của PinaKey
(Kiểu gõ / Bảng mã) cũng do fcitx5 vẽ.
