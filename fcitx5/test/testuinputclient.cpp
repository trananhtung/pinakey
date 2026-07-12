/*
 * Test client uinput (issue #91): lần connect đầu thất bại KHÔNG được khoá vĩnh viễn —
 * client phải thử kết nối lại (có throttle) khi daemon xuất hiện/restart muộn.
 * GPL-3.0-or-later.
 */
#include "../src/uinputclient.h"

#include <fcitx-utils/log.h>

#include <chrono>
#include <cstring>
#include <cstdlib>
#include <string>

#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

namespace {

// Server SEQPACKET trên socket FILESYSTEM (#72), mô phỏng pinakey-uinput-server.
int listenOn(const std::string &path) {
    ::unlink(path.c_str());
    int fd = socket(AF_UNIX, SOCK_SEQPACKET, 0);
    FCITX_ASSERT(fd >= 0);
    struct sockaddr_un addr {};
    addr.sun_family = AF_UNIX;
    FCITX_ASSERT(path.size() < sizeof(addr.sun_path));
    std::memcpy(addr.sun_path, path.c_str(), path.size());
    FCITX_ASSERT(bind(fd, reinterpret_cast<struct sockaddr *>(&addr), sizeof(addr)) == 0);
    FCITX_ASSERT(listen(fd, 1) == 0);
    return fd;
}

} // namespace

int main() {
    using namespace std::chrono_literals;
    using fcitx::pinakey::UinputClient;

    char tmpl[] = "/tmp/pinakey-uinput-test-XXXXXX";
    FCITX_ASSERT(mkdtemp(tmpl) != nullptr);
    const std::string sockName = std::string(tmpl) + "/uinput.sock";

    // Đồng hồ giả để test throttle tất định, không cần sleep.
    std::chrono::steady_clock::time_point now{};
    UinputClient client(sockName, 5s, [&now] { return now; });

    // Daemon chưa chạy → lần đầu thất bại.
    FCITX_ASSERT(!client.available());

    // Daemon xuất hiện ngay sau đó nhưng CHƯA hết cửa sổ throttle → vẫn false (không
    // connect() mỗi phím gõ).
    int server = listenOn(sockName);
    FCITX_ASSERT(!client.available());

    // Hết cửa sổ throttle → phải kết nối lại được (lõi issue #91: trước đây
    // triedAndFailed_ khoá vĩnh viễn, chỉ restart fcitx5 mới hồi phục).
    now += 6s;
    FCITX_ASSERT(client.available());

    // Kết nối dùng được thật: server nhận đúng số Backspace.
    int conn = accept(server, nullptr, nullptr);
    FCITX_ASSERT(conn >= 0);
    client.sendBackspaces(3);
    int n = 0;
    FCITX_ASSERT(recv(conn, &n, sizeof(n), 0) == static_cast<ssize_t>(sizeof(n)));
    FCITX_ASSERT(n == 3);

    // Daemon "chết" (đóng cả hai đầu) → send thất bại → client tự nhả fd.
    ::close(conn);
    ::close(server);
    client.sendBackspaces(1);
    client.sendBackspaces(1);

    // Daemon restart → client phải hồi phục, không cần restart tiến trình.
    server = listenOn(sockName);
    now += 6s;
    FCITX_ASSERT(client.available());
    conn = accept(server, nullptr, nullptr);
    FCITX_ASSERT(conn >= 0);
    client.sendBackspaces(2);
    n = 0;
    FCITX_ASSERT(recv(conn, &n, sizeof(n), 0) == static_cast<ssize_t>(sizeof(n)));
    FCITX_ASSERT(n == 2);

    ::close(conn);
    ::close(server);
    ::unlink(sockName.c_str());
    ::rmdir(tmpl);
    return 0;
}
