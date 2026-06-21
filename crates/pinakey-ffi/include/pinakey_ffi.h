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
 * Preedit có nên gạch chân không (theo cờ IB_NO_UNDERLINE của người dùng).
 *
 * # Safety
 * `e` hợp lệ.
 */
bool pk_engine_preedit_underline(const PkEngine *e);

/**
 * Đặt lại buffer soạn thảo (tương ứng `reset()` của fcitx5 khi đổi focus/huỷ).
 *
 * # Safety
 * `e` hợp lệ.
 */
void pk_engine_reset(PkEngine *e);

/**
 * Đặt tên chương trình của input context (vd `firefox`) để bật cách khắc phục theo ứng dụng.
 *
 * # Safety
 * `e` hợp lệ; `program` là chuỗi C hợp lệ hoặc null.
 */
void pk_engine_set_program(PkEngine *e,
                           const char *program);

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

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* PINAKEY_FFI_H */
