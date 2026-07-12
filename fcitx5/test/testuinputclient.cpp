/*
 * Test client uinput (issue #91/#105/#106):
 * - #91: lần connect đầu thất bại không khoá vĩnh viễn — retry có throttle.
 * - #105: connect() AF_UNIX thành công ngay khi còn backlog — client chỉ được coi là
 *   available sau byte hello daemon gửi khi CHẤP NHẬN (auth OK); send-fail cũng tính
 *   vào throttle (không connect mỗi phím gõ khi daemon từ chối).
 * - #106: sendBackspaces trả bool — caller không chờ ACK ma khi send thất bại.
 * GPL-3.0-or-later.
 */
#include "../src/socketpath.h"
#include "../src/uinputclient.h"

#include <fcitx-utils/log.h>

#include <chrono>
#include <cstdlib>
#include <cstring>
#include <string>
#include <thread>

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

// Daemon CHẤP NHẬN: accept rồi gửi hello (#105). Chạy trong thread vì accept chặn.
std::thread acceptHello(int server, int *connOut) {
    return std::thread([server, connOut] {
        int conn = accept(server, nullptr, nullptr);
        FCITX_ASSERT(conn >= 0);
        const char hello = fcitx::pinakey::kUinputHello;
        FCITX_ASSERT(send(conn, &hello, 1, MSG_NOSIGNAL) == 1);
        *connOut = conn;
    });
}

// Daemon TỪ CHỐI (auth fail): accept rồi đóng ngay, không hello — như pinakey-server.
std::thread acceptReject(int server) {
    return std::thread([server] {
        int conn = accept(server, nullptr, nullptr);
        FCITX_ASSERT(conn >= 0);
        ::close(conn);
    });
}

} // namespace

int main() {
    using namespace std::chrono_literals;
    using fcitx::pinakey::UinputClient;

    char tmpl[] = "/tmp/pinakey-uinput-test-XXXXXX";
    FCITX_ASSERT(mkdtemp(tmpl) != nullptr);
    const std::string sockName = std::string(tmpl) + "/uinput.sock";

    // Đồng hồ giả để test throttle tất định, không cần sleep. #123: cửa sổ hello nới lên
    // 5s (mặc định 200ms) — thread acceptHello trễ lịch trên CI quá tải không làm test đỏ
    // giả; các kịch bản dưới không đo thời gian hello nên nới rộng vô hại.
    std::chrono::steady_clock::time_point now{};
    UinputClient client(sockName, 5s, [&now] { return now; }, /*helloTimeoutMs=*/5000);

    // Daemon chưa chạy → lần đầu thất bại.
    FCITX_ASSERT(!client.available());

    // Daemon xuất hiện ngay sau đó nhưng CHƯA hết cửa sổ throttle → vẫn false (không
    // connect() mỗi phím gõ).
    int server = listenOn(sockName);
    FCITX_ASSERT(!client.available());

    // Hết throttle + daemon CHẤP NHẬN (hello) → available (lõi #91).
    now += 6s;
    int conn = -1;
    {
        auto t = acceptHello(server, &conn);
        FCITX_ASSERT(client.available());
        t.join();
    }
    FCITX_ASSERT(conn >= 0);

    // Kết nối dùng được thật: send trả true và server nhận đúng số Backspace (#106).
    FCITX_ASSERT(client.sendBackspaces(3));
    int n = 0;
    FCITX_ASSERT(recv(conn, &n, sizeof(n), 0) == static_cast<ssize_t>(sizeof(n)));
    FCITX_ASSERT(n == 3);

    // Daemon "chết": send phải trả FALSE (caller không chờ ACK ma, #106) và thất bại này
    // TÍNH VÀO THROTTLE (#105) — available() ngay sau đó không connect lại.
    ::close(conn);
    ::close(server);
    bool sent = client.sendBackspaces(1);
    if (sent) { // send đầu có thể lọt vào buffer trước khi kernel biết peer đóng
        sent = client.sendBackspaces(1);
    }
    FCITX_ASSERT(!sent);
    FCITX_ASSERT(!client.available()); // trong cửa sổ throttle → không thử lại

    // Daemon TỪ CHỐI sau accept (fcitx5 ngoài allowlist, #105): connect được nhưng không
    // hello → available phải FALSE, và thất bại tính vào throttle.
    server = listenOn(sockName);
    now += 6s;
    {
        auto t = acceptReject(server);
        FCITX_ASSERT(!client.available());
        t.join();
    }
    FCITX_ASSERT(!client.available()); // ngay sau đó: throttled, không connect lại

    // Daemon chấp nhận trở lại (restart đúng cấu hình) → hồi phục.
    now += 6s;
    {
        auto t = acceptHello(server, &conn);
        FCITX_ASSERT(client.available());
        t.join();
    }
    FCITX_ASSERT(client.sendBackspaces(2));
    n = 0;
    FCITX_ASSERT(recv(conn, &n, sizeof(n), 0) == static_cast<ssize_t>(sizeof(n)));
    FCITX_ASSERT(n == 2);

    // Daemon nghẽn (không đọc, buffer đầy): sendBackspaces phải trả false trong thời gian
    // hữu hạn (EAGAIN → chờ ngắn → bỏ), không treo main thread vô hạn (#106).
    bool everFailed = false;
    for (int i = 0; i < 1000000; ++i) {
        if (!client.sendBackspaces(999)) {
            everFailed = true;
            break;
        }
    }
    FCITX_ASSERT(everFailed);
    FCITX_ASSERT(!client.available()); // nghẽn → coi như chết phiên này, throttle

    // #123: helloTimeoutMs tiêm được thật sự có tác dụng — hello tới MUỘN hơn cửa sổ mặc
    // định 200ms (300ms), client cấu hình 5s vẫn phải nhận được. Nếu tham số bị bỏ qua
    // (poll vẫn 200ms cứng) thì available() timeout → test đỏ.
    {
        UinputClient patient(sockName, 5s, [&now] { return now; }, /*helloTimeoutMs=*/5000);
        int server2 = listenOn(sockName);
        now += 6s;
        int conn2 = -1;
        std::thread lateHello([server2, &conn2] {
            int c = accept(server2, nullptr, nullptr);
            FCITX_ASSERT(c >= 0);
            std::this_thread::sleep_for(std::chrono::milliseconds(300));
            const char hello = fcitx::pinakey::kUinputHello;
            FCITX_ASSERT(send(c, &hello, 1, MSG_NOSIGNAL) == 1);
            conn2 = c;
        });
        FCITX_ASSERT(patient.available())
            << "hello tới sau 300ms phải được chấp nhận với cửa sổ 5s";
        lateHello.join();
        ::close(conn2);
        ::close(server2);
    }

    ::close(conn);
    ::close(server);
    ::unlink(sockName.c_str());
    ::rmdir(tmpl);
    return 0;
}
