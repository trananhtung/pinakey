/*
 * Bind socket lắng nghe cho daemon uinput (issue #72). Không phụ thuộc fcitx5 — chỉ libc.
 * GPL-3.0-or-later.
 */
#ifndef _PINAKEY_SERVER_SERVERSOCKET_H_
#define _PINAKEY_SERVER_SERVERSOCKET_H_

#include <cerrno>
#include <cstring>
#include <string>

#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

namespace pinakey {

/// Socket tại `path` còn daemon nào SỐNG đang lắng nghe không — probe bằng connect().
/// ENOENT/ECONNREFUSED = chết/không tồn tại; connect được = còn sống.
inline bool socketAlive(const struct sockaddr_un &addr) {
    int probe = socket(AF_UNIX, SOCK_SEQPACKET | SOCK_CLOEXEC, 0);
    if (probe < 0) {
        return false;
    }
    const bool alive =
        connect(probe, reinterpret_cast<const struct sockaddr *>(&addr), sizeof(addr)) == 0;
    close(probe);
    return alive;
}

/// Tạo socket SEQPACKET lắng nghe tại `sockPath` với thuộc tính bảo mật của #72:
/// thư mục cha 0700 (tạo nếu chưa có, siết lại nếu đã có), socket 0600 ngay từ lúc sinh
/// (qua umask — không có cửa sổ chmod), dọn socket cũ CHỈ khi daemon trước đã chết —
/// daemon khác còn sống thì trả -1/EADDRINUSE, không cướp listener của nó.
/// Trả về fd (non-blocking, cloexec) hoặc -1 nếu thất bại.
inline int bindUinputServerSocket(const std::string &sockPath) {
    struct sockaddr_un addr {};
    if (sockPath.size() >= sizeof(addr.sun_path)) {
        errno = ENAMETOOLONG;
        return -1;
    }
    addr.sun_family = AF_UNIX;
    std::memcpy(addr.sun_path, sockPath.c_str(), sockPath.size());

    const std::string dir = sockPath.substr(0, sockPath.rfind('/'));
    if (mkdir(dir.c_str(), 0700) != 0 && errno != EEXIST) {
        return -1;
    }
    if (chmod(dir.c_str(), 0700) != 0) { // đã tồn tại (systemd RuntimeDirectory…) → siết lại
        return -1;
    }
    if (socketAlive(addr)) {
        errno = EADDRINUSE;
        return -1;
    }
    unlink(sockPath.c_str());

    int fd = socket(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK | SOCK_CLOEXEC, 0);
    if (fd < 0) {
        return -1;
    }
    const mode_t oldUmask = umask(0177); // socket sinh ra đã 0600
    const int bindRet = bind(fd, reinterpret_cast<struct sockaddr *>(&addr), sizeof(addr));
    umask(oldUmask);
    if (bindRet != 0 || listen(fd, 4) != 0) {
        close(fd);
        return -1;
    }
    return fd;
}

} // namespace pinakey

#endif // _PINAKEY_SERVER_SERVERSOCKET_H_
