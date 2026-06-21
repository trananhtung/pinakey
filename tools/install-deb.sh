#!/usr/bin/env bash
# Cách cài DỄ NHẤT cho người dùng cuối: tải gói .deb mới nhất từ GitHub Releases rồi cài.
# Không cần build, không cần Rust/CMake. Chỉ cần Ubuntu/Debian + quyền sudo.
#
#   bash tools/install-deb.sh
# hoặc một dòng:
#   curl -fsSL https://raw.githubusercontent.com/trananhtung/pinakey/main/tools/install-deb.sh | bash
set -euo pipefail
REPO="trananhtung/pinakey"

command -v curl >/dev/null 2>&1 || { echo "✗ Cần 'curl'. Cài: sudo apt install curl"; exit 1; }
command -v apt  >/dev/null 2>&1 || { echo "✗ Script này dành cho Debian/Ubuntu (cần apt)."; exit 1; }

echo "==> Tìm gói .deb mới nhất từ GitHub Releases ($REPO)…"
url="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep -oE '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]+\.deb"' \
  | head -1 | sed -E 's/.*"(https[^"]+)".*/\1/')"
if [ -z "${url:-}" ]; then
  echo "✗ Chưa có gói .deb dựng sẵn trong release mới nhất."
  echo "  Hãy build từ nguồn: bash tools/install-fcitx5.sh"
  exit 1
fi

tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
deb="$tmp/fcitx5-pinakey.deb"
echo "==> Tải: $url"
curl -fSL "$url" -o "$deb"

echo "==> Cài (cần quyền sudo)…"
sudo apt install -y "$deb"

echo ""
echo "✓ Đã cài PinaKey. Tiếp theo:"
echo "  1) fcitx5 -r -d   (hoặc đăng xuất/đăng nhập lại)"
echo "  2) fcitx5-configtool → thêm 'PinaKey'  (bỏ tick 'Only Show Current Language' nếu không thấy)"
echo "  3) Ctrl+Space để chuyển, gõ thử: vieetj → việt"
