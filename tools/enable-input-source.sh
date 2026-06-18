#!/usr/bin/env bash
# Append ('ibus','BambooRs') to the GNOME input-source list (idempotent, additive).
# Existing sources are preserved. Switch between sources with Super+Space.
set -euo pipefail

KEY="org.gnome.desktop.input-sources sources"
cur="$(gsettings get org.gnome.desktop.input-sources sources)"

if [[ "$cur" == *"'BambooRs'"* ]]; then
    echo "BambooRs already in input sources: $cur"
    exit 0
fi

# Insert before the closing bracket of the list.
if [[ "$cur" == "@a(ss) []" || "$cur" == "[]" ]]; then
    new="[('ibus', 'BambooRs')]"
else
    new="${cur%]}, ('ibus', 'BambooRs')]"
fi

gsettings set org.gnome.desktop.input-sources sources "$new"
echo "Input sources now: $(gsettings get org.gnome.desktop.input-sources sources)"
echo "Press Super+Space to switch to 'Bamboo (Rust)' and start typing."
