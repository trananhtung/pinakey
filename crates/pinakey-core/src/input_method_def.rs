//! Các bảng định nghĩa kiểu gõ — chuyển từ `input_method_def.go`.
//!
//! Go dùng `map[string]string`; ở đây ta dùng `Vec<(&str, &str)>` có thứ tự để việc phân tích
//! mang tính tất định (Go duyệt map theo thứ tự ngẫu nhiên, còn engine thì không phụ thuộc thứ tự
//! đối với khoá đã khớp).

use std::collections::HashMap;

pub type InputMethodDefinition = Vec<(&'static str, &'static str)>;

/// Tên kiểu gõ "Telex đơn giản / hạn chế" (issue #16).
pub const SIMPLE_TELEX: &str = "Telex (đơn giản)";

/// Kiểu gõ này có phải Telex đơn giản không. Khi đúng, lớp dựng engine tắt `FREE_TONE_MARKING`
/// để gõ dấu chặt chẽ (dấu áp ngay, không tự dời) — giống tuỳ chọn restricted Telex của Bamboo.
pub fn is_simple_telex(name: &str) -> bool {
    name == SIMPLE_TELEX
}

/// Dạng owned của tất cả định nghĩa, theo cấu trúc `name -> (key -> dòng quy tắc)`, để lưu trong
/// config và round-trip qua JSON (tương ứng với `GetInputMethodDefinitions` của Go).
pub fn input_method_definitions_owned() -> HashMap<String, HashMap<String, String>> {
    input_method_definitions()
        .into_iter()
        .map(|(name, def)| {
            (
                name.to_string(),
                def.into_iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            )
        })
        .collect()
}

/// Trả về danh sách các cặp `(name, definition)`, giữ nguyên thứ tự ổn định.
pub fn input_method_definitions() -> Vec<(&'static str, InputMethodDefinition)> {
    vec![
        (
            "Telex",
            vec![
                ("z", "XoaDauThanh"),
                ("s", "DauSac"),
                ("f", "DauHuyen"),
                ("r", "DauHoi"),
                ("x", "DauNga"),
                ("j", "DauNang"),
                ("a", "A_Â"),
                ("e", "E_Ê"),
                ("o", "O_Ô"),
                ("w", "UOA_ƯƠĂ"),
                ("d", "D_Đ"),
            ],
        ),
        (
            // Telex "đơn giản / hạn chế" (issue #16): cùng bộ phím với Telex chuẩn, nhưng khi chọn
            // kiểu này engine tắt FREE_TONE_MARKING (gõ dấu chặt chẽ — dấu áp ngay, không tự dời),
            // giống tuỳ chọn restricted Telex của Bamboo. Việc tắt cờ do build engine xử lý dựa trên
            // tên kiểu gõ (xem `is_simple_telex`).
            "Telex (đơn giản)",
            vec![
                ("z", "XoaDauThanh"),
                ("s", "DauSac"),
                ("f", "DauHuyen"),
                ("r", "DauHoi"),
                ("x", "DauNga"),
                ("j", "DauNang"),
                ("a", "A_Â"),
                ("e", "E_Ê"),
                ("o", "O_Ô"),
                ("w", "UOA_ƯƠĂ"),
                ("d", "D_Đ"),
            ],
        ),
        (
            "VNI",
            vec![
                ("0", "XoaDauThanh"),
                ("1", "DauSac"),
                ("2", "DauHuyen"),
                ("3", "DauHoi"),
                ("4", "DauNga"),
                ("5", "DauNang"),
                ("6", "AEO_ÂÊÔ"),
                ("7", "UO_ƯƠ"),
                ("8", "A_Ă"),
                ("9", "D_Đ"),
            ],
        ),
        (
            "VIQR",
            vec![
                ("0", "XoaDauThanh"),
                ("'", "DauSac"),
                ("`", "DauHuyen"),
                ("?", "DauHoi"),
                ("~", "DauNga"),
                (".", "DauNang"),
                ("^", "AEO_ÂÊÔ"),
                ("+", "UO_ƯƠ"),
                ("*", "UO_ƯƠ"),
                ("(", "A_Ă"),
                ("d", "D_Đ"),
            ],
        ),
        (
            "Microsoft layout",
            vec![
                ("8", "DauSac"),
                ("5", "DauHuyen"),
                ("6", "DauHoi"),
                ("7", "DauNga"),
                ("9", "DauNang"),
                ("1", "__ă"),
                ("!", "_Ă"),
                ("2", "__â"),
                ("@", "_Â"),
                ("3", "__ê"),
                ("#", "_Ê"),
                ("4", "__ô"),
                ("$", "_Ô"),
                ("0", "__đ"),
                (")", "_Đ"),
                ("[", "__ư"),
                ("{", "_Ư"),
                ("]", "__ơ"),
                ("}", "_Ơ"),
            ],
        ),
        (
            "Telex 2",
            vec![
                ("z", "XoaDauThanh"),
                ("s", "DauSac"),
                ("f", "DauHuyen"),
                ("r", "DauHoi"),
                ("x", "DauNga"),
                ("j", "DauNang"),
                ("a", "A_Â"),
                ("e", "E_Ê"),
                ("o", "O_Ô"),
                ("w", "UOA_ƯƠĂ__Ư"),
                ("d", "D_Đ"),
                ("]", "__ư"),
                ("[", "__ơ"),
                ("}", "_Ư"),
                ("{", "_Ơ"),
            ],
        ),
        (
            "Telex + VNI",
            vec![
                ("z", "XoaDauThanh"),
                ("s", "DauSac"),
                ("f", "DauHuyen"),
                ("r", "DauHoi"),
                ("x", "DauNga"),
                ("j", "DauNang"),
                ("a", "A_Â"),
                ("e", "E_Ê"),
                ("o", "O_Ô"),
                ("w", "UOA_ƯƠĂ"),
                ("d", "D_Đ"),
                ("0", "XoaDauThanh"),
                ("1", "DauSac"),
                ("2", "DauHuyen"),
                ("3", "DauHoi"),
                ("4", "DauNga"),
                ("5", "DauNang"),
                ("6", "AEO_ÂÊÔ"),
                ("7", "UO_ƯƠ"),
                ("8", "A_Ă"),
                ("9", "D_Đ"),
            ],
        ),
        (
            "Telex + VNI + VIQR",
            vec![
                ("z", "XoaDauThanh"),
                ("s", "DauSac"),
                ("f", "DauHuyen"),
                ("r", "DauHoi"),
                ("x", "DauNga"),
                ("j", "DauNang"),
                ("a", "A_Â"),
                ("e", "E_Ê"),
                ("o", "O_Ô"),
                ("w", "UOA_ƯƠĂ"),
                ("d", "D_Đ"),
                ("0", "XoaDauThanh"),
                ("1", "DauSac"),
                ("2", "DauHuyen"),
                ("3", "DauHoi"),
                ("4", "DauNga"),
                ("5", "DauNang"),
                ("6", "AEO_ÂÊÔ"),
                ("7", "UO_ƯƠ"),
                ("8", "A_Ă"),
                ("9", "D_Đ"),
                ("'", "DauSac"),
                ("`", "DauHuyen"),
                ("?", "DauHoi"),
                ("~", "DauNga"),
                (".", "DauNang"),
                ("^", "AEO_ÂÊÔ"),
                ("+", "UO_ƯƠ"),
                ("*", "UO_ƯƠ"),
                ("(", "A_Ă"),
                ("\\", "D_Đ"),
            ],
        ),
        (
            "VNI Bàn phím tiếng Pháp",
            vec![
                ("&", "XoaDauThanh"),
                ("é", "DauSac"),
                ("\"", "DauHuyen"),
                ("'", "DauHoi"),
                ("(", "DauNga"),
                ("-", "DauNang"),
                ("è", "AEO_ÂÊÔ"),
                ("_", "UO_ƯƠ"),
                ("ç", "A_Ă"),
                ("à", "D_Đ"),
            ],
        ),
        (
            "Telex W",
            vec![
                ("z", "XoaDauThanh"),
                ("s", "DauSac"),
                ("f", "DauHuyen"),
                ("r", "DauHoi"),
                ("x", "DauNga"),
                ("j", "DauNang"),
                ("a", "A_Â"),
                ("e", "E_Ê"),
                ("o", "O_Ô"),
                ("w", "UOA_ƯƠĂ__Ư"),
                ("d", "D_Đ"),
            ],
        ),
    ]
}
