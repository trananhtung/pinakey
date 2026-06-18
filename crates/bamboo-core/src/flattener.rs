//! Composition → string rendering — ported from `flattener.go`.

use crate::rules::{mode, EffectType, Mark, TransRef};
use crate::utils::{add_mark_to_char, add_tone_to_char, to_lower, to_upper};
use std::collections::HashMap;
use std::rc::Rc;

pub fn flatten(composition: &[TransRef], mode: u32) -> String {
    get_canvas(composition, mode).into_iter().collect()
}

/// Returns the rendered characters. The `appendingMap` in Go is keyed by pointer identity; we
/// key by the `Rc`'s allocation address to reproduce that exactly.
pub fn get_canvas(composition: &[TransRef], mode_flags: u32) -> Vec<char> {
    let english = mode_flags & mode::ENGLISH != 0;
    let mut canvas = Vec::new();
    let mut appending_map: HashMap<usize, Vec<TransRef>> = HashMap::new();
    let mut appending_list: Vec<TransRef> = Vec::new();

    for trans in composition {
        let t = trans.borrow();
        if english {
            if t.rule.key == '\0' {
                continue;
            }
            drop(t);
            appending_list.push(trans.clone());
        } else if t.rule.effect_type == EffectType::Appending {
            if t.rule.key == '\0' {
                continue;
            }
            drop(t);
            appending_list.push(trans.clone());
        } else if let Some(target) = &t.target {
            let key = Rc::as_ptr(target) as usize;
            appending_map.entry(key).or_default().push(trans.clone());
        }
    }

    let empty: Vec<TransRef> = Vec::new();
    for appending_trans in &appending_list {
        let at = appending_trans.borrow();
        let key = Rc::as_ptr(appending_trans) as usize;
        let trans_list = appending_map.get(&key).unwrap_or(&empty);
        let mut chr;
        if english {
            chr = at.rule.key;
        } else {
            chr = at.rule.effect_on;
            for trans in trans_list {
                let tr = trans.borrow();
                match tr.rule.effect_type {
                    EffectType::MarkTransformation => {
                        if tr.rule.effect == Mark::Raw as u8 {
                            chr = at.rule.key;
                        } else {
                            chr = add_mark_to_char(chr, tr.rule.effect);
                        }
                    }
                    EffectType::ToneTransformation => {
                        chr = add_tone_to_char(chr, tr.rule.effect);
                    }
                    _ => {}
                }
            }
        }
        if mode_flags & mode::TONE_LESS != 0 {
            chr = add_tone_to_char(chr, 0);
        }
        if mode_flags & mode::MARK_LESS != 0 {
            chr = add_mark_to_char(chr, 0);
        }
        if mode_flags & mode::LOWER_CASE != 0 {
            chr = to_lower(chr);
        } else if at.is_upper_case {
            chr = to_upper(chr);
        }
        canvas.push(chr);
    }
    canvas
}
