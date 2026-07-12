/*
 * Bind socket lắng nghe cho daemon uinput (issue #72). Không phụ thuộc fcitx5 — chỉ libc.
 * GPL-3.0-or-later.
 */
#ifndef _PINAKEY_SERVER_SERVERSOCKET_H_
#define _PINAKEY_SERVER_SERVERSOCKET_H_

#include <cerrno>
#include <cstring>
#include <string>

#include <fcntl.h>
#include <sys/file.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

namespace pinakey {

/// Tạo socket SEQPACKET lắng nghe tại `sockPath` với thuộc tính bảo mật của #72:
/// thư mục cha 0700 (tạo nếu chưa có, siết lại nếu đã có), socket 0600 ngay từ lúc sinh
/// (qua umask — không có cửa sổ chmod). Chống hai daemon giẫm nhau bằng flock giữ SUỐT ĐỜI
/// tiến trình trên `uinput.lock` cùng thư mục: instance sau thất bại ngay (EADDRINUSE),
/// không có cửa sổ probe/unlink/bind; lock tự nhả khi tiến trình chết nên socket sót lại
/// từ lần thoát bẩn chắc chắn stale → unlink an toàn.
/// Trả về fd lắng nghe (non-blocking, cloexec) hoặc -1 nếu thất bại. `lockFdOut` nhận fd
/// của file lock (đóng nó = nhả lock); truyền nullptr để giữ lock tới hết đời tiến trình.
inline int bindUinputServerSocket(const std::string &sockPath, int *lockFdOut = nullptr) {
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
    const std::string lockPath = dir + "/uinput.lock";
    int lockFd = open(lockPath.c_str(), O_RDWR | O_CREAT | O_CLOEXEC, 0600);
    if (lockFd < 0) {
        return -1;
    }
    if (flock(lockFd, LOCK_EX | LOCK_NB) != 0) {
        close(lockFd); // daemon khác đang sống (giữ lock) → không đụng socket của nó
        errno = EADDRINUSE;
        return -1;
    }
    unlink(sockPath.c_str()); // giữ lock rồi → socket còn lại chắc chắn stale

    int fd = socket(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK | SOCK_CLOEXEC, 0);
    if (fd < 0) {
        close(lockFd);
        return -1;
    }
    const mode_t oldUmask = umask(0177); // socket sinh ra đã 0600
    const int bindRet = bind(fd, reinterpret_cast<struct sockaddr *>(&addr), sizeof(addr));
    umask(oldUmask);
    if (bindRet != 0 || listen(fd, 4) != 0) {
        close(fd);
        close(lockFd);
        return -1;
    }
    if (lockFdOut) {
        *lockFdOut = lockFd;
    } // else: cố ý giữ lockFd mở tới hết đời tiến trình (daemon)
    return fd;
}

} // namespace pinakey

#endif // _PINAKEY_SERVER_SERVERSOCKET_H_
