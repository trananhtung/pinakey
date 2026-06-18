//! Vietnamese input method transformation engine — a Rust port of
//! [BambooEngine/bamboo-core](https://github.com/BambooEngine/bamboo-core).
//!
//! The behavioural test suite (ported verbatim from upstream's `bamboo_test.go`,
//! `utils_test.go`, and `rules_parser_test.go`) lives in `tests/` and exercises this public API
//! as an external consumer would.

mod bamboo_utils;
mod charset;
mod charset_def;
mod engine;
mod flattener;
mod input_method_def;
mod rules;
mod rules_parser;
mod spelling;
mod utils;

// Public API (mirrors the Go `bamboo` package).
pub use charset::{encode, get_charset_names, UNICODE};
pub use engine::{new_engine, BambooEngine, IEngine};
pub use flattener::flatten;
pub use input_method_def::{
    input_method_definitions, input_method_definitions_owned, InputMethodDefinition,
};
pub use rules::{flag, mode, EffectType, Mark, Rule, Tone, Transformation};
pub use rules_parser::{
    build_input_method_from_pairs, get_appending_rule, parse_builtin_input_method,
    parse_input_method, parse_rules, parse_toneless_rule, parse_toneless_rules, InputMethod,
};
pub use utils::{
    add_mark_to_char, add_tone_to_char, find_tone_from_char, has_any_vietnamese_rune,
    has_any_vietnamese_vowel, is_punctuation_mark, is_vowel, is_word_break_symbol,
};
