/*
 * Addon fcitx5 cho PinaKey — vỏ C++ mỏng bọc lõi engine Rust (qua C-ABI `pinakey_ffi.h`).
 *
 * Mô hình giống fcitx5-cskk: mỗi InputContext giữ một `PkEngine*` (con trỏ mờ tới lõi Rust);
 * lớp C++ chỉ đọc phím từ fcitx5, bơm vào lõi, rồi áp kết quả (commit/preedit) trở lại fcitx5.
 * Toàn bộ logic tiếng Việt nằm ở Rust — đây chỉ là lớp tích hợp.
 *
 * GPL-3.0-or-later.
 */
#ifndef _PINAKEY_FCITX5_PINAKEY_H_
#define _PINAKEY_FCITX5_PINAKEY_H_

#include <fcitx/action.h>
#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/instance.h>
#include <fcitx/menu.h>

#include <fcitx-utils/event.h>

#include <chrono>
#include <cstdint>
#include <memory>
#include <string>
#include <utility>
#include <vector>

extern "C" {
#include <pinakey_ffi.h>
}

namespace fcitx {

class PinaKeyEngine;

/// Trạng thái bộ gõ gắn với MỘT input context. Sở hữu một `PkEngine*` của lõi Rust.
class PinaKeyState : public InputContextProperty {
public:
    PinaKeyState(PinaKeyEngine *engine, InputContext *ic);
    ~PinaKeyState() override;

    // Sở hữu raw `core_` (PkEngine*) và free trong destructor → cấm copy/move để không bao giờ
    // double-free do sao chép nhầm (rule of five đầy đủ cho lớp owning raw pointer).
    PinaKeyState(const PinaKeyState &) = delete;
    PinaKeyState &operator=(const PinaKeyState &) = delete;
    PinaKeyState(PinaKeyState &&) = delete;
    PinaKeyState &operator=(PinaKeyState &&) = delete;

    void keyEvent(KeyEvent &keyEvent);
    void reset();
    /// `imSwitch` = deactivate do người dùng ĐỔI input method (Ctrl+Space) — fcitx5 không tự
    /// commit client preedit trong trường hợp này, khác với mất focus (có tự commit).
    void deactivate(bool imSwitch);
    PkEngine *core() { return core_; }
    InputContext *ic() { return ic_; }
    /// Chọn emoji thứ `index` trong danh sách hiện tại rồi thoát chế độ emoji (gọi từ CandidateWord).
    void emojiSelect(int index);

private:
    void applyResult();
    void applyReplaceResult();
    // #7: reset segment đang theo dõi nếu con trỏ đã nhảy / văn bản đổi (so với surrounding text),
    // tránh deleteSurroundingText xoá nhầm ký tự ở vị trí mới.
    void resetIfDocumentDiverged();
    // Gõ không gạch chân cho app không có SurroundingText (terminal…) qua daemon uinput + ACK:
    // hoãn commit, bơm (N+1) Backspace, ĐẾM Backspace bơm-ngược quay về fcitx, chỉ commit chuỗi
    // mới sau khi xác nhận đã xoá đủ — triệt tiêu cuộc đua "commit trước khi xoá xong" (như
    // fcitx5-lotus). Thay cho đường commit-ngay cũ (#28) vốn racy.
    void startUinputReplace();             // performReplacement: bơm Backspace, hoãn commit
    void handleUinputAck(KeyEvent &keyEvent); // xử lý Backspace bơm-ngược; commit khi đủ
    void replayBufferedKeys();             // gõ nhanh khi đang xoá → replay sau khi ACK xong
    bool wantReplaceMode() const; // có dùng diff-and-replace (SurroundingText hoặc uinput) không
    bool useUinput() const;       // không có SurroundingText nhưng có server uinput
    bool shouldPassThrough() const;

    // ----- tra cứu emoji / hex (issue #11/#26) -----
    bool handleEmojiKey(KeyEvent &keyEvent); // trả true nếu đã "nuốt" phím
    void startEmoji();
    void cancelEmoji(bool commitLiteral);
    void updateEmojiUI();
    std::vector<std::string> emojiCandidates_; // danh sách emoji hiện khớp

    PinaKeyEngine *engine_;
    InputContext *ic_;
    PkEngine *core_;
    bool emojiMode_ = false;
    std::string emojiQuery_; // gồm cả dấu ':' đầu, ví dụ ":grin"

    // ----- trạng thái ACK cho chế độ uinput (xoá-bằng-Backspace, app không có SurroundingText) -----
    bool deleting_ = false;             // đang trong chuỗi xoá tự động (chờ Backspace bơm-ngược)
    int expectedBackspaces_ = 0;        // tổng Backspace đã bơm (N ký tự xoá + 1 phím trigger)
    int currentBackspaceCount_ = 0;     // số Backspace bơm-ngược đã thấy quay về
    std::string pendingCommit_;         // chuỗi mới, commit sau khi xoá xong
    std::vector<std::pair<uint32_t, uint32_t>> bufferedKeys_; // (sym,state) gõ trong lúc đang xoá
    std::chrono::steady_clock::time_point deletingSince_;     // mốc bắt đầu xoá (lưới an toàn timeout)
};

/// Engine fcitx5 (một thực thể addon). Đăng ký factory tạo `PinaKeyState` cho mỗi input context.
class PinaKeyEngine : public InputMethodEngineV2 {
public:
    explicit PinaKeyEngine(Instance *instance);

    void keyEvent(const InputMethodEntry &entry, KeyEvent &keyEvent) override;
    void reset(const InputMethodEntry &entry, InputContextEvent &event) override;
    void activate(const InputMethodEntry &entry, InputContextEvent &event) override;
    void deactivate(const InputMethodEntry &entry, InputContextEvent &event) override;
    // Khi PinaKey đang được chọn, hiển thị nhãn "V" (chỉ báo đang gõ tiếng Việt) thay cho icon.
    std::string subModeLabelImpl(const InputMethodEntry &entry, InputContext &ic) override;
    std::string subModeIconImpl(const InputMethodEntry &entry, InputContext &ic) override;

    Instance *instance() { return instance_; }
    auto *factory() { return &factory_; }
    PinaKeyState *state(InputContext *ic) { return ic->propertyFor(&factory_); }

    /// Đổi kiểu gõ / bảng mã cho MỌI input context đang sống + cập nhật dấu chọn trong menu
    /// (issue #12/#17). Áp dụng cho phiên hiện tại; lưu bền vững do GUI thiết lập đảm nhiệm.
    void applyInputMethod(const std::string &name);
    void applyCharset(const std::string &name);

private:
    void setupStatusMenu();
    void addStatusActions(InputContext *ic);
    void setupReloadTimer(); // #20 live-reload macro/dict
    void checkReload();

    Instance *instance_;
    FactoryFor<PinaKeyState> factory_;

    // #20: theo dõi file macro/dict để nạp lại khi sửa, không cần khởi động lại.
    std::unique_ptr<EventSourceTime> reloadTimer_;
    std::vector<std::string> reloadFiles_;
    std::vector<uint64_t> reloadMtimes_;

    // Menu khu vực trạng thái: chọn kiểu gõ + bảng mã (issue #12/#17).
    std::unique_ptr<SimpleAction> imRootAction_;
    std::unique_ptr<SimpleAction> charsetRootAction_;
    std::unique_ptr<Menu> imMenu_;
    std::unique_ptr<Menu> charsetMenu_;
    std::vector<std::unique_ptr<SimpleAction>> imItems_;
    std::vector<std::unique_ptr<SimpleAction>> charsetItems_;
    std::vector<std::string> imNames_;
    std::vector<std::string> charsetNames_;
};

class PinaKeyEngineFactory : public AddonFactory {
public:
    AddonInstance *create(AddonManager *manager) override {
        return new PinaKeyEngine(manager->instance());
    }
};

} // namespace fcitx

#endif // _PINAKEY_FCITX5_PINAKEY_H_
