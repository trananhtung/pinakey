/*
 * Đường socket uinput dùng chung giữa addon và daemon (issue #72). GPL-3.0-or-later.
 */
#ifndef _PINAKEY_FCITX5_SOCKETPATH_H_
#define _PINAKEY_FCITX5_SOCKETPATH_H_

#include <cstdlib>
#include <string>

#include <sys/types.h>
#include <unistd.h>

namespace fcitx::pinakey {

/// #105: byte hello daemon gửi ngay sau khi CHẤP NHẬN client (auth OK). Với AF_UNIX,
/// connect() thành công ngay khi listener còn chỗ backlog — chưa nói lên daemon có nhận
/// mình hay không; client chỉ được coi kết nối là dùng được sau khi nhận byte này.
inline constexpr char kUinputHello = 'P';

/// #72: socket FILESYSTEM `$XDG_RUNTIME_DIR/pinakey/uinput.sock` (thư mục 0700, socket 0600)
/// thay cho abstract namespace — abstract không có quyền filesystem nên tiến trình khác user
/// cũng *thử* kết nối được. Không có env thì fallback `/run/user/<uid>` theo chuẩn systemd.
inline std::string uinputSocketPath(const char *runtimeDir, uid_t uid) {
    std::string base = (runtimeDir && *runtimeDir)
                           ? std::string(runtimeDir)
                           : "/run/user/" + std::to_string(uid);
    return base + "/pinakey/uinput.sock";
}

inline std::string uinputSocketPath() {
    return uinputSocketPath(std::getenv("XDG_RUNTIME_DIR"), getuid());
}

} // namespace fcitx::pinakey

#endif // _PINAKEY_FCITX5_SOCKETPATH_H_
