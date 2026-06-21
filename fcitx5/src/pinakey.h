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

#include <fcitx/addonfactory.h>
#include <fcitx/addoninstance.h>
#include <fcitx/addonmanager.h>
#include <fcitx/inputcontext.h>
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputcontextproperty.h>
#include <fcitx/inputmethodengine.h>
#include <fcitx/instance.h>

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

    void keyEvent(KeyEvent &keyEvent);
    void reset();
    void deactivate();
    PkEngine *core() { return core_; }

private:
    void applyResult();
    void applyReplaceResult();
    bool wantReplaceMode() const;
    bool shouldPassThrough() const;

    PinaKeyEngine *engine_;
    InputContext *ic_;
    PkEngine *core_;
};

/// Engine fcitx5 (một thực thể addon). Đăng ký factory tạo `PinaKeyState` cho mỗi input context.
class PinaKeyEngine : public InputMethodEngineV2 {
public:
    explicit PinaKeyEngine(Instance *instance);

    void keyEvent(const InputMethodEntry &entry, KeyEvent &keyEvent) override;
    void reset(const InputMethodEntry &entry, InputContextEvent &event) override;
    void activate(const InputMethodEntry &entry, InputContextEvent &event) override;
    void deactivate(const InputMethodEntry &entry, InputContextEvent &event) override;

    Instance *instance() { return instance_; }
    auto *factory() { return &factory_; }
    PinaKeyState *state(InputContext *ic) { return ic->propertyFor(&factory_); }

private:
    Instance *instance_;
    FactoryFor<PinaKeyState> factory_;
};

class PinaKeyEngineFactory : public AddonFactory {
public:
    AddonInstance *create(AddonManager *manager) override {
        return new PinaKeyEngine(manager->instance());
    }
};

} // namespace fcitx

#endif // _PINAKEY_FCITX5_PINAKEY_H_
