#!/usr/bin/env bash
#
# PinaKey — trình cài đặt một dòng cho Linux/IBus.
#
#   Cài đặt:
#     curl -fsSL https://raw.githubusercontent.com/trananhtung/pinakey/main/tools/install-online.sh | bash
#
#   Gỡ cài đặt:
#     curl -fsSL https://raw.githubusercontent.com/trananhtung/pinakey/main/tools/install-online.sh | bash -s -- --uninstall
#
# Script tự nhận diện CPU (x86_64 / aarch64), tải binary từ bản release mới nhất
# trên GitHub, đăng ký engine với IBus và thêm PinaKey vào nguồn nhập GNOME.
# Chỉ thao tác ghi vào hệ thống (component XML + binary) mới cần sudo.
#
# Biến môi trường tùy chọn:
#   PINAKEY_REPO=owner/repo     repo GitHub để tải (mặc định: trananhtung/pinakey)
#   PINAKEY_VERSION=v0.1.0      ghim một phiên bản cụ thể (mặc định: bản mới nhất)
#
set -euo pipefail

REPO="${PINAKEY_REPO:-trananhtung/pinakey}"
LIBDIR="/usr/local/lib/pinakey"
BIN="$LIBDIR/ibus-engine-pinakey"
ICON="$LIBDIR/vi.svg"
COMP="/usr/share/ibus/component/pinakey.xml"

# ----- tiện ích in ấn -----
say()  { printf '\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m !\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31mLỗi:\033[0m %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "Cần lệnh '$1' nhưng không có. Hãy cài rồi chạy lại."; }

# Chỉ dùng sudo khi chưa phải root.
SUDO=""
if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
    need sudo
    SUDO="sudo"
fi

detect_arch() {
    case "$(uname -m)" in
        x86_64 | amd64) echo "x86_64" ;;
        aarch64 | arm64) echo "aarch64" ;;
        *) die "Kiến trúc CPU '$(uname -m)' chưa được hỗ trợ (chỉ có x86_64 và aarch64)." ;;
    esac
}

# Tag release mới nhất từ GitHub API (repo công khai không cần token).
latest_tag() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
        | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
}

download() { # $1=url  $2=đích
    curl -fSL --retry 3 -o "$2" "$1" || die "Tải thất bại: $1"
}

# Tìm địa chỉ bus của IBus cho phiên hiện tại rồi nạp lại registry (best-effort).
refresh_ibus() {
    command -v ibus >/dev/null 2>&1 || return 0
    if [[ -z "${IBUS_ADDRESS:-}" && -r /etc/machine-id ]]; then
        local mid af
        mid="$(cat /etc/machine-id)"
        af="$(ls -t "$HOME"/.config/ibus/bus/"${mid}"-* 2>/dev/null | head -1 || true)"
        [[ -n "$af" ]] && export IBUS_ADDRESS="$(grep -E '^IBUS_ADDRESS=' "$af" | cut -d= -f2-)"
    fi
    ibus write-cache >/dev/null 2>&1 || true
    ibus restart >/dev/null 2>&1 || true
}

# Thêm ('ibus','PinaKey') vào danh sách nguồn nhập GNOME (idempotent, giữ nguyên cái cũ).
enable_input_source() {
    command -v gsettings >/dev/null 2>&1 || return 1
    local cur new
    cur="$(gsettings get org.gnome.desktop.input-sources sources 2>/dev/null)" || return 1
    [[ "$cur" == *"'PinaKey'"* ]] && return 0
    if [[ "$cur" == "@a(ss) []" || "$cur" == "[]" ]]; then
        new="[('ibus', 'PinaKey')]"
    else
        new="${cur%]}, ('ibus', 'PinaKey')]"
    fi
    gsettings set org.gnome.desktop.input-sources sources "$new"
}

# Sinh component XML (IBus chỉ quét /usr/share/ibus/component nên file này cần root).
render_component() { # $1=phiên bản (không có 'v')
    cat <<XML
<?xml version="1.0" encoding="utf-8"?>
<component>
    <name>org.freedesktop.IBus.pinakey</name>
    <description>PinaKey — Vietnamese input engine for IBus</description>
    <exec>$BIN --ibus</exec>
    <version>$1</version>
    <author>Tung Tran &lt;tunganhtran94@gmail.com&gt;</author>
    <license>GPLv3</license>
    <homepage>https://github.com/$REPO</homepage>
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
        </engine>
    </engines>
</component>
XML
}

uninstall() {
    say "Đang gỡ PinaKey..."
    # Bỏ khỏi nguồn nhập GNOME (theo từng người dùng, không cần sudo).
    if command -v gsettings >/dev/null 2>&1; then
        local cur new
        cur="$(gsettings get org.gnome.desktop.input-sources sources 2>/dev/null || echo '')"
        if [[ "$cur" == *"'PinaKey'"* ]]; then
            new="$(printf '%s' "$cur" | sed "s/, ('ibus', 'PinaKey')//; s/('ibus', 'PinaKey'), //; s/('ibus', 'PinaKey')//")"
            gsettings set org.gnome.desktop.input-sources sources "$new" || true
        fi
    fi
    $SUDO rm -f "$COMP"
    $SUDO rm -rf "$LIBDIR"
    refresh_ibus
    say "Đã gỡ PinaKey xong."
}

install_pinakey() {
    local arch ver tmp
    arch="$(detect_arch)"

    ver="${PINAKEY_VERSION:-}"
    [[ -n "$ver" ]] || ver="$(latest_tag || true)"
    [[ -n "$ver" ]] || die "Không tìm thấy bản release nào trong repo '$REPO'. Hãy tạo release trước (đẩy tag vX.Y.Z), hoặc đặt PINAKEY_VERSION."
    say "Cài PinaKey $ver cho $arch"

    local base="https://github.com/$REPO/releases/download/$ver"
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT

    say "Tải binary + icon..."
    download "$base/pinakey-$arch" "$tmp/ibus-engine-pinakey"
    download "$base/vi.svg" "$tmp/vi.svg" || warn "Không tải được icon, sẽ bỏ qua."
    chmod +x "$tmp/ibus-engine-pinakey"

    say "Cài binary + icon -> $LIBDIR"
    $SUDO install -d -m0755 "$LIBDIR"
    $SUDO install -m0755 "$tmp/ibus-engine-pinakey" "$BIN"
    [[ -f "$tmp/vi.svg" ]] && $SUDO install -m0644 "$tmp/vi.svg" "$ICON"

    say "Đăng ký component IBus -> $COMP"
    $SUDO install -d -m0755 "$(dirname "$COMP")"
    render_component "${ver#v}" > "$tmp/pinakey.xml"
    $SUDO install -m0644 "$tmp/pinakey.xml" "$COMP"

    say "Làm mới IBus"
    refresh_ibus

    say "Thêm 'PinaKey' vào nguồn nhập GNOME"
    enable_input_source \
        || warn "Chưa thêm tự động được. Mở Settings > Keyboard > Input Sources và thêm 'PinaKey — Bộ gõ tiếng Việt'."

    printf '\n'
    say "Hoàn tất!"
    cat <<EOF
  • Nhấn Super+Space để chuyển sang "PinaKey — Bộ gõ tiếng Việt".
  • Gõ thử Telex:  vieetj  ->  việt
  • Kiểm tra:      ibus list-engine | grep PinaKey
  • Gỡ cài đặt:    curl -fsSL https://raw.githubusercontent.com/$REPO/main/tools/install-online.sh | bash -s -- --uninstall
EOF
}

main() {
    need curl
    case "${1:-}" in
        -u | --uninstall) uninstall ;;
        "") install_pinakey ;;
        *) die "Tham số không hợp lệ: '$1' (chỉ hỗ trợ --uninstall)." ;;
    esac
}

main "$@"
