#!/usr/bin/env bash
# Remove the Rust ibus-bamboo ("BambooRs") installed by tools/install.sh. Leaves the Go engine
# untouched. Run so the sudo prompt works:   ! bash tools/uninstall.sh
set -euo pipefail

BIN_DIR="$HOME/.local/lib/ibus-bamboo-rs"
SYS_COMP="/usr/share/ibus/component/bamboo-rs.xml"

# Drop ('ibus','BambooRs') from GNOME input sources.
if command -v gsettings >/dev/null 2>&1; then
    cur="$(gsettings get org.gnome.desktop.input-sources sources 2>/dev/null || echo '')"
    if [[ "$cur" == *"'BambooRs'"* ]]; then
        new="$(printf '%s' "$cur" | sed "s/, ('ibus', 'BambooRs')//; s/('ibus', 'BambooRs'), //; s/('ibus', 'BambooRs')//")"
        gsettings set org.gnome.desktop.input-sources sources "$new" || true
        echo "Removed BambooRs from GNOME input sources."
    fi
fi

echo "Removing $SYS_COMP (needs sudo)"
sudo rm -f "$SYS_COMP"
echo "Removing $BIN_DIR"
rm -rf "$BIN_DIR"

if [[ -z "${IBUS_ADDRESS:-}" ]]; then
    MID="$(cat /etc/machine-id)"
    AF="$(ls -t "$HOME"/.config/ibus/bus/${MID}-* 2>/dev/null | head -1 || true)"
    [[ -n "$AF" ]] && export IBUS_ADDRESS="$(grep -E '^IBUS_ADDRESS=' "$AF" | cut -d= -f2-)"
fi
ibus write-cache >/dev/null 2>&1 || true
ibus restart      >/dev/null 2>&1 || true
echo "Done. The Go 'Bamboo' engine is unaffected."
