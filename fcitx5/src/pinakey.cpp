/*
 * Addon fcitx5 cho PinaKey — hiện thực. Xem pinakey.h để biết kiến trúc.
 * GPL-3.0-or-later.
 */
#include "pinakey.h"

#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysymgen.h>
#include <fcitx-utils/textformatflags.h>
#include <fcitx/candidatelist.h>
#include <fcitx/event.h>
#include <fcitx/inputpanel.h>
#include <fcitx/statusarea.h>
#include <fcitx/text.h>
#include <fcitx/userinterfacemanager.h>

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

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

    // #9/#19: app trong danh sách loại trừ tiếng Anh, hoặc ô mật khẩu → để phím đi thẳng.
    if (shouldPassThrough()) {
        return;
    }

    // #11/#26: đang ở chế độ tra emoji?
    if (emojiMode_ && handleEmojiKey(keyEvent)) {
        return;
    }

    const uint32_t sym = static_cast<uint32_t>(keyEvent.rawKey().sym());
    const uint32_t state = static_cast<uint32_t>(keyEvent.rawKey().states());

    // #11/#26: ':' khi không đang soạn dở → mở tra emoji (gõ :tên hoặc :u<hex>).
    if (!emojiMode_ && sym == FcitxKey_colon && !pk_engine_is_composing(core_)) {
        startEmoji();
        keyEvent.filterAndAccept();
        return;
    }

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

/// Có nên để phím đi thẳng (không gõ tiếng Việt) không: app trong danh sách loại trừ (#9) hoặc
/// đang ở ô nhập mật khẩu (#19, tự loại trừ).
bool PinaKeyState::shouldPassThrough() const {
    return pk_engine_program_excluded(core_) ||
           ic_->capabilityFlags().test(CapabilityFlag::Password);
}

/// Kết thúc phiên khi rời input method / mất focus (#6): commit phần đang soạn để không kẹt/mất chữ.
void PinaKeyState::deactivate() {
    if (!wantReplaceMode()) {
        if (const char *p = pk_engine_flush_preedit(core_); p && p[0] != '\0') {
            ic_->commitString(p);
        }
    } else {
        pk_engine_reset(core_);
    }
    ic_->inputPanel().reset();
    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
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
    setupStatusMenu();
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
    addStatusActions(ic); // menu chọn kiểu gõ / bảng mã (#12/#17)
}

void PinaKeyEngine::deactivate(const InputMethodEntry & /*entry*/, InputContextEvent &event) {
    // #6: khi rời input method / mất focus, commit phần preedit đang soạn (không để kẹt chữ).
    state(event.inputContext())->deactivate();
}

// ============================ Menu khu vực trạng thái (#12/#17) ============================

void PinaKeyEngine::setupStatusMenu() {
    auto &uim = instance_->userInterfaceManager();

    imMenu_ = std::make_unique<Menu>();
    imRootAction_ = std::make_unique<SimpleAction>();
    imRootAction_->setShortText("Kiểu gõ");
    imRootAction_->setMenu(imMenu_.get());
    uim.registerAction("pinakey-im", imRootAction_.get());
    for (uint32_t i = 0; i < pk_input_method_count(); ++i) {
        std::string name = pk_input_method_name_at(i);
        auto act = std::make_unique<SimpleAction>();
        act->setShortText(name);
        act->setCheckable(true);
        act->connect<SimpleAction::Activated>(
            [this, name](InputContext *) { applyInputMethod(name); });
        uim.registerAction("pinakey-im-" + std::to_string(i), act.get());
        imMenu_->addAction(act.get());
        imItems_.push_back(std::move(act));
        imNames_.push_back(std::move(name));
    }

    charsetMenu_ = std::make_unique<Menu>();
    charsetRootAction_ = std::make_unique<SimpleAction>();
    charsetRootAction_->setShortText("Bảng mã");
    charsetRootAction_->setMenu(charsetMenu_.get());
    uim.registerAction("pinakey-charset", charsetRootAction_.get());
    for (uint32_t i = 0; i < pk_charset_count(); ++i) {
        std::string name = pk_charset_name_at(i);
        auto act = std::make_unique<SimpleAction>();
        act->setShortText(name);
        act->setCheckable(true);
        act->connect<SimpleAction::Activated>(
            [this, name](InputContext *) { applyCharset(name); });
        uim.registerAction("pinakey-charset-" + std::to_string(i), act.get());
        charsetMenu_->addAction(act.get());
        charsetItems_.push_back(std::move(act));
        charsetNames_.push_back(std::move(name));
    }
}

void PinaKeyEngine::addStatusActions(InputContext *ic) {
    ic->statusArea().addAction(StatusGroup::InputMethod, imRootAction_.get());
    ic->statusArea().addAction(StatusGroup::InputMethod, charsetRootAction_.get());
    const std::string curIM = pk_engine_input_method(state(ic)->core());
    for (size_t i = 0; i < imItems_.size(); ++i) {
        imItems_[i]->setChecked(imNames_[i] == curIM);
    }
    const std::string curCS = pk_engine_charset(state(ic)->core());
    for (size_t i = 0; i < charsetItems_.size(); ++i) {
        charsetItems_[i]->setChecked(charsetNames_[i] == curCS);
    }
}

void PinaKeyEngine::applyInputMethod(const std::string &name) {
    instance_->inputContextManager().foreach([&](InputContext *ic) {
        pk_engine_set_input_method(state(ic)->core(), name.c_str());
        return true;
    });
    for (size_t i = 0; i < imItems_.size(); ++i) {
        imItems_[i]->setChecked(imNames_[i] == name);
    }
    if (auto *ic = instance_->mostRecentInputContext()) {
        ic->updateUserInterface(UserInterfaceComponent::StatusArea);
    }
}

void PinaKeyEngine::applyCharset(const std::string &name) {
    instance_->inputContextManager().foreach([&](InputContext *ic) {
        pk_engine_set_charset(state(ic)->core(), name.c_str());
        return true;
    });
    for (size_t i = 0; i < charsetItems_.size(); ++i) {
        charsetItems_[i]->setChecked(charsetNames_[i] == name);
    }
    if (auto *ic = instance_->mostRecentInputContext()) {
        ic->updateUserInterface(UserInterfaceComponent::StatusArea);
    }
}

// ============================ Tra cứu emoji / hex (#11/#26) ============================

namespace {
std::string utf32ToUtf8(char32_t cp) {
    std::string out;
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
    return out;
}

/// Candidate emoji: chọn → gọi PinaKeyState::emojiSelect(index).
class EmojiCandidate : public CandidateWord {
public:
    EmojiCandidate(PinaKeyState *state, int index, Text text)
        : state_(state), index_(index) {
        setText(std::move(text));
    }
    void select(InputContext * /*ic*/) const override { state_->emojiSelect(index_); }

private:
    PinaKeyState *state_;
    int index_;
};
} // namespace

void PinaKeyState::startEmoji() {
    emojiMode_ = true;
    emojiQuery_ = ":";
    updateEmojiUI();
}

void PinaKeyState::updateEmojiUI() {
    emojiCandidates_.clear();
    const std::string query = emojiQuery_.size() > 1 ? emojiQuery_.substr(1) : "";

    // Hex Unicode: ":u<hex>" → ký tự tương ứng (issue #11 nhập hexadecimal).
    if (query.size() >= 2 && (query[0] == 'u' || query[0] == 'U')) {
        const std::string hex = query.substr(1);
        if (!hex.empty() &&
            hex.find_first_not_of("0123456789abcdefABCDEF") == std::string::npos) {
            char32_t cp = static_cast<char32_t>(strtoul(hex.c_str(), nullptr, 16));
            if (cp != 0 && cp <= 0x10FFFF) {
                emojiCandidates_.push_back(utf32ToUtf8(cp));
            }
        }
    }

    if (!query.empty()) {
        const char *res = pk_emoji_query(query.c_str());
        std::string s = res ? res : "";
        size_t pos = 0;
        while (pos <= s.size() && emojiCandidates_.size() < 30) {
            size_t nl = s.find('\n', pos);
            std::string item =
                s.substr(pos, nl == std::string::npos ? std::string::npos : nl - pos);
            if (!item.empty()) {
                emojiCandidates_.push_back(item);
            }
            if (nl == std::string::npos) {
                break;
            }
            pos = nl + 1;
        }
    }

    auto list = std::make_unique<CommonCandidateList>();
    list->setPageSize(9);
    list->setLayoutHint(CandidateLayoutHint::Vertical);
    for (size_t i = 0; i < emojiCandidates_.size(); ++i) {
        Text label;
        label.append(std::to_string((i % 9) + 1) + ". " + emojiCandidates_[i]);
        list->append(std::make_unique<EmojiCandidate>(this, static_cast<int>(i), std::move(label)));
    }
    if (!emojiCandidates_.empty()) {
        list->setGlobalCursorIndex(0);
    }

    auto &panel = ic_->inputPanel();
    panel.reset();
    panel.setCandidateList(std::move(list));
    Text pre;
    pre.append(emojiQuery_);
    pre.setCursor(static_cast<int>(pre.textLength()));
    if (ic_->capabilityFlags().test(CapabilityFlag::Preedit)) {
        panel.setClientPreedit(pre);
    } else {
        panel.setPreedit(pre);
    }
    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
}

void PinaKeyState::emojiSelect(int index) {
    if (index >= 0 && index < static_cast<int>(emojiCandidates_.size())) {
        ic_->commitString(emojiCandidates_[index]);
    }
    emojiMode_ = false;
    emojiQuery_.clear();
    emojiCandidates_.clear();
    ic_->inputPanel().reset();
    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
}

void PinaKeyState::cancelEmoji(bool commitLiteral) {
    const std::string literal = emojiQuery_;
    emojiMode_ = false;
    emojiQuery_.clear();
    emojiCandidates_.clear();
    ic_->inputPanel().reset();
    if (commitLiteral && !literal.empty()) {
        ic_->commitString(literal);
    }
    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
}

bool PinaKeyState::handleEmojiKey(KeyEvent &keyEvent) {
    const uint32_t sym = static_cast<uint32_t>(keyEvent.rawKey().sym());

    // Chế độ hex (":u<hex>") cần gõ chữ số → KHÔNG dùng số để chọn candidate.
    const bool hexMode =
        emojiQuery_.size() >= 2 && (emojiQuery_[1] == 'u' || emojiQuery_[1] == 'U');

    if (!hexMode && sym >= FcitxKey_1 && sym <= FcitxKey_9) {
        const int idx = static_cast<int>(sym - FcitxKey_1);
        if (idx < static_cast<int>(emojiCandidates_.size())) {
            emojiSelect(idx);
            keyEvent.filterAndAccept();
            return true;
        }
    }
    if (sym == FcitxKey_Return || sym == FcitxKey_KP_Enter) {
        if (!emojiCandidates_.empty()) {
            emojiSelect(0);
        } else {
            cancelEmoji(true);
        }
        keyEvent.filterAndAccept();
        return true;
    }
    if (sym == FcitxKey_space) {
        if (!emojiCandidates_.empty()) {
            emojiSelect(0);
        } else {
            cancelEmoji(true);
        }
        keyEvent.filterAndAccept();
        return true;
    }
    if (sym == FcitxKey_Escape) {
        cancelEmoji(true);
        keyEvent.filterAndAccept();
        return true;
    }
    if (sym == FcitxKey_BackSpace) {
        if (emojiQuery_.size() > 1) {
            emojiQuery_.pop_back();
            updateEmojiUI();
        } else {
            cancelEmoji(false);
        }
        keyEvent.filterAndAccept();
        return true;
    }
    // Ký tự ASCII in được (chữ/dấu) → nối vào truy vấn.
    if (sym >= 0x21 && sym < 0x7f) {
        emojiQuery_.push_back(static_cast<char>(sym));
        updateEmojiUI();
        keyEvent.filterAndAccept();
        return true;
    }
    // Phím khác (mũi tên…) → thoát emoji, commit phần đã gõ, để phím đi tiếp.
    cancelEmoji(true);
    return false;
}

} // namespace fcitx

FCITX_ADDON_FACTORY(fcitx::PinaKeyEngineFactory)
