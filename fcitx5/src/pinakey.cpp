/*
 * Addon fcitx5 cho PinaKey — hiện thực. Xem pinakey.h để biết kiến trúc.
 * GPL-3.0-or-later.
 */
#include "pinakey.h"

#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/textformatflags.h>
#include <fcitx/event.h>
#include <fcitx/inputpanel.h>
#include <fcitx/text.h>

#include <cstdint>
#include <string>

namespace fcitx {

namespace {
/// Bit "phím nhả" mà C-ABI quy ước (1<<30). fcitx5 không dùng bit này trong `states()`
/// (Virtual=1<<29, Repeat=1<<31), nên đặt riêng cho release là an toàn.
constexpr uint32_t kPkModRelease = 1u << 30;

/// Tên engine để lõi Rust nạp cấu hình `~/.config/pinakey/ibus-PinaKey.config.json` — dùng chung
/// file cấu hình với frontend IBus trong giai đoạn chạy song song.
constexpr char kConfigName[] = "PinaKey";
} // namespace

// ----------------------------------- PinaKeyState -----------------------------------

PinaKeyState::PinaKeyState(PinaKeyEngine *engine, InputContext *ic)
    : engine_(engine), ic_(ic), core_(pk_engine_new_from_name(kConfigName)) {
    // Tên chương trình đang focus → bật các cách khắc phục theo ứng dụng ở lõi.
    pk_engine_set_program(core_, ic->program().c_str());
}

PinaKeyState::~PinaKeyState() { pk_engine_free(core_); }

void PinaKeyState::reset() {
    pk_engine_reset(core_);
    ic_->inputPanel().reset();
    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
}

void PinaKeyState::keyEvent(KeyEvent &keyEvent) {
    // Phím nhả: lõi không xử lý; để đi tiếp.
    if (keyEvent.isRelease()) {
        return;
    }

    const uint32_t sym = static_cast<uint32_t>(keyEvent.rawKey().sym());
    const uint32_t state = static_cast<uint32_t>(keyEvent.rawKey().states());

    if (wantReplaceMode()) {
        // Gõ không gạch chân: commit thẳng + xoá-chèn qua surrounding text.
        const bool handled = pk_engine_process_key_replace(core_, sym, state);
        applyReplaceResult();
        if (handled) {
            keyEvent.filterAndAccept();
        }
        return;
    }

    const bool handled = pk_engine_process_key(core_, sym, state);
    applyResult();
    if (handled) {
        keyEvent.filterAndAccept();
    }
}

/// Dùng chế độ "gõ không gạch chân" khi người dùng bật cờ và ứng dụng hỗ trợ SurroundingText
/// (để có thể xoá ký tự đã chèn). Nếu không, lùi về chế độ preedit cổ điển.
bool PinaKeyState::wantReplaceMode() const {
    return pk_engine_no_underline(core_) &&
           ic_->capabilityFlags().test(CapabilityFlag::SurroundingText);
}

/// Áp lệnh thay thế: xoá N ký tự trước con trỏ rồi commit chuỗi mới. Không hiện preedit.
void PinaKeyState::applyReplaceResult() {
    const uint32_t del = pk_engine_replace_delete(core_);
    if (del > 0) {
        ic_->deleteSurroundingText(-static_cast<int>(del), del);
    }
    if (const char *ins = pk_engine_replace_insert(core_); ins && ins[0] != '\0') {
        ic_->commitString(ins);
    }
    // Bảo đảm không còn preedit sót lại khi chuyển từ chế độ preedit sang replace.
    auto &panel = ic_->inputPanel();
    if (!panel.empty()) {
        panel.reset();
        ic_->updatePreedit();
        ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
    }
}

/// Áp kết quả của lần process_key gần nhất: commit chuỗi (nếu có) rồi cập nhật preedit.
void PinaKeyState::applyResult() {
    if (const char *commit = pk_engine_commit(core_); commit && commit[0] != '\0') {
        ic_->commitString(commit);
    }

    auto &panel = ic_->inputPanel();
    panel.reset();

    if (pk_engine_preedit_visible(core_)) {
        const char *p = pk_engine_preedit(core_);
        const std::string preedit = p ? p : "";
        const TextFormatFlags flags =
            pk_engine_preedit_underline(core_)
                ? TextFormatFlags{TextFormatFlag::Underline}
                : TextFormatFlags{TextFormatFlag::NoFlag};

        Text text;
        text.append(preedit, flags);
        // Con trỏ luôn ở cuối preedit ⇒ vị trí byte = độ dài byte.
        text.setCursor(static_cast<int>(text.textLength()));

        if (ic_->capabilityFlags().test(CapabilityFlag::Preedit)) {
            panel.setClientPreedit(text);
        } else {
            panel.setPreedit(text);
        }
    }

    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
}

// ----------------------------------- PinaKeyEngine -----------------------------------

PinaKeyEngine::PinaKeyEngine(Instance *instance)
    : instance_(instance), factory_([this](InputContext &ic) {
          return new PinaKeyState(this, &ic);
      }) {
    instance_->inputContextManager().registerProperty("pinakeyState", &factory_);
}

void PinaKeyEngine::keyEvent(const InputMethodEntry & /*entry*/, KeyEvent &keyEvent) {
    state(keyEvent.inputContext())->keyEvent(keyEvent);
}

void PinaKeyEngine::reset(const InputMethodEntry & /*entry*/, InputContextEvent &event) {
    state(event.inputContext())->reset();
}

void PinaKeyEngine::activate(const InputMethodEntry & /*entry*/, InputContextEvent &event) {
    auto *ic = event.inputContext();
    pk_engine_set_program(state(ic)->core(), ic->program().c_str());
}

void PinaKeyEngine::deactivate(const InputMethodEntry & /*entry*/, InputContextEvent &event) {
    // Khi rời input method: dọn preedit để không kẹt chữ (chế độ commit-on-focus-out sẽ thêm ở #6).
    state(event.inputContext())->reset();
}

} // namespace fcitx

FCITX_ADDON_FACTORY(fcitx::PinaKeyEngineFactory)
