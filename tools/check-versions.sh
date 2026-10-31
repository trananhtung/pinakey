#!/usr/bin/env bash
# Bảo đảm version đồng bộ với NGUỒN DUY NHẤT là Cargo.toml (workspace.package.version).
# - fcitx5/CMakeLists.txt đọc thẳng từ Cargo.toml lúc cấu hình nên luôn khớp (không cần kiểm).
# - PKGBUILD (AUR) và flake.nix (Nix) là file độc lập, không đọc được Cargo.toml lúc build,
#   nên kiểm ở CI để bắt lệch sớm (đúng lỗi đã gặp: Cargo.toml kẹt 0.2.0 khi các nơi khác 0.2.1).
set -euo pipefail
cd "$(dirname "$0")/.."

cargo_ver=$(grep -m1 -oP '^\s*version\s*=\s*"\K[0-9]+\.[0-9]+\.[0-9]+' Cargo.toml)
pkgbuild_ver=$(grep -m1 -oP '^pkgver=\K[0-9]+\.[0-9]+\.[0-9]+' packaging/PKGBUILD)
flake_ver=$(grep -m1 -oP 'version\s*=\s*"\K[0-9]+\.[0-9]+\.[0-9]+' packaging/flake.nix)
# Input mã nguồn của flake phải ghim đúng tag phát hành (github:…/vX.Y.Z), không theo main.
flake_src_ver=$(grep -m1 -oP 'pinakey-src\.url\s*=\s*"github:[^/]+/[^/]+/v\K[0-9]+\.[0-9]+\.[0-9]+' \
    packaging/flake.nix || true)

fail=0
check() {
    if [ "$2" != "$cargo_ver" ]; then
        echo "✗ LỆCH version: $1 = $2  (Cargo.toml = $cargo_ver)"
        fail=1
    fi
}
check "packaging/PKGBUILD (pkgver)" "$pkgbuild_ver"
check "packaging/flake.nix (version)" "$flake_ver"
check "packaging/flake.nix (tag pinakey-src)" "${flake_src_ver:-<không ghim tag>}"

if [ "$fail" -ne 0 ]; then
    echo "Cập nhật cho khớp Cargo.toml ($cargo_ver) rồi chạy lại." >&2
    exit 1
fi
echo "✓ Version đồng bộ với Cargo.toml = $cargo_ver (PKGBUILD, flake.nix, tag nguồn của flake)"
