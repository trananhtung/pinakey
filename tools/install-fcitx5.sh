#!/usr/bin/env bash
# Build, test rồi cài addon fcitx5 của PinaKey.
# Bước cài cần quyền root (chép pinakey.so + .conf vào /usr) nên sẽ gọi sudo.
#
#   bash tools/install-fcitx5.sh
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> Cấu hình + build (cargo tự build lõi Rust)…"
cmake -S fcitx5 -B fcitx5/build -DCMAKE_INSTALL_PREFIX=/usr
cmake --build fcitx5/build

echo "==> Chạy test tích hợp…"
ctest --test-dir fcitx5/build --output-on-failure

echo "==> Cài đặt (cần sudo)…"
sudo cmake --install fcitx5/build

echo "==> Khởi động lại fcitx5…"
fcitx5 -r -d >/dev/null 2>&1 || true

cat <<'EOF'

Xong! Tiếp theo:
  1. Mở fcitx5-configtool (hoặc: fcitx5-configtool &)
  2. Thêm input method "PinaKey" (ngôn ngữ: Tiếng Việt; bỏ tick "Only Current Language" nếu không thấy)
  3. Nhấn Ctrl+Space để chuyển sang PinaKey, rồi gõ thử: vieetj -> việt
EOF
