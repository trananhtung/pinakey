//! Charset encoding — ported from `encoder.go` (data in `charset_def.rs`).
//!
//! Legacy Vietnamese charsets are byte encodings, so `encode` returns `Vec<u8>` (a Go `string`
//! is a byte sequence). For `UNICODE`, the input is returned unchanged.

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

/// Encode `input` into the named charset, returning raw bytes.
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

/// All available charset names, with `UNICODE` first.
pub fn get_charset_names() -> Vec<String> {
    let mut names = vec![UNICODE.to_string()];
    for name in CHARSETS.keys() {
        names.push((*name).to_string());
    }
    names
}
