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
    explicit DocInputContext(InputContextManager &mgr)
        : InputContext(mgr, "docapp") {
        setCapabilityFlags(CapabilityFlags{CapabilityFlag::SurroundingText});
        created();
    }
    ~DocInputContext() override { destroy(); }

    const char *frontend() const override { return "doc"; }

    void commitStringImpl(const std::string &text) override {
        auto u = fromUtf8(text);
        doc_.insert(cursor_, u);
        cursor_ += u.size();
        syncSurrounding();
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

void sendKeys(InputContext *ic, const std::string &keys) {
    for (char c : keys) {
        Key key = (c == ' ') ? Key("space") : Key(std::string(1, c));
        KeyEvent ke(ic, key, false);
        ic->keyEvent(ke);
    }
}

void expectType(DocInputContext *ic, const std::string &keys,
                const std::string &expected) {
    sendKeys(ic, keys);
    FCITX_ASSERT(ic->text() == expected)
        << "gõ \"" << keys << "\" => \"" << ic->text() << "\", mong đợi \"" << expected << "\"";
}

} // namespace

int main() {
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
    dispatcher.schedule([&instance]() {
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

        instance.exit();
    });

    return instance.exec();
}
