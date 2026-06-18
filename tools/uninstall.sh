#!/usr/bin/env bash
# Remove PinaKey installed by tools/install.sh. Run so the sudo prompt works:
#   ! bash tools/uninstall.sh
set -euo pipefail

BIN_DIR="$HOME/.local/lib/pinakey"
SYS_COMP="/usr/share/ibus/component/pinakey.xml"

# Drop ('ibus','PinaKey') from GNOME input sources.
if command -v gsettings >/dev/null 2>&1; then
    cur="$(gsettings get org.gnome.desktop.input-sources sources 2>/dev/null || echo '')"
    if [[ "$cur" == *"'PinaKey'"* ]]; then
        new="$(printf '%s' "$cur" | sed "s/, ('ibus', 'PinaKey')//; s/('ibus', 'PinaKey'), //; s/('ibus', 'PinaKey')//")"
        gsettings set org.gnome.desktop.input-sources sources "$new" || true
        echo "Removed PinaKey from GNOME input sources."
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
echo "Done."
