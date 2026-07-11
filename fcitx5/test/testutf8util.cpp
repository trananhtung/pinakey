/*
 * Test hàm tiện ích UTF-8 dùng chung của addon. GPL-3.0-or-later.
 */
#include "../src/utf8util.h"

#include <fcitx-utils/log.h>

int main() {
    using fcitx::pinakey::surroundingBytePosBeforeCursor;
    FCITX_ASSERT(surroundingBytePosBeforeCursor("", 0) == 0);
    FCITX_ASSERT(surroundingBytePosBeforeCursor("abc", 0) == 0);
    FCITX_ASSERT(surroundingBytePosBeforeCursor("abc", 2) == 2);
    FCITX_ASSERT(surroundingBytePosBeforeCursor("abc", 3) == 3);
    // "việt": v(1) i(1) ệ(3) t(1) byte — con trỏ sau 3 ký tự = 5 byte.
    FCITX_ASSERT(surroundingBytePosBeforeCursor("việt", 3) == 5);
    // Con trỏ vượt cuối chuỗi → clamp về cuối.
    FCITX_ASSERT(surroundingBytePosBeforeCursor("việt", 10) == 6);
    // đ (2 byte) + 😀 (4 byte, ngoài BMP) — con trỏ sau 2 ký tự = 6 byte.
    FCITX_ASSERT(surroundingBytePosBeforeCursor("đ😀x", 2) == 6);
    return 0;
}
