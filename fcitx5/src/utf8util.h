/*
 * Tiện ích UTF-8 dùng chung cho addon PinaKey. GPL-3.0-or-later.
 */
#ifndef _PINAKEY_FCITX5_UTF8UTIL_H_
#define _PINAKEY_FCITX5_UTF8UTIL_H_

#include <cstddef>
#include <string>

namespace fcitx::pinakey {

/// Đổi vị trí con trỏ `cursor` (đếm theo KÝ TỰ Unicode, như fcitx5 báo trong SurroundingText)
/// ra byte-offset trong chuỗi UTF-8 `text` — tức độ dài byte của phần văn bản trước con trỏ.
/// Byte đầu mỗi ký tự quyết định độ dài; byte không hợp lệ tính 1 byte để không kẹt vòng lặp
/// (an toàn với dữ liệu bất kỳ từ app). Con trỏ vượt cuối chuỗi → clamp về cuối.
inline size_t surroundingBytePosBeforeCursor(const std::string &text, unsigned int cursor) {
    size_t bytePos = 0;
    for (unsigned int chars = 0; bytePos < text.size() && chars < cursor; ++chars) {
        const unsigned char c = static_cast<unsigned char>(text[bytePos]);
        bytePos += (c < 0x80)            ? 1
                   : ((c >> 5) == 0x6)   ? 2
                   : ((c >> 4) == 0xe)   ? 3
                   : ((c >> 3) == 0x1e)  ? 4
                                         : 1;
    }
    return bytePos;
}

} // namespace fcitx::pinakey

#endif // _PINAKEY_FCITX5_UTF8UTIL_H_
