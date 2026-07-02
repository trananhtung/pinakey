/* PinaKey C-ABI — để addon fcitx5 C++ dùng lại lõi engine Rust. GPL-3.0-or-later. */

#ifndef PINAKEY_FFI_H
#define PINAKEY_FFI_H

#pragma once

/* CẢNH BÁO: file sinh tự động bởi cbindgen. Đừng sửa tay. Tạo lại: tools/gen-ffi-header.sh */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Trạng thái engine cho một input context fcitx5. Con trỏ mờ phía C.
 */
typedef struct PkEngine PkEngine;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Tạo engine với cấu hình mặc định (Telex, Unicode, cờ chuẩn).
 */
PkEngine *pk_engine_new(void);

/**
 * Tạo engine, nạp cấu hình người dùng theo `engine_name` (file
 * `~/.config/pinakey/ibus-<name>.config.json`); thiếu file thì dùng mặc định.
 *
 * # Safety
 * `name` là chuỗi C NUL-terminated hợp lệ hoặc null.
 */
PkEngine *pk_engine_new_from_name(const char *name);

/**
 * Tạo engine từ chuỗi JSON cấu hình; null hoặc JSON sai → cấu hình mặc định.
 *
 * # Safety
 * `json` là chuỗi C NUL-terminated hợp lệ hoặc null.
 */
PkEngine *pk_engine_new_from_json(const char *json);

/**
 * Giải phóng engine. Sau lời gọi này, mọi con trỏ chuỗi đã lấy ra đều không còn hợp lệ.
 *
 * # Safety
 * `e` phải do `pk_engine_new*` trả về và chưa từng được free; hoặc null (không làm gì).
 */
void pk_engine_free(PkEngine *e);

/**
 * Xử lý một sự kiện phím. `keyval` là keysym X11/fcitx5, `state` là mặt nạ modifier (bật bit
 * `MOD_RELEASE = 1<<30` nếu là phím nhả). Trả về `true` nếu engine đã "nuốt" phím (C++ gọi
 * `keyEvent.filterAndAccept()`); `false` thì C++ để phím đi tiếp.
 *
 * Sau khi gọi, đọc kết quả qua `pk_engine_commit` / `pk_engine_preedit*`.
 *
 * # Safety
 * `e` phải là con trỏ engine hợp lệ.
 */
bool pk_engine_process_key(PkEngine *e,
                           uint32_t keyval,
                           uint32_t state);

/**
 * Xử lý phím cho chế độ **gõ không gạch chân**: thay vì preedit, trả về một lệnh thay thế. Sau khi
 * gọi, C++ đọc `pk_engine_replace_delete` (số ký tự cuối cần xoá) và `pk_engine_replace_insert`
 * (chuỗi cần chèn) rồi áp bằng `deleteSurroundingText(-n, n)` + `commitString`. Trả về `handled`.
 *
 * # Safety
 * `e` phải là con trỏ engine hợp lệ.
 */
bool pk_engine_process_key_replace(PkEngine *e,
                                   uint32_t keyval,
                                   uint32_t state);

/**
 * Số ký tự (Unicode) ở cuối cần xoá khỏi tài liệu cho lần `process_key_replace` gần nhất.
 *
 * # Safety
 * `e` hợp lệ.
 */
uint32_t pk_engine_replace_delete(const PkEngine *e);

/**
 * Chuỗi segment mà engine TIN là đang hiển thị ngay trước con trỏ trong tài liệu (`prev_displayed`).
 * C++ đối chiếu với surrounding text trước con trỏ để phát hiện con trỏ đã nhảy / văn bản đổi
 * (khi đó phải reset trước khi xử lý phím, tránh deleteSurroundingText xoá nhầm). Con trỏ trả về
 * hợp lệ tới lần gọi kế tiếp; C++ phải copy ngay nếu cần giữ.
 *
 * # Safety
 * `e` phải là con trỏ engine hợp lệ.
 */
const char *pk_engine_replace_segment(const PkEngine *e);

/**
 * Chuỗi cần chèn (commit) cho lần `process_key_replace` gần nhất.
 *
 * # Safety
 * `e` hợp lệ; con trỏ trả về dùng được tới lần gọi kế tiếp.
 */
const char *pk_engine_replace_insert(const PkEngine *e);

/**
 * Người dùng có bật chế độ "gõ không gạch chân" không (cờ IB_NO_UNDERLINE). C++ dùng cờ này (cùng
 * với khả năng SurroundingText của ứng dụng) để chọn giữa chế độ replace và preedit.
 *
 * # Safety
 * `e` hợp lệ.
 */
bool pk_engine_no_underline(const PkEngine *e);

/**
 * Chuỗi cần commit từ lần `process_key` gần nhất (rỗng nếu không có gì để commit).
 *
 * # Safety
 * `e` hợp lệ; con trỏ trả về chỉ dùng được tới lần `process_key`/`reset`/`free` kế tiếp.
 */
const char *pk_engine_commit(const PkEngine *e);

/**
 * Văn bản preedit hiện tại (rỗng nếu không hiển thị preedit).
 *
 * # Safety
 * Như `pk_engine_commit`.
 */
const char *pk_engine_preedit(const PkEngine *e);

/**
 * Vị trí con trỏ trong preedit (số ký tự).
 *
 * # Safety
 * `e` hợp lệ.
 */
uint32_t pk_engine_preedit_cursor(const PkEngine *e);

/**
 * Preedit có nên hiển thị không.
 *
 * # Safety
 * `e` hợp lệ.
 */
bool pk_engine_preedit_visible(const PkEngine *e);

/**
 * Engine có đang soạn dở một segment không (preedit hiển thị, hoặc đang theo dõi đoạn ở chế độ
 * không-gạch-chân). C++ dùng để biết có nên kích hoạt tra emoji bằng `:` hay không (issue #11/#26).
 *
 * # Safety
 * `e` hợp lệ.
 */
bool pk_engine_is_composing(const PkEngine *e);

/**
 * Preedit có nên gạch chân không (theo cờ IB_NO_UNDERLINE của người dùng).
 *
 * # Safety
 * `e` hợp lệ.
 */
bool pk_engine_preedit_underline(const PkEngine *e);

/**
 * Nạp lại file macro + từ điển từ đĩa (issue #20, live-reload) mà không đổi cấu hình đang chạy.
 *
 * # Safety
 * `e` hợp lệ.
 */
void pk_engine_reload(PkEngine *e);

/**
 * Đặt lại buffer soạn thảo (tương ứng `reset()` của fcitx5 khi đổi focus/huỷ).
 *
 * # Safety
 * `e` hợp lệ.
 */
void pk_engine_reset(PkEngine *e);

/**
 * Kết thúc phiên soạn khi mất focus (issue #6): trả về phần preedit đang hiển thị để C++ commit
 * (tránh kẹt/mất chữ), rồi reset engine. Dùng cho chế độ preedit; ở chế độ gõ-không-gạch-chân
 * văn bản đã nằm sẵn trong tài liệu nên C++ chỉ gọi `pk_engine_reset`.
 *
 * # Safety
 * `e` hợp lệ; con trỏ trả về dùng được tới lần gọi kế tiếp.
 */
const char *pk_engine_flush_preedit(PkEngine *e);

/**
 * Đặt tên chương trình của input context (vd `firefox`) để bật cách khắc phục theo ứng dụng.
 *
 * # Safety
 * `e` hợp lệ; `program` là chuỗi C hợp lệ hoặc null.
 */
void pk_engine_set_program(PkEngine *e,
                           const char *program);

/**
 * Chương trình đang focus (đặt qua `pk_engine_set_program`) có nằm trong danh sách loại trừ tiếng
 * Anh không (issue #9). C++ dùng cờ này để cho phím đi thẳng (pass-through), không gõ tiếng Việt.
 *
 * # Safety
 * `e` hợp lệ.
 */
bool pk_engine_program_excluded(const PkEngine *e);

/**
 * Surrounding text của chương trình đang focus (đặt qua `pk_engine_set_program`) có KHÔNG đáng
 * tin không (issue #66). LibreOffice (soffice) báo surrounding text lạc hậu/thiếu dấu cách khi gõ
 * nhanh → C++ phải bỏ qua đường diff-replace và dùng preedit cho các app này dù chúng quảng cáo
 * khả năng SurroundingText.
 *
 * # Safety
 * `e` hợp lệ.
 */
bool pk_engine_surrounding_text_unreliable(const PkEngine *e);

/**
 * #65: dấu cách kế tiếp có nên biến thành ". " không (double-space kết câu nhanh, cần bật
 * option). C++ hỏi TRƯỚC khi đưa phím space vào engine; nếu `true` và app cho xoá surrounding
 * text thì C++ tự xoá dấu cách cũ + commit ". " rồi gọi `pk_engine_double_space_consume`.
 *
 * # Safety
 * `e` hợp lệ.
 */
bool pk_engine_double_space_armed(const PkEngine *e);

/**
 * #65: báo engine rằng addon đã thực hiện double-space→". " (đóng cửa sổ; nếu bật viết hoa
 * đầu câu thì chữ cái kế tiếp sẽ tự hoa).
 *
 * # Safety
 * `e` hợp lệ.
 */
void pk_engine_double_space_consume(PkEngine *e);

/**
 * Đổi kiểu gõ ("Telex" / "VNI" / "VIQR" …) và dựng lại engine biến đổi.
 *
 * # Safety
 * `e` hợp lệ; `name` là chuỗi C hợp lệ hoặc null (null = không đổi).
 */
void pk_engine_set_input_method(PkEngine *e, const char *name);

/**
 * Đổi bảng mã đầu ra ("Unicode", "TCVN3", …).
 *
 * # Safety
 * `e` hợp lệ; `name` là chuỗi C hợp lệ hoặc null (null = không đổi).
 */
void pk_engine_set_charset(PkEngine *e, const char *name);

/**
 * Tên kiểu gõ hiện tại.
 *
 * # Safety
 * `e` hợp lệ; con trỏ trả về hợp lệ tới lần đổi cấu hình kế tiếp.
 */
const char *pk_engine_input_method(const PkEngine *e);

/**
 * Tên bảng mã hiện tại.
 *
 * # Safety
 * `e` hợp lệ; con trỏ trả về hợp lệ tới lần đổi cấu hình kế tiếp.
 */
const char *pk_engine_charset(const PkEngine *e);

/**
 * Số kiểu gõ dựng sẵn.
 */
uint32_t pk_input_method_count(void);

/**
 * Tên kiểu gõ thứ `i` (rỗng nếu ngoài phạm vi).
 *
 * # Safety
 * Con trỏ trả về sống suốt vòng đời tiến trình.
 */
const char *pk_input_method_name_at(uint32_t i);

/**
 * Số bảng mã đầu ra.
 */
uint32_t pk_charset_count(void);

/**
 * Tên bảng mã thứ `i` (rỗng nếu ngoài phạm vi).
 *
 * # Safety
 * Con trỏ trả về sống suốt vòng đời tiến trình.
 */
const char *pk_charset_name_at(uint32_t i);

/**
 * Tra emoji theo `query` — fuzzy trên shortname (`heart_eyes`, gõ tắt `heye` vẫn khớp), keyword
 * và ascii (":)"), kết quả xếp theo độ khớp. **Query rỗng → danh sách emoji gần dùng** (mới nhất
 * trước). Trả về mỗi dòng một emoji, phân tách bằng `\n` (tối đa 60). Con trỏ trả về hợp lệ tới
 * lần gọi `pk_emoji_query` kế tiếp TRÊN CÙNG THREAD; C++ phải sao chép ngay.
 *
 * # Safety
 * `query` là chuỗi C hợp lệ hoặc null.
 */
const char *pk_emoji_query(const char *query);

/**
 * Ghi nhận một emoji vừa được commit (#63): đưa lên đầu lịch sử gần dùng và persist best-effort
 * (lỗi ghi đĩa chỉ cảnh báo ra stderr, không làm hỏng phiên gõ).
 *
 * # Safety
 * `emoji` là chuỗi C hợp lệ hoặc null.
 */
void pk_emoji_record_use(const char *emoji);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* PINAKEY_FFI_H */
