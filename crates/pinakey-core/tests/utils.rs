//! Character-utility tests, ported verbatim from the upstream Go suite `utils_test.go`.

use pinakey_core::{add_mark_to_char, add_tone_to_char, find_tone_from_char, is_vowel, Mark, Tone};

#[test]
fn test_is_vowel() {
    assert!(is_vowel('a'));
    assert!(is_vowel('á'));
    assert!(!is_vowel('b'));
    let tvowels = "aàáảãạăằắẳẵặâầấẩẫậeèéẻẽẹêềếểễệiìíỉĩịoòóỏõọôồốổỗộơờớởỡợuùúủũụưừứửữựyỳýỷỹỵ";
    for v in tvowels.chars() {
        assert!(is_vowel(v), "{} should be a vowel", v);
    }
}

#[test]
fn test_get_tone_from_char() {
    assert_eq!(find_tone_from_char('e'), Tone::None);
    assert_eq!(find_tone_from_char('è'), Tone::Grave);
    assert_eq!(find_tone_from_char('é'), Tone::Acute);
    assert_eq!(find_tone_from_char('ẽ'), Tone::Tilde);
    assert_eq!(find_tone_from_char('ẻ'), Tone::Hook);
    assert_eq!(find_tone_from_char('ạ'), Tone::Dot);
}

#[test]
fn test_add_tone_to_char() {
    assert_eq!(add_tone_to_char('a', Tone::Dot as u8), 'ạ');
    assert_eq!(add_tone_to_char('y', 0), 'y');
    assert_eq!(add_mark_to_char('y', 0), 'y');
}

#[test]
fn test_add_mark_to_char() {
    assert_eq!(add_mark_to_char('ạ', Mark::Breve as u8), 'ặ');
}
