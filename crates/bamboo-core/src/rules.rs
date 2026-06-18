//! Core types and constants — ported from `bamboo.go` (consts) and `rules_parser.go` (types).
//!
//! `rune` in Go maps to `char` in Rust. Go uses `rune(0)` for "virtual"/absent keys; we use
//! `'\0'` for the same purpose.

use std::cell::RefCell;
use std::rc::Rc;

/// Bit flags controlling how a composition is processed / flattened (`Mode` in Go).
pub mod mode {
    pub const VIETNAMESE: u32 = 1 << 0;
    pub const ENGLISH: u32 = 1 << 1;
    pub const TONE_LESS: u32 = 1 << 2;
    pub const MARK_LESS: u32 = 1 << 3;
    pub const LOWER_CASE: u32 = 1 << 4;
    pub const FULL_TEXT: u32 = 1 << 5;
    pub const PUNCTUATION: u32 = 1 << 6;
    pub const IN_REVERSE_ORDER: u32 = 1 << 7;
}

/// Engine feature flags (the `E*` constants in Go).
pub mod flag {
    pub const FREE_TONE_MARKING: u32 = 1 << 0;
    pub const STD_TONE_STYLE: u32 = 1 << 1;
    pub const AUTO_CORRECT_ENABLED: u32 = 1 << 2;
    pub const STD_FLAGS: u32 = FREE_TONE_MARKING | STD_TONE_STYLE | AUTO_CORRECT_ENABLED;
}

/// What kind of effect a rule applies (`EffectType` in Go).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    Appending = 0,
    MarkTransformation = 1,
    ToneTransformation = 2,
    Replacing = 3,
}

/// A diacritic mark family index (`Mark` in Go). Stored numerically as `u8` in `Rule.effect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mark {
    None = 0,
    Hat = 1,
    Breve = 2,
    Horn = 3,
    Dash = 4,
    Raw = 5,
}

/// A tone index (`Tone` in Go). Stored numerically as `u8` in `Rule.effect`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Tone {
    None = 0,
    Grave = 1,
    Acute = 2,
    Hook = 3,
    Tilde = 4,
    Dot = 5,
}

impl Tone {
    pub fn from_u8(v: u8) -> Tone {
        match v {
            1 => Tone::Grave,
            2 => Tone::Acute,
            3 => Tone::Hook,
            4 => Tone::Tilde,
            5 => Tone::Dot,
            _ => Tone::None,
        }
    }
}

/// A transformation rule (`Rule` in Go).
#[derive(Debug, Clone)]
pub struct Rule {
    pub key: char,
    pub effect: u8, // a Tone or Mark value depending on effect_type
    pub effect_type: EffectType,
    pub effect_on: char,
    pub result: char,
    pub appended_rules: Vec<Rule>,
}

impl Default for Rule {
    fn default() -> Self {
        Rule {
            key: '\0',
            effect: 0,
            effect_type: EffectType::Appending,
            effect_on: '\0',
            result: '\0',
            appended_rules: Vec::new(),
        }
    }
}

impl Rule {
    pub fn set_tone(&mut self, tone: Tone) {
        self.effect = tone as u8;
    }
    pub fn set_mark(&mut self, mark: Mark) {
        self.effect = mark as u8;
    }
    pub fn get_tone(&self) -> Tone {
        Tone::from_u8(self.effect)
    }
    pub fn get_mark(&self) -> Mark {
        match self.effect {
            1 => Mark::Hat,
            2 => Mark::Breve,
            3 => Mark::Horn,
            4 => Mark::Dash,
            5 => Mark::Raw,
            _ => Mark::None,
        }
    }
}

/// A single transformation in a composition (`Transformation` in Go).
///
/// `target` is an aliased pointer to another transformation in the same composition; we use
/// `Rc<RefCell<..>>` so that pointer identity (`Rc::ptr_eq`) and in-place mutation behave like
/// Go's `*Transformation`.
#[derive(Debug)]
pub struct Transformation {
    pub rule: Rule,
    pub target: Option<TransRef>,
    pub is_upper_case: bool,
}

/// Shared, mutable reference to a `Transformation` — the Rust analogue of Go's `*Transformation`.
pub type TransRef = Rc<RefCell<Transformation>>;

impl Transformation {
    pub fn new_ref(rule: Rule, target: Option<TransRef>, is_upper_case: bool) -> TransRef {
        Rc::new(RefCell::new(Transformation {
            rule,
            target,
            is_upper_case,
        }))
    }
}
