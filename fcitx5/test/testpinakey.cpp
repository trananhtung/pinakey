/*
 * Test tích hợp addon fcitx5 PinaKey, dùng testfrontend của fcitx5 (cùng cơ chế bộ test của
 * fcitx5): dựng một Instance riêng, bật addon `pinakey`, tạo input context, bơm phím và kiểm tra
 * chuỗi commit. KHÔNG đụng tới fcitx5 đang chạy của người dùng.
 *
 * GPL-3.0-or-later.
 */
#include "testfrontend_public.h"

#include <fcitx-utils/eventdispatcher.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/testing.h>
#include <fcitx/addonmanager.h>
#include <fcitx/candidatelist.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputpanel.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputmethodgroup.h>
#include <fcitx/inputmethodmanager.h>
#include <fcitx/instance.h>

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <string>

using namespace fcitx;

namespace {

/// Gõ lần lượt các ký tự ASCII (một keysym mỗi ký tự) vào input context `uuid`.
void typeAscii(AddonInstance *testfrontend, ICUUID uuid, const std::string &keys) {
    for (char c : keys) {
        testfrontend->call<ITestFrontend::keyEvent>(uuid, Key(std::string(1, c)), false);
    }
}

void sendKey(AddonInstance *testfrontend, ICUUID uuid, const char *keyName) {
    testfrontend->call<ITestFrontend::keyEvent>(uuid, Key(keyName), false);
}

} // namespace

int main() {
    // Cách ly config/persist khỏi máy người chạy test: XDG_CONFIG_HOME trỏ vào build dir và được
    // dọn sạch mỗi lần chạy — engine dùng config mặc định, lịch sử emoji (#63) bắt đầu rỗng.
    const std::string xdgDir = std::string(TESTING_BINARY_DIR) + "/testpinakey-xdg";
    std::filesystem::remove_all(xdgDir);
    setenv("XDG_CONFIG_HOME", xdgDir.c_str(), 1);

    setupTestingEnvironment(TESTING_BINARY_DIR,
                            {PINAKEY_ADDON_SO_DIR},
                            {PINAKEY_TEST_DATA_DIR, FCITX_SYS_PKGDATADIR});

    char arg0[] = "testpinakey";
    char arg1[] = "--disable=all";
    char arg2[] = "--enable=testim,testfrontend,testui,keyboard,pinakey";
    char *argv[] = {arg0, arg1, arg2};

    Log::setLogRule("default=3");
    Instance instance(FCITX_ARRAY_SIZE(argv), argv);
    instance.addonManager().registerDefaultLoader(nullptr);

    EventDispatcher dispatcher;
    dispatcher.attach(&instance.eventLoop());
    dispatcher.schedule([&instance]() {
        // 1) Addon nạp được chưa?
        auto *pinakey = instance.addonManager().addon("pinakey", true);
        FCITX_ASSERT(pinakey) << "addon pinakey không nạp được";

        // 2) Nhóm input method: keyboard-us + pinakey.
        auto group = instance.inputMethodManager().currentGroup();
        group.inputMethodList().clear();
        group.inputMethodList().push_back(InputMethodGroupItem("keyboard-us"));
        group.inputMethodList().push_back(InputMethodGroupItem("pinakey"));
        group.setDefaultInputMethod("");
        instance.inputMethodManager().setGroup(std::move(group));

        // 3) Tạo input context + chọn PinaKey cho nó.
        auto *testfrontend = instance.addonManager().addon("testfrontend");
        FCITX_ASSERT(testfrontend);
        auto uuid = testfrontend->call<ITestFrontend::createInputContext>("testapp");
        auto *ic = instance.inputContextManager().findByUUID(uuid);
        FCITX_ASSERT(ic);
        ic->focusIn();
        instance.setCurrentInputMethod(ic, "pinakey", true);

        // 4) Telex: "vieetj" -> preedit "việt"; SPACE commit "việt ".
        testfrontend->call<ITestFrontend::pushCommitExpectation>("việt ");
        typeAscii(testfrontend, uuid, "vieetj");
        sendKey(testfrontend, uuid, "space");

        // 5) Câu nhiều từ: "tieengs" -> "tiếng", SPACE commit; rồi "vieet" + "j" -> "việt", SPACE.
        testfrontend->call<ITestFrontend::pushCommitExpectation>("tiếng ");
        typeAscii(testfrontend, uuid, "tieengs");
        sendKey(testfrontend, uuid, "space");

        // 6) Fallback không phải tiếng Việt: "loz" giữ nguyên, SPACE commit "loz ".
        testfrontend->call<ITestFrontend::pushCommitExpectation>("loz ");
        typeAscii(testfrontend, uuid, "loz");
        sendKey(testfrontend, uuid, "space");

        // 7) Dấu chấm câu sau từ: "anh" + "." -> commit "anh" rồi "." (tùy chuẩn), kiểm tra
        //    đơn giản: "as" -> "á", ENTER là word-break-ish? dùng SPACE cho chắc.
        testfrontend->call<ITestFrontend::pushCommitExpectation>("á ");
        typeAscii(testfrontend, uuid, "as");
        sendKey(testfrontend, uuid, "space");

        // 8) Tra emoji bằng hex (#11/#26): ":u1f600" + Enter -> 😀 (U+1F600).
        testfrontend->call<ITestFrontend::pushCommitExpectation>("\xF0\x9F\x98\x80"); // 😀
        sendKey(testfrontend, uuid, "colon");
        typeAscii(testfrontend, uuid, "u1f600");
        sendKey(testfrontend, uuid, "Return");

        // 9) Trong chế độ emoji, tổ hợp Ctrl+C phải THOÁT emoji (commit literal ":") và KHÔNG
        //    nuốt 'c' làm query (bug fix: phím modifier đi tiếp).
        testfrontend->call<ITestFrontend::pushCommitExpectation>(":");
        sendKey(testfrontend, uuid, "colon");
        testfrontend->call<ITestFrontend::keyEvent>(uuid, Key("Control+c"), false);

        // 10) ':' (mở emoji) + SPACE khi không có ứng viên → chốt literal ":" và KHÔNG nuốt dấu
        //     cách (bug fix: Space được forward, không bị mất).
        testfrontend->call<ITestFrontend::pushCommitExpectation>(":");
        sendKey(testfrontend, uuid, "colon");
        bool spaceHandled =
            testfrontend->call<ITestFrontend::sendKeyEvent>(uuid, Key("space"), false);
        FCITX_ASSERT(!spaceHandled) << "dấu cách sau ':' phải được forward (không bị nuốt)";

        // 11) ":u<hex>" trỏ vào surrogate (U+D800–DFFF) → KHÔNG được sinh ứng viên (mã hoá ra
        //     UTF-8 không hợp lệ); Enter khi không có ứng viên chốt literal ":ud800".
        testfrontend->call<ITestFrontend::pushCommitExpectation>(":ud800");
        sendKey(testfrontend, uuid, "colon");
        typeAscii(testfrontend, uuid, "ud800");
        sendKey(testfrontend, uuid, "Return");

        // 12) Trạng thái emoji phải bị dọn khi mất focus: ':' rồi focusOut → deactivate chốt
        //     literal ":" (không mất chữ đã gõ); sau khi focusIn lại, phím gõ tiếp phải được xử
        //     lý tiếng Việt bình thường chứ không bị nuốt vào query emoji vô hình cũ.
        testfrontend->call<ITestFrontend::pushCommitExpectation>(":");
        sendKey(testfrontend, uuid, "colon");
        ic->focusOut();
        ic->focusIn();
        testfrontend->call<ITestFrontend::pushCommitExpectation>("á ");
        typeAscii(testfrontend, uuid, "as");
        sendKey(testfrontend, uuid, "space");

        // 13) #63 lịch sử gần dùng: 😀 đã được commit ở bước 8 → mở ':' với query rỗng phải hiện
        //     lịch sử làm candidate; phím '1' chọn emoji gần nhất. (Space/Enter với query rỗng
        //     vẫn chốt literal ':' như bước 10 — lịch sử chỉ chọn bằng phím số / click.)
        testfrontend->call<ITestFrontend::pushCommitExpectation>("\xF0\x9F\x98\x80"); // 😀
        sendKey(testfrontend, uuid, "colon");
        sendKey(testfrontend, uuid, "1");

        // 14) #63 fuzzy shortname: ":heart_eyes" + Enter → 😍 (trước đây shortname không được
        //     index nên không match).
        testfrontend->call<ITestFrontend::pushCommitExpectation>("\xF0\x9F\x98\x8D"); // 😍
        sendKey(testfrontend, uuid, "colon");
        typeAscii(testfrontend, uuid, "heart_eyes");
        sendKey(testfrontend, uuid, "Return");

        // 15) #63 thứ tự lịch sử: giờ là [😍, 😀] (mới nhất trước) → ':' + phím '2' chọn 😀.
        testfrontend->call<ITestFrontend::pushCommitExpectation>("\xF0\x9F\x98\x80"); // 😀
        sendKey(testfrontend, uuid, "colon");
        sendKey(testfrontend, uuid, "2");

        // 15b) #97 chọn bằng phím số phải theo TRANG HIỆN TẠI: query nhiều kết quả (>9),
        //      lật sang trang 2 (như bấm nút mũi tên trên panel bằng chuột) rồi bấm '1' —
        //      phải commit đúng emoji đang mang nhãn "1." trên màn hình, không phải mục 1
        //      của trang đầu.
        sendKey(testfrontend, uuid, "colon");
        typeAscii(testfrontend, uuid, "face");
        {
            auto cl = ic->inputPanel().candidateList();
            FCITX_ASSERT(cl && cl->toPageable() && cl->toPageable()->hasNext())
                << "query ':face' phải có nhiều hơn 1 trang candidate";
            cl->toPageable()->next();
            const std::string label = cl->candidate(0).text().toString(); // "1. <emoji>"
            FCITX_ASSERT(label.size() > 3 && label[0] == '1');
            testfrontend->call<ITestFrontend::pushCommitExpectation>(label.substr(3));
        }
        sendKey(testfrontend, uuid, "1");

        // 16) #69 áp config tức thì: ghi config VNI rồi gọi reloadConfig() (đúng đường mà
        //     D-Bus ReloadAddonConfig của fcitx5 gọi vào) → gõ VNI ăn ngay trên input context
        //     ĐANG MỞ, không cần khởi động lại. Đặt cuối cùng vì các bước trước dùng Telex.
        {
            const std::string cfgDir = std::string(std::getenv("XDG_CONFIG_HOME")) + "/pinakey";
            std::filesystem::create_directories(cfgDir);
            std::ofstream cfg(cfgDir + "/ibus-PinaKey.config.json");
            cfg << "{\"InputMethod\":\"VNI\"}";
        }
        pinakey->reloadConfig();
        testfrontend->call<ITestFrontend::pushCommitExpectation>("á ");
        typeAscii(testfrontend, uuid, "a1"); // VNI: 1 = dấu sắc
        sendKey(testfrontend, uuid, "space");

        instance.exit();
    });

    return instance.exec();
}
