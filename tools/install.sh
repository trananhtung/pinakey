#!/usr/bin/env bash
# Install PinaKey as an IBus engine. The binary + icon live in your home; only the small
# component XML needs root (IBus only scans /usr/share/ibus/component on most setups).
# Reversible via tools/uninstall.sh.
#
# Run it so the sudo prompt works:   ! bash tools/install.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SRC="$REPO_ROOT/target/release/pinakey"
BIN_DIR="$HOME/.local/lib/pinakey"
BIN_DST="$BIN_DIR/ibus-engine-pinakey"
ICON="$BIN_DIR/vi.svg"
SYS_COMP="/usr/share/ibus/component/pinakey.xml"
# Giao diện thiết lập đồ họa (tùy chọn — chỉ cài nếu đã build).
SETTINGS_SRC="$REPO_ROOT/target/release/pinakey-settings"
SETTINGS_DST="$HOME/.local/bin/pinakey-settings"
DESKTOP="$HOME/.local/share/applications/pinakey-settings.desktop"

if [[ ! -x "$BIN_SRC" ]]; then
    echo "Release binary not found: $BIN_SRC"
    echo "Build it first:  cargo build --release -p pinakey"
    exit 1
fi

echo "1/4 Installing binary + icon -> $BIN_DIR"
mkdir -p "$BIN_DIR"
install -m 0755 "$BIN_SRC" "$BIN_DST"
install -m 0644 "$REPO_ROOT/tools/icons/vi.svg" "$ICON"

echo "2/4 Installing component (needs sudo) -> $SYS_COMP"
TMP_XML="$(mktemp -u)"   # -u: name only, so the heredoc creates it (avoids noclobber)
cat > "$TMP_XML" <<XML
<?xml version="1.0" encoding="utf-8"?>
<component>
    <name>org.freedesktop.IBus.pinakey</name>
    <description>PinaKey — Vietnamese input engine for IBus</description>
    <exec>$BIN_DST --ibus</exec>
    <version>0.1.0</version>
    <author>Tung Tran &lt;tunganhtran94@gmail.com&gt;</author>
    <license>GPLv3</license>
    <homepage>https://github.com/trananhtung/pinakey</homepage>
    <textdomain>pinakey</textdomain>
    <engines>
        <engine>
            <symbol>vi</symbol>
            <name>PinaKey</name>
            <language>vi</language>
            <license>GPLv3</license>
            <author>Tung Tran &lt;tunganhtran94@gmail.com&gt;</author>
            <icon>$ICON</icon>
            <layout>default</layout>
            <longname>PinaKey — Bộ gõ tiếng Việt</longname>
            <description>Tưởng niệm Francisco de Pina (1585–1625), người đặt nền móng chữ Quốc Ngữ</description>
            <rank>1</rank>
            <setup>$SETTINGS_DST</setup>
        </engine>
    </engines>
</component>
XML
sudo install -m 0644 "$TMP_XML" "$SYS_COMP"
rm -f "$TMP_XML"

echo "3/4 Refreshing IBus registry"
if [[ -z "${IBUS_ADDRESS:-}" ]]; then
    MID="$(cat /etc/machine-id)"
    AF="$(ls -t "$HOME"/.config/ibus/bus/${MID}-* 2>/dev/null | head -1 || true)"
    [[ -n "$AF" ]] && export IBUS_ADDRESS="$(grep -E '^IBUS_ADDRESS=' "$AF" | cut -d= -f2-)"
fi
ibus write-cache >/dev/null 2>&1 || true
ibus restart      >/dev/null 2>&1 || true

echo "4/5 Adding 'PinaKey' to GNOME input sources"
bash "$REPO_ROOT/tools/enable-input-source.sh" || true

echo "5/5 Installing settings GUI (optional)"
if [[ -x "$SETTINGS_SRC" ]]; then
    mkdir -p "$(dirname "$SETTINGS_DST")" "$(dirname "$DESKTOP")"
    install -m 0755 "$SETTINGS_SRC" "$SETTINGS_DST"
    cat > "$DESKTOP" <<DESK
[Desktop Entry]
Type=Application
Name=PinaKey — Thiết lập
Comment=Thiết lập bộ gõ tiếng Việt PinaKey
Exec=$SETTINGS_DST
Icon=$ICON
Terminal=false
Categories=Utility;
Keywords=vietnamese;input;tiếng việt;bộ gõ;
DESK
    echo "    -> $SETTINGS_DST (+ mục menu 'PinaKey — Thiết lập')"
else
    echo "    (bỏ qua: chưa build. Chạy 'cargo build --release -p pinakey-settings --features gui')"
fi

echo
echo "Done. Verify:  ibus list-engine | grep PinaKey"
echo "Switch with Super+Space to 'PinaKey — Bộ gõ tiếng Việt', then type 'vieetj' -> 'việt'."
echo "Settings GUI:  pinakey-settings   (hoặc mở 'PinaKey — Thiết lập' từ menu ứng dụng)"
echo "Uninstall:     bash tools/uninstall.sh"
