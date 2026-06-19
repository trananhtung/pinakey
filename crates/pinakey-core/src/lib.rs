//! PinaKey — engine biến đổi cho bộ gõ tiếng Việt (Telex / VNI / VIQR).
//!
//! Bộ kiểm thử hành vi nằm trong `tests/` và sử dụng API công khai này giống như một bên tiêu thụ
//! bên ngoài.

mod charset;
mod charset_def;
mod dict;
mod engine;
mod flattener;
mod input_method_def;
mod rules;
mod rules_parser;
mod spelling;
mod transform_utils;
mod utils;

// API công khai.
pub use charset::{encode, get_charset_names, UNICODE};
pub use dict::Dictionary;
pub use engine::{new_engine, IEngine, PinaKeyEngine};
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
