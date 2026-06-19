//! Logic cho các chế độ nhập "sửa lỗi" (backspace-correction) — chuyển thể từ `engine_backspace.go`.
//!
//! Khác với chế độ Preedit (giữ văn bản chưa chốt trong vùng preedit gạch chân), các chế độ này
//! commit thẳng văn bản ra ứng dụng rồi chỉnh sửa bằng cách **xóa lùi** (backspace) phần đã thay
//! đổi và gõ lại phần đuôi. Phần tính toán "cần xóa mấy ký tự, gõ lại gì" là logic thuần, độc lập
//! với cách phát phím (XTest / forward key / surrounding text) nên được unit-test đầy đủ ở đây.

use pinakey_config::flags as cfg;

use crate::core::Action;

/// Một phép hiệu chỉnh để biến chuỗi đang hiển thị `old` thành `new`: xóa lùi `backspaces` ký tự
/// rồi chèn `insert`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    pub backspaces: usize,
    pub insert: String,
}

/// Tính phần đuôi khác nhau giữa `old` và `new` dựa trên tiền tố chung (so theo ký tự Unicode, vì
/// app xóa/chèn theo ký tự chứ không theo byte).
pub fn diff_correction(old: &str, new: &str) -> Correction {
    let old_chars: Vec<char> = old.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    let max = old_chars.len().min(new_chars.len());
    let mut common = 0;
    while common < max && old_chars[common] == new_chars[common] {
        common += 1;
    }
    Correction {
        backspaces: old_chars.len() - common,
        insert: new_chars[common..].iter().collect(),
    }
}

/// Dịch một [`Correction`] thành danh sách [`Action`] tùy theo chế độ nhập đang dùng:
/// - [`cfg::SURROUNDING_TEXT_IM`] → xóa qua `delete_surrounding_text`.
/// - [`cfg::XTEST_FAKE_KEY_EVENT_IM`] → xóa qua tiêm phím XTest (chỉ X11/XWayland).
/// - còn lại (forward key event) → phát N phím BackSpace, chạy được cả trên Wayland.
pub fn correction_actions(input_mode: i32, corr: &Correction) -> Vec<Action> {
    let mut out = Vec::new();
    if corr.backspaces > 0 {
        let n = corr.backspaces as u32;
        match input_mode {
            cfg::SURROUNDING_TEXT_IM => out.push(Action::DeleteSurroundingText {
                offset: -(n as i32),
                nchars: n,
            }),
            cfg::XTEST_FAKE_KEY_EVENT_IM => out.push(Action::FakeBackspaces(n)),
            // BACKSPACE_FORWARDING / SHIFT_LEFT_FORWARDING / FORWARD_AS_COMMIT: cùng đi qua
            // forward_key_event (phát phím BackSpace), là đường chạy được trên cả Wayland.
            _ => out.push(Action::ForwardBackspaces(n)),
        }
    }
    if !corr.insert.is_empty() {
        out.push(Action::CommitText(corr.insert.clone()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_keeps_common_prefix() {
        // gõ dấu nặng: "viet" -> "việt": chung tiền tố "vi", xóa "et", chèn "ệt".
        let c = diff_correction("viet", "việt");
        assert_eq!(
            c,
            Correction {
                backspaces: 2,
                insert: "ệt".to_string()
            }
        );
    }

    #[test]
    fn diff_append_only_no_backspace() {
        let c = diff_correction("vi", "vie");
        assert_eq!(
            c,
            Correction {
                backspaces: 0,
                insert: "e".to_string()
            }
        );
    }

    #[test]
    fn diff_identical_is_noop() {
        let c = diff_correction("việt", "việt");
        assert_eq!(
            c,
            Correction {
                backspaces: 0,
                insert: String::new()
            }
        );
    }

    #[test]
    fn diff_shrink_inserts_nothing() {
        // xóa bớt: "việt" -> "việ".
        let c = diff_correction("việt", "việ");
        assert_eq!(
            c,
            Correction {
                backspaces: 1,
                insert: String::new()
            }
        );
    }

    #[test]
    fn diff_counts_unicode_chars_not_bytes() {
        // "đ" là 2 byte UTF-8 nhưng chỉ 1 ký tự cần 1 backspace.
        let c = diff_correction("đa", "đ");
        assert_eq!(c.backspaces, 1);
        assert_eq!(c.insert, "");
    }

    #[test]
    fn actions_surrounding_text_uses_delete_surrounding() {
        let corr = Correction {
            backspaces: 2,
            insert: "ệt".to_string(),
        };
        let acts = correction_actions(cfg::SURROUNDING_TEXT_IM, &corr);
        assert_eq!(
            acts,
            vec![
                Action::DeleteSurroundingText {
                    offset: -2,
                    nchars: 2
                },
                Action::CommitText("ệt".to_string()),
            ]
        );
    }

    #[test]
    fn actions_xtest_uses_fake_backspaces() {
        let corr = Correction {
            backspaces: 3,
            insert: "x".to_string(),
        };
        let acts = correction_actions(cfg::XTEST_FAKE_KEY_EVENT_IM, &corr);
        assert_eq!(
            acts,
            vec![
                Action::FakeBackspaces(3),
                Action::CommitText("x".to_string())
            ]
        );
    }

    #[test]
    fn actions_forwarding_uses_forward_backspaces() {
        let corr = Correction {
            backspaces: 1,
            insert: "ê".to_string(),
        };
        let acts = correction_actions(cfg::BACKSPACE_FORWARDING_IM, &corr);
        assert_eq!(
            acts,
            vec![
                Action::ForwardBackspaces(1),
                Action::CommitText("ê".to_string())
            ]
        );
    }

    #[test]
    fn actions_no_backspace_only_commits() {
        let corr = Correction {
            backspaces: 0,
            insert: "v".to_string(),
        };
        let acts = correction_actions(cfg::BACKSPACE_FORWARDING_IM, &corr);
        assert_eq!(acts, vec![Action::CommitText("v".to_string())]);
    }

    #[test]
    fn actions_empty_insert_only_deletes() {
        let corr = Correction {
            backspaces: 2,
            insert: String::new(),
        };
        let acts = correction_actions(cfg::SURROUNDING_TEXT_IM, &corr);
        assert_eq!(
            acts,
            vec![Action::DeleteSurroundingText {
                offset: -2,
                nchars: 2
            }]
        );
    }
}
