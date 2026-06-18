//! CVC spelling validation — ported from `spelling.go`.
//!
//! A Vietnamese syllable splits into first-consonant (C), vowel (V) and last-consonant (C).
//! Each part is looked up in a sequence table; pairs are validated via the CV / VC matrices.

use crate::utils::add_mark_to_toneless_char;

const FIRST_CONSONANT_SEQS: &[&str] = &[
    "b d đ g gh m n nh p ph r s t tr v z",
    "c h k kh qu th",
    "ch gi l ng ngh x",
    "đ l",
    "h",
];

const VOWEL_SEQS: &[&str] = &[
    "ê i ua uê uy y",
    "a iê oa uyê yê",
    "â ă e o oo ô ơ oe u ư uâ uô ươ",
    "oă",
    "uơ",
    "ai ao au âu ay ây eo êu ia iêu iu oai oao oay oeo oi ôi ơi ưa uây ui ưi uôi ươi ươu ưu uya uyu yêu",
    "ă",
    "i",
];

const LAST_CONSONANT_SEQS: &[&str] = &["ch nh", "c ng", "m n p t", "k", "c"];

const CV_MATRIX: &[&[usize]] = &[
    &[0, 1, 2, 5],
    &[0, 1, 2, 3, 4, 5],
    &[0, 1, 2, 3, 5],
    &[6],
    &[7],
];

const VC_MATRIX: &[&[usize]] = &[
    &[0, 2],
    &[0, 1, 2],
    &[1, 2],
    &[1, 2],
    &[],
    &[],
    &[3],
    &[4],
];

/// Returns the matching row indices (empty == no match, mirroring Go's nil).
fn lookup(seq: &[&str], input: &str, input_is_full: bool, input_is_complete: bool) -> Vec<usize> {
    let mut ret = Vec::new();
    let input_chars: Vec<char> = input.chars().collect();
    let input_len = input_chars.len();
    for (index, row) in seq.iter().enumerate() {
        let mut rows: Vec<char> = row.chars().collect();
        rows.push(' ');
        let mut i = 0usize;
        for (j, &ch) in rows.iter().enumerate() {
            if ch != ' ' {
                continue;
            }
            let canvas = &rows[i..j];
            i = j + 1;
            if canvas.len() < input_len || (input_is_full && canvas.len() > input_len) {
                continue;
            }
            let mut is_match = true;
            for (k, &ic) in input_chars.iter().enumerate() {
                if ic != canvas[k]
                    && !(!input_is_complete && add_mark_to_toneless_char(canvas[k], 0) == ic)
                {
                    is_match = false;
                    break;
                }
            }
            if is_match {
                ret.push(index);
                break;
            }
        }
    }
    ret
}

pub fn is_valid_cvc(fc: &str, vo: &str, lc: &str, input_is_full_complete: bool) -> bool {
    let mut fc_indexes: Vec<usize> = Vec::new();
    let mut vo_indexes: Vec<usize> = Vec::new();
    let mut lc_indexes: Vec<usize> = Vec::new();

    if !fc.is_empty() {
        fc_indexes = lookup(
            FIRST_CONSONANT_SEQS,
            fc,
            input_is_full_complete || !vo.is_empty(),
            true,
        );
        if fc_indexes.is_empty() {
            return false;
        }
    }
    if !vo.is_empty() {
        vo_indexes = lookup(
            VOWEL_SEQS,
            vo,
            input_is_full_complete || !lc.is_empty(),
            input_is_full_complete,
        );
        if vo_indexes.is_empty() {
            return false;
        }
    }
    if !lc.is_empty() {
        lc_indexes = lookup(LAST_CONSONANT_SEQS, lc, input_is_full_complete, true);
        if lc_indexes.is_empty() {
            return false;
        }
    }
    if vo_indexes.is_empty() {
        // first consonant only
        return !fc_indexes.is_empty();
    }
    if !fc_indexes.is_empty() {
        // first consonant + vowel
        let ret = is_valid_cv(&fc_indexes, &vo_indexes);
        if !ret || lc_indexes.is_empty() {
            return ret;
        }
    }
    if !lc_indexes.is_empty() {
        is_valid_vc(&vo_indexes, &lc_indexes)
    } else {
        true
    }
}

fn is_valid_cv(fc_indexes: &[usize], vo_indexes: &[usize]) -> bool {
    for &fc in fc_indexes {
        for &c in CV_MATRIX[fc] {
            for &vo in vo_indexes {
                if c == vo {
                    return true;
                }
            }
        }
    }
    false
}

fn is_valid_vc(vo_indexes: &[usize], lc_indexes: &[usize]) -> bool {
    for &vo in vo_indexes {
        for &c in VC_MATRIX[vo] {
            for &lc in lc_indexes {
                if c == lc {
                    return true;
                }
            }
        }
    }
    false
}
