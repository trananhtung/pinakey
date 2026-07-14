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

    // #154: lead byte đa byte CỤT ở cuối chuỗi (UTF-8 không hợp lệ) không được trả vị trí
    // vượt text.size() — nếu không, caller text.compare/text[pos-1] ném out_of_range/UB.
    // "a \xF0": 'a'(+1) ' '(+1) 0xF0 lead 4-byte (+4) = 6 thô, phải clamp về size()==3.
    FCITX_ASSERT(surroundingBytePosBeforeCursor(std::string("a \xF0"), 3) == 3);
    // "\xE1\xBB": lead 3-byte cụt (chỉ 2 byte) — clamp về size()==2.
    FCITX_ASSERT(surroundingBytePosBeforeCursor(std::string("\xE1\xBB"), 1) == 2);
    // Lead 4-byte đứng một mình — clamp về size()==1.
    FCITX_ASSERT(surroundingBytePosBeforeCursor(std::string("\xF0"), 1) == 1);
    // Cursor lớn hơn số ký tự trên chuỗi cụt vẫn clamp về size().
    FCITX_ASSERT(surroundingBytePosBeforeCursor(std::string("x\xF0"), 99) == 2);

    // parseHexCodepoint (#158): validate trên giá trị gốc, không cắt 64→32 bit.
    using fcitx::pinakey::parseHexCodepoint;
    char32_t cp = 0;
    // Hợp lệ: 'A' = U+0041, và codepoint cao nhất U+10FFFF.
    FCITX_ASSERT(parseHexCodepoint("41", cp) && cp == 0x41);
    FCITX_ASSERT(parseHexCodepoint("1F600", cp) && cp == 0x1F600); // 😀
    FCITX_ASSERT(parseHexCodepoint("10FFFF", cp) && cp == 0x10FFFF);
    // Chống tái diễn: 0x100000041 (> 0x10FFFF) phải bị LOẠI, không cắt còn 0x41.
    cp = 0xFFFF; // giá trị canh gác — parse thất bại không được đụng vào.
    FCITX_ASSERT(!parseHexCodepoint("100000041", cp));
    FCITX_ASSERT(cp == 0xFFFF);
    // Ngay trên trần Unicode.
    FCITX_ASSERT(!parseHexCodepoint("110000", cp));
    // Surrogate U+D800–DFFF bị loại (UTF-8 không hợp lệ).
    FCITX_ASSERT(!parseHexCodepoint("D800", cp));
    FCITX_ASSERT(!parseHexCodepoint("DFFF", cp));
    FCITX_ASSERT(parseHexCodepoint("D7FF", cp) && cp == 0xD7FF);
    FCITX_ASSERT(parseHexCodepoint("E000", cp) && cp == 0xE000);
    // 0 và rỗng bị loại; chuỗi hex tràn 64-bit (ERANGE) bị loại.
    FCITX_ASSERT(!parseHexCodepoint("0", cp));
    FCITX_ASSERT(!parseHexCodepoint("", cp));
    FCITX_ASSERT(!parseHexCodepoint("FFFFFFFFFFFFFFFFFF", cp));
    // Ký tự không phải hex bị loại.
    FCITX_ASSERT(!parseHexCodepoint("1G", cp));
    return 0;
}
