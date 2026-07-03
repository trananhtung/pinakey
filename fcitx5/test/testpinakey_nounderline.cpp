/*
 * Test end-to-end chế độ "gõ không gạch chân" (SurroundingText diff-and-replace) qua fcitx5 thật.
 *
 * testfrontend của fcitx5 KHÔNG bật khả năng SurroundingText và deleteSurroundingText chỉ là no-op,
 * nên ở đây ta tự dựng một InputContext giả lập một ô văn bản: nó bật cap SurroundingText, và hiện
 * thực commitStringImpl + deleteSurroundingTextImpl trên một bộ đệm tài liệu (UTF-32). Gõ phím vào
 * engine pinakey thật, rồi so nội dung tài liệu cuối — đúng thứ người dùng sẽ thấy, KHÔNG có preedit.
 *
 * GPL-3.0-or-later.
 */
#include <fcitx-utils/eventdispatcher.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/testing.h>
#include <fcitx/addonmanager.h>
#include <fcitx/event.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputmethodgroup.h>
#include <fcitx/inputmethodmanager.h>
#include <fcitx/instance.h>

#include <algorithm>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <string>

using namespace fcitx;

namespace {

std::u32string fromUtf8(const std::string &s) {
    std::u32string out;
    size_t i = 0;
    while (i < s.size()) {
        unsigned char c = static_cast<unsigned char>(s[i]);
        char32_t cp;
        int n;
        if (c < 0x80) {
            cp = c;
            n = 1;
        } else if ((c >> 5) == 0x6) {
            cp = c & 0x1f;
            n = 2;
        } else if ((c >> 4) == 0xe) {
            cp = c & 0x0f;
            n = 3;
        } else if ((c >> 3) == 0x1e) {
            cp = c & 0x07;
            n = 4;
        } else {
            cp = c;
            n = 1;
        }
        for (int k = 1; k < n && i + k < s.size(); ++k) {
            cp = (cp << 6) | (static_cast<unsigned char>(s[i + k]) & 0x3f);
        }
        out.push_back(cp);
        i += n;
    }
    return out;
}

std::string toUtf8(const std::u32string &s) {
    std::string out;
    for (char32_t cp : s) {
        if (cp < 0x80) {
            out.push_back(static_cast<char>(cp));
        } else if (cp < 0x800) {
            out.push_back(static_cast<char>(0xC0 | (cp >> 6)));
            out.push_back(static_cast<char>(0x80 | (cp & 0x3f)));
        } else if (cp < 0x10000) {
            out.push_back(static_cast<char>(0xE0 | (cp >> 12)));
            out.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3f)));
            out.push_back(static_cast<char>(0x80 | (cp & 0x3f)));
        } else {
            out.push_back(static_cast<char>(0xF0 | (cp >> 18)));
            out.push_back(static_cast<char>(0x80 | ((cp >> 12) & 0x3f)));
            out.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3f)));
            out.push_back(static_cast<char>(0x80 | (cp & 0x3f)));
        }
    }
    return out;
}

/// InputContext giả lập một ô văn bản hỗ trợ surrounding text.
class DocInputContext : public InputContext {
public:
    explicit DocInputContext(InputContextManager &mgr, const std::string &program = "docapp")
        : InputContext(mgr, program) {
        setCapabilityFlags(CapabilityFlags{CapabilityFlag::SurroundingText});
        created();
    }
    ~DocInputContext() override { destroy(); }

    const char *frontend() const override { return "doc"; }

    /// Số lần addon gọi deleteSurroundingText — phân biệt đường replace (diff xoá-chèn, >0 khi
    /// gõ dấu) với đường preedit (#67: không bao giờ đụng surrounding text).
    int deleteCalls() const { return deleteCalls_; }

    void commitStringImpl(const std::string &text) override {
        auto u = fromUtf8(text);
        doc_.insert(cursor_, u);
        cursor_ += u.size();
        syncSurrounding();
    }
    void deleteSurroundingTextImpl(int offset, unsigned int size) override {
        ++deleteCalls_;
        long start = static_cast<long>(cursor_) + offset;
        if (start < 0) {
            start = 0;
        }
        if (static_cast<size_t>(start) > doc_.size()) {
            start = static_cast<long>(doc_.size());
        }
        size_t n = size;
        if (static_cast<size_t>(start) + n > doc_.size()) {
            n = doc_.size() - static_cast<size_t>(start);
        }
        doc_.erase(static_cast<size_t>(start), n);
        cursor_ = static_cast<size_t>(start);
        syncSurrounding();
    }
    void forwardKeyImpl(const ForwardKeyEvent & /*key*/) override {}
    void updatePreeditImpl() override {}

    std::string text() const { return toUtf8(doc_); }
    void clearDoc() {
        doc_.clear();
        cursor_ = 0;
        syncSurrounding();
    }
    /// Mô phỏng người dùng click chuột để dời con trỏ tới vị trí `pos` (theo ký tự Unicode).
    void clickAt(size_t pos) {
        cursor_ = pos > doc_.size() ? doc_.size() : pos;
        syncSurrounding();
    }

    /// Cập nhật surrounding text của fcitx5 cho khớp tài liệu — như một app cư xử đúng mực.
    void syncSurrounding() {
        surroundingText().setText(toUtf8(doc_), cursor_, cursor_);
        updateSurroundingText();
    }

private:
    std::u32string doc_;
    size_t cursor_ = 0;
    int deleteCalls_ = 0;
};

/// InputContext giả lập LibreOffice Writer (issue #66): CÓ khả năng SurroundingText nhưng báo cáo
/// KHÔNG đáng tin — khi gõ nhanh, surrounding text gửi cho IM là ảnh CŨ của tài liệu (chậm một
/// nhịp) và thiếu dấu cách. Addon phải nhận diện qua program ("soffice") và rơi về preedit;
/// nếu vẫn dùng diff-replace, phép kiểm con-trỏ-nhảy (#7) sẽ misfire giữa từ và nát chữ.
class LibreOfficeLikeInputContext : public InputContext {
public:
    explicit LibreOfficeLikeInputContext(InputContextManager &mgr)
        : InputContext(mgr, "soffice") {
        setCapabilityFlags(
            CapabilityFlags{CapabilityFlag::SurroundingText, CapabilityFlag::Preedit});
        created();
    }
    ~LibreOfficeLikeInputContext() override { destroy(); }

    const char *frontend() const override { return "doc"; }

    void commitStringImpl(const std::string &text) override {
        auto u = fromUtf8(text);
        doc_.insert(cursor_, u);
        cursor_ += u.size();
        publishStaleSurrounding();
    }
    void deleteSurroundingTextImpl(int offset, unsigned int size) override {
        long start = static_cast<long>(cursor_) + offset;
        if (start < 0) {
            start = 0;
        }
        if (static_cast<size_t>(start) > doc_.size()) {
            start = static_cast<long>(doc_.size());
        }
        size_t n = size;
        if (static_cast<size_t>(start) + n > doc_.size()) {
            n = doc_.size() - static_cast<size_t>(start);
        }
        doc_.erase(static_cast<size_t>(start), n);
        cursor_ = static_cast<size_t>(start);
        publishStaleSurrounding();
    }
    void forwardKeyImpl(const ForwardKeyEvent & /*key*/) override {}
    void updatePreeditImpl() override {}

    std::string text() const { return toUtf8(doc_); }

private:
    /// Công bố snapshot TRƯỚC ĐÓ của tài liệu (chậm một nhịp so với thực tế) và bỏ hết dấu cách —
    /// đúng hai tật của LO Writer khi gõ nhanh mà issue #66 mô tả.
    void publishStaleSurrounding() {
        std::u32string stale;
        for (char32_t c : lastSnapshot_) {
            if (c != U' ') {
                stale.push_back(c);
            }
        }
        surroundingText().setText(toUtf8(stale), static_cast<unsigned int>(stale.size()),
                                  static_cast<unsigned int>(stale.size()));
        updateSurroundingText();
        lastSnapshot_ = doc_;
    }

    std::u32string doc_;
    std::u32string lastSnapshot_;
    size_t cursor_ = 0;
};

/// Giả lập ô nhập có autocomplete kiểu thanh địa chỉ (#60): app có thể có sẵn vùng bôi chọn
/// (URL cũ được chọn toàn bộ khi focus); sau một lần commit, autocomplete chèn gợi ý và BÔI
/// CHỌN nó. Mô phỏng cả "vùng chết": deleteSurroundingText khi đang có selection bị áp vào
/// VÙNG CHỌN thay vì ký tự trước con trỏ (hành vi quan sát được ở Chromium). commitString
/// khi đang có selection sẽ THAY THẾ vùng chọn (hành vi app chuẩn).
class AutofillInputContext : public InputContext {
public:
    explicit AutofillInputContext(InputContextManager &mgr)
        : InputContext(mgr, "autofillapp") {
        setCapabilityFlags(CapabilityFlags{CapabilityFlag::SurroundingText});
        created();
    }
    ~AutofillInputContext() override { destroy(); }
    const char *frontend() const override { return "doc"; }

    int deletesWhileSelection() const { return deletesWhileSelection_; }
    std::string text() const { return toUtf8(doc_); }
    void clearDoc() {
        doc_.clear();
        autofill_.clear();
        cursor_ = anchor_ = 0;
        syncSurrounding();
    }
    /// Focus thanh địa chỉ: URL cũ đang được bôi chọn toàn bộ.
    void setAllSelected(const std::string &text) {
        doc_ = fromUtf8(text);
        cursor_ = 0;
        anchor_ = doc_.size();
        autofill_.clear();
        syncSurrounding();
    }
    /// Bật gợi ý autocomplete MỘT LẦN: lần commit kế tiếp sẽ chèn `suffix` và bôi chọn nó.
    void armAutofill(const std::string &suffix) { autofill_ = fromUtf8(suffix); }
    /// App có sẵn chữ phía sau con trỏ (không đụng con trỏ).
    void appendText(const std::string &t) {
        auto u = fromUtf8(t);
        doc_.insert(doc_.size(), u);
        syncSurrounding();
    }
    /// Người dùng bôi chọn [cursor, anchor) (đơn vị: ký tự Unicode).
    void selectRange(size_t cursor, size_t anchor) {
        cursor_ = cursor;
        anchor_ = anchor;
        syncSurrounding();
    }

    void commitStringImpl(const std::string &text) override {
        eraseSelection(); // app chuẩn: commit khi có selection → thay thế vùng chọn
        auto u = fromUtf8(text);
        doc_.insert(cursor_, u);
        cursor_ += u.size();
        anchor_ = cursor_;
        if (!autofill_.empty()) { // autocomplete: chèn gợi ý và bôi chọn (một lần)
            doc_.insert(cursor_, autofill_);
            anchor_ = cursor_ + autofill_.size();
            autofill_.clear();
        }
        syncSurrounding();
    }
    void deleteSurroundingTextImpl(int offset, unsigned int size) override {
        if (cursor_ != anchor_) {
            // Vùng chết #60: app áp lệnh xoá vào vùng chọn, KHÔNG xoá trước con trỏ.
            ++deletesWhileSelection_;
            eraseSelection();
            syncSurrounding();
            return;
        }
        long start = static_cast<long>(cursor_) + offset;
        if (start < 0) {
            start = 0;
        }
        if (static_cast<size_t>(start) > doc_.size()) {
            start = static_cast<long>(doc_.size());
        }
        size_t n = size;
        if (static_cast<size_t>(start) + n > doc_.size()) {
            n = doc_.size() - static_cast<size_t>(start);
        }
        doc_.erase(static_cast<size_t>(start), n);
        cursor_ = anchor_ = static_cast<size_t>(start);
        syncSurrounding();
    }
    void forwardKeyImpl(const ForwardKeyEvent &) override {}
    void updatePreeditImpl() override {}

private:
    void eraseSelection() {
        if (cursor_ == anchor_) {
            return;
        }
        const size_t lo = std::min(cursor_, anchor_);
        const size_t hi = std::max(cursor_, anchor_);
        doc_.erase(lo, hi - lo);
        cursor_ = anchor_ = lo;
    }
    void syncSurrounding() {
        surroundingText().setText(toUtf8(doc_), static_cast<unsigned int>(cursor_),
                                  static_cast<unsigned int>(anchor_));
        updateSurroundingText();
    }
    std::u32string doc_, autofill_;
    size_t cursor_ = 0, anchor_ = 0;
    int deletesWhileSelection_ = 0;
};

/// Bảng chuỗi Telex chạy trên mọi hồ sơ có tài liệu — buffer cuối phải đúng từng byte.
struct TelexCase {
    const char *keys;
    const char *expected;
};
constexpr TelexCase kTelexCases[] = {
    {"tieengs vieetj", "tiếng việt"},
    {"ddaay laf vieejt", "đây là việt"},
    {"chaof banj", "chào bạn"},
    {"ddoongf ys", "đồng ý"},
};

/// Hồ sơ no-st-preedit: app KHÔNG có SurroundingText (terminal thuần) → addon phải đi đường
/// preedit, tuyệt đối không deleteSurroundingText; từ chốt vào tài liệu khi ngắt từ.
class NoStInputContext : public InputContext {
public:
    explicit NoStInputContext(InputContextManager &mgr) : InputContext(mgr, "plainterm") {
        setCapabilityFlags(CapabilityFlags{});
        created();
    }
    ~NoStInputContext() override { destroy(); }
    const char *frontend() const override { return "doc"; }
    int deleteCalls() const { return deleteCalls_; }
    void commitStringImpl(const std::string &text) override { doc_ += text; }
    void deleteSurroundingTextImpl(int, unsigned int) override { ++deleteCalls_; }
    void forwardKeyImpl(const ForwardKeyEvent &) override {}
    void updatePreeditImpl() override {}
    std::string text() const { return doc_; }
    void clearDoc() { doc_.clear(); }

private:
    std::string doc_;
    int deleteCalls_ = 0;
};

void sendKeys(InputContext *ic, const std::string &keys) {
    for (char c : keys) {
        Key key = (c == ' ') ? Key("space") : Key(std::string(1, c));
        KeyEvent ke(ic, key, false);
        ic->keyEvent(ke);
    }
}

template <typename IC>
void expectType(IC *ic, const std::string &keys, const std::string &expected) {
    sendKeys(ic, keys);
    FCITX_ASSERT(ic->text() == expected)
        << "gõ \"" << keys << "\" => \"" << ic->text() << "\", mong đợi \"" << expected << "\"";
}

} // namespace

int main() {
    // Cách ly config khỏi máy chạy test + bật cờ IB_DOUBLE_SPACE_PERIOD cho kịch bản #65.
    // IBflags = IB_STD_FLAGS (1081840) | IB_DOUBLE_SPACE_PERIOD (1<<22 = 4194304) = 5276144;
    // các trường khác giữ mặc định qua serde. Double-space chỉ kích hoạt khi gõ 2 dấu cách
    // liên tiếp nên không ảnh hưởng các kịch bản còn lại.
    const std::string xdgDir =
        std::string(TESTING_BINARY_DIR) + "/testpinakey-nounderline-xdg";
    std::filesystem::remove_all(xdgDir);
    std::filesystem::create_directories(xdgDir + "/pinakey");
    {
        std::ofstream cfg(xdgDir + "/pinakey/ibus-PinaKey.config.json");
        cfg << "{\"IBflags\":5276144}";
    }
    setenv("XDG_CONFIG_HOME", xdgDir.c_str(), 1);

    setupTestingEnvironment(TESTING_BINARY_DIR,
                            {PINAKEY_ADDON_SO_DIR},
                            {PINAKEY_TEST_DATA_DIR, FCITX_SYS_PKGDATADIR});

    char arg0[] = "testpinakey_nounderline";
    char arg1[] = "--disable=all";
    char arg2[] = "--enable=testim,testfrontend,testui,keyboard,pinakey";
    char *argv[] = {arg0, arg1, arg2};

    Log::setLogRule("default=3");
    Instance instance(FCITX_ARRAY_SIZE(argv), argv);
    instance.addonManager().registerDefaultLoader(nullptr);

    EventDispatcher dispatcher;
    dispatcher.attach(&instance.eventLoop());
    dispatcher.schedule([&instance, &xdgDir]() {
        auto *pinakey = instance.addonManager().addon("pinakey", true);
        FCITX_ASSERT(pinakey);

        auto group = instance.inputMethodManager().currentGroup();
        group.inputMethodList().clear();
        group.inputMethodList().push_back(InputMethodGroupItem("keyboard-us"));
        group.inputMethodList().push_back(InputMethodGroupItem("pinakey"));
        group.setDefaultInputMethod("");
        instance.inputMethodManager().setGroup(std::move(group));

        auto ic = std::make_unique<DocInputContext>(instance.inputContextManager());
        ic->focusIn();
        instance.setCurrentInputMethod(ic.get(), "pinakey", true);
        FCITX_ASSERT(ic->capabilityFlags().test(CapabilityFlag::SurroundingText));

        // Gõ không gạch chân: tài liệu nhận thẳng văn bản tiếng Việt, KHÔNG qua preedit.
        expectType(ic.get(), "tieengs vieetj", "tiếng việt");
        ic->reset();
        ic->clearDoc();
        expectType(ic.get(), "ddaay laf vieejt", "đây là việt");

        // #7: con trỏ nhảy giữa chừng (người dùng click chuột) rồi gõ tiếp KHÔNG được xoá nhầm
        // ký tự ở vị trí mới. Gõ "vie" (doc="vie"), click về đầu, gõ "j" (nặng): vì segment "vie"
        // đã ở chỗ khác con trỏ, engine phải reset và xử lý "j" như phím mới tại đầu — không
        // deleteSurroundingText xoá nhầm "v". Mong đợi "vie" còn nguyên (không thành "ẹie").
        ic->reset();
        ic->clearDoc();
        sendKeys(ic.get(), "vie");
        FCITX_ASSERT(ic->text() == "vie") << "trước khi nhảy con trỏ: " << ic->text();
        ic->clickAt(0); // người dùng click về đầu ô
        sendKeys(ic.get(), "j");
        FCITX_ASSERT(ic->text() == "jvie")
            << "con trỏ nhảy rồi gõ tiếp bị xoá nhầm ký tự: doc=\"" << ic->text()
            << "\", mong đợi \"jvie\" (giữ nguyên \"vie\")";

        // #65: double-space → ". " (bật qua config ở đầu main): từ + 2 dấu cách liên tiếp →
        // dấu cách cũ bị xoá (deleteSurroundingText) và ". " được commit; gõ tiếp bình thường.
        ic->reset();
        ic->clearDoc();
        sendKeys(ic.get(), "tieengs  ");
        FCITX_ASSERT(ic->text() == "tiếng. ")
            << "double-space: gõ \"tieengs␣␣\" => \"" << ic->text()
            << "\", mong đợi \"tiếng. \"";
        sendKeys(ic.get(), "hai ");
        FCITX_ASSERT(ic->text() == "tiếng. hai ")
            << "sau double-space gõ tiếp: doc=\"" << ic->text() << "\"";

        // #65 an toàn: click chuột dời con trỏ khi cửa sổ double-space đang mở (app không gửi
        // reset) rồi bấm space → KHÔNG được xoá ký tự ở vị trí mới / chèn ". ". Văn bản trước
        // con trỏ mới không kết thúc bằng "từ + dấu cách" → addon phải bỏ qua.
        ic->reset();
        ic->clearDoc();
        sendKeys(ic.get(), "tieengs "); // doc="tiếng ", cửa sổ double-space mở
        ic->clickAt(0);                 // người dùng click về đầu ô (không reset engine)
        sendKeys(ic.get(), " ");        // space tại vị trí mới
        FCITX_ASSERT(ic->text() == "tiếng ")
            << "double-space sau khi click chuột phá văn bản: doc=\"" << ic->text()
            << "\", mong đợi \"tiếng \" (nguyên vẹn)";

        // #67: terminal (kitty) CÓ quảng cáo SurroundingText nhưng rule built-in ép preedit →
        // addon không bao giờ gọi deleteSurroundingText, chữ vẫn đúng (qua đường commit preedit).
        auto kitty =
            std::make_unique<DocInputContext>(instance.inputContextManager(), "kitty");
        kitty->focusIn();
        instance.setCurrentInputMethod(kitty.get(), "pinakey", true);
        sendKeys(kitty.get(), "vieetj ");
        FCITX_ASSERT(kitty->text() == "việt ")
            << "kitty: doc=\"" << kitty->text() << "\", mong đợi \"việt \"";
        FCITX_ASSERT(kitty->deleteCalls() == 0)
            << "kitty phải đi đường preedit, không được deleteSurroundingText ("
            << kitty->deleteCalls() << " lần)";

        // #67: rule NGƯỜI DÙNG thắng built-in — ~/.config/pinakey/transport-rules.conf ép
        // "preedit" cho app thường (mặc định Auto→replace). Engine đọc rule lúc tạo context
        // nên phải ghi file TRƯỚC khi tạo context mới.
        {
            std::ofstream rules(xdgDir + "/pinakey/transport-rules.conf");
            rules << "# rule người dùng\npreedit userpreeditapp\n";
        }
        auto up = std::make_unique<DocInputContext>(instance.inputContextManager(),
                                                    "userpreeditapp");
        up->focusIn();
        instance.setCurrentInputMethod(up.get(), "pinakey", true);
        sendKeys(up.get(), "vieetj ");
        FCITX_ASSERT(up->text() == "việt ")
            << "override người dùng: doc=\"" << up->text() << "\"";
        FCITX_ASSERT(up->deleteCalls() == 0)
            << "rule người dùng 'preedit' phải thắng Auto (deleteCalls="
            << up->deleteCalls() << ")";

        // #66: LibreOffice Writer — app CÓ SurroundingText nhưng báo cáo không đáng tin (lạc hậu
        // khi gõ nhanh, thiếu dấu cách). Addon phải nhận diện program "soffice" → dùng preedit
        // thay vì diff-replace, nên gõ nhanh đoạn dài vẫn ra đúng chữ.
        auto lo = std::make_unique<LibreOfficeLikeInputContext>(instance.inputContextManager());
        lo->focusIn();
        instance.setCurrentInputMethod(lo.get(), "pinakey", true);
        sendKeys(lo.get(), "vieetj tieengs ");
        FCITX_ASSERT(lo->text() == "việt tiếng ")
            << "LibreOffice giả lập: gõ \"vieetj tieengs \" => \"" << lo->text()
            << "\", mong đợi \"việt tiếng \" (phải rơi về preedit, không diff-replace)";

        // ============== Ma trận hồ sơ × chuỗi gõ (#70) ==============
        // reliable-st: editor lành (ST, cursor==anchor) — commit thẳng, đúng từng byte.
        ic->focusIn(); // focus đang ở hồ sơ LibreOffice phía trên
        for (const auto &c : kTelexCases) {
            ic->reset();
            ic->clearDoc();
            expectType(ic.get(), c.keys, c.expected);
        }

        // no-st-preedit: không SurroundingText → preedit; thêm dấu cách cuối để chốt từ chót.
        auto plain = std::make_unique<NoStInputContext>(instance.inputContextManager());
        plain->focusIn();
        instance.setCurrentInputMethod(plain.get(), "pinakey", true);
        for (const auto &c : kTelexCases) {
            plain->reset();
            plain->clearDoc();
            expectType(plain.get(), std::string(c.keys) + " ",
                       std::string(c.expected) + " ");
        }
        // Từ đang soạn dở (chưa có phím ngắt từ) phải còn nguyên trong preedit — tài liệu
        // chưa được nhận gì (góp ý review: khoá cả trạng thái "trước ranh giới từ").
        plain->reset();
        plain->clearDoc();
        sendKeys(plain.get(), "tieengs");
        FCITX_ASSERT(plain->text().empty())
            << "no-st-preedit commit sớm từ đang soạn dở: doc=\"" << plain->text() << "\"";
        plain->reset();
        FCITX_ASSERT(plain->deleteCalls() == 0)
            << "no-st-preedit không bao giờ được deleteSurroundingText ("
            << plain->deleteCalls() << " lần)";

        // ============== B1 (#60): selection guard ==============
        auto af = std::make_unique<AutofillInputContext>(instance.inputContextManager());
        af->focusIn();
        instance.setCurrentInputMethod(af.get(), "pinakey", true);

        // (a) Focus thanh địa chỉ: URL cũ bôi chọn toàn bộ → cả từ soạn trong preedit, chuỗi
        // mới THAY THẾ vùng chọn khi chốt. (Bảo vệ hồi quy — đường đúng cho case phổ biến.)
        af->setAllSelected("example.com");
        sendKeys(af.get(), "dd ");
        FCITX_ASSERT(af->text() == "đ ")
            << "omnibox focus-selected: doc=\"" << af->text() << "\", mong đợi \"đ \"";
        FCITX_ASSERT(af->deletesWhileSelection() == 0)
            << "deleteSurroundingText trong lúc có selection: " << af->deletesWhileSelection();

        // (b) Timing khó: ô trống, ký tự đầu commit xong thì autocomplete chèn gợi ý + bôi chọn.
        // Phím sau KHÔNG được diff-replace (vùng chết) — kết quả an toàn xác định là "dd "
        // (đúng phím đã gõ, không rối thành "dđ "), tuyệt đối không xoá lên selection.
        af->reset();
        af->clearDoc();
        af->armAutofill("uckduckgo.com");
        sendKeys(af.get(), "dd ");
        FCITX_ASSERT(af->text() == "dd ")
            << "omnibox autocomplete giữa từ: doc=\"" << af->text()
            << "\", mong đợi \"dd \" (an toàn, không \"dđ \")";
        FCITX_ASSERT(af->deletesWhileSelection() == 0)
            << "deleteSurroundingText trong lúc có selection: " << af->deletesWhileSelection();

        // (c) Người dùng bôi chọn rồi gõ: gõ "vie", app có sẵn chữ sau con trỏ, bôi chọn phần
        // đó rồi gõ "j " → "j " thay thế vùng chọn (hành vi app chuẩn), không xoá nhầm.
        af->reset();
        af->clearDoc();
        sendKeys(af.get(), "vie");
        af->appendText("fix");
        af->selectRange(3, 6); // bôi chọn "fix", con trỏ vẫn sau "vie"
        sendKeys(af.get(), "j ");
        FCITX_ASSERT(af->text() == "viej ")
            << "bôi chọn rồi gõ: doc=\"" << af->text() << "\", mong đợi \"viej \"";
        FCITX_ASSERT(af->deletesWhileSelection() == 0)
            << "deleteSurroundingText trong lúc có selection: " << af->deletesWhileSelection();

        // ============== modifier-noise ==============
        // Phím có Ctrl/Alt/Super và modifier đơn không sinh lệnh xoá, không phá segment —
        // trước guard ở engine, Ctrl+A commit ngang buffer và Alt+Tab lọt vào nhánh Tab.
        ic->focusIn();
        ic->reset();
        ic->clearDoc();
        sendKeys(ic.get(), "vie");
        FCITX_ASSERT(ic->text() == "vie");
        const int delsBefore = ic->deleteCalls();
        for (const char *k :
             {"Control+A", "Control+C", "Alt+Tab", "Shift_L", "Control_L", "Super_L"}) {
            KeyEvent ke(ic.get(), Key(k), false);
            ic->keyEvent(ke);
        }
        FCITX_ASSERT(ic->text() == "vie")
            << "phím modifier làm đổi tài liệu: doc=\"" << ic->text() << "\"";
        FCITX_ASSERT(ic->deleteCalls() == delsBefore)
            << "phím modifier sinh deleteSurroundingText";
        // Segment vẫn sống: gõ tiếp biến "vie" thành "việt " đúng chỗ.
        expectType(ic.get(), "ejt ", "việt ");

        instance.exit();
    });

    return instance.exec();
}
