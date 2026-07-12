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

    // Bind lần đầu: tự tạo thư mục. Giữ lockFd — đóng nó là nhả quyền sở hữu đường socket.
    int lockFd = -1;
    int fd = bindUinputServerSocket(path, &lockFd);
    FCITX_ASSERT(fd >= 0);
    FCITX_ASSERT(lockFd >= 0);

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

    // Daemon khác ĐANG SỐNG (còn giữ lock) trên cùng đường → không được cướp socket của nó.
    int secondLock = -1;
    int second = bindUinputServerSocket(path, &secondLock);
    FCITX_ASSERT(second == -1);
    ::close(fd);
    ::close(lockFd); // "daemon chết" → lock tự nhả

    // Daemon trước thoát bẩn để lại socket cũ (không ai giữ lock) → bind lại phải thành công.
    fd = bindUinputServerSocket(path, &lockFd);
    FCITX_ASSERT(fd >= 0);
    ::close(fd);
    ::close(lockFd);

    // Đường quá dài cho sun_path → trả -1, không crash.
    const std::string tooLong = std::string(tmpl) + "/" + std::string(120, 'a') + "/uinput.sock";
    FCITX_ASSERT(bindUinputServerSocket(tooLong) == -1);

    // #115: đường TƯƠNG ĐỐI (XDG_RUNTIME_DIR không chuẩn) phải bị bác EINVAL — không được
    // mkdir/bind theo CWD của daemon (tạo rác ./run/... mà client không bao giờ tìm thấy).
    // Chdir vào thư mục tạm rỗng để phép kiểm "không tạo 'run'" không phụ thuộc CWD sẵn có.
    FCITX_ASSERT(chdir(tmpl) == 0);
    errno = 0;
    FCITX_ASSERT(bindUinputServerSocket("run/pinakey/uinput.sock") == -1);
    FCITX_ASSERT(errno == EINVAL);
    struct stat relSt;
    FCITX_ASSERT(stat("run", &relSt) != 0) << "không được tạo thư mục 'run' theo CWD";

    // #115: đường tuyệt đối nhưng thư mục cha là "/" (hoặc toàn dấu '/') cũng phải bác EINVAL —
    // không được mkdir/chmod thẳng lên thư mục gốc.
    for (const char *bad : {"/uinput.sock", "//uinput.sock", "///uinput.sock"}) {
        errno = 0;
        FCITX_ASSERT(bindUinputServerSocket(bad) == -1) << bad;
        FCITX_ASSERT(errno == EINVAL) << bad << " errno=" << errno;
    }
    FCITX_ASSERT(chdir("/") == 0); // rời khỏi tmpl trước khi rmdir bên dưới

    ::unlink(path.c_str());
    ::unlink((dir + "/uinput.lock").c_str());
    ::rmdir(dir.c_str());
    ::rmdir(tmpl);
    return 0;
}
