/*
 * Tiện ích UTF-8 dùng chung cho addon PinaKey. GPL-3.0-or-later.
 */
#ifndef _PINAKEY_FCITX5_UTF8UTIL_H_
#define _PINAKEY_FCITX5_UTF8UTIL_H_

#include <algorithm>
#include <cerrno>
#include <cstddef>
#include <cstdlib>
#include <string>

namespace fcitx::pinakey {

/// Phân tích chuỗi hex của ":u<hex>" thành codepoint Unicode hợp lệ (issue #11).
///
/// Trả `true` và gán `out` khi hex nằm trong `[1, 0x10FFFF]` và KHÔNG phải surrogate
/// (`U+D800`–`U+DFFF`, mã hoá UTF-8 không hợp lệ). Kiểm phạm vi trên giá trị `unsigned long`
/// **gốc** (chưa cắt) và bắt `errno == ERANGE` cho chuỗi hex tràn cả 64-bit — nên `:u100000041`
/// (`0x100000041`, ngoài phạm vi) bị loại thay vì cắt còn `0x41` rồi hiện "A" (#158).
inline bool parseHexCodepoint(const std::string &hex, char32_t &out) {
    if (hex.empty() ||
        hex.find_first_not_of("0123456789abcdefABCDEF") != std::string::npos) {
        return false;
    }
    errno = 0;
    char *end = nullptr;
    const unsigned long value = std::strtoul(hex.c_str(), &end, 16);
    if (errno == ERANGE || end == hex.c_str() || *end != '\0') {
        return false;
    }
    const bool surrogate = value >= 0xD800 && value <= 0xDFFF;
    if (value == 0 || value > 0x10FFFF || surrogate) {
        return false;
    }
    out = static_cast<char32_t>(value);
    return true;
}

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
    // Lead byte đa byte cụt ở cuối chuỗi (UTF-8 không hợp lệ do app gửi) cộng 2–4 byte có thể
    // vượt text.size(). Clamp để giữ đúng lời hứa "không bao giờ vượt cuối chuỗi" — nếu không,
    // caller (text.compare/text[pos-1]) ném std::out_of_range làm sập cả fcitx5. (#154)
    return std::min(bytePos, text.size());
}

} // namespace fcitx::pinakey

#endif // _PINAKEY_FCITX5_UTF8UTIL_H_
