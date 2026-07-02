//! Tìm kiếm emoji fuzzy (issue #63): match subsequence có chấm điểm trên chỉ mục phẳng
//! shortname + keyword + ascii. Khác trie tiền tố (giữ nguyên cho tương thích), chỉ mục này
//! cho phép gõ tắt kiểu `heye` → `heart_eyes` và lần đầu tiên tra được theo shortname.

use serde::Deserialize;
use std::collections::HashMap;

/// Chấm điểm khớp fuzzy của `query` trên `key` (không phân biệt hoa/thường). `None` = `query`
/// không phải subsequence của `key`. Điểm cao hơn = khớp tốt hơn:
/// - +16 khớp ngay ký tự đầu key; +10 khớp tại đầu-từ (đầu key hoặc sau `_` `-` ` `);
/// - +8 mỗi cặp ký tự khớp liền mạch; −1 mỗi ký tự của key (key ngắn thắng khi hoà).
///
/// Khớp greedy trái→phải: không tối ưu tuyệt đối nhưng đủ đúng cho khóa emoji ngắn và O(len).
pub fn fuzzy_score(query: &str, key: &str) -> Option<i32> {
    let q: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    let key_lower = key.to_lowercase();
    let key_len = key_lower.chars().count() as i32;
    score_prepared(&q, &key_lower, key_len)
}

/// Lõi chấm điểm với query + key đã chuẩn hoá chữ thường sẵn và `key_len` đếm trước —
/// [`EmojiIndex::fuzzy_query`] gọi hàm này trên ~11k khóa mỗi phím gõ (hot path): không cấp phát,
/// không case-mapping, và thoát sớm ngay khi query khớp hết (điểm không đổi ở phần đuôi key).
fn score_prepared(q: &[char], key_lower: &str, key_len: i32) -> Option<i32> {
    if q.is_empty() {
        return None; // query rỗng do lịch sử gần dùng xử lý, không fuzzy.
    }
    let mut score = 0i32;
    let mut qi = 0usize;
    let mut prev_matched = false; // ký tự key NGAY TRƯỚC có khớp không (thưởng liền mạch)
    let mut prev_char = '\0';
    for (ki, kc) in key_lower.chars().enumerate() {
        if kc != q[qi] {
            prev_matched = false;
            prev_char = kc;
            continue;
        }
        if ki == 0 {
            score += 16;
        }
        if ki == 0 || matches!(prev_char, '_' | '-' | ' ') {
            score += 10;
        }
        if prev_matched {
            score += 8;
        }
        prev_matched = true;
        prev_char = kc;
        qi += 1;
        if qi == q.len() {
            // Khớp xong: phần đuôi key không đổi điểm nữa (phạt độ dài dùng key_len đếm sẵn).
            return Some(score - key_len);
        }
    }
    None // duyệt hết key mà query chưa khớp đủ
}

/// Một entry EmojiOne — chỉ các trường cần cho chỉ mục tìm kiếm. Khác loader trie (chỉ lấy
/// keywords + ascii), chỉ mục fuzzy lấy cả **shortname** (`:heart_eyes:`) — tên chuẩn người
/// dùng gõ sau `:` — nếu bỏ thì `heart_eyes` không bao giờ khớp.
#[derive(Debug, Deserialize, Default)]
struct EmojiOneEntry {
    #[serde(default)]
    shortname: String,
    #[serde(default)]
    shortname_alternates: Vec<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    ascii: Vec<String>,
}

/// Chỉ mục phẳng `(khóa đã lowercase, emoji, số ký tự của khóa)` cho fuzzy search — ~11k mục với
/// bảng EmojiOne đầy đủ, quét tuyến tính mỗi truy vấn vẫn dưới mili-giây. Lowercase + độ dài
/// tính sẵn lúc dựng để hot path không case-mapping / không đếm lại.
#[derive(Default)]
pub struct EmojiIndex {
    entries: Vec<(String, String, i32)>,
}

impl EmojiIndex {
    /// Dựng chỉ mục từ JSON EmojiOne (cùng định dạng với [`crate::load_emojione_from_str`]).
    pub fn from_emojione_str(json: &str) -> Result<Self, serde_json::Error> {
        let map: HashMap<String, EmojiOneEntry> = serde_json::from_str(json)?;
        let mut entries = Vec::new();
        for (code, e) in map {
            let mut emoji = String::new();
            for code_point in code.split('-') {
                if let Ok(cp) = u32::from_str_radix(code_point, 16) {
                    if let Some(c) = char::from_u32(cp) {
                        emoji.push(c);
                    }
                }
            }
            if emoji.is_empty() {
                continue;
            }
            let mut push = |key: &str| {
                if !key.is_empty() {
                    let lower = key.to_lowercase();
                    let len = lower.chars().count() as i32;
                    entries.push((lower, emoji.clone(), len));
                }
            };
            let shortnames = std::iter::once(&e.shortname).chain(&e.shortname_alternates);
            for s in shortnames {
                push(s.trim_matches(':'));
            }
            for k in e.keywords.iter().chain(&e.ascii) {
                push(k);
            }
        }
        Ok(EmojiIndex { entries })
    }

    /// Tra fuzzy: emoji xếp theo điểm giảm dần (hoà → khóa ngắn/abc trước cho ổn định),
    /// loại trùng, tối đa `limit` kết quả. Query rỗng → rỗng (lịch sử gần dùng xử lý riêng).
    pub fn fuzzy_query(&self, query: &str, limit: usize) -> Vec<String> {
        if query.is_empty() {
            return Vec::new();
        }
        let q: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
        let mut scored: Vec<(i32, &str, &str)> = Vec::new();
        for (key, emoji, key_len) in &self.entries {
            if let Some(s) = score_prepared(&q, key, *key_len) {
                scored.push((s, key, emoji));
            }
        }
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.cmp(b.1))
                .then_with(|| a.2.cmp(b.2))
        });
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for (_, _, emoji) in scored {
            if seen.insert(emoji) {
                out.push(emoji.to_string());
                if out.len() >= limit {
                    break;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_prefix_beats_scattered() {
        // Khớp liền mạch từ đầu phải điểm cao hơn khớp rải rác.
        let prefix = fuzzy_score("grin", "grinning").unwrap();
        let scattered = fuzzy_score("grin", "green_running").unwrap();
        assert!(
            prefix > scattered,
            "prefix {prefix} phải > scattered {scattered}"
        );
    }

    #[test]
    fn score_word_start_bonus() {
        // Khớp tại đầu-từ (sau `_`) điểm cao hơn khớp giữa từ, cùng độ dài key.
        let word_start = fuzzy_score("he", "x_head").unwrap();
        let mid_word = fuzzy_score("he", "ashead").unwrap();
        assert!(
            word_start > mid_word,
            "đầu từ {word_start} phải > giữa từ {mid_word}"
        );
    }

    #[test]
    fn score_shorter_key_wins_ties() {
        // Cùng kiểu khớp → key ngắn hơn thắng (phạt theo độ dài).
        let short = fuzzy_score("cat", "cat").unwrap();
        let long = fuzzy_score("cat", "cat_with_wry_smile").unwrap();
        assert!(short > long, "key ngắn {short} phải > key dài {long}");
    }

    #[test]
    fn score_case_insensitive_and_no_match() {
        assert!(fuzzy_score("d", ":D").is_some()); // ascii key chữ hoa vẫn khớp
        assert!(fuzzy_score("xyz", "heart_eyes").is_none()); // không phải subsequence
        assert!(fuzzy_score("", "heart").is_none()); // query rỗng do recents xử lý, không fuzzy
        assert!(fuzzy_score("hearts", "heart").is_none()); // query dài hơn phần khớp được
    }

    fn index() -> EmojiIndex {
        EmojiIndex::from_emojione_str(include_str!("../data/emojione.json")).unwrap()
    }

    #[test]
    fn query_shortname_abbreviation_tops() {
        // Tiêu chí #63: `heye` phải ra heart_eyes (😍) ở top — shortname phải nằm trong chỉ mục.
        let res = index().fuzzy_query("heye", 60);
        let pos = res.iter().position(|e| e == "😍");
        assert!(
            pos.is_some_and(|p| p < 5),
            "😍 phải trong top 5, kết quả: {:?}",
            &res[..res.len().min(8)]
        );
    }

    #[test]
    fn query_full_shortname_matches() {
        // `heart_eyes` gõ đủ — trước đây KHÔNG match vì loader bỏ qua shortname.
        let res = index().fuzzy_query("heart_eyes", 60);
        assert_eq!(
            res.first().map(String::as_str),
            Some("😍"),
            "kết quả: {res:?}"
        );
    }

    #[test]
    fn query_keyword_prefix_still_tops() {
        // Hành vi cũ (prefix keyword) không hồi quy: grin → 😀 ở top.
        let res = index().fuzzy_query("grin", 60);
        let pos = res.iter().position(|e| e == "😀");
        assert!(
            pos.is_some_and(|p| p < 5),
            "😀 phải trong top 5, kết quả: {:?}",
            &res[..res.len().min(8)]
        );
    }

    #[test]
    fn query_empty_returns_nothing_and_dedups() {
        let idx = index();
        assert!(idx.fuzzy_query("", 60).is_empty());
        let res = idx.fuzzy_query("smile", 60);
        let mut seen = std::collections::HashSet::new();
        assert!(
            res.iter().all(|e| seen.insert(e.clone())),
            "kết quả trùng lặp"
        );
        assert!(res.len() <= 60);
    }

    #[test]
    fn query_full_table_fast_enough() {
        // Tiêu chí #63: không chậm đi rõ rệt. 100 truy vấn trên toàn bảng (~11k khóa) phải
        // xong trong giới hạn rộng rãi (CI runner chậm vẫn dư).
        let idx = index();
        let t0 = std::time::Instant::now();
        for q in ["heye", "grin", "smile", "ca", "th"]
            .iter()
            .cycle()
            .take(100)
        {
            let _ = idx.fuzzy_query(q, 60);
        }
        let dt = t0.elapsed();
        assert!(dt.as_secs_f64() < 2.0, "100 truy vấn mất {dt:?} (> 2s)");
    }
}
