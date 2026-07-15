/*
 * Addon fcitx5 cho PinaKey — hiện thực. Xem pinakey.h để biết kiến trúc.
 * GPL-3.0-or-later.
 */
#include "pinakey.h"

#include "socketpath.h"
#include "uinputclient.h"
#include "utf8util.h"

#include <fcitx-utils/capabilityflags.h>
#include <fcitx-utils/key.h>
#include <fcitx-utils/keysymgen.h>
#include <fcitx-utils/log.h>
#include <fcitx-utils/textformatflags.h>
#include <fcitx/candidatelist.h>
#include <fcitx/event.h>
#include <fcitx/inputpanel.h>
#include <fcitx/statusarea.h>
#include <fcitx/text.h>
#include <fcitx/userinterfacemanager.h>

#include <sys/stat.h>
#include <unistd.h>

#include <cctype>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace fcitx {

namespace {
/// Bit "phím nhả" mà C-ABI quy ước (1<<30). fcitx5 không dùng bit này trong `states()`
/// (Virtual=1<<29, Repeat=1<<31), nên đặt riêng cho release là an toàn.
constexpr uint32_t kPkModRelease = 1u << 30;

/// Tên engine để lõi Rust nạp cấu hình `~/.config/pinakey/ibus-PinaKey.config.json` — dùng chung
/// file cấu hình với frontend IBus trong giai đoạn chạy song song.
constexpr char kConfigName[] = "PinaKey";

/// #60: bật log chẩn đoán surrounding text bằng env `PINAKEY_DEBUG_SURROUNDING=1`. Đọc một lần
/// rồi cache — MẶC ĐỊNH TẮT, không đụng hành vi hay hiệu năng đường nóng. Dùng cho phiên đo thủ
/// công semantics `deleteSurroundingText` của app (Chromium omnibox…) trước khi viết heuristic.
bool debugSurroundingEnabled() {
    static const bool enabled = [] {
        const char *e = std::getenv("PINAKEY_DEBUG_SURROUNDING");
        return e && e[0] == '1';
    }();
    return enabled;
}

/// Client tới daemon uinput (issue #28/#91/#72, xem uinputclient.h) — một kết nối dùng chung
/// cho cả tiến trình addon, socket filesystem trong $XDG_RUNTIME_DIR như daemon quy ước.
pinakey::UinputClient &uinputClient() {
    static pinakey::UinputClient client(pinakey::uinputSocketPath());
    return client;
}

/// Phím Backspace (kể cả phím bơm-ngược từ daemon uinput). FcitxKey_BackSpace == 0xff08 == 65288;
/// thêm 8 (ASCII BS) cho chắc với một số frontend.
bool isBackspaceSym(uint32_t sym) {
    return sym == FcitxKey_BackSpace || sym == 8u;
}

/// Modifier "thật" (Ctrl=1<<2, Alt=1<<3, Hyper=1<<5, Super=1<<6, Super2=1<<26, Hyper2=1<<27,
/// Meta=1<<28) — dùng để nhận diện tổ hợp phím tắt. KHÔNG gồm Shift và các bit khoá
/// CapsLock (1<<1) / NumLock (1<<4): chúng có thể bật thường trực khi gõ văn bản (#108),
/// đồng bộ với is_valid_state() phía Rust.
constexpr uint32_t kRealModMask =
    static_cast<uint32_t>(KeyState::Ctrl) | static_cast<uint32_t>(KeyState::Alt) |
    static_cast<uint32_t>(KeyState::Hyper) | static_cast<uint32_t>(KeyState::Super) |
    static_cast<uint32_t>(KeyState::Super2) | static_cast<uint32_t>(KeyState::Hyper2) |
    static_cast<uint32_t>(KeyState::Meta);

/// Bit auto-repeat (giữ phím). Nhánh double-space phải loại: giữ space chỉ là chuỗi dấu cách,
/// không phải "nhấn space lần hai".
constexpr uint32_t kModRepeat = static_cast<uint32_t>(KeyState::Repeat);

/// #117: keySymToUTF8 trả chuỗi KHÔNG rỗng cho cả phím điều khiển (Return="\r", Tab="\t",
/// Escape="\x1b", Delete="\x7f") — chỉ chuỗi in được mới là "văn bản" để đệm/commit nguyên văn;
/// commit control char thô vào tài liệu là rác + mất chức năng phím.
bool isPrintableText(const std::string &u) {
    return !u.empty() && !(u.size() == 1 && (static_cast<unsigned char>(u[0]) < 0x20 ||
                                             u[0] == '\x7f'));
}
} // namespace

// ----------------------------------- PinaKeyState -----------------------------------

PinaKeyState::PinaKeyState(PinaKeyEngine *engine, InputContext *ic)
    : engine_(engine), ic_(ic), core_(pk_engine_new_from_name(kConfigName)) {
    // Tên chương trình đang focus → bật các cách khắc phục theo ứng dụng ở lõi.
    pk_engine_set_program(core_, ic->program().c_str());
    // #111: người dùng đã đổi Kiểu gõ/Bảng mã qua menu khay trong phiên này → cửa sổ mở sau
    // phải theo lựa chọn đó, không quay về giá trị trong config file.
    engine_->applySessionOverrides(core_);
}

PinaKeyState::~PinaKeyState() { pk_engine_free(core_); }

/// #156: DỌN (vứt bỏ) khối trạng thái ACK uinput để không kẹt nếu bị ngắt giữa chuỗi xoá (reset,
/// đổi focus, click…). Chuỗi xoá dở gắn với vị trí con trỏ cũ; nếu để sống sót, nhánh timeout ở
/// keyEvent kế tiếp sẽ commit `pendingCommit_` "ma" vào vị trí mới + cộng nợ orphanBackspaces_ làm
/// nuốt Backspace thật của người dùng.
void PinaKeyState::clearUinputAckState() {
    deleting_ = false;
    expectedBackspaces_ = 0;
    currentBackspaceCount_ = 0;
    pendingCommit_.clear();
    bufferedKeys_.clear();
    pendingForwardKey_.reset();
    orphanBackspaces_ = 0;
}

void PinaKeyState::reset() {
    clearUinputAckState();

    // Dọn trạng thái emoji: reset là "vứt bỏ" (không commit) — nếu để sót, phím gõ sau khi quay
    // lại context này bị nuốt vào query emoji vô hình.
    emojiMode_ = false;
    emojiQuery_.clear();
    emojiCandidates_.clear();

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

    // #125: Backspace "mồ côi" — bơm cho chuỗi xoá ĐÃ timeout, giao muộn (round-trip chậm,
    // không phải mất hẳn). Nuốt bỏ để chúng không ăn quota ACK của chuỗi mới (xoá ký tự thật
    // + commit sớm) hay rơi vào engine như Backspace NGƯỜI DÙNG (mở chuỗi mới nữa — lệch pha
    // tự khuếch đại). Chúng đi cùng pipeline input nên luôn về TRƯỚC Backspace của chuỗi mới;
    // quá 1000ms coi như mất hẳn — Backspace lúc đó là của người dùng thật.
    //
    // Đây là heuristic có chủ đích: Backspace bơm-ngược quay về là key event kernel thuần,
    // KHÔNG thể mang định danh chuỗi/generation (muốn có phải đổi protocol ACK sang socket).
    // Nếu chúng thật sự bị rơi (hiếm — daemon còn sống mới mở được chuỗi mới), giá phải trả
    // bị chặn ở ≤ nợ mồ côi trong 1000ms và KHÔNG tự khuếch đại như để chúng lẫn vào ACK.
    if (orphanBackspaces_ > 0) {
        const auto orphanAge = std::chrono::duration_cast<std::chrono::milliseconds>(
                                   std::chrono::steady_clock::now() - orphanSince_)
                                   .count();
        if (orphanAge > 1000) {
            orphanBackspaces_ = 0;
        } else if (isBackspaceSym(static_cast<uint32_t>(keyEvent.rawKey().sym()))) {
            --orphanBackspaces_;
            keyEvent.filterAndAccept();
            return;
        }
    }

    // ACK uinput: đang trong chuỗi xoá tự động → các phím Backspace bơm-ngược từ daemon đi qua đây.
    if (deleting_) {
        const uint32_t s = static_cast<uint32_t>(keyEvent.rawKey().sym());
        if (isBackspaceSym(s)) {
            handleUinputAck(keyEvent); // tự để-đi-tiếp (trung gian) hoặc commit + nuốt (trigger)
            return;
        }
        // Lưới an toàn: nếu Backspace bơm-ngược không quay về trong 500ms (round-trip thất bại),
        // bỏ ACK, commit phần đang chờ rồi xử lý phím này như thường — không để kẹt cứng.
        const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
                                 std::chrono::steady_clock::now() - deletingSince_)
                                 .count();
        if (elapsed > 500) {
            if (!pendingCommit_.empty()) {
                ic_->commitString(pendingCommit_);
            }
            pendingCommit_.clear();
            deleting_ = false;
            // #125: phần Backspace chưa quay về của chuỗi này có thể chỉ MUỘN chứ không mất
            // — ghi nợ để nuốt bỏ khi chúng về, không cho lẫn vào chuỗi mới / đường thường.
            orphanBackspaces_ += expectedBackspaces_ - currentBackspaceCount_;
            orphanSince_ = std::chrono::steady_clock::now();
            expectedBackspaces_ = 0;
            currentBackspaceCount_ = 0;
            flushPendingForward(); // #118: không để phím chức năng hoãn bị rơi khi timeout
            // #96: bơm các phím đã đệm TRƯỚC khi xử lý phím hiện tại — bỏ qua thì phím mới
            // chen lên trước (đảo thứ tự) còn phím cũ kẹt chờ một chuỗi ACK không liên quan
            // trong tương lai (chèn sai ngữ cảnh) hoặc mất hẳn tới reset().
            replayBufferedKeys();
            // KHÔNG return (trừ khi replay lại mở chuỗi xoá mới) → xử lý phím hiện tại bên dưới.
        }
        if (deleting_) {
            // Chưa timeout — hoặc replay ở trên vừa mở một chuỗi xoá mới: đệm phím để replay
            // sau, nuốt phím lúc này. #118: đệm CẢ phím non-text (Enter/Tab/mũi tên…) — khi
            // replay chúng đi qua engine rồi được forwardKey cho app, không mất; chỉ modifier
            // đứng một mình là bỏ qua (không mang thông tin).
            if (!Key(static_cast<KeySym>(s)).isModifier() && bufferedKeys_.size() < 32) {
                bufferedKeys_.emplace_back(s, static_cast<uint32_t>(keyEvent.rawKey().states()));
            }
            keyEvent.filterAndAccept();
            return;
        }
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
    // #109: tổ hợp có modifier thật (Ctrl+Shift+; trên layout US cho sym=colon — shortcut của
    // app) không thuộc về emoji — để đi tiếp, đồng bộ với check trong handleEmojiKey.
    if (!emojiMode_ && sym == FcitxKey_colon && (state & kRealModMask) == 0 &&
        !pk_engine_is_composing(core_)) {
        startEmoji();
        keyEvent.filterAndAccept();
        return;
    }

    // #65: double-space → ". " (option, mặc định tắt — engine chỉ "arm" khi cờ bật). Dấu cách
    // thứ hai ngay sau khi commit "từ ": xoá dấu cách cũ + commit ". ". Cần app cho xoá
    // surrounding text; app báo cáo không đáng tin (#66) thì thôi — engine tự disarm khi
    // thấy phím space đi qua như thường. surroundingEndsWithWordSpace() chống trường hợp
    // người dùng click dời con trỏ mà app không gửi reset: vị trí mới không kết thúc bằng
    // "từ + dấu cách" thì tuyệt đối không xoá-chèn.
    // #108: chỉ loại tổ hợp có modifier THẬT (và auto-repeat) — `state == 0` cũ chặn nhầm cả
    // NumLock/CapsLock (bit khoá luôn có trong states() khi đèn sáng) làm tính năng chết lặng lẽ.
    if (sym == FcitxKey_space && (state & (kRealModMask | kModRepeat)) == 0 &&
        pk_engine_double_space_armed(core_) &&
        ic_->capabilityFlags().test(CapabilityFlag::SurroundingText) &&
        !pk_engine_surrounding_text_unreliable(core_) && surroundingEndsWithWordSpace()) {
        ic_->deleteSurroundingText(-1, 1);
        ic_->commitString(". ");
        pk_engine_double_space_consume(core_);
        keyEvent.filterAndAccept();
        return;
    }

    // Gõ không gạch chân #1: app hỗ trợ SurroundingText → xoá-chèn tại chỗ. Riêng app báo
    // surrounding text không đáng tin (LibreOffice, #66) thì bỏ qua → rơi xuống preedit ở dưới.
    if (pk_engine_no_underline(core_) &&
        ic_->capabilityFlags().test(CapabilityFlag::SurroundingText) &&
        !pk_engine_surrounding_text_unreliable(core_)) {
        debugLogSurrounding("replace-branch"); // #60: đo trạng thái trước khi quyết định.
        // #60: đang có vùng chọn (autocomplete bôi chọn gợi ý, hoặc người dùng bôi chọn rồi
        // gõ) → app có thể áp deleteSurroundingText vào vùng chọn thay vì trước con trỏ
        // (vùng chết đã quan sát ở Chromium) → xoá nhầm. Nhường preedit tới khi hết selection.
        if (!surroundingHasSelection()) {
            resetIfDocumentDiverged(); // #7: con trỏ nhảy → quên segment cũ, không xoá nhầm.
            const bool handled = pk_engine_process_key_replace(core_, sym, state);
            applyReplaceResult();
            if (handled) {
                keyEvent.filterAndAccept();
            }
            return;
        }
        // Selection xuất hiện GIỮA từ (engine còn theo dõi segment đã commit): reset trước khi
        // rơi xuống preedit — không reset thì preedit soạn tiếp trên buffer cũ, hiện chữ đè
        // cạnh đoạn đã commit trong tài liệu → đúp ký tự kiểu "dđ".
        if (const char *seg = pk_engine_replace_segment(core_); seg && seg[0] != '\0') {
            pk_engine_reset(core_);
        }
    }

    // Gõ không gạch chân #2 (#28): app KHÔNG có SurroundingText nhưng có daemon uinput → bơm
    // Backspace + commit có ĐỒNG BỘ ACK (xem startUinputReplace/handleUinputAck).
    if (useUinput()) {
        const bool handled = pk_engine_process_key_replace(core_, sym, state);
        // #106: send thất bại → KHÔNG nuốt phím — để nó đi tiếp tới app (gõ mộc nhất quán
        // với việc lõi đã reset), thay vì phím biến mất im lặng.
        const bool sent = startUinputReplace();
        if (handled && sent) {
            keyEvent.filterAndAccept();
        }
        return;
    }

    // Mặc định: chế độ preedit (không gạch chân về mặt kiểu dáng do cờ IB_NO_UNDERLINE).
    const bool handled = pk_engine_process_key(core_, sym, state);
    applyResult();
    if (handled) {
        keyEvent.filterAndAccept();
    }
}

/// Có dùng diff-and-replace (gõ không gạch chân) không — qua SurroundingText hoặc qua uinput.
/// App có surrounding text không đáng tin (LibreOffice, #66) không tính: nó dùng preedit.
bool PinaKeyState::wantReplaceMode() const {
    if (!pk_engine_no_underline(core_)) {
        return false;
    }
    return (ic_->capabilityFlags().test(CapabilityFlag::SurroundingText) &&
            !pk_engine_surrounding_text_unreliable(core_) && !surroundingHasSelection()) ||
           useUinput();
}

/// Có dùng chế độ uinput+ACK (xoá-bằng-Backspace) cho app KHÔNG có SurroundingText không.
///
/// MẶC ĐỊNH TẮT. Trên GNOME Wayland, terminal nói chuyện với fcitx5 qua frontend D-Bus (lớp IBus),
/// nơi GNOME không bảo đảm thứ tự giữa "xoá" và "commit" → mọi kỹ thuật xoá-bằng-Backspace
/// (uinput commit-ngay, forwardKey, kể cả uinput+ACK kiểu Lotus) đều rối ký tự. Vì vậy mặc định
/// rơi về preedit (ổn định 100%); app có SurroundingText (trình duyệt/editor) vẫn gõ không gạch
/// chân qua đường #1. Đặt `PINAKEY_UINPUT=1` để bật lại uinput+ACK (thử nghiệm, tự chịu rủi ro).
bool PinaKeyState::useUinput() const {
    static const bool enabled = [] {
        const char *e = std::getenv("PINAKEY_UINPUT");
        return e != nullptr && e[0] == '1' && e[1] == '\0';
    }();
    if (!enabled) {
        return false;
    }
    return pk_engine_no_underline(core_) &&
           !ic_->capabilityFlags().test(CapabilityFlag::SurroundingText) &&
           uinputClient().available();
}

/// Bắt đầu thay thế qua uinput (app không có SurroundingText): bơm Backspace, HOÃN commit.
/// Khác đường cũ (#28) — không commit ngay để tránh cuộc đua "commit tới trước khi Backspace
/// kịp xoá". Chuỗi mới được cất ở `pendingCommit_` và chỉ commit trong handleUinputAck khi đã
/// đếm đủ Backspace bơm-ngược (xác nhận app xoá xong).
bool PinaKeyState::startUinputReplace() {
    const uint32_t del = pk_engine_replace_delete(core_);
    const char *ins = pk_engine_replace_insert(core_);
    const std::string insert = (ins && ins[0] != '\0') ? std::string(ins) : std::string();

    // Chế độ replace không hiện preedit → dọn panel nếu còn sót.
    auto &panel = ic_->inputPanel();
    if (!panel.empty()) {
        panel.reset();
        ic_->updatePreedit();
        ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
    }

    if (del == 0) {
        // Không cần xoá (gõ ký tự mới bình thường) → commit ngay, không cần ACK.
        if (!insert.empty()) {
            ic_->commitString(insert);
        }
        return true;
    }

    // Cần xoá `del` ký tự: bơm (del+1) Backspace. `del` cái đầu được để-đi-tiếp để xoá thật;
    // cái thứ (del+1) là "trigger" — khi nó quay về nghĩa là đã xoá xong → commit + nuốt trigger.
    // #106: CHỈ vào trạng thái chờ ACK khi thông điệp đã thật sự đi — send thất bại mà vẫn chờ
    // thì treo 500ms rồi commit đè chữ cũ chưa xoá (đúp chữ im lặng).
    if (!uinputClient().sendBackspaces(static_cast<int>(del) + 1)) {
        // Không xoá được chữ cũ → bỏ biến đổi lần này (văn bản giữ nguyên như gõ mộc) và
        // reset lõi để trạng thái engine không lệch với tài liệu.
        pk_engine_reset(core_);
        return false;
    }
    pendingCommit_ = insert;
    currentBackspaceCount_ = 0;
    expectedBackspaces_ = static_cast<int>(del) + 1;
    deleting_ = true;
    deletingSince_ = std::chrono::steady_clock::now();
    return true;
}

/// Xử lý một phím Backspace bơm-ngược (từ daemon uinput) trong lúc `deleting_`.
/// - Backspace trung gian (< expected): KHÔNG nuốt → để đi tiếp tới app, thật sự xoá 1 ký tự.
/// - Backspace cuối (== expected, là trigger): app đã xoá xong → commit chuỗi mới rồi NUỐT trigger
///   để nó không xoá nhầm một ký tự thật, sau đó replay các phím đã đệm (nếu user gõ nhanh).
void PinaKeyState::handleUinputAck(KeyEvent &keyEvent) {
    currentBackspaceCount_ += 1;
    if (currentBackspaceCount_ < expectedBackspaces_) {
        return; // trung gian: để phím đi tiếp (không filterAndAccept) → app xoá 1 ký tự
    }
    // Trigger: chờ một nhịp ngắn cho app kịp xử lý các Backspace vừa để-đi-tiếp, rồi commit.
    std::this_thread::sleep_for(std::chrono::milliseconds(5));
    if (!pendingCommit_.empty()) {
        ic_->commitString(pendingCommit_);
    }
    pendingCommit_.clear();
    deleting_ = false;
    expectedBackspaces_ = 0;
    currentBackspaceCount_ = 0;
    keyEvent.filterAndAccept(); // nuốt phím trigger (+1)
    flushPendingForward(); // #118: phím chức năng mở chuỗi xoá này — forward sau khi commit
    replayBufferedKeys();
}

/// #118: forward một "cú gõ" phím chức năng cho app — cả press lẫn release, vì sự kiện release
/// gốc đã đi qua từ trước (isRelease return sớm) không khớp cặp; press mồ côi có thể làm app
/// coi phím còn đang giữ.
void PinaKeyState::forwardKeyTap(uint32_t sym, uint32_t state) {
    const Key key(static_cast<KeySym>(sym), KeyStates(state));
    ic_->forwardKey(key, /*isRelease=*/false);
    ic_->forwardKey(key, /*isRelease=*/true);
}

/// #118: forward phím non-text đã hoãn (chờ chuỗi xoá của chính nó hoàn tất) cho app.
void PinaKeyState::flushPendingForward() {
    if (!pendingForwardKey_) {
        return;
    }
    const auto [fs, fst] = *pendingForwardKey_;
    pendingForwardKey_.reset();
    forwardKeyTap(fs, fst);
}

/// Replay các phím người dùng gõ trong lúc đang xoá. Xử lý lần lượt; nếu một phím lại sinh ra
/// lệnh xoá (deleting_ = true) thì dừng — phần còn lại sẽ được replay khi ACK lần đó hoàn tất.
void PinaKeyState::replayBufferedKeys() {
    while (!bufferedKeys_.empty() && !deleting_) {
        const auto [s, st] = bufferedKeys_.front();
        bufferedKeys_.erase(bufferedKeys_.begin());
        const std::string u = Key::keySymToUTF8(static_cast<KeySym>(s));
        // Tổ hợp mang modifier thật (Ctrl+C…) là LỆNH, không phải văn bản — forward như phím
        // chức năng, tuyệt đối không commit literal 'c' vào tài liệu.
        const bool isText = isPrintableText(u) && (st & kRealModMask) == 0;
        const bool handled = pk_engine_process_key_replace(core_, s, st);
        const bool sent = startUinputReplace();
        if (!sent && isText) {
            // #106: phím này đã bị NUỐT từ lúc đệm (filterAndAccept) — không gửi được lệnh
            // xoá thì commit nguyên văn ký tự (gõ mộc, lõi đã reset) để nó không mất im lặng.
            // #117: chỉ ký tự in được; control char thô không được vào tài liệu.
            ic_->commitString(u);
        }
        if (!isText && !(handled && sent)) {
            // #118: phím chức năng (Enter/Tab/mũi tên, tổ hợp modifier) — sự kiện gốc đã bị
            // nuốt lúc đệm, forward lại cho app để không mất. Chỉ khi engine tiêu thụ VÀ gửi
            // trót lọt (vd Tab bung macro thành công) mới nuốt — khớp đường gõ thường
            // `if (handled && sent) filterAndAccept()`; send fail thì bung macro đã bị vứt
            // (core reset) nên phím thô vẫn phải tới app.
            if (deleting_) {
                pendingForwardKey_ = std::make_pair(s, st);
            } else {
                forwardKeyTap(s, st);
            }
        }
        if (!sent) {
            // #116: client vừa fail (throttle 5s) — mọi phím còn lại chắc chắn cũng fail
            // hoặc để sót trạng thái composing trong core (phím del==0 commit thẳng nhưng
            // giữ prev_displayed → phím thật kế tiếp đi đường preedit trên core bẩn, đúp
            // chữ "vviệt"). Flush nguyên văn phần còn lại rồi DỪNG, để core sạch.
            for (const auto &[rs, rst] : bufferedKeys_) {
                const std::string ru = Key::keySymToUTF8(static_cast<KeySym>(rs));
                if (isPrintableText(ru) && (rst & kRealModMask) == 0) {
                    ic_->commitString(ru);
                } else {
                    forwardKeyTap(rs, rst);
                }
            }
            bufferedKeys_.clear();
            pk_engine_reset(core_);
            return;
        }
    }
}

/// Có nên để phím đi thẳng (không gõ tiếng Việt) không: app trong danh sách loại trừ (#9) hoặc
/// đang ở ô nhập mật khẩu (#19, tự loại trừ).
bool PinaKeyState::shouldPassThrough() const {
    return pk_engine_program_excluded(core_) ||
           ic_->capabilityFlags().test(CapabilityFlag::Password);
}

/// Kết thúc phiên khi rời input method / mất focus (#6): commit phần đang soạn để không kẹt/mất chữ.
void PinaKeyState::deactivate(bool imSwitch) {
    // Ai chốt phần đang soạn? (mô hình fcitx5-unikey)
    //   - Đổi IM (Ctrl+Space): fcitx5 KHÔNG tự commit → addon phải commit tay, kẻo mất chữ.
    //   - Mất focus + app có CapabilityFlag::Preedit: fcitx5 (≥5.1) TỰ commit client preedit →
    //     addon không được commit tay lần nữa, kẻo ĐÚP chữ ("viêt" → "viêtviêt").
    //   - Mất focus + app không có client preedit: không ai chốt → addon commit tay (#6).
    const bool clientPreedit = ic_->capabilityFlags().test(CapabilityFlag::Preedit);
    const bool mustCommit = imSwitch || !clientPreedit;
    // Đang tra emoji dở: thoát chế độ emoji để phím gõ sau không bị nuốt vào query cũ; chốt
    // literal ":query" khi không ai khác chốt hộ.
    if (emojiMode_) {
        cancelEmoji(mustCommit);
    }
    if (!wantReplaceMode()) {
        // flush_preedit luôn được gọi để reset trạng thái soạn dở của lõi.
        if (const char *p = pk_engine_flush_preedit(core_); p && p[0] != '\0' && mustCommit) {
            ic_->commitString(p);
        }
    } else {
        pk_engine_reset(core_);
    }
    // #156: dọn khối trạng thái ACK uinput như reset(). fcitx5 lúc focus-out chỉ bảo đảm gọi
    // deactivate (không bảo đảm có ResetEvent từ client), nên nếu bỏ sót thì chuỗi xoá dở sống
    // qua đổi focus: quay lại context gõ tiếp sẽ commit pendingCommit_ "ma" + nuốt Backspace thật.
    clearUinputAckState();
    ic_->inputPanel().reset();
    ic_->updatePreedit();
    ic_->updateUserInterface(UserInterfaceComponent::InputPanel);
}

/// #7: Nếu con trỏ đã nhảy / văn bản đổi (người dùng click chuột, bấm mũi tên, app sửa text…),
/// segment mà engine đang theo dõi (`prev_displayed`) không còn nằm ngay trước con trỏ. Khi đó
/// `deleteSurroundingText(-n, n)` sẽ xoá nhầm ký tự ở vị trí mới. Đối chiếu surrounding text trước
/// con trỏ với segment; nếu lệch thì reset để phím tiếp theo được xử lý mới tại đúng chỗ.
void PinaKeyState::resetIfDocumentDiverged() {
    if (!ic_->capabilityFlags().test(CapabilityFlag::SurroundingText)) {
        return;
    }
    const char *segPtr = pk_engine_replace_segment(core_);
    if (!segPtr || segPtr[0] == '\0') {
        return; // engine không theo dõi segment nào → không cần kiểm.
    }
    if (!ic_->surroundingText().isValid()) {
        return; // không đọc được surrounding text → giữ nguyên hành vi cũ (không hồi quy).
    }
    const std::string segment(segPtr);
    const std::string &text = ic_->surroundingText().text();
    const unsigned int cursor = ic_->surroundingText().cursor();
    const size_t bytePos = pinakey::surroundingBytePosBeforeCursor(text, cursor);
    // UTF-8 tự đồng bộ: phần văn bản trước con trỏ (text[0..bytePos]) kết thúc bằng `segment`
    // (so byte) ⟺ đúng theo ký tự. So trực tiếp trên `text`, không cấp phát chuỗi con (hot path).
    const bool endsWithSegment =
        bytePos >= segment.size() &&
        text.compare(bytePos - segment.size(), segment.size(), segment) == 0;
    if (!endsWithSegment) {
        pk_engine_reset(core_);
    }
}

/// #65 double-space: văn bản NGAY TRƯỚC con trỏ có kết thúc bằng "ký tự từ + một dấu cách"
/// không (không có selection). Đây là điều kiện an toàn trước khi xoá dấu cách + chèn ". ":
/// nếu người dùng đã click dời con trỏ (app không gửi reset nên engine còn "armed"), vị trí
/// mới thường không khớp mẫu này → bỏ qua, tuyệt đối không phá văn bản ở chỗ mới.
bool PinaKeyState::surroundingEndsWithWordSpace() const {
    if (!ic_->surroundingText().isValid()) {
        return false;
    }
    const auto &st = ic_->surroundingText();
    if (st.cursor() != st.anchor()) {
        return false; // đang có selection → không đụng.
    }
    const std::string &text = st.text();
    const unsigned int cursor = st.cursor();
    const size_t bytePos = pinakey::surroundingBytePosBeforeCursor(text, cursor);
    // Ký tự ngay trước con trỏ phải là MỘT dấu cách…
    if (bytePos < 2 || text[bytePos - 1] != ' ') {
        return false;
    }
    // …và ký tự trước dấu cách phải là "ký tự từ": lùi qua các byte continuation UTF-8 để tới
    // byte đầu của ký tự. Ký tự đa byte (chữ Việt có dấu) tính là chữ; ASCII thì loại khoảng
    // trắng + dấu câu (khớp điều kiện arm phía engine: chữ/số ngay trước dấu cách).
    size_t p = bytePos - 2;
    while (p > 0 && (static_cast<unsigned char>(text[p]) & 0xC0) == 0x80) {
        --p;
    }
    const unsigned char lead = static_cast<unsigned char>(text[p]);
    if (lead >= 0x80) {
        return true;
    }
    return std::isalnum(lead) != 0;
}

/// #60: surrounding text đang có vùng chọn không (cursor != anchor)? Không đọc được
/// surrounding text thì coi như không có — giữ nguyên hành vi cũ, không hồi quy.
bool PinaKeyState::surroundingHasSelection() const {
    if (!ic_->surroundingText().isValid()) {
        return false;
    }
    const auto &st = ic_->surroundingText();
    return st.cursor() != st.anchor();
}

/// #60: chẩn đoán — in surrounding text (text/cursor/anchor) tại điểm gọi `where`. Chỉ chạy khi
/// PINAKEY_DEBUG_SURROUNDING=1; tắt thì trả về ngay (không đọc surrounding text, không cấp phát).
void PinaKeyState::debugLogSurrounding(const char *where) const {
    if (!debugSurroundingEnabled()) {
        return;
    }
    const auto &st = ic_->surroundingText();
    if (!st.isValid()) {
        FCITX_INFO() << "[pinakey #60] " << where << " surrounding=INVALID";
        return;
    }
    FCITX_INFO() << "[pinakey #60] " << where << " cursor=" << st.cursor()
                 << " anchor=" << st.anchor() << " selection=" << (st.cursor() != st.anchor())
                 << " len=" << st.text().size() << " text=\"" << st.text() << "\"";
}

/// Áp lệnh thay thế: xoá N ký tự trước con trỏ rồi commit chuỗi mới. Không hiện preedit.
void PinaKeyState::applyReplaceResult() {
    const uint32_t del = pk_engine_replace_delete(core_);
    const char *ins = pk_engine_replace_insert(core_);
    if (debugSurroundingEnabled()) {
        // #60: kết quả từng lời gọi — số ký tự yêu cầu xoá + chuỗi chèn. Đối chiếu với thay đổi
        // thật trên màn hình để đo semantics deleteSurroundingText của app khi có/không selection.
        FCITX_INFO() << "[pinakey #60] applyReplace deleteSurroundingText(-" << del << "," << del
                     << ") insert=\"" << (ins ? ins : "") << "\"";
    }
    if (del > 0) {
        ic_->deleteSurroundingText(-static_cast<int>(del), del);
    }
    if (ins && ins[0] != '\0') {
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
    setupReloadTimer();
}

void PinaKeyEngine::setupReloadTimer() {
    // Khớp cách lõi Rust tìm thư mục cấu hình (dirs::config_dir()): ưu tiên $XDG_CONFIG_HOME,
    // fallback $HOME/.config — nếu không, watcher canh mtime của file không bao giờ tồn tại.
    std::string base;
    if (const char *xdg = std::getenv("XDG_CONFIG_HOME"); xdg && *xdg) {
        base = xdg;
    } else if (const char *home = std::getenv("HOME"); home && *home) {
        base = std::string(home) + "/.config";
    } else {
        return;
    }
    const std::string dir = base + "/pinakey/";
    reloadFiles_ = {dir + "ibus-PinaKey.macro.text", dir + "dict.txt"};
    reloadFingerprints_.assign(reloadFiles_.size(), pinakey::FileFingerprint{});
    for (size_t i = 0; i < reloadFiles_.size(); ++i) {
        reloadFingerprints_[i] = pinakey::fileFingerprint(reloadFiles_[i]);
    }
    // #69: canh cả file config (fallback khi GUI không gọi được D-Bus).
    configFile_ = dir + "ibus-PinaKey.config.json";
    configFingerprint_ = pinakey::fileFingerprint(configFile_);
    constexpr uint64_t kInterval = 2000000; // 2s
    reloadTimer_ = instance_->eventLoop().addTimeEvent(
        CLOCK_MONOTONIC, now(CLOCK_MONOTONIC) + kInterval, 0,
        [this](EventSourceTime *src, uint64_t) {
            checkReload();
            src->setTime(now(CLOCK_MONOTONIC) + kInterval);
            return true;
        });
}

void PinaKeyEngine::checkReload() {
    // #69: file config đổi → nạp lại TOÀN BỘ cấu hình (bao trùm cả macro/dict).
    if (!configFile_.empty()) {
        if (auto fp = pinakey::fileFingerprint(configFile_); fp != configFingerprint_) {
            configFingerprint_ = fp;
            reloadConfig();
            return;
        }
    }
    bool changed = false;
    for (size_t i = 0; i < reloadFiles_.size(); ++i) {
        auto fp = pinakey::fileFingerprint(reloadFiles_[i]);
        if (fp != reloadFingerprints_[i]) {
            reloadFingerprints_[i] = fp;
            changed = true;
        }
    }
    if (!changed) {
        return;
    }
    instance_->inputContextManager().foreach([this](InputContext *ic) {
        pk_engine_reload(state(ic)->core());
        return true;
    });
}

void PinaKeyEngine::reloadConfig() {
    // Cập nhật mtime cache (cả config lẫn macro/dict — pk_engine_reload_config nạp lại tất) để
    // watcher không nạp lại lần nữa ngay sau lời gọi D-Bus.
    if (!configFile_.empty()) {
        configFingerprint_ = pinakey::fileFingerprint(configFile_);
    }
    for (size_t i = 0; i < reloadFiles_.size(); ++i) {
        reloadFingerprints_[i] = pinakey::fileFingerprint(reloadFiles_[i]);
    }
    // #111: người dùng vừa LƯU config (GUI/sửa file) — file trở lại là nguồn sự thật, lựa
    // chọn menu tạm của phiên hết hiệu lực (nếu không, IC mới sẽ đè ngược giá trị vừa lưu).
    sessionInputMethod_.clear();
    sessionCharset_.clear();
    instance_->inputContextManager().foreach([this](InputContext *ic) {
        pk_engine_reload_config(state(ic)->core());
        return true;
    });
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
    // #6: khi rời input method / mất focus, chốt phần đang soạn (không để kẹt/mất/đúp chữ).
    const bool imSwitch = event.type() == EventType::InputContextSwitchInputMethod;
    state(event.inputContext())->deactivate(imSwitch);
}

std::string PinaKeyEngine::subModeLabelImpl(const InputMethodEntry & /*entry*/,
                                            InputContext & /*ic*/) {
    // Hiển thị "V" khi PinaKey đang được chọn (đang gõ tiếng Việt).
    return "V";
}

std::string PinaKeyEngine::subModeIconImpl(const InputMethodEntry & /*entry*/,
                                           InputContext & /*ic*/) {
    // Rỗng → panel dùng nhãn "V" ở trên làm chỉ báo trạng thái.
    return {};
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

void PinaKeyEngine::applySessionOverrides(PkEngine *core) const {
    if (!sessionInputMethod_.empty()) {
        pk_engine_set_input_method(core, sessionInputMethod_.c_str());
    }
    if (!sessionCharset_.empty()) {
        pk_engine_set_charset(core, sessionCharset_.c_str());
    }
}

void PinaKeyEngine::applyInputMethod(const std::string &name) {
    sessionInputMethod_ = name;
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
    sessionCharset_ = name;
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
        // Kiểm phạm vi/surrogate trên giá trị gốc trong parseHexCodepoint — tránh truncate
        // 64→32 bit trước khi validate (loại surrogate vì UTF-8 của nó bị D-Bus từ chối). (#158)
        char32_t cp = 0;
        if (pinakey::parseHexCodepoint(hex, cp)) {
            emojiCandidates_.push_back(utf32ToUtf8(cp));
        }
    }

    // #63: query có chữ → tra fuzzy; query RỖNG (vừa mở ':') → pk_emoji_query trả lịch sử emoji
    // gần dùng làm candidate (chọn bằng phím số/click; Space/Enter vẫn chốt literal ':').
    {
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
        // #63: ghi vào lịch sử gần dùng — lần mở ':' sau, query rỗng sẽ hiện lại emoji này.
        pk_emoji_record_use(emojiCandidates_[index].c_str());
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
    const uint32_t state = static_cast<uint32_t>(keyEvent.rawKey().states());

    // Tổ hợp có Ctrl/Alt/Super/Meta (vd Ctrl+C, Alt+Tab) → thoát chế độ emoji và để phím đi tiếp,
    // KHÔNG nuốt thành ký tự truy vấn.
    if (state & kRealModMask) {
        cancelEmoji(true);
        return false;
    }

    // Chế độ hex (":u<hex>") cần gõ chữ số → KHÔNG dùng số để chọn candidate.
    const bool hexMode =
        emojiQuery_.size() >= 2 && (emojiQuery_[1] == 'u' || emojiQuery_[1] == 'U');

    if (!hexMode && sym >= FcitxKey_1 && sym <= FcitxKey_9) {
        const int idx = static_cast<int>(sym - FcitxKey_1);
        // #97: nhãn 1..9 là vị trí TRONG TRANG hiện tại — chọn qua candidate list của panel
        // (candidate(i) là mục thứ i của trang đang xem, như click), vì người dùng có thể đã
        // lật trang bằng chuột; chỉ số tuyệt đối vào emojiCandidates_ chỉ đúng ở trang đầu.
        if (auto cl = ic_->inputPanel().candidateList(); cl && idx < cl->size()) {
            cl->candidate(idx).select(ic_);
            keyEvent.filterAndAccept();
            return true;
        }
    }
    // #63: candidate lịch sử (query rỗng) KHÔNG auto-chọn bằng Enter/Space — ':' + Enter/Space
    // trong văn bản thường phải tiếp tục ra literal ':'; lịch sử chỉ chọn bằng phím số / click.
    const bool hasQuery = emojiQuery_.size() > 1;
    // #127: như phím số (#97), Enter/Space chọn mục 1 của TRANG đang xem — người dùng có thể
    // đã lật trang bằng chuột; emojiSelect(0) là chỉ số tuyệt đối nên chỉ đúng ở trang đầu.
    const auto selectFirstOnPage = [&] {
        if (auto cl = ic_->inputPanel().candidateList(); cl && cl->size() > 0) {
            cl->candidate(0).select(ic_);
        } else {
            emojiSelect(0);
        }
    };
    if (sym == FcitxKey_Return || sym == FcitxKey_KP_Enter) {
        if (hasQuery && !emojiCandidates_.empty()) {
            selectFirstOnPage();
            keyEvent.filterAndAccept();
            return true;
        }
        // Không có ứng viên: chốt ":query" như văn bản thường, để Enter đi tiếp (xuống dòng).
        cancelEmoji(true);
        return false;
    }
    if (sym == FcitxKey_space) {
        if (hasQuery && !emojiCandidates_.empty()) {
            selectFirstOnPage();
            keyEvent.filterAndAccept();
            return true;
        }
        // Không có ứng viên: chốt ":query", để DẤU CÁCH đi tiếp (không nuốt mất).
        cancelEmoji(true);
        return false;
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
