<!--
Cảm ơn bạn đã đóng góp! Vài dòng dưới đây giúp người review khỏi phải tự dựng lại bối cảnh.
Sửa nhỏ (typo, tài liệu) thì xoá bớt mục không liên quan — đừng để trống cho có.
Quy ước commit + cổng CI: xem CONTRIBUTING.md.
-->

## Vấn đề

<!-- Sửa cái gì và vì sao. Có issue thì ghi "Closes #NN". Là bug thì mô tả triệu chứng người
     dùng thấy (gõ gì → ra gì → mong đợi gì). -->

## Cách sửa

<!-- Hướng giải quyết, và những gì CỐ Ý không làm trong PR này. -->

## Bằng chứng

<!-- Dán lệnh ĐÃ CHẠY và kết quả THẬT của nó, đừng viết lại từ trí nhớ.
     Bug tái hiện được: cho thấy test mới fail trước khi sửa, pass sau khi sửa. -->

```
$ cargo test --workspace
$ ctest --test-dir fcitx5/build --output-on-failure
```

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `ctest --test-dir fcitx5/build --output-on-failure` (nếu có đụng `fcitx5/`)
- [ ] `bash tools/run-e2e.sh` (nếu có đụng đường gõ đầu-cuối)

## Môi trường đã thử tay

<!-- Bộ gõ phụ thuộc mạnh vào môi trường. Ghi rõ: distro, X11 hay Wayland, desktop, app đã gõ thử. -->

- Distro / phiên bản:
- Phiên: X11 / Wayland — desktop:
- App đã gõ thử:

## Rủi ro & đường lùi

<!-- Có thể hỏng ở đâu, phần nào chưa kiểm được, muốn gỡ ra thì làm gì. -->
