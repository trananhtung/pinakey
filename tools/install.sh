#!/usr/bin/env bash
# Install the Rust ibus-bamboo as the IBus engine "BambooRs", side-by-side with any existing
# Go ibus-bamboo. The binary lives in your home; only the small component XML needs root
# (IBus only scans /usr/share/ibus/component on this system). Reversible via tools/uninstall.sh.
#
# Run it so the sudo prompt works:   ! bash tools/install.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SRC="$REPO_ROOT/target/release/ibus-bamboo"
BIN_DIR="$HOME/.local/lib/ibus-bamboo-rs"
BIN_DST="$BIN_DIR/ibus-engine-bamboo-rs"
SYS_COMP="/usr/share/ibus/component/bamboo-rs.xml"
ICON="$BIN_DIR/vi.svg"   # self-contained icon shipped in tools/icons/

if [[ ! -x "$BIN_SRC" ]]; then
    echo "Release binary not found: $BIN_SRC"
    echo "Build it first:  cargo build --release -p ibus-bamboo"
    exit 1
fi

echo "1/4 Installing binary + icon -> $BIN_DIR"
mkdir -p "$BIN_DIR"
install -m 0755 "$BIN_SRC" "$BIN_DST"
install -m 0644 "$REPO_ROOT/tools/icons/vi.svg" "$ICON"

echo "2/4 Installing component (needs sudo) -> $SYS_COMP"
TMP_XML="$(mktemp)"
cat > "$TMP_XML" <<XML
<?xml version="1.0" encoding="utf-8"?>
<component>
    <name>org.freedesktop.IBus.bamboors</name>
    <description>Vietnamese input engine for IBus (Rust port)</description>
    <exec>$BIN_DST --ibus</exec>
    <version>0.1.0</version>
    <author>Tung Tran &lt;tunganhtran94@gmail.com&gt;</author>
    <license>GPLv3</license>
    <homepage>https://github.com/BambooEngine/ibus-bamboo/</homepage>
    <textdomain>ibus-bamboo-rs</textdomain>
    <engines>
        <engine>
            <symbol>vi</symbol>
            <name>BambooRs</name>
            <language>vi</language>
            <license>GPLv3</license>
            <author>Tung Tran &lt;tunganhtran94@gmail.com&gt;</author>
            <icon>$ICON</icon>
            <layout>default</layout>
            <longname>Bamboo (Rust)</longname>
            <description>Vietnamese input method editor — pure-Rust port</description>
            <rank>1</rank>
        </engine>
    </engines>
</component>
XML
sudo install -m 0644 "$TMP_XML" "$SYS_COMP"
rm -f "$TMP_XML"
# Drop the unused user-dir copy from earlier attempts (this IBus build doesn't scan it).
rm -f "$HOME/.local/share/ibus/component/bamboo-rs.xml" 2>/dev/null || true

echo "3/4 Refreshing IBus registry"
if [[ -z "${IBUS_ADDRESS:-}" ]]; then
    MID="$(cat /etc/machine-id)"
    AF="$(ls -t "$HOME"/.config/ibus/bus/${MID}-* 2>/dev/null | head -1 || true)"
    [[ -n "$AF" ]] && export IBUS_ADDRESS="$(grep -E '^IBUS_ADDRESS=' "$AF" | cut -d= -f2-)"
fi
ibus write-cache >/dev/null 2>&1 || true
ibus restart      >/dev/null 2>&1 || true

echo "4/4 Adding 'BambooRs' to GNOME input sources"
bash "$REPO_ROOT/tools/enable-input-source.sh" || true

echo
echo "Done. Verify:  ibus list-engine | grep BambooRs"
echo "Switch with Super+Space to 'Bamboo (Rust)', then type 'vieetj' -> 'việt'."
echo "Uninstall:     bash tools/uninstall.sh"
