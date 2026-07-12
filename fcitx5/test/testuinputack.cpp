/*
 * Test tích hợp chế độ uinput+ACK (app KHÔNG có SurroundingText, PINAKEY_UINPUT=1) qua fcitx5
 * thật + daemon giả lập (hello #105):
 *  - Vòng ACK trọn vẹn: bơm N+1 Backspace, N cái đầu đi tiếp xoá thật, cái cuối commit chuỗi mới.
 *  - #96: Backspace bơm-ngược không quay về trong 500ms → nhánh timeout phải replay các phím đã
 *    đệm TRƯỚC phím hiện tại (đúng thứ tự), không bỏ quên/nuốt.
 * GPL-3.0-or-later.
 */
#include <fcitx-utils/eventdispatcher.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysymgen.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/testing.h>
#include <fcitx/addonmanager.h>
#include <fcitx/event.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputmethodgroup.h>
#include <fcitx/inputmethodmanager.h>
#include <fcitx/instance.h>

#include <atomic>
#include <chrono>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

#include "../src/socketpath.h"

using namespace fcitx;

namespace {

/// Daemon uinput giả lập: nghe trên socket filesystem (#72), gửi hello khi nhận client (#105),
/// gom các `count` nhận được. KHÔNG tự bơm Backspace — test chủ động bơm (hoặc không, để test
/// timeout), vì trong đời thực Backspace quay về qua compositor thành key event.
class FakeDaemon {
public:
    explicit FakeDaemon(const std::string &sockPath) : path_(sockPath) {
        ::unlink(path_.c_str());
        server_ = socket(AF_UNIX, SOCK_SEQPACKET, 0);
        FCITX_ASSERT(server_ >= 0);
        struct sockaddr_un addr {};
        addr.sun_family = AF_UNIX;
        FCITX_ASSERT(path_.size() < sizeof(addr.sun_path));
        std::memcpy(addr.sun_path, path_.c_str(), path_.size());
        FCITX_ASSERT(bind(server_, reinterpret_cast<struct sockaddr *>(&addr), sizeof(addr)) == 0);
        FCITX_ASSERT(listen(server_, 1) == 0);
        thread_ = std::thread([this] { run(); });
    }
    ~FakeDaemon() {
        stop_.store(true);
        ::shutdown(server_, SHUT_RDWR);
        // Thread có thể đang kẹt trong recv() trên kết nối ĐÃ accept (client addon giữ kết
        // nối suốt đời tiến trình) — phải shutdown cả nó, không chỉ listener.
        const int c = conn_.load();
        if (c >= 0) {
            ::shutdown(c, SHUT_RDWR);
        }
        if (thread_.joinable()) {
            thread_.join();
        }
        ::close(server_);
        ::unlink(path_.c_str());
    }

    /// Chờ (tối đa ~2s) tới khi nhận được `n` thông điệp count.
    bool waitCounts(size_t n) {
        for (int i = 0; i < 200; ++i) {
            {
                std::lock_guard<std::mutex> lk(mu_);
                if (counts_.size() >= n) {
                    return true;
                }
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        return false;
    }
    std::vector<int> counts() {
        std::lock_guard<std::mutex> lk(mu_);
        return counts_;
    }

private:
    void run() {
        while (!stop_.load()) {
            int c = accept(server_, nullptr, nullptr);
            if (c < 0) {
                return;
            }
            conn_.store(c);
            const char hello = fcitx::pinakey::kUinputHello;
            (void)send(c, &hello, 1, MSG_NOSIGNAL);
            int count = 0;
            while (recv(c, &count, sizeof(count), 0) == static_cast<ssize_t>(sizeof(count))) {
                std::lock_guard<std::mutex> lk(mu_);
                counts_.push_back(count);
            }
            conn_.store(-1);
            ::close(c);
        }
    }

    std::string path_;
    int server_ = -1;
    std::atomic<int> conn_{-1};
    std::thread thread_;
    std::mutex mu_;
    std::vector<int> counts_;
    std::atomic<bool> stop_{false};
};

/// Terminal giả lập: KHÔNG có capability nào; văn bản chỉ đổi qua commit và qua các phím
/// KHÔNG bị addon nuốt (Backspace đi tiếp xoá thật một ký tự — như app thật xử lý keystroke).
class TermInputContext : public InputContext {
public:
    explicit TermInputContext(InputContextManager &mgr) : InputContext(mgr, "plainterm") {
        setCapabilityFlags(CapabilityFlags{});
        created();
    }
    ~TermInputContext() override { destroy(); }
    const char *frontend() const override { return "doc"; }
    void commitStringImpl(const std::string &text) override { doc_ += text; }
    void deleteSurroundingTextImpl(int, unsigned int) override {}
    void forwardKeyImpl(const ForwardKeyEvent &) override {}
    void updatePreeditImpl() override {}

    /// Bơm một phím qua addon; phím KHÔNG bị nuốt thì áp vào "tài liệu" như app thật.
    void type(const Key &key) {
        KeyEvent ke(this, key, false);
        const bool filtered = keyEvent(ke);
        if (filtered) {
            return;
        }
        if (ke.rawKey().sym() == FcitxKey_BackSpace) {
            if (!doc_.empty()) {
                // Xoá 1 ký tự UTF-8 cuối.
                size_t i = doc_.size() - 1;
                while (i > 0 && (static_cast<unsigned char>(doc_[i]) & 0xC0) == 0x80) {
                    --i;
                }
                doc_.erase(i);
            }
            return;
        }
        const std::string u = Key::keySymToUTF8(ke.rawKey().sym());
        doc_ += u;
    }
    void typeString(const std::string &keys) {
        for (char c : keys) {
            type(c == ' ' ? Key("space") : Key(std::string(1, c)));
        }
    }

    std::string text() const { return doc_; }
    void clearDoc() { doc_.clear(); }

private:
    std::string doc_;
};

} // namespace

int main() {
    // Cách ly config + runtime dir; bật PINAKEY_UINPUT trước khi addon đọc env (cache một lần).
    const std::string base = std::string(TESTING_BINARY_DIR) + "/testuinputack-xdg";
    std::filesystem::remove_all(base);
    std::filesystem::create_directories(base + "/config/pinakey");
    std::filesystem::create_directories(base + "/runtime/pinakey");
    {
        std::ofstream cfg(base + "/config/pinakey/ibus-PinaKey.config.json");
        cfg << "{\"IBflags\":1081840}"; // IB_STD_FLAGS (có no-underline)
    }
    setenv("XDG_CONFIG_HOME", (base + "/config").c_str(), 1);
    setenv("XDG_RUNTIME_DIR", (base + "/runtime").c_str(), 1);
    setenv("PINAKEY_UINPUT", "1", 1);

    FakeDaemon daemon(base + "/runtime/pinakey/uinput.sock");

    setupTestingEnvironment(TESTING_BINARY_DIR, {PINAKEY_ADDON_SO_DIR},
                            {PINAKEY_TEST_DATA_DIR, FCITX_SYS_PKGDATADIR});

    char arg0[] = "testuinputack";
    char arg1[] = "--disable=all";
    char arg2[] = "--enable=testim,testfrontend,testui,keyboard,pinakey";
    char *argv[] = {arg0, arg1, arg2};

    Log::setLogRule("default=3");
    Instance instance(FCITX_ARRAY_SIZE(argv), argv);
    instance.addonManager().registerDefaultLoader(nullptr);

    EventDispatcher dispatcher;
    dispatcher.attach(&instance.eventLoop());
    dispatcher.schedule([&instance, &daemon]() {
        auto *pinakey = instance.addonManager().addon("pinakey", true);
        FCITX_ASSERT(pinakey);

        auto group = instance.inputMethodManager().currentGroup();
        group.inputMethodList().clear();
        group.inputMethodList().push_back(InputMethodGroupItem("keyboard-us"));
        group.inputMethodList().push_back(InputMethodGroupItem("pinakey"));
        group.setDefaultInputMethod("");
        instance.inputMethodManager().setGroup(std::move(group));

        auto ic = std::make_unique<TermInputContext>(instance.inputContextManager());
        ic->focusIn();
        instance.setCurrentInputMethod(ic.get(), "pinakey", true);

        // ---- Vòng ACK trọn vẹn: dd → đ ----
        // 'd' thứ nhất: commit "d". 'd' thứ hai: đ (del=1) → gửi count=2, HOÃN commit.
        ic->typeString("dd");
        FCITX_ASSERT(daemon.waitCounts(1));
        FCITX_ASSERT(daemon.counts()[0] == 2) << "count=" << daemon.counts()[0];
        FCITX_ASSERT(ic->text() == "d") << "trước ACK: doc=\"" << ic->text() << "\"";
        // Backspace bơm-ngược #1 (trung gian): KHÔNG bị nuốt → xoá thật 'd'.
        ic->type(Key("BackSpace"));
        FCITX_ASSERT(ic->text() == "") << "sau BS trung gian: doc=\"" << ic->text() << "\"";
        // Backspace #2 (trigger): bị nuốt + commit "đ".
        ic->type(Key("BackSpace"));
        FCITX_ASSERT(ic->text() == "đ") << "sau trigger: doc=\"" << ic->text() << "\"";

        // ---- #96: timeout 500ms phải replay phím đệm TRƯỚC phím hiện tại ----
        ic->reset();
        ic->clearDoc();
        ic->typeString("dd"); // doc="d", deleting_, chờ ACK không bao giờ tới
        FCITX_ASSERT(daemon.waitCounts(2));
        ic->typeString("e"); // gõ nhanh trong lúc chờ → bị đệm ("đe" — giữ từ hợp lệ để
                             // không kích hoạt khôi phục nguyên văn, vốn mở chuỗi xoá mới)
        std::this_thread::sleep_for(std::chrono::milliseconds(600));
        ic->typeString("n"); // timeout: commit "đ" + replay 'e' RỒI MỚI xử lý 'n'
        FCITX_ASSERT(ic->text() == "dđen")
            << "#96: doc=\"" << ic->text() << "\", mong đợi \"dđen\" (phím đệm 'e' phải ra "
               "trước 'n', không mất)";

        instance.exit();
    });
    instance.exec();
    return 0;
}
