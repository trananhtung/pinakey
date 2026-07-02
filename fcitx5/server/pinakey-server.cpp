/*
 * pinakey-uinput-server — daemon đặc quyền bơm phím Backspace cho chế độ "gõ không gạch chân"
 * trong các ứng dụng KHÔNG hỗ trợ SurroundingText (issue #28).
 *
 * Vì sao cần daemon riêng: addon fcitx5 chạy trong tiến trình fcitx5 (không đặc quyền) nên không mở
 * được `/dev/uinput`. Daemon này (cấp quyền qua udev/systemd) mở uinput, lắng nghe trên một
 * **abstract Unix socket**, xác thực client (UID trùng + tiến trình là `/usr/bin/fcitx5`), rồi với
 * mỗi số `count` nhận được sẽ phát `count` lần Backspace. Mô hình theo fcitx5-lotus nhưng rút gọn
 * (chỉ bàn phím; reset khi click chuột do fcitx5 đảm nhiệm qua reset()).
 *
 * GPL-3.0-or-later.
 */
#include <fcntl.h>
#include <linux/uinput.h>
#include <poll.h>
#include <pwd.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#include <atomic>
#include <cstddef>
#include <cerrno>
#include <climits>
#include <cstdio>
#include <cstring>
#include <csignal>
#include <string>
#include <vector>

namespace {

std::atomic<bool> g_running{true};

void onSignal(int) { g_running.store(false); }

/// RAII cho file descriptor.
class Fd {
public:
    explicit Fd(int fd = -1) : fd_(fd) {}
    ~Fd() { reset(); }
    Fd(const Fd &) = delete;
    Fd &operator=(const Fd &) = delete;
    int get() const { return fd_; }
    bool valid() const { return fd_ >= 0; }
    void reset(int fd = -1) {
        if (fd_ >= 0) {
            close(fd_);
        }
        fd_ = fd;
    }

private:
    int fd_;
};

std::string usernameForUid(uid_t uid) {
    long bufSize = sysconf(_SC_GETPW_R_SIZE_MAX);
    if (bufSize <= 0) {
        bufSize = 16384;
    }
    std::vector<char> buf(static_cast<size_t>(bufSize));
    struct passwd pwd {};
    struct passwd *result = nullptr;
    if (getpwuid_r(uid, &pwd, buf.data(), buf.size(), &result) == 0 && result) {
        return result->pw_name;
    }
    return "unknown";
}

uid_t uidForUsername(const std::string &name) {
    char buf[4096];
    struct passwd pwd {};
    struct passwd *result = nullptr;
    if (getpwnam_r(name.c_str(), &pwd, buf, sizeof(buf), &result) == 0 && result) {
        return result->pw_uid;
    }
    return static_cast<uid_t>(-1);
}

/// Thiết bị bàn phím ảo chỉ có phím Backspace.
class UinputKeyboard {
public:
    bool init() {
        int fd = open("/dev/uinput", O_WRONLY | O_NONBLOCK);
        if (fd < 0) {
            return false;
        }
        fd_.reset(fd);
        if (ioctl(fd, UI_SET_EVBIT, EV_KEY) < 0 || ioctl(fd, UI_SET_KEYBIT, KEY_BACKSPACE) < 0) {
            return false;
        }
        struct uinput_setup setup {};
        setup.id.bustype = BUS_USB;
        setup.id.vendor = 0x9527;
        setup.id.product = 0x4b59; // 'KY'
        std::strncpy(setup.name, "PinaKey Virtual Backspace", UINPUT_MAX_NAME_SIZE - 1);
        if (ioctl(fd, UI_DEV_SETUP, &setup) < 0 || ioctl(fd, UI_DEV_CREATE) < 0) {
            return false;
        }
        sleep(1); // chờ desktop nhận thiết bị mới
        return true;
    }
    ~UinputKeyboard() {
        if (fd_.valid()) {
            ioctl(fd_.get(), UI_DEV_DESTROY);
        }
    }
    void sendBackspace() {
        struct input_event ev[4] {};
        ev[0].type = EV_KEY;
        ev[0].code = KEY_BACKSPACE;
        ev[0].value = 1; // nhấn
        ev[1].type = EV_SYN;
        ev[1].code = SYN_REPORT;
        ev[2].type = EV_KEY;
        ev[2].code = KEY_BACKSPACE;
        ev[2].value = 0; // nhả
        ev[3].type = EV_SYN;
        ev[3].code = SYN_REPORT;
        auto _ = write(fd_.get(), ev, sizeof(ev));
        (void)_;
    }

private:
    Fd fd_;
};

} // namespace

int main(int argc, char *argv[]) {
    std::string targetUser;
    if (argc == 3 && std::strcmp(argv[1], "-u") == 0) {
        targetUser = argv[2];
    } else {
        targetUser = usernameForUid(getuid());
    }
    const uid_t expectedUid = uidForUsername(targetUser);
    if (expectedUid == static_cast<uid_t>(-1)) {
        std::fprintf(stderr, "pinakey-server: không tìm thấy UID cho user %s\n", targetUser.c_str());
        return 1;
    }
    std::fprintf(stderr, "pinakey-server: phục vụ user %s (uid %u)\n", targetUser.c_str(),
                 expectedUid);

    UinputKeyboard kbd;
    if (!kbd.init()) {
        std::fprintf(stderr, "pinakey-server: không mở được /dev/uinput (cần quyền/udev)\n");
        return 1;
    }

    // Abstract socket: "pinakeysocket-<user>-kb".
    std::string sockName = "pinakeysocket-" + targetUser + "-kb";
    Fd server(socket(AF_UNIX, SOCK_SEQPACKET | SOCK_NONBLOCK, 0));
    if (!server.valid()) {
        std::perror("socket");
        return 1;
    }
    struct sockaddr_un addr {};
    addr.sun_family = AF_UNIX;
    addr.sun_path[0] = '\0'; // NUL đầu => abstract namespace
    const size_t maxLen = sizeof(addr.sun_path) - 2;
    if (sockName.size() > maxLen) {
        sockName.resize(maxLen);
    }
    std::memcpy(&addr.sun_path[1], sockName.c_str(), sockName.size());
    socklen_t addrLen =
        static_cast<socklen_t>(offsetof(struct sockaddr_un, sun_path) + 1 + sockName.size());
    if (bind(server.get(), reinterpret_cast<struct sockaddr *>(&addr), addrLen) != 0) {
        std::perror("bind");
        return 1;
    }
    listen(server.get(), 4);

    struct sigaction sa {};
    sa.sa_handler = onSignal;
    sigaction(SIGTERM, &sa, nullptr);
    sigaction(SIGINT, &sa, nullptr);

    Fd client;
    std::vector<struct pollfd> fds(2);
    while (g_running.load(std::memory_order_acquire)) {
        fds[0] = {server.get(), POLLIN, 0};
        fds[1] = {client.valid() ? client.get() : -1, POLLIN, 0};
        int ret = poll(fds.data(), fds.size(), -1);
        if (ret < 0) {
            if (errno == EINTR) {
                continue;
            }
            break;
        }
        // kết nối mới: xác thực rồi giữ một client duy nhất
        if (fds[0].revents & POLLIN) {
            int c = accept4(server.get(), nullptr, nullptr, SOCK_NONBLOCK);
            if (c >= 0) {
                struct ucred cred {};
                socklen_t len = sizeof(cred);
                bool ok = false;
                if (getsockopt(c, SOL_SOCKET, SO_PEERCRED, &cred, &len) == 0 &&
                    cred.uid == expectedUid) {
                    char link[64];
                    char exe[PATH_MAX] = {0};
                    std::snprintf(link, sizeof(link), "/proc/%d/exe", cred.pid);
                    ssize_t n = readlink(link, exe, sizeof(exe) - 1);
                    if (n > 0) {
                        exe[n] = '\0';
                        // #72: binary fcitx5 ở các prefix cài đặt chuẩn — so ĐƯỜNG DẪN THẬT
                        // của tiến trình (readlink /proc/<pid>/exe, không tin argv[0]/cmdline).
                        static const char *kAllowedExes[] = {
                            "/usr/bin/fcitx5",
                            "/usr/local/bin/fcitx5",
                        };
                        for (const char *allowed : kAllowedExes) {
                            if (std::strcmp(exe, allowed) == 0) {
                                ok = true;
                                break;
                            }
                        }
                    }
                }
                if (ok) {
                    client.reset(c);
                    std::fprintf(stderr, "pinakey-server: fcitx5 đã kết nối\n");
                } else {
                    close(c);
                }
            }
        }
        // dữ liệu từ client: một int = số Backspace cần bơm
        if (client.valid() && (fds[1].revents & (POLLIN | POLLHUP | POLLERR))) {
            int count = 0;
            ssize_t n = recv(client.get(), &count, sizeof(count), 0);
            if (n < 0) {
                // EINTR (tín hiệu) / EAGAIN (poll đánh thức giả trên socket non-blocking):
                // client chưa chết — giữ kết nối, yêu cầu đang bay không bị mất.
                if (errno != EINTR && errno != EAGAIN && errno != EWOULDBLOCK) {
                    client.reset(-1);
                }
            } else if (n == 0) {
                client.reset(-1); // peer đóng kết nối
            } else if (count > 0 && count < 1000) {
                for (int i = 0; i < count && g_running.load(); ++i) {
                    kbd.sendBackspace();
                    usleep(1500); // nhịp nhẹ để ứng dụng kịp xử lý
                }
            }
        }
    }
    std::fprintf(stderr, "pinakey-server: kết thúc\n");
    return 0;
}
