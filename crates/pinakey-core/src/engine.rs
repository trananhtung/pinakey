//! `PinaKeyEngine` and the public `IEngine` API.

use crate::flattener::flatten;
use crate::rules::{flag, mode, Rule, TransRef, Transformation};
use crate::rules_parser::InputMethod;
use crate::transform_utils::{
    break_composition, extract_last_syllable, extract_last_word,
    extract_last_word_with_punctuation_marks, find_last_appending_trans, find_target,
    generate_fallback_transformations, generate_transformations as gen_trans, is_valid,
    matches_uoh_tail, new_appending_trans, refresh_last_tone_target as refresh_lt,
};
use crate::utils::{can_process_key, is_upper, to_lower};
use std::rc::Rc;

/// Public engine API, mirroring Go's `IEngine` interface.
pub trait IEngine {
    fn set_flag(&mut self, flag: u32);
    fn get_input_method(&self) -> &InputMethod;
    fn process_key(&mut self, key: char, mode_flags: u32);
    fn process_string(&mut self, s: &str, mode_flags: u32);
    fn get_processed_string(&self, mode_flags: u32) -> String;
    fn is_valid(&self, input_is_full_complete: bool) -> bool;
    fn can_process_key(&self, key: char) -> bool;
    fn remove_last_char(&mut self, refresh_last_tone_target: bool);
    fn restore_last_word(&mut self, to_vietnamese: bool);
    fn reset(&mut self);
}

pub struct PinaKeyEngine {
    composition: Vec<TransRef>,
    input_method: InputMethod,
    flags: u32,
}

pub fn new_engine(input_method: InputMethod, flags: u32) -> PinaKeyEngine {
    PinaKeyEngine {
        composition: Vec::new(),
        input_method,
        flags,
    }
}

impl PinaKeyEngine {
    pub fn get_flag(&self) -> u32 {
        self.flags
    }

    fn get_applicable_rules(&self, key: char) -> Vec<Rule> {
        let lower = to_lower(key);
        self.input_method
            .rules
            .iter()
            .filter(|r| r.key == lower)
            .cloned()
            .collect()
    }

    fn find_target_by_key(&self, composition: &[TransRef], key: char) -> (Option<TransRef>, Rule) {
        find_target(composition, &self.get_applicable_rules(key), self.flags)
    }

    fn engine_generate_transformations(
        &self,
        composition: &[TransRef],
        lower_key: char,
        is_upper_case: bool,
    ) -> Vec<TransRef> {
        let mut transformations = gen_trans(
            composition,
            &self.get_applicable_rules(lower_key),
            self.flags,
            lower_key,
            is_upper_case,
        );
        if transformations.is_empty() {
            transformations = generate_fallback_transformations(
                &self.get_applicable_rules(lower_key),
                lower_key,
                is_upper_case,
            );
            let mut new_composition = composition.to_vec();
            new_composition.extend(transformations.iter().cloned());
            if let Some(virtual_trans) = self.apply_uow_shortcut(&new_composition) {
                transformations.push(virtual_trans);
            }
        }
        let mut with_new = composition.to_vec();
        with_new.extend(transformations.iter().cloned());
        let refreshed = self.engine_refresh_last_tone_target(&with_new);
        transformations.extend(refreshed);
        transformations
    }

    fn apply_uow_shortcut(&self, syllable: &[TransRef]) -> Option<TransRef> {
        let s = flatten(syllable, mode::TONE_LESS | mode::LOWER_CASE);
        if !self.input_method.super_keys.is_empty() && matches_uoh_tail(&s) {
            let (target, mut missing_rule) =
                self.find_target_by_key(syllable, self.input_method.super_keys[0]);
            if let Some(target) = target {
                missing_rule.key = '\0';
                return Some(Transformation::new_ref(missing_rule, Some(target), false));
            }
        }
        None
    }

    fn engine_refresh_last_tone_target(&self, syllable: &[TransRef]) -> Vec<TransRef> {
        if self.flags & flag::FREE_TONE_MARKING != 0 && is_valid(syllable, false) {
            return refresh_lt(syllable, self.flags & flag::STD_TONE_STYLE != 0);
        }
        Vec::new()
    }

    fn new_composition(
        &self,
        composition: &[TransRef],
        key: char,
        is_upper_case: bool,
    ) -> Vec<TransRef> {
        let (mut previous, mut last_syllable) = extract_last_syllable(composition);
        let generated = self.engine_generate_transformations(&last_syllable, key, is_upper_case);
        last_syllable.extend(generated);
        previous.extend(last_syllable);
        previous
    }
}

impl IEngine for PinaKeyEngine {
    fn set_flag(&mut self, flag: u32) {
        self.flags = flag;
    }

    fn get_input_method(&self) -> &InputMethod {
        &self.input_method
    }

    fn is_valid(&self, input_is_full_complete: bool) -> bool {
        let (_, last) = extract_last_word(&self.composition, &self.input_method.keys);
        is_valid(&last, input_is_full_complete)
    }

    fn get_processed_string(&self, mode_flags: u32) -> String {
        let tmp: Vec<TransRef>;
        if mode_flags & mode::FULL_TEXT != 0 {
            tmp = self.composition.clone();
        } else if mode_flags & mode::PUNCTUATION != 0 {
            let (_, t) = extract_last_word_with_punctuation_marks(
                &self.composition,
                &self.input_method.keys,
            );
            return flatten(&t, mode::VIETNAMESE);
        } else {
            let (_, t) = extract_last_word(&self.composition, &self.input_method.keys);
            tmp = t;
        }
        flatten(&tmp, mode_flags)
    }

    fn can_process_key(&self, key: char) -> bool {
        can_process_key(key, &self.input_method.keys)
    }

    fn process_string(&mut self, s: &str, mode_flags: u32) {
        for key in s.chars() {
            self.process_key(key, mode_flags);
        }
    }

    fn process_key(&mut self, key: char, mode_flags: u32) {
        let lower_key = to_lower(key);
        let is_upper_case = is_upper(key);
        if mode_flags & mode::ENGLISH != 0 || !self.can_process_key(lower_key) {
            if mode_flags & mode::IN_REVERSE_ORDER != 0 {
                let mut new_comp = vec![new_appending_trans(lower_key, is_upper_case)];
                new_comp.extend(self.composition.iter().cloned());
                self.composition = new_comp;
                return;
            }
            self.composition
                .push(new_appending_trans(lower_key, is_upper_case));
            return;
        }
        let comp = std::mem::take(&mut self.composition);
        self.composition = self.new_composition(&comp, lower_key, is_upper_case);
    }

    fn restore_last_word(&mut self, to_vietnamese: bool) {
        let (mut previous, last_comb) =
            extract_last_word(&self.composition, &self.input_method.keys);
        if last_comb.is_empty() {
            return;
        }
        if !to_vietnamese {
            previous.extend(break_composition(&last_comb));
            self.composition = previous;
        } else {
            let mut new_comp: Vec<TransRef> = Vec::new();
            for tnx in &last_comb {
                let (key, is_upper) = {
                    let t = tnx.borrow();
                    (t.rule.key, t.is_upper_case)
                };
                new_comp = self.new_composition(&new_comp, key, is_upper);
            }
            previous.extend(new_comp);
            self.composition = previous;
        }
    }

    fn reset(&mut self) {
        self.composition.clear();
    }

    fn remove_last_char(&mut self, refresh_last_tone_target: bool) {
        let last_appending = match find_last_appending_trans(&self.composition) {
            Some(t) => t,
            None => return,
        };
        let last_key = last_appending.borrow().rule.key;
        if !self.can_process_key(last_key) {
            self.composition.pop();
            return;
        }
        let (mut previous, last_comb) =
            extract_last_word(&self.composition, &self.input_method.keys);
        let mut new_comb: Vec<TransRef> = Vec::new();
        for t in &last_comb {
            let is_target = matches!(&t.borrow().target, Some(x) if Rc::ptr_eq(x, &last_appending));
            if is_target || Rc::ptr_eq(t, &last_appending) {
                continue;
            }
            new_comb.push(t.clone());
        }
        if refresh_last_tone_target {
            let refreshed = self.engine_refresh_last_tone_target(&new_comb);
            new_comb.extend(refreshed);
        }
        previous.extend(new_comb);
        self.composition = previous;
    }
}
