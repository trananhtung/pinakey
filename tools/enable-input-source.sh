#!/usr/bin/env bash
# Append ('ibus','PinaKey') to the GNOME input-source list (idempotent, additive).
# Existing sources are preserved. Switch between sources with Super+Space.
set -euo pipefail

cur="$(gsettings get org.gnome.desktop.input-sources sources)"

if [[ "$cur" == *"'PinaKey'"* ]]; then
    echo "PinaKey already in input sources: $cur"
    exit 0
fi

if [[ "$cur" == "@a(ss) []" || "$cur" == "[]" ]]; then
    new="[('ibus', 'PinaKey')]"
else
    new="${cur%]}, ('ibus', 'PinaKey')]"
fi

gsettings set org.gnome.desktop.input-sources sources "$new"
echo "Input sources now: $(gsettings get org.gnome.desktop.input-sources sources)"
echo "Press Super+Space to switch to 'PinaKey' and start typing."
