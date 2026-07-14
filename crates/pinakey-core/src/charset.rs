//! Mã hoá bảng ký tự (charset) — chuyển từ `encoder.go` (dữ liệu nằm trong `charset_def.rs`).
//!
//! Các bảng mã tiếng Việt cũ là bảng mã theo byte, nên `encode` trả về `Vec<u8>` (một `string`
//! trong Go vốn là chuỗi byte). Riêng `UNICODE` thì đầu vào được trả về nguyên vẹn.

use crate::charset_def::charset_definitions;
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub const UNICODE: &str = "Unicode";

type Charset = HashMap<char, &'static [u8]>;

static CHARSETS: Lazy<HashMap<&'static str, Charset>> = Lazy::new(|| {
    let mut m: HashMap<&'static str, Charset> = HashMap::new();
    for (name, entries) in charset_definitions() {
        let mut cs: Charset = HashMap::new();
        for (chr, bytes) in entries {
            cs.insert(chr, bytes);
        }
        m.insert(name, cs);
    }
    m
});

/// Mã hoá `input` theo bảng mã có tên cho trước, trả về chuỗi byte thô.
pub fn encode(charset_name: &str, input: &str) -> Vec<u8> {
    if charset_name == UNICODE {
        return input.as_bytes().to_vec();
    }
    match CHARSETS.get(charset_name) {
        Some(charset) => {
            let mut output = Vec::new();
            for chr in input.chars() {
                match charset.get(&chr) {
                    Some(out) => output.extend_from_slice(out),
                    None => {
                        let mut buf = [0u8; 4];
                        output.extend_from_slice(chr.encode_utf8(&mut buf).as_bytes());
                    }
                }
            }
            output
        }
        None => input.as_bytes().to_vec(),
    }
}

/// Danh sách tên tất cả bảng mã hiện có, với `UNICODE` đứng đầu.
///
/// Thứ tự sau `UNICODE` là **thứ tự khai báo** của `charset_definitions()` (một `Vec`), nên
/// tất định giữa các phiên — không duyệt `CHARSETS` (HashMap) vì thứ tự iteration ngẫu nhiên
/// theo tiến trình sẽ làm menu "Bảng mã" nhảy vị trí mỗi lần khởi động.
pub fn get_charset_names() -> Vec<String> {
    let mut names = vec![UNICODE.to_string()];
    for (name, _entries) in charset_definitions() {
        names.push(name.to_string());
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charset_names_unicode_dau_tien() {
        assert_eq!(get_charset_names()[0], UNICODE);
    }

    #[test]
    fn charset_names_theo_thu_tu_khai_bao() {
        // Thứ tự sau "Unicode" phải khớp thứ tự khai báo của charset_definitions().
        let expected: Vec<String> = std::iter::once(UNICODE.to_string())
            .chain(
                charset_definitions()
                    .into_iter()
                    .map(|(name, _)| name.to_string()),
            )
            .collect();
        assert_eq!(get_charset_names(), expected);
    }

    #[test]
    fn charset_names_tat_dinh_giua_cac_lan_goi() {
        // Chống tái diễn issue #165: duyệt HashMap cho thứ tự ngẫu nhiên mỗi tiến trình.
        // Trong cùng một tiến trình, RandomState cố định nên nhiều lần gọi vẫn giống nhau;
        // phép so sánh này khẳng định get_charset_names() không phụ thuộc iteration HashMap.
        let a = get_charset_names();
        let b = get_charset_names();
        assert_eq!(a, b);
        // Và thứ tự phải là thứ tự Vec khai báo, không phải thứ tự keys() của HashMap.
        let declared: Vec<String> = charset_definitions()
            .into_iter()
            .map(|(name, _)| name.to_string())
            .collect();
        assert_eq!(&a[1..], declared.as_slice());
    }
}
