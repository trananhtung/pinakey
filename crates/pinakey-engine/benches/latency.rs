//! Benchmark độ trễ lõi PinaKey (issue #71) — reproducible bằng criterion:
//!
//! ```sh
//! cargo bench -p pinakey-engine
//! # báo cáo HTML (nếu bật feature mặc định của criterion): target/criterion/report/index.html
//! ```
//!
//! Đo đúng đường nóng mà mỗi phím gõ của người dùng đi qua (`process_key_event`), không đo
//! I/O hay D-Bus. Con số quan trọng: độ trễ THÊM VÀO mỗi phím phải không đáng kể so với
//! ngưỡng cảm nhận được (~10ms).

use criterion::{criterion_group, criterion_main, Criterion};
use pinakey_config::default_cfg;
use pinakey_engine::EngineCore;
use std::hint::black_box;

/// Gõ một chuỗi ASCII qua engine, trả về tổng số action (black_box chống tối ưu hoá rỗng).
fn type_str(core: &mut EngineCore, s: &str) -> usize {
    let mut n = 0;
    for c in s.chars() {
        let (_h, actions) = core.process_key_event(c as u32, 0, 0);
        n += actions.len();
    }
    n
}

/// Một phím giữa từ đang soạn — trường hợp phổ biến nhất khi gõ trôi chảy.
fn bench_per_key(c: &mut Criterion) {
    c.bench_function("per_key_mid_word", |b| {
        let mut core = EngineCore::new(default_cfg());
        type_str(&mut core, "vie");
        b.iter(|| {
            // "e" thứ hai kích hoạt biến đổi ê — phím "nặng" điển hình của Telex.
            let (_h, actions) = core.process_key_event(black_box('e' as u32), 0, 0);
            core.process_key_event(0xff08, 0, 0); // Backspace trả lại trạng thái "vie"
            black_box(actions.len())
        });
    });
}

/// Cả một câu dài gõ liên tục (kèm dấu + word-break) — thông lượng thực tế.
fn bench_paragraph(c: &mut Criterion) {
    const PARAGRAPH: &str = "tieengs vieejt laf ngoon nguwx cuar nguwowfi vieejt nam \
                             chungs ta cufng nhau giwx gifn suwj trong sangs cuar tieengs vieejt ";
    c.bench_function("paragraph_88_keys", |b| {
        b.iter(|| {
            let mut core = EngineCore::new(default_cfg());
            black_box(type_str(&mut core, black_box(PARAGRAPH)))
        });
    });
}

/// Worst-case: từ dài không phải tiếng Việt → engine phải khôi phục nguyên văn khi ngắt từ
/// (auto non-VN restore) — đường đắt nhất của một phím đơn.
fn bench_worst_case_restore(c: &mut Criterion) {
    c.bench_function("worst_case_non_vn_restore", |b| {
        b.iter(|| {
            let mut core = EngineCore::new(default_cfg());
            type_str(&mut core, "aáàrgh mixedwordxs");
            let (_h, actions) = core.process_key_event(black_box(' ' as u32), 0, 0);
            black_box(actions.len())
        });
    });
}

/// Tra emoji fuzzy trên toàn bảng (~11k khóa) — mỗi phím trong chế độ `:` quét một lần.
fn bench_emoji_fuzzy(c: &mut Criterion) {
    let index = pinakey_emoji::EmojiIndex::from_emojione_str(include_str!(
        "../../pinakey-emoji/data/emojione.json"
    ))
    .unwrap();
    c.bench_function("emoji_fuzzy_full_table", |b| {
        b.iter(|| black_box(index.fuzzy_query(black_box("heye"), 60)).len());
    });
}

criterion_group!(
    benches,
    bench_per_key,
    bench_paragraph,
    bench_worst_case_restore,
    bench_emoji_fuzzy
);
criterion_main!(benches);
