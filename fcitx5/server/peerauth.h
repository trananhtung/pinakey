/*
 * Xác thực client cho daemon uinput (issue #72). Không phụ thuộc fcitx5 — chỉ libc.
 * GPL-3.0-or-later.
 */
#ifndef _PINAKEY_SERVER_PEERAUTH_H_
#define _PINAKEY_SERVER_PEERAUTH_H_

#include <climits>
#include <cstdio>
#include <cstring>
#include <string>

#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

namespace pinakey {

/// Binary fcitx5 ở các prefix cài đặt chuẩn — so ĐƯỜNG DẪN THẬT của tiến trình
/// (readlink /proc/<pid>/exe, không tin argv[0]/cmdline vì giả được).
inline bool exeAllowed(const char *exe) {
    static constexpr const char *const kAllowedExes[] = {
        "/usr/bin/fcitx5",
        "/usr/local/bin/fcitx5",
    };
    for (const char *allowed : kAllowedExes) {
        if (std::strcmp(exe, allowed) == 0) {
            return true;
        }
    }
    return false;
}

/// Lớp xác thực thuần (tách để test): UID phải trùng user phục vụ VÀ exe nằm trong allowlist.
inline bool peerAuthorized(uid_t peerUid, const std::string &peerExe, uid_t expectedUid) {
    return peerUid == expectedUid && exeAllowed(peerExe.c_str());
}

/// Đọc SO_PEERCRED của socket đã kết nối. Trả false nếu getsockopt thất bại.
inline bool peerCredentials(int fd, struct ucred &cred) {
    socklen_t len = sizeof(cred);
    return getsockopt(fd, SOL_SOCKET, SO_PEERCRED, &cred, &len) == 0;
}

/// Xác thực đầy đủ qua fd: SO_PEERCRED + readlink /proc/<pid>/exe + allowlist.
inline bool peerAuthorized(int fd, uid_t expectedUid) {
    struct ucred cred {};
    if (!peerCredentials(fd, cred) || cred.uid != expectedUid) {
        return false;
    }
    char link[64];
    char exe[PATH_MAX] = {0};
    std::snprintf(link, sizeof(link), "/proc/%d/exe", cred.pid);
    ssize_t n = readlink(link, exe, sizeof(exe) - 1);
    if (n <= 0) {
        return false;
    }
    exe[n] = '\0';
    return peerAuthorized(cred.uid, exe, expectedUid);
}

} // namespace pinakey

#endif // _PINAKEY_SERVER_PEERAUTH_H_
