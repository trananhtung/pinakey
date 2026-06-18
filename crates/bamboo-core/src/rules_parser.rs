//! Rule / input-method parsing — ported from `rules_parser.go`.

use crate::input_method_def::{input_method_definitions, InputMethodDefinition};
use crate::rules::{EffectType, Mark, Rule, Tone};
use crate::utils::{add_tone_to_char, find_mark_from_char, is_vowel, mark_family};
use once_cell::sync::Lazy;
use regex::Regex;

fn tone_from_name(line: &str) -> Option<Tone> {
    match line {
        "XoaDauThanh" => Some(Tone::None),
        "DauSac" => Some(Tone::Acute),
        "DauHuyen" => Some(Tone::Grave),
        "DauNga" => Some(Tone::Tilde),
        "DauNang" => Some(Tone::Dot),
        "DauHoi" => Some(Tone::Hook),
        _ => None,
    }
}

/// Parsed input method (`InputMethod` in Go).
#[derive(Debug, Clone, Default)]
pub struct InputMethod {
    pub name: String,
    pub rules: Vec<Rule>,
    pub super_keys: Vec<char>,
    pub tone_keys: Vec<char>,
    pub appending_keys: Vec<char>,
    pub keys: Vec<char>,
}

pub fn parse_input_method(
    defs: &[(&'static str, InputMethodDefinition)],
    im_name: &str,
) -> InputMethod {
    for (name, def) in defs {
        if *name == im_name {
            return build_input_method(name, def);
        }
    }
    InputMethod::default()
}

/// Convenience: parse from the built-in definition tables.
pub fn parse_builtin_input_method(im_name: &str) -> InputMethod {
    parse_input_method(&input_method_definitions(), im_name)
}

fn build_input_method(name: &str, def: &InputMethodDefinition) -> InputMethod {
    let mut im = InputMethod {
        name: name.to_string(),
        ..Default::default()
    };
    for (key_str, line) in def {
        let keys: Vec<char> = key_str.chars().collect();
        if keys.is_empty() {
            continue;
        }
        let key = keys[0];
        im.rules.extend(parse_rules(key, line));
        if line.to_lowercase().contains("uo") {
            im.super_keys.push(key);
        }
        im.keys.push(key);
    }
    for rule in &im.rules {
        if rule.effect_type == EffectType::Appending {
            im.appending_keys.push(rule.key);
        }
        if rule.effect_type == EffectType::ToneTransformation {
            im.tone_keys.push(rule.key);
        }
    }
    im
}

pub fn parse_rules(key: char, line: &str) -> Vec<Rule> {
    if let Some(tone) = tone_from_name(line) {
        vec![Rule {
            key,
            effect_type: EffectType::ToneTransformation,
            effect: tone as u8,
            ..Default::default()
        }]
    } else {
        parse_toneless_rules(key, line)
    }
}

static REG_DSL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([a-zA-Z]+)_(\p{L}+)([\p{L}_]*)").unwrap());
static REG_DSL_APPENDING: Lazy<Regex> = Lazy::new(|| Regex::new(r"(_?)_(\p{L}+)").unwrap());

pub fn parse_toneless_rules(key: char, line: &str) -> Vec<Rule> {
    let mut rules = Vec::new();
    // MatchString tests the original line; submatch is taken on the lowercased line.
    if REG_DSL.is_match(line) {
        let lower = line.to_lowercase();
        if let Some(caps) = REG_DSL.captures(&lower) {
            let effective_ons: Vec<char> = caps.get(1).unwrap().as_str().chars().collect();
            let results: Vec<char> = caps.get(2).unwrap().as_str().chars().collect();
            for (i, &effective_on) in effective_ons.iter().enumerate() {
                if i >= results.len() {
                    continue;
                }
                let result = results[i];
                let effect = match find_mark_from_char(result) {
                    Some(m) => m,
                    None => continue,
                };
                rules.extend(parse_toneless_rule(key, effective_on, result, effect));
            }
            let part3 = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            if let Some(rule) = get_appending_rule(key, part3) {
                rules.push(rule);
            }
        }
    } else if let Some(rule) = get_appending_rule(key, line) {
        rules.push(rule);
    }
    rules
}

pub fn parse_toneless_rule(key: char, effective_on: char, result: char, effect: Mark) -> Vec<Rule> {
    let mut rules = Vec::new();
    // NOTE: Go's `for tone := range tones` iterates the *index* 0..6 (the slice values are
    // unused); we replicate that exactly.
    for chr in mark_family(effective_on) {
        if chr == result {
            rules.push(Rule {
                key,
                effect_type: EffectType::MarkTransformation,
                effect: 0,
                effect_on: result,
                result: effective_on,
                ..Default::default()
            });
        } else if is_vowel(chr) {
            for tone in 0u8..6 {
                rules.push(Rule {
                    key,
                    effect_type: EffectType::MarkTransformation,
                    effect_on: add_tone_to_char(chr, tone),
                    effect: effect as u8,
                    result: add_tone_to_char(result, tone),
                    ..Default::default()
                });
            }
        } else {
            rules.push(Rule {
                key,
                effect_type: EffectType::MarkTransformation,
                effect_on: chr,
                effect: effect as u8,
                result,
                ..Default::default()
            });
        }
    }
    rules
}

pub fn get_appending_rule(key: char, value: &str) -> Option<Rule> {
    if REG_DSL_APPENDING.is_match(value) {
        let caps = REG_DSL_APPENDING.captures(value)?;
        let chars: Vec<char> = caps.get(2).unwrap().as_str().chars().collect();
        let mut rule = Rule {
            key,
            effect_type: EffectType::Appending,
            effect_on: chars[0],
            result: chars[0],
            ..Default::default()
        };
        if chars.len() > 1 {
            for &chr in &chars[1..] {
                rule.appended_rules.push(Rule {
                    key,
                    effect_type: EffectType::Appending,
                    effect_on: chr,
                    result: chr,
                    ..Default::default()
                });
            }
        }
        Some(rule)
    } else {
        None
    }
}
