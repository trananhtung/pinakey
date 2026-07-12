/*
 * Test resolve đường socket uinput (#72): filesystem trong $XDG_RUNTIME_DIR, không còn
 * abstract namespace. GPL-3.0-or-later.
 */
#include "../src/socketpath.h"

#include <fcitx-utils/log.h>

int main() {
    using fcitx::pinakey::uinputSocketPath;

    // Có XDG_RUNTIME_DIR → dùng thẳng.
    FCITX_ASSERT(uinputSocketPath("/run/user/1000", 1000) == "/run/user/1000/pinakey/uinput.sock");

    // Không có (nullptr/rỗng) → fallback /run/user/<uid> theo chuẩn systemd.
    FCITX_ASSERT(uinputSocketPath(nullptr, 1234) == "/run/user/1234/pinakey/uinput.sock");
    FCITX_ASSERT(uinputSocketPath("", 1234) == "/run/user/1234/pinakey/uinput.sock");

    // Wrapper đọc env thật phải trả về đường không rỗng và kết thúc bằng tên file chuẩn.
    const std::string p = uinputSocketPath();
    FCITX_ASSERT(p.size() > sizeof("/pinakey/uinput.sock"));
    FCITX_ASSERT(p.rfind("/pinakey/uinput.sock") == p.size() - sizeof("/pinakey/uinput.sock") + 1);
    return 0;
}
