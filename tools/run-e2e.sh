#!/usr/bin/env bash
# Chạy E2E test cho PinaKey: dựng fcitx5 thật (headless, D-Bus riêng) với PinaKey làm IM mặc định,
# rồi bơm phím qua dbusfrontend và kiểm chuỗi ra (fcitx5/test/e2e/pinakey_e2e.py).
#
# CI (PinaKey đã cài hệ thống vào /usr): chỉ cần `bash tools/run-e2e.sh`.
# Local (PinaKey ở ~/.local): truyền addon/data dir:
#   PINAKEY_E2E_ADDON_DIRS="$HOME/.local/lib/fcitx5:/usr/lib/x86_64-linux-gnu/fcitx5" \
#   PINAKEY_E2E_DATA_HOME="$HOME/.local/share" bash tools/run-e2e.sh
set -euo pipefail
cd "$(dirname "$0")/.."

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/cfg/fcitx5"
cat > "$TMP/cfg/fcitx5/profile" <<'EOF'
[Groups/0]
Name=Default
Default Layout=us
DefaultIM=pinakey

[Groups/0/Items/0]
Name=keyboard-us
Layout=

[Groups/0/Items/1]
Name=pinakey
Layout=

[GroupOrder]
0=Default
EOF

export XDG_CONFIG_HOME="$TMP/cfg"
[ -n "${PINAKEY_E2E_ADDON_DIRS:-}" ] && export FCITX_ADDON_DIRS="$PINAKEY_E2E_ADDON_DIRS"
[ -n "${PINAKEY_E2E_DATA_HOME:-}" ] && export XDG_DATA_HOME="$PINAKEY_E2E_DATA_HOME"
unset DISPLAY WAYLAND_DISPLAY || true

exec dbus-run-session -- python3 fcitx5/test/e2e/pinakey_e2e.py
