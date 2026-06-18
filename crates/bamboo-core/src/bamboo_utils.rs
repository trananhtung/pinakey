//! The transformation algorithm — ported from `bamboo_utils.go`.
//!
//! Go uses `[]*Transformation` with aliased `Target` pointers, pointer-identity comparisons and
//! in-place target mutation. We mirror that with `Rc<RefCell<Transformation>>`: identity via
//! `Rc::ptr_eq`, mutation via `borrow_mut`.

use crate::flattener::{flatten, get_canvas};
use crate::rules::{mode, EffectType, Mark, Rule, Tone, TransRef, Transformation};
use crate::spelling::is_valid_cvc;
use crate::utils::{find_tone_from_char, is_vowel};
use once_cell::sync::Lazy;
use regex::Regex;
use std::rc::Rc;

static REG_UO_H_TAIL: Lazy<Regex> = Lazy::new(|| Regex::new(r"(uơ|ưo)\p{L}+").unwrap());
static REG_UH_O: Lazy<Regex> = Lazy::new(|| Regex::new(r"(ưo|ươ)").unwrap());

/// Build a new composition = `composition` ++ [extra], preserving Rc identities.
fn with_appended(composition: &[TransRef], extra: TransRef) -> Vec<TransRef> {
    let mut v = composition.to_vec();
    v.push(extra);
    v
}

fn ptr_id(t: &TransRef) -> usize {
    Rc::as_ptr(t) as usize
}

fn same(a: &TransRef, b: &TransRef) -> bool {
    Rc::ptr_eq(a, b)
}

pub fn find_last_appending_trans(composition: &[TransRef]) -> Option<TransRef> {
    for trans in composition.iter().rev() {
        if trans.borrow().rule.effect_type == EffectType::Appending {
            return Some(trans.clone());
        }
    }
    None
}

pub fn new_appending_trans(key: char, is_upper_case: bool) -> TransRef {
    Transformation::new_ref(
        Rule {
            key,
            effect_on: key,
            effect_type: EffectType::Appending,
            result: key,
            ..Default::default()
        },
        None,
        is_upper_case,
    )
}

fn generate_appending_trans(rules: &[Rule], lower_key: char, is_upper_case: bool) -> TransRef {
    for rule in rules {
        if rule.key == lower_key && rule.effect_type == EffectType::Appending {
            let mut rule = rule.clone();
            let _is_upper_case = is_upper_case || crate::utils::is_upper(rule.effect_on);
            rule.effect_on = crate::utils::to_lower(rule.effect_on);
            rule.result = rule.effect_on;
            return Transformation::new_ref(rule, None, _is_upper_case);
        }
    }
    new_appending_trans(lower_key, is_upper_case)
}

fn filter_appending_composition(composition: &[TransRef]) -> Vec<TransRef> {
    composition
        .iter()
        .filter(|t| t.borrow().rule.effect_type == EffectType::Appending)
        .cloned()
        .collect()
}

fn find_root_target(target: &TransRef) -> TransRef {
    let next = target.borrow().target.clone();
    match next {
        None => target.clone(),
        Some(n) => find_root_target(&n),
    }
}

pub fn is_valid(composition: &[TransRef], input_is_full_complete: bool) -> bool {
    if composition.len() <= 1 {
        return true;
    }
    // last tone checking
    for trans in composition.iter().rev() {
        if trans.borrow().rule.effect_type == EffectType::ToneTransformation {
            let last_tone = Tone::from_u8(trans.borrow().rule.effect);
            if !has_valid_tone(composition, last_tone) {
                return false;
            }
            break;
        }
    }
    // spell checking
    let (fc, vo, lc) = extract_cvc_trans(composition);
    let m = mode::VIETNAMESE | mode::LOWER_CASE | mode::TONE_LESS;
    is_valid_cvc(
        &flatten(&fc, m),
        &flatten(&vo, m),
        &flatten(&lc, m),
        input_is_full_complete,
    )
}

fn get_right_most_vowels(composition: &[TransRef]) -> Vec<TransRef> {
    let (_, vo, _) = extract_cvc_trans(composition);
    vo
}

fn find_tone_target(composition: &[TransRef], std_style: bool) -> Option<TransRef> {
    if composition.is_empty() {
        return None;
    }
    let mut target: Option<TransRef> = None;
    let (_, vo, lc) = extract_cvc_trans(composition);
    let vowels = filter_appending_composition(&vo);
    if vowels.len() == 1 {
        target = Some(vowels[0].clone());
    } else if vowels.len() == 2 && std_style {
        for trans in &vo {
            let result = trans.borrow().rule.result;
            if result == 'ơ' || result == 'ê' {
                let tgt = trans.borrow().target.clone();
                target = Some(tgt.unwrap_or_else(|| trans.clone()));
            }
        }
        if target.is_none() {
            target = Some(if !lc.is_empty() {
                vowels[1].clone()
            } else {
                vowels[0].clone()
            });
        }
    } else if vowels.len() == 2 {
        if !lc.is_empty() {
            target = Some(vowels[1].clone());
        } else {
            let s = flatten(
                &vowels,
                mode::ENGLISH | mode::LOWER_CASE | mode::TONE_LESS | mode::MARK_LESS,
            );
            if s == "oa" || s == "oe" || s == "uy" || s == "ue" || s == "uo" {
                target = Some(vowels[1].clone());
            } else {
                target = Some(vowels[0].clone());
            }
        }
    } else if vowels.len() == 3 {
        let s = flatten(
            &vowels,
            mode::ENGLISH | mode::LOWER_CASE | mode::TONE_LESS | mode::MARK_LESS,
        );
        if s == "uye" {
            target = Some(vowels[2].clone());
        } else {
            target = Some(vowels[1].clone());
        }
    }
    target
}

fn has_valid_tone(composition: &[TransRef], tone: Tone) -> bool {
    if tone == Tone::None || tone == Tone::Acute || tone == Tone::Dot {
        return true;
    }
    let (_, _, lc) = extract_cvc_trans(composition);
    if lc.is_empty() {
        return true;
    }
    let last_consonants = flatten(&lc, mode::ENGLISH | mode::LOWER_CASE);
    // These consonants have to go with ACUTE, DOT accents.
    for s in ["c", "k", "p", "t", "ch"] {
        if s == last_consonants {
            return false;
        }
    }
    true
}

fn get_last_tone_transformation(composition: &[TransRef]) -> Option<TransRef> {
    for trans in composition.iter().rev() {
        let t = trans.borrow();
        if t.rule.effect_type == EffectType::ToneTransformation && t.target.is_some() {
            return Some(trans.clone());
        }
    }
    None
}

fn is_free(composition: &[TransRef], trans: &Option<TransRef>, effect_type: EffectType) -> bool {
    for t in composition {
        let tb = t.borrow();
        let target_matches = match (&tb.target, trans) {
            (Some(x), Some(y)) => Rc::ptr_eq(x, y),
            (None, None) => true,
            _ => false,
        };
        if target_matches && tb.rule.effect_type == effect_type {
            return false;
        }
    }
    true
}

fn extract_atomic_trans(
    composition: &[TransRef],
    last: &[TransRef],
    last_is_vowel: bool,
) -> (Vec<TransRef>, Vec<TransRef>) {
    let mut comp: Vec<TransRef> = composition.to_vec();
    let mut last_v: Vec<TransRef> = last.to_vec();
    loop {
        if comp.is_empty() {
            return (comp, last_v);
        }
        let tmp = comp[comp.len() - 1].clone();
        let (target_none, result) = {
            let t = tmp.borrow();
            (t.target.is_none(), t.rule.result)
        };
        if target_none && last_is_vowel != is_vowel(result) {
            return (comp, last_v);
        }
        last_v.insert(0, comp.pop().unwrap());
    }
}

fn extract_cvc_appending_trans(
    composition: &[TransRef],
) -> (Vec<TransRef>, Vec<TransRef>, Vec<TransRef>) {
    let (head, last_consonant) = extract_atomic_trans(composition, &[], false);
    let (mut first_consonant, mut vowel) = extract_atomic_trans(&head, &[], true);
    let mut last_consonant = last_consonant;
    if !last_consonant.is_empty() && vowel.is_empty() && first_consonant.is_empty() {
        first_consonant = last_consonant;
        vowel = Vec::new();
        last_consonant = Vec::new();
    }

    // 'gi' and 'qu' are considered qualified consonants:
    //   ['g', 'ia', ''] -> ['gi', 'a', '']    ['q', 'ua', ''] -> ['qu', 'a', '']
    //   except ['g', 'ie', 'ng'] -> ['g', 'ie', 'ng']
    if first_consonant.len() == 1 && !vowel.is_empty() {
        let fc0 = first_consonant[0].borrow().rule.result;
        let v0 = vowel[0].borrow().rule.result;
        // The `!(… && …)` form mirrors upstream `bamboo_utils.go` so the port stays
        // diff-comparable against the Go source; clippy's De Morgan rewrite would diverge.
        #[allow(clippy::nonminimal_bool)]
        let gi_case = fc0 == 'g'
            && v0 == 'i'
            && vowel.len() > 1
            && !(vowel[1].borrow().rule.result == 'e' && !last_consonant.is_empty());
        let qu_case = fc0 == 'q' && v0 == 'u';
        if gi_case || qu_case {
            first_consonant.push(vowel[0].clone());
            vowel = vowel[1..].to_vec();
        }
    }
    (first_consonant, vowel, last_consonant)
}

pub fn extract_cvc_trans(
    composition: &[TransRef],
) -> (Vec<TransRef>, Vec<TransRef>, Vec<TransRef>) {
    use std::collections::HashMap;
    let mut trans_map: HashMap<usize, Vec<TransRef>> = HashMap::new();
    let mut appending_list: Vec<TransRef> = Vec::new();
    for trans in composition {
        let target = trans.borrow().target.clone();
        match target {
            None => appending_list.push(trans.clone()),
            Some(t) => trans_map.entry(ptr_id(&t)).or_default().push(trans.clone()),
        }
    }
    let (fc, vo, lc) = extract_cvc_appending_trans(&appending_list);
    // Go ranges over the original slice length, appending grouped effects afterwards.
    let extend_with_effects = |base: &[TransRef]| -> Vec<TransRef> {
        let mut out = base.to_vec();
        for t in base {
            if let Some(v) = trans_map.get(&ptr_id(t)) {
                out.extend(v.iter().cloned());
            }
        }
        out
    };
    (
        extend_with_effects(&fc),
        extend_with_effects(&vo),
        extend_with_effects(&lc),
    )
}

pub fn extract_last_word_with_punctuation_marks(
    composition: &[TransRef],
    _effect_keys: &[char],
) -> (Vec<TransRef>, Vec<TransRef>) {
    let n = composition.len();
    for i in (0..n).rev() {
        let canvas = get_canvas(&composition[i..], mode::ENGLISH);
        if canvas.is_empty() {
            continue;
        }
        let c = canvas[0];
        if crate::utils::is_space(c) {
            if i == n - 1 {
                return (composition.to_vec(), Vec::new());
            }
            return (composition[..i + 1].to_vec(), composition[i + 1..].to_vec());
        }
    }
    (Vec::new(), composition.to_vec())
}

pub fn extract_last_word(
    composition: &[TransRef],
    effect_keys: &[char],
) -> (Vec<TransRef>, Vec<TransRef>) {
    let n = composition.len();
    for i in (0..n).rev() {
        let canvas = get_canvas(
            &composition[i..],
            mode::VIETNAMESE | mode::LOWER_CASE | mode::TONE_LESS | mode::MARK_LESS,
        );
        if canvas.is_empty() {
            continue;
        }
        let c = canvas[0];
        if !crate::utils::is_alpha(c) && !crate::utils::in_key_list(effect_keys, c) {
            if i == n - 1 {
                return (composition.to_vec(), Vec::new());
            }
            return (composition[..i + 1].to_vec(), composition[i + 1..].to_vec());
        }
    }
    (Vec::new(), composition.to_vec())
}

pub fn extract_last_syllable(composition: &[TransRef]) -> (Vec<TransRef>, Vec<TransRef>) {
    let (mut previous, last) = extract_last_word(composition, &[]);
    let mut anchor = 0usize;
    for i in 0..last.len() {
        if !is_valid(&last[anchor..i + 1], false) {
            anchor = i;
        }
    }
    if anchor > 0 {
        previous.extend(last[..anchor].iter().cloned());
    }
    (previous, last[anchor..].to_vec())
}

fn find_mark_target(composition: &[TransRef], rules: &[Rule]) -> (Option<TransRef>, Rule) {
    let s = flatten(composition, mode::VIETNAMESE);
    for i in (0..composition.len()).rev() {
        let trans = &composition[i];
        let trans_result = trans.borrow().rule.result;
        for rule in rules {
            if rule.effect_type != EffectType::MarkTransformation {
                continue;
            }
            if trans_result == rule.effect_on && rule.effect > 0 {
                let target = find_root_target(trans);
                let temp = Transformation::new_ref(rule.clone(), Some(target.clone()), false);
                if s == flatten(&with_appended(composition, temp), mode::VIETNAMESE) {
                    continue;
                }
                let temp2 = Transformation::new_ref(rule.clone(), Some(target.clone()), false);
                let tmp = with_appended(composition, temp2);
                if is_valid(&tmp, false) {
                    return (Some(target), rule.clone());
                }
            }
        }
    }
    (None, Rule::default())
}

pub fn find_target(
    composition: &[TransRef],
    applicable_rules: &[Rule],
    flags: u32,
) -> (Option<TransRef>, Rule) {
    let s = flatten(composition, mode::VIETNAMESE);
    for applicable_rule in applicable_rules {
        if applicable_rule.effect_type != EffectType::ToneTransformation {
            continue;
        }
        let mut target: Option<TransRef> = None;
        if flags & crate::rules::flag::FREE_TONE_MARKING != 0 {
            if has_valid_tone(composition, Tone::from_u8(applicable_rule.effect)) {
                target =
                    find_tone_target(composition, flags & crate::rules::flag::STD_TONE_STYLE != 0);
            }
        } else if let Some(last_appending) = find_last_appending_trans(composition) {
            if is_vowel(last_appending.borrow().rule.effect_on) {
                target = Some(last_appending);
            }
        }
        let temp = Transformation::new_ref(applicable_rule.clone(), target.clone(), false);
        if s == flatten(&with_appended(composition, temp), mode::VIETNAMESE) {
            continue;
        }
        if Tone::from_u8(applicable_rule.effect) == Tone::None
            && is_free(composition, &target, EffectType::ToneTransformation)
        {
            let should_nil = match &target {
                Some(t) => find_tone_from_char(t.borrow().rule.result) == Tone::None,
                None => false,
            };
            if should_nil {
                target = None;
            }
        }
        return (target, applicable_rule.clone());
    }
    find_mark_target(composition, applicable_rules)
}

fn generate_undo_transformations(
    composition: &[TransRef],
    rules: &[Rule],
    flags: u32,
) -> Vec<TransRef> {
    let mut transformations: Vec<TransRef> = Vec::new();
    let s = flatten(
        composition,
        mode::VIETNAMESE | mode::TONE_LESS | mode::LOWER_CASE,
    );
    for rule in rules {
        if rule.effect_type == EffectType::ToneTransformation {
            let mut target: Option<TransRef> = None;
            if flags & crate::rules::flag::FREE_TONE_MARKING != 0 {
                if has_valid_tone(composition, Tone::from_u8(rule.effect)) {
                    target = find_tone_target(
                        composition,
                        flags & crate::rules::flag::STD_TONE_STYLE != 0,
                    );
                }
            } else if let Some(last_appending) = find_last_appending_trans(composition) {
                if is_vowel(last_appending.borrow().rule.effect_on) {
                    target = Some(last_appending);
                }
            }
            if target.is_none() {
                continue;
            }
            let trans = Transformation::new_ref(
                Rule {
                    effect_type: EffectType::ToneTransformation,
                    effect: 0,
                    key: '\0',
                    ..Default::default()
                },
                target,
                false,
            );
            transformations.push(trans);
        } else if rule.effect_type == EffectType::MarkTransformation {
            for i in (0..composition.len()).rev() {
                let trans = &composition[i];
                if trans.borrow().rule.result == rule.effect_on {
                    let target = find_root_target(trans);
                    let trans2 = Transformation::new_ref(
                        Rule {
                            key: '\0',
                            effect_type: EffectType::MarkTransformation,
                            effect: 0,
                            ..Default::default()
                        },
                        Some(target),
                        false,
                    );
                    if s == flatten(
                        &with_appended(composition, trans2.clone()),
                        mode::VIETNAMESE | mode::TONE_LESS | mode::LOWER_CASE,
                    ) {
                        continue;
                    }
                    transformations.push(trans2);
                }
            }
        }
    }
    transformations
}

pub fn generate_transformations(
    composition: &[TransRef],
    applicable_rules: &[Rule],
    flags: u32,
    lower_key: char,
    is_upper_case: bool,
) -> Vec<TransRef> {
    let mut transformations: Vec<TransRef> = Vec::new();
    // Double typing an effect key undoes it, e.g. w + w -> w (Telex 2)
    if !composition.is_empty() {
        let last = composition[composition.len() - 1].clone();
        let (etype, key, result) = {
            let r = &last.borrow().rule;
            (r.effect_type, r.key, r.result)
        };
        if etype == EffectType::Appending && key == lower_key && key != result {
            transformations.push(Transformation::new_ref(
                Rule {
                    effect_type: EffectType::MarkTransformation,
                    effect: Mark::Raw as u8,
                    key: '\0',
                    ..Default::default()
                },
                Some(last),
                false,
            ));
            return transformations;
        }
    }
    let (target, applicable_rule) = find_target(composition, applicable_rules, flags);
    if let Some(target) = target {
        transformations.push(Transformation::new_ref(
            applicable_rule.clone(),
            Some(target),
            is_upper_case,
        ));
        if applicable_rule.effect_type != EffectType::MarkTransformation {
            return transformations;
        }
        let mut new_comp = composition.to_vec();
        new_comp.extend(transformations.iter().cloned());
        if is_valid(&new_comp, true) {
            return transformations;
        }
        // uow typing shortcut: virtual Mark::Horn targeting 'u' or 'o'.
        let (target2, mut virtual_rule) = find_target(&new_comp, applicable_rules, flags);
        if let Some(target2) = target2 {
            virtual_rule.key = '\0';
            transformations.push(Transformation::new_ref(virtual_rule, Some(target2), false));
            return transformations;
        }
    } else {
        // ươ/ưo(i/c/ng) + o -> uô
        if REG_UH_O.is_match(&flatten(
            composition,
            mode::VIETNAMESE | mode::TONE_LESS | mode::LOWER_CASE,
        )) {
            let vowels = filter_appending_composition(&get_right_most_vowels(composition));
            let trans = Transformation::new_ref(
                Rule {
                    effect_type: EffectType::MarkTransformation,
                    key: '\0',
                    effect: Mark::None as u8,
                    ..Default::default()
                },
                Some(vowels[0].clone()),
                false,
            );
            let (target3, applicable_rule3) = find_target(
                &with_appended(composition, trans.clone()),
                applicable_rules,
                flags,
            );
            if let Some(target3) = target3 {
                if !same(&target3, &vowels[0]) {
                    transformations.push(trans);
                    transformations.push(Transformation::new_ref(
                        applicable_rule3,
                        Some(target3),
                        is_upper_case,
                    ));
                    return transformations;
                }
            }
        }
        let undo_trans = generate_undo_transformations(composition, applicable_rules, flags);
        if !undo_trans.is_empty() {
            transformations.extend(undo_trans);
            transformations.push(new_appending_trans(lower_key, is_upper_case));
        }
    }
    transformations
}

pub fn generate_fallback_transformations(
    applicable_rules: &[Rule],
    lower_key: char,
    is_upper_case: bool,
) -> Vec<TransRef> {
    let mut transformations: Vec<TransRef> = Vec::new();
    let trans = generate_appending_trans(applicable_rules, lower_key, is_upper_case);
    let appended_rules = trans.borrow().rule.appended_rules.clone();
    transformations.push(trans);
    for appended_rule in appended_rules {
        let mut appended_rule = appended_rule;
        let _is_upper_case = is_upper_case || crate::utils::is_upper(appended_rule.effect_on);
        appended_rule.key = '\0';
        appended_rule.effect_on = crate::utils::to_lower(appended_rule.effect_on);
        appended_rule.result = appended_rule.effect_on;
        transformations.push(Transformation::new_ref(appended_rule, None, _is_upper_case));
    }
    transformations
}

pub fn break_composition(composition: &[TransRef]) -> Vec<TransRef> {
    let mut result = Vec::new();
    for trans in composition {
        let (key, is_upper) = {
            let t = trans.borrow();
            (t.rule.key, t.is_upper_case)
        };
        if key == '\0' {
            continue;
        }
        result.push(new_appending_trans(key, is_upper));
    }
    result
}

pub fn refresh_last_tone_target(composition: &[TransRef], std_style: bool) -> Vec<TransRef> {
    let mut transformations: Vec<TransRef> = Vec::new();
    let rightmost_vowels = get_right_most_vowels(composition);
    let last_tone_trans = get_last_tone_transformation(composition);
    if rightmost_vowels.is_empty() || last_tone_trans.is_none() {
        return Vec::new();
    }
    let last_tone_trans = last_tone_trans.unwrap();
    let new_tone_target = find_tone_target(composition, std_style);
    let current_target = last_tone_trans.borrow().target.clone();
    let differs = match (&current_target, &new_tone_target) {
        (Some(a), Some(b)) => !Rc::ptr_eq(a, b),
        (None, None) => false,
        _ => true,
    };
    if differs {
        last_tone_trans.borrow_mut().target = new_tone_target.clone();
        transformations.push(Transformation::new_ref(
            Rule {
                key: '\0',
                effect_type: EffectType::ToneTransformation,
                effect: Tone::None as u8,
                ..Default::default()
            },
            new_tone_target.clone(),
            false,
        ));
        let mut override_rule = last_tone_trans.borrow().rule.clone();
        override_rule.key = '\0';
        transformations.push(Transformation::new_ref(
            override_rule,
            new_tone_target,
            false,
        ));
    }
    transformations
}

/// Used by the engine (`regUOhTail` match against the toneless lowercase syllable).
pub fn matches_uoh_tail(s: &str) -> bool {
    REG_UO_H_TAIL.is_match(s)
}
