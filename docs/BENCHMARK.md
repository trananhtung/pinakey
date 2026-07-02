# Benchmark độ trễ (issue #71)

PinaKey trả lời câu hỏi "performance có ngon không?" bằng số liệu **tự chạy lại được**,
không phải cảm tính. Toàn bộ benchmark nằm trong repo, đo bằng
[criterion](https://github.com/bheisler/criterion.rs):

```sh
cargo bench -p pinakey-engine
# hoặc: bash tools/bench.sh
```

## Đo cái gì

Benchmark đo đúng **đường nóng mà mỗi phím gõ đi qua** (`EngineCore::process_key_event` —
lõi Rust thuần, không I/O, không D-Bus). Đây là độ trễ PinaKey **thêm vào** mỗi phím; phần
còn lại (fcitx5, D-Bus, toolkit vẽ chữ) như nhau với mọi bộ gõ.

| Benchmark | Nội dung | Kết quả tham chiếu¹ |
|---|---|---|
| `per_key_mid_word` | 1 phím biến đổi Telex giữa từ đang soạn (trường hợp phổ biến nhất) | **~14 µs** |
| `paragraph_88_keys` | Câu 88 phím gõ liên tục, đủ dấu + word-break | ~2,1 ms (≈24 µs/phím) |
| `worst_case_non_vn_restore` | 18 phím từ hỗn tạp + khôi phục nguyên văn khi ngắt từ (đường đắt nhất) | ~0,6 ms |
| `emoji_fuzzy_full_table` | 1 truy vấn fuzzy quét toàn bảng emoji ~11k khóa (mỗi phím trong chế độ `:`) | ~180 µs |

¹ Đo ngày 2026-07-02 trên Intel Core i7-1260P (laptop), Linux 6.17, rustc 1.90, build
release. Máy khác số khác — hãy tự chạy; điều quan trọng là **bậc độ lớn**.

## Đọc số thế nào

- Ngưỡng con người bắt đầu cảm nhận trễ gõ phím vào khoảng **10 ms**. Phím nặng nhất của
  PinaKey (~14 µs) cách ngưỡng đó **~700 lần**.
- Không có GC: lõi Rust không có garbage collector — không có khựng ngẫu nhiên giữa chừng.
- Không sleep/chờ trong event loop: addon fcitx5 không chặn vòng lặp sự kiện; đường uinput
  (thử nghiệm) đồng bộ bằng ACK thật thay vì ngủ chờ.
- Chất lượng đo được: toàn bộ hành vi engine phủ bằng unit test thuần Rust (chạy trong ms),
  cộng e2e chạy **fcitx5 thật** trong CI cho mọi PR.

## Ghi chú reproducibility

- Benchmark không chạy trong CI (số trên runner chia sẻ tài nguyên không ổn định);
  criterion tự lưu baseline ở `target/criterion/` để bạn so sánh giữa các lần chạy/commit.
- Khi thay đổi đường nóng của engine, chạy `cargo bench -p pinakey-engine` trước/sau để
  thấy chênh lệch (criterion in % thay đổi so với lần trước).
