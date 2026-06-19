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
pub fn get_charset_names() -> Vec<String> {
    let mut names = vec![UNICODE.to_string()];
    for name in CHARSETS.keys() {
        names.push((*name).to_string());
    }
    names
}
