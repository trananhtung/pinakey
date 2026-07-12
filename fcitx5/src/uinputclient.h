/*
 * Client tới daemon uinput của PinaKey (issue #28/#91). GPL-3.0-or-later.
 */
#ifndef _PINAKEY_FCITX5_UINPUTCLIENT_H_
#define _PINAKEY_FCITX5_UINPUTCLIENT_H_

#include <chrono>
#include <cstddef>
#include <cstring>
#include <functional>
#include <string>
#include <utility>

#include <poll.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#include "socketpath.h"

namespace fcitx::pinakey {

/// Client tới daemon uinput — một kết nối dùng chung cho cả tiến trình addon. Gửi số lượng
/// Backspace cần bơm cho các app không hỗ trợ SurroundingText. Nếu không kết nối được (daemon
/// chưa cài/chạy), `available()` trả false và addon lùi về chế độ preedit.
///
/// #91: thất bại KHÔNG khoá vĩnh viễn — client thử kết nối lại sau mỗi `retryInterval`
/// (throttle để không connect() mỗi phím gõ), nên daemon khởi động muộn hoặc restart vẫn
/// được nhận mà không cần restart fcitx5. Đồng hồ inject được để test tất định.
class UinputClient {
public:
    using Clock = std::chrono::steady_clock;

    explicit UinputClient(std::string sockPath,
                          Clock::duration retryInterval = std::chrono::seconds(5),
                          std::function<Clock::time_point()> now = [] { return Clock::now(); })
        : sockPath_(std::move(sockPath)), retryInterval_(retryInterval), now_(std::move(now)) {}
    ~UinputClient() {
        if (fd_ >= 0) {
            ::close(fd_);
        }
    }
    UinputClient(const UinputClient &) = delete;
    UinputClient &operator=(const UinputClient &) = delete;

    bool available() {
        if (fd_ >= 0) {
            return true;
        }
        const auto t = now_();
        if (failedOnce_ && t - lastAttempt_ < retryInterval_) {
            return false;
        }
        lastAttempt_ = t;
        tryConnect();
        failedOnce_ = fd_ < 0;
        return fd_ >= 0;
    }
    /// Trả true khi thông điệp đã thật sự vào socket. Trả false = daemon chết/từ chối/nghẽn —
    /// caller KHÔNG được vào trạng thái chờ ACK (#106: chờ ACK ma → treo 500ms rồi commit đè
    /// chữ cũ, đúp chữ im lặng).
    bool sendBackspaces(int n) {
        if (n <= 0) {
            return true; // không có gì để gửi
        }
        if (!available()) {
            return false;
        }
        // MSG_DONTWAIT: tuyệt đối không block vô hạn main thread của fcitx5 (đơ bàn phím).
        if (trySend(n)) {
            return true;
        }
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            // #106: buffer đầy nhưng kết nối còn lành (daemon đang bận vòng bơm) — chờ ngắn
            // CÓ GIỚI HẠN rồi thử lại một lần, đừng vứt lệnh xoá ngay.
            struct pollfd p{fd_, POLLOUT, 0};
            if (poll(&p, 1, kSendWaitMs) > 0 && trySend(n)) {
                return true;
            }
        }
        ::close(fd_);
        fd_ = -1;
        failedOnce_ = true; // #105: send-fail cũng tính vào throttle — không connect mỗi phím
        lastAttempt_ = now_();
        return false;
    }

private:
    static constexpr int kHelloTimeoutMs = 200; // chờ hello sau connect (1 lần mỗi cửa sổ retry)
    static constexpr int kSendWaitMs = 50;      // chờ POLLOUT khi buffer đầy (EAGAIN)

    bool trySend(int n) {
        return send(fd_, &n, sizeof(n), MSG_NOSIGNAL | MSG_DONTWAIT) ==
               static_cast<ssize_t>(sizeof(n));
    }

    void tryConnect() {
        int fd = socket(AF_UNIX, SOCK_SEQPACKET | SOCK_CLOEXEC, 0);
        if (fd < 0) {
            return;
        }
        // #72: socket FILESYSTEM (0600 trong $XDG_RUNTIME_DIR) thay abstract namespace —
        // quyền filesystem chặn tiến trình khác user ngay từ connect().
        struct sockaddr_un addr {};
        addr.sun_family = AF_UNIX;
        if (sockPath_.size() >= sizeof(addr.sun_path)) {
            ::close(fd);
            return;
        }
        std::memcpy(addr.sun_path, sockPath_.c_str(), sockPath_.size());
        if (connect(fd, reinterpret_cast<struct sockaddr *>(&addr), sizeof(addr)) != 0) {
            ::close(fd);
            return;
        }
        // #105: với AF_UNIX, connect() OK ngay khi còn backlog — daemon có thể từ chối
        // (auth fail) ngay sau accept. Chỉ coi là kết nối khi nhận được byte hello; không
        // có trong kHelloTimeoutMs → thất bại (tính vào throttle ở available()).
        struct pollfd p{fd, POLLIN, 0};
        char hello = 0;
        if (poll(&p, 1, kHelloTimeoutMs) <= 0 || recv(fd, &hello, 1, 0) != 1 ||
            hello != kUinputHello) {
            ::close(fd);
            return;
        }
        fd_ = fd;
    }

    std::string sockPath_;
    Clock::duration retryInterval_;
    std::function<Clock::time_point()> now_;
    int fd_ = -1;
    bool failedOnce_ = false;
    Clock::time_point lastAttempt_{};
};

} // namespace fcitx::pinakey

#endif // _PINAKEY_FCITX5_UINPUTCLIENT_H_
