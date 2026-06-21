//! Các kiểu dữ liệu và hằng số cốt lõi của engine biến đổi.
//!
//! `rune` bên Go tương ứng với `char` trong Rust. Go dùng `rune(0)` cho các phím "ảo"/không tồn
//! tại; ở đây ta dùng `'\0'` với cùng mục đích đó.

use std::cell::RefCell;
use std::rc::Rc;

/// Các cờ bit điều khiển cách một composition được xử lý / làm phẳng (`Mode` bên Go).
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

/// Các cờ tính năng của engine (các hằng số `E*` bên Go).
pub mod flag {
    pub const FREE_TONE_MARKING: u32 = 1 << 0;
    pub const STD_TONE_STYLE: u32 = 1 << 1;
    /// DÀNH RIÊNG / KHÔNG DÙNG (issue #8). Cờ này từng nằm trong `STD_FLAGS` nhưng **không nơi nào
    /// đọc** — hành vi tự-sửa thực tế do `IB_AUTO_NON_VN_RESTORE` ở tầng engine điều khiển. Đã gỡ
    /// khỏi `STD_FLAGS` để không gây hiểu nhầm; giữ giá trị bit để cấu hình cũ vẫn nạp được (bỏ qua
    /// vô hại) và không xê dịch ý nghĩa các bit khác.
    pub const AUTO_CORRECT_ENABLED: u32 = 1 << 2;
    pub const STD_FLAGS: u32 = FREE_TONE_MARKING | STD_TONE_STYLE;
}

/// Loại hiệu ứng mà một rule áp dụng (`EffectType` bên Go).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    Appending = 0,
    MarkTransformation = 1,
    ToneTransformation = 2,
    Replacing = 3,
}

/// Chỉ số nhóm dấu phụ (diacritic mark) (`Mark` bên Go). Lưu dưới dạng số `u8` trong `Rule.effect`.
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

/// Chỉ số dấu thanh (`Tone` bên Go). Lưu dưới dạng số `u8` trong `Rule.effect`.
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

/// Một rule biến đổi (`Rule` bên Go).
#[derive(Debug, Clone)]
pub struct Rule {
    pub key: char,
    pub effect: u8, // giá trị Tone hoặc Mark, tùy theo effect_type
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

/// Một biến đổi đơn lẻ trong một composition (`Transformation` bên Go).
///
/// `target` là con trỏ alias trỏ tới một transformation khác trong cùng composition; ta dùng
/// `Rc<RefCell<..>>` để định danh con trỏ (`Rc::ptr_eq`) và việc sửa tại chỗ hoạt động giống như
/// `*Transformation` bên Go.
#[derive(Debug)]
pub struct Transformation {
    pub rule: Rule,
    pub target: Option<TransRef>,
    pub is_upper_case: bool,
}

/// Tham chiếu dùng chung, có thể thay đổi tới một `Transformation` — tương đương `*Transformation` bên Go trong Rust.
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
