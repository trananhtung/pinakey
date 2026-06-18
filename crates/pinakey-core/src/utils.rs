//! Vowel / mark / tone tables and helpers — ported from `utils.go`.

use crate::rules::{Mark, Tone};
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub static VOWELS: Lazy<Vec<char>> = Lazy::new(|| {
    "aàáảãạăằắẳẵặâầấẩẫậeèéẻẽẹêềếểễệiìíỉĩịoòóỏõọôồốổỗộơờớởỡợuùúủũụưừứửữựyỳýỷỹỵ"
        .chars()
        .collect()
});

pub const PUNCTUATION_MARKS: &[char] = &[
    ',', ';', ':', '.', '"', '\'', '!', '?', ' ', '<', '>', '=', '+', '-', '*', '/', '\\', '_',
    '~', '`', '@', '#', '$', '%', '^', '&', '(', ')', '{', '}', '[', ']', '|',
];

/// Mark family for each base char. Each value has exactly 5 runes; `'_'` means "no char at this
/// mark position". Index = `Mark` value (None, Hat, Breve, Horn, Dash).
static MARKS_MAPS: Lazy<HashMap<char, [char; 5]>> = Lazy::new(|| {
    let raw: &[(char, &str)] = &[
        ('a', "aâă__"),
        ('â', "aâă__"),
        ('ă', "aâă__"),
        ('e', "eê___"),
        ('ê', "eê___"),
        ('o', "oô_ơ_"),
        ('ô', "oô_ơ_"),
        ('ơ', "oô_ơ_"),
        ('u', "u__ư_"),
        ('ư', "u__ư_"),
        ('d', "d___đ"),
        ('đ', "d___đ"),
    ];
    raw.iter()
        .map(|(k, s)| {
            let mut arr = ['_'; 5];
            for (i, c) in s.chars().enumerate() {
                arr[i] = c;
            }
            (*k, arr)
        })
        .collect()
});

pub fn is_space(key: char) -> bool {
    key == ' '
}

pub fn is_punctuation_mark(key: char) -> bool {
    PUNCTUATION_MARKS.contains(&key)
}

pub fn is_word_break_symbol(key: char) -> bool {
    is_punctuation_mark(key) || key.is_ascii_digit()
}

pub fn is_vowel(chr: char) -> bool {
    VOWELS.contains(&chr)
}

/// Position of `chr` in `VOWELS`, or -1.
pub fn find_vowel_position(chr: char) -> isize {
    VOWELS
        .iter()
        .position(|&v| v == chr)
        .map_or(-1, |p| p as isize)
}

fn get_mark_family(chr: char) -> Vec<char> {
    let mut result = Vec::new();
    if let Some(arr) = MARKS_MAPS.get(&chr) {
        for &c in arr.iter() {
            if c != '_' {
                result.push(c);
            }
        }
    }
    result
}

/// Position of `chr` within its own mark family string, or -1 (matches Go `FindMarkPosition`).
pub fn find_mark_position(chr: char) -> isize {
    if let Some(arr) = MARKS_MAPS.get(&chr) {
        for (pos, &v) in arr.iter().enumerate() {
            if v == chr {
                return pos as isize;
            }
        }
    }
    -1
}

pub fn find_mark_from_char(chr: char) -> Option<Mark> {
    let pos = find_mark_position(chr);
    if pos >= 0 {
        Some(match pos {
            1 => Mark::Hat,
            2 => Mark::Breve,
            3 => Mark::Horn,
            4 => Mark::Dash,
            5 => Mark::Raw,
            _ => Mark::None,
        })
    } else {
        None
    }
}

pub fn add_mark_to_toneless_char(chr: char, mark: u8) -> char {
    if let Some(arr) = MARKS_MAPS.get(&chr) {
        let idx = mark as usize;
        if idx < arr.len() && arr[idx] != '_' {
            return arr[idx];
        }
    }
    chr
}

pub fn add_mark_to_char(chr: char, mark: u8) -> char {
    let tone = find_tone_from_char(chr);
    let chr = add_tone_to_char(chr, 0);
    let chr = add_mark_to_toneless_char(chr, mark);
    add_tone_to_char(chr, tone as u8)
}

pub fn is_alpha(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_uppercase()
}

pub fn in_key_list(keys: &[char], key: char) -> bool {
    keys.contains(&key)
}

pub fn find_tone_from_char(chr: char) -> Tone {
    let pos = find_vowel_position(chr);
    if pos == -1 {
        return Tone::None;
    }
    Tone::from_u8((pos % 6) as u8)
}

pub fn add_tone_to_char(chr: char, tone: u8) -> char {
    let pos = find_vowel_position(chr);
    if pos > -1 {
        let current_tone = pos % 6;
        let offset = tone as isize - current_tone;
        VOWELS[(pos + offset) as usize]
    } else {
        chr
    }
}

pub fn can_process_key(lower_key: char, effect_keys: &[char]) -> bool {
    if is_alpha(lower_key) || in_key_list(effect_keys, lower_key) {
        return true;
    }
    if is_word_break_symbol(lower_key) {
        return false;
    }
    is_vietnamese_rune(lower_key)
}

pub fn is_vietnamese_rune(lower_key: char) -> bool {
    if find_tone_from_char(lower_key) != Tone::None {
        return true;
    }
    lower_key != add_mark_to_toneless_char(lower_key, 0)
}

pub fn has_any_vietnamese_rune(word: &str) -> bool {
    word.chars().any(|chr| is_vietnamese_rune(to_lower(chr)))
}

pub fn has_any_vietnamese_vowel(word: &str) -> bool {
    word.chars().any(|chr| is_vowel(to_lower(chr)))
}

/// Helper mirroring Go's `unicode.ToLower` for a single rune as used by this engine.
pub fn to_lower(c: char) -> char {
    // Go's unicode.ToLower returns a single rune for all chars used here.
    let mut it = c.to_lowercase();
    let first = it.next().unwrap_or(c);
    if it.next().is_some() {
        c // multi-char lowercasing not expected in this domain; keep original
    } else {
        first
    }
}

/// Helper mirroring Go's `unicode.ToUpper` for a single rune.
pub fn to_upper(c: char) -> char {
    let mut it = c.to_uppercase();
    let first = it.next().unwrap_or(c);
    if it.next().is_some() {
        c
    } else {
        first
    }
}

/// Helper mirroring Go's `unicode.IsUpper`.
pub fn is_upper(c: char) -> bool {
    c.is_uppercase()
}

// Re-export so the parser can reference the mark family without duplicating the table.
pub(crate) fn mark_family(chr: char) -> Vec<char> {
    get_mark_family(chr)
}
