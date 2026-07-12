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

#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

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
    void sendBackspaces(int n) {
        if (n <= 0 || !available()) {
            return;
        }
        // MSG_DONTWAIT: daemon kẹt (buffer đầy) thì coi như chết và nhả fd — tuyệt đối không
        // block main thread của fcitx5 (đơ toàn bộ bàn phím).
        if (send(fd_, &n, sizeof(n), MSG_NOSIGNAL | MSG_DONTWAIT) <= 0) {
            ::close(fd_);
            fd_ = -1;             // sẽ thử kết nối lại lần sau
            failedOnce_ = false;  // daemon vừa còn sống → lần available() kế thử ngay
        }
    }

private:
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
