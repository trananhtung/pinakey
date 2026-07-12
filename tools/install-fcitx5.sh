#!/usr/bin/env bash
# Build + cài addon fcitx5 của PinaKey TỪ NGUỒN (cho người tự build / nhà phát triển).
# Người dùng cuối nên cài gói .deb dựng sẵn: `bash tools/install-deb.sh` (xem USAGE.md).
#
#   bash tools/install-fcitx5.sh [--clean] [--uinput] [--add-im | --no-add-im]
#     --clean       xoá fcitx5/build trước khi cấu hình lại (khi cache CMake hỏng)
#     --uinput      build kèm daemon uinput (-DPINAKEY_BUILD_UINPUT_SERVER=ON, xem USAGE.md mục 9)
#     --add-im      tự thêm PinaKey vào cấu hình fcitx5 (không hỏi)
#     --no-add-im   không tự thêm (tự mở configtool)
set -euo pipefail
cd "$(dirname "$0")/.."

CLEAN=0
UINPUT=0
ADD_IM=ask
for arg in "$@"; do
  case "$arg" in
    --clean) CLEAN=1 ;;
    --uinput) UINPUT=1 ;;
    --add-im) ADD_IM=yes ;;
    --no-add-im) ADD_IM=no ;;
    -h|--help) sed -e '1d' -e '/^[^#]/,$d' "$0"; exit 0 ;;
    *) echo "Tham số lạ: $arg (xem --help)"; exit 1 ;;
  esac
done

# ---------------------------------------------------------------------------
# 1. Tiền kiểm công cụ / thư viện — báo rõ thiếu gì thay vì lỗi build khó hiểu.
# ---------------------------------------------------------------------------
miss=()
for c in cmake g++ cargo rustc pkg-config; do
  command -v "$c" >/dev/null 2>&1 || miss+=("$c")
done
if command -v pkg-config >/dev/null 2>&1 && ! pkg-config --exists Fcitx5Core 2>/dev/null; then
  miss+=("libfcitx5core-dev")
fi
if [ ${#miss[@]} -ne 0 ]; then
  echo "✗ Thiếu: ${miss[*]}"
  echo "  Cài (Debian/Ubuntu):"
  echo "    sudo apt install fcitx5 fcitx5-configtool libfcitx5core-dev libfcitx5utils-dev \\"
  echo "                     libfcitx5config-dev fcitx5-modules-dev extra-cmake-modules cmake g++ pkg-config"
  echo "  Rust: cài qua https://rustup.rs rồi 'rustup update' (cần >= 1.85)."
  exit 1
fi
# Rust >= 1.85
rv="$(rustc --version | awk '{print $2}')"
if [ "$(printf '%s\n1.85.0\n' "$rv" | sort -V | head -1)" != "1.85.0" ]; then
  echo "✗ Cần Rust >= 1.85 (đang có $rv). Chạy: rustup update"
  exit 1
fi
echo "✓ Dependency OK (rustc $rv)"

# ---------------------------------------------------------------------------
# 2. Xin quyền sudo sớm — fail nhanh & rõ nếu không có TTY/quyền (tránh dừng giữa chừng).
# ---------------------------------------------------------------------------
if [ "$(id -u)" -ne 0 ]; then
  echo "==> Bước cài cần quyền quản trị — sẽ hỏi mật khẩu sudo."
  if ! sudo -v; then
    echo "✗ Không lấy được quyền sudo. Hãy chạy trong cửa sổ Terminal thật."
    exit 1
  fi
fi

# ---------------------------------------------------------------------------
# 3. Build (cargo tự build lõi Rust làm staticlib).
# ---------------------------------------------------------------------------
[ "$CLEAN" = 1 ] && { echo "==> Xoá fcitx5/build (--clean)…"; rm -rf fcitx5/build; }
echo "==> Cấu hình + build…"
CMAKE_ARGS=(-DCMAKE_INSTALL_PREFIX=/usr -DCMAKE_BUILD_TYPE=Release)
# Chỉ ép ON khi có --uinput; không có cờ thì giữ nguyên giá trị trong cache CMake
# (tránh âm thầm tắt daemon của người đã tự cấu hình ON trước đó).
if [ "$UINPUT" = 1 ]; then
  CMAKE_ARGS+=(-DPINAKEY_BUILD_UINPUT_SERVER=ON)
fi
cmake -S fcitx5 -B fcitx5/build "${CMAKE_ARGS[@]}"
cmake --build fcitx5/build

# ---------------------------------------------------------------------------
# 4. Test tích hợp — KHÔNG chặn cài (máy thiếu testfrontend thì bỏ qua).
# ---------------------------------------------------------------------------
echo "==> Test tích hợp (bỏ qua nếu thiếu testfrontend)…"
ctest --test-dir fcitx5/build --output-on-failure || echo "  (bỏ qua: không có/không pass test — vẫn tiếp tục cài)"

# ---------------------------------------------------------------------------
# 5. Cài vào /usr (bền vững, không cần biến môi trường).
# ---------------------------------------------------------------------------
echo "==> Cài vào /usr…"
sudo cmake --install fcitx5/build
command -v gtk-update-icon-cache >/dev/null 2>&1 && \
  sudo gtk-update-icon-cache -q -f /usr/share/icons/hicolor 2>/dev/null || true

# ---------------------------------------------------------------------------
# 6. Tự thêm PinaKey vào cấu hình fcitx5 (tuỳ chọn, có backup).
# ---------------------------------------------------------------------------
PROFILE="${XDG_CONFIG_HOME:-$HOME/.config}/fcitx5/profile"
add_im() {
  if [ ! -f "$PROFILE" ]; then
    echo "  (Chưa có $PROFILE — mở fcitx5-configtool để thêm PinaKey.)"
    return 1
  fi
  if grep -q '^Name=pinakey$' "$PROFILE"; then
    echo "  ✓ PinaKey đã có trong cấu hình."
    return 0
  fi
  cp -f "$PROFILE" "$PROFILE.bak.pinakey"
  python3 - "$PROFILE" <<'PY' || return 1
import sys, re
p = sys.argv[1]
s = open(p, encoding="utf-8").read()
idx = [int(m.group(1)) for m in re.finditer(r'^\[Groups/0/Items/(\d+)\]', s, re.M)]
if not idx:
    sys.exit(1)  # cấu trúc lạ — để người dùng tự thêm
n = max(idx) + 1
block = f"\n[Groups/0/Items/{n}]\nName=pinakey\nLayout=\n"
# chèn sau khối item cuối của group 0
m = list(re.finditer(r'^\[Groups/0/Items/\d+\][^\[]*', s, re.M))[-1]
s = s[:m.end()] + block + s[m.end():]
open(p, "w", encoding="utf-8").write(s)
PY
  if grep -q '^Name=pinakey$' "$PROFILE"; then
    echo "  ✓ Đã thêm PinaKey vào cấu hình (backup: $PROFILE.bak.pinakey)."
    return 0
  fi
  echo "  ✗ Không thêm được tự động — khôi phục backup; hãy dùng fcitx5-configtool."
  mv -f "$PROFILE.bak.pinakey" "$PROFILE"
  return 1
}

do_add=no
case "$ADD_IM" in
  yes) do_add=yes ;;
  no)  do_add=no ;;
  ask)
    if [ -t 0 ]; then
      read -r -p "Tự thêm PinaKey vào fcitx5 luôn? [Y/n] " ans
      case "${ans:-Y}" in [Nn]*) do_add=no ;; *) do_add=yes ;; esac
    fi ;;
esac
[ "$do_add" = yes ] && add_im || true

# ---------------------------------------------------------------------------
# 7. Khởi động lại fcitx5 + xác minh addon nạp được.
# ---------------------------------------------------------------------------
echo "==> Khởi động lại fcitx5…"
fcitx5 -r -d >/dev/null 2>&1 || true
sleep 1
if command -v fcitx5-diagnose >/dev/null 2>&1 && fcitx5-diagnose 2>/dev/null | grep -qi pinakey; then
  echo "✓ fcitx5 đã nhận addon PinaKey."
fi

cat <<'EOF'

================== HOÀN TẤT ==================
Nếu chưa tự thêm IM, mở:  fcitx5-configtool  → thêm "PinaKey"
(bỏ tick "Only Show Current Language" nếu không thấy).
Nhấn Ctrl+Space để chuyển sang PinaKey, gõ thử:  vieetj → việt

Lỗi "(Not available)" sau khi đăng nhập lại? fcitx5 chưa bật ở mức phiên:
   im-config -n fcitx5   rồi ĐĂNG XUẤT / đăng nhập lại.

Gõ không gạch chân ở trình duyệt/editor (Surrounding Text): tự bật, ổn định.
Terminal mặc định dùng preedit (gõ vẫn đúng). Chế độ uinput bỏ gạch chân ở
terminal là THỬ NGHIỆM, tắt mặc định, không ổn định trên GNOME Wayland — xem
USAGE.md mục 9 nếu muốn thử.
=============================================
EOF
