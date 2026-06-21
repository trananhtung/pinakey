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
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputmethodgroup.h>
#include <fcitx/inputmethodmanager.h>
#include <fcitx/instance.h>

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

        instance.exit();
    });

    return instance.exec();
}
