/*
 * Test bind socket server uinput (#72): thư mục 0700, socket 0600 ngay từ lúc sinh ra
 * (umask, không có cửa sổ chmod), dọn socket cũ khi bind lại. GPL-3.0-or-later.
 */
#include "../server/serversocket.h"

#include <fcitx-utils/log.h>

#include <cstdlib>
#include <cstring>
#include <string>

#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

int main() {
    using pinakey::bindUinputServerSocket;

    char tmpl[] = "/tmp/pinakey-srvsock-XXXXXX";
    FCITX_ASSERT(mkdtemp(tmpl) != nullptr);
    const std::string dir = std::string(tmpl) + "/pinakey";
    const std::string path = dir + "/uinput.sock";

    // Bind lần đầu: tự tạo thư mục.
    int fd = bindUinputServerSocket(path);
    FCITX_ASSERT(fd >= 0);

    // Thuộc tính bảo mật cốt lõi: thư mục 0700, socket 0600.
    struct stat st {};
    FCITX_ASSERT(stat(dir.c_str(), &st) == 0);
    FCITX_ASSERT((st.st_mode & 07777) == 0700);
    FCITX_ASSERT(stat(path.c_str(), &st) == 0);
    FCITX_ASSERT((st.st_mode & 07777) == 0600);

    // Client cùng user kết nối được thật.
    int c = socket(AF_UNIX, SOCK_SEQPACKET, 0);
    FCITX_ASSERT(c >= 0);
    struct sockaddr_un addr {};
    addr.sun_family = AF_UNIX;
    std::memcpy(addr.sun_path, path.c_str(), path.size());
    FCITX_ASSERT(connect(c, reinterpret_cast<struct sockaddr *>(&addr), sizeof(addr)) == 0);
    ::close(c);
    ::close(fd);

    // Daemon trước thoát bẩn để lại socket cũ → bind lại phải thành công (unlink stale).
    fd = bindUinputServerSocket(path);
    FCITX_ASSERT(fd >= 0);
    ::close(fd);

    // Đường quá dài cho sun_path → trả -1, không crash.
    const std::string tooLong = std::string(tmpl) + "/" + std::string(120, 'a') + "/uinput.sock";
    FCITX_ASSERT(bindUinputServerSocket(tooLong) == -1);

    ::unlink(path.c_str());
    ::rmdir(dir.c_str());
    ::rmdir(tmpl);
    return 0;
}
