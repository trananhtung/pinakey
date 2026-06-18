# rust-ibus-bamboo — Design

Date: 2026-06-18

## Goal

Port the Go project [BambooEngine/ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo)
(a Vietnamese IBus input method engine) to **pure Rust**, producing a binary that can
replace the original `ibus-bamboo`. Target both **X11** and **Wayland**, using pure-Rust
crates (`zbus` for D-Bus/IBus, `x11rb` for X11, `wayland-client` for Wayland) — no cgo / C.

## Decomposition (Cargo workspace)

Built bottom-up. Each crate has one clear purpose, a defined interface, and is testable
in isolation.

| # | Crate | Go origin | Depends on | Nature |
|---|-------|-----------|------------|--------|
| 1 | `bamboo-core` | `bamboo-core/*` | — | Vietnamese transformation engine: Telex/VNI/VIQR rules, spelling check, charset encoding, trie. Pure logic. |
| 2 | `bamboo-config` | `config/*` | core | Read/write config file, flags, input-method definitions. |
| 3 | `bamboo-emoji` | `emoji.go`, `trie.go`, `mactab.go` | core | Emoji trie lookup + macro table. |
| 4 | `bamboo-platform` | `x11*.{go,c}`, `wl_*.go`, `gnome_*` | — | X11 (x11rb) + Wayland (wayland-client): window-class introspection, clipboard, fake key/backspace injection. |
| 5 | `bamboo-ibus` | `engine*.go`, `client.go`, `prop.go`, `ibus_const.go`, `fake_engine.go` | 1–4 + zbus | IBus engine over D-Bus: factory, process_key_event, preedit, commit, lookup table, properties. |
| 6 | `ibus-bamboo` (bin) | `main.go`, `version.go` | all | Binary: arg parsing, D-Bus bus, component registration, wiring. |

Order: 1 → (2, 3, 4 in parallel) → 5 → 6.

## Sub-project 1: bamboo-core (this iteration)

### Why first
Pure logic, no system dependencies, and the original ships a comprehensive Go test suite
(`bamboo_test.go`, `utils_test.go`, `rules_parser_test.go`). We port those tests verbatim
as the gold-standard verification — bamboo-core is "done" only when every ported test passes.

### Key porting decision: pointer aliasing
The Go engine represents a syllable as `[]*Transformation` where each `Transformation` may
point (`Target *Transformation`) at another element in the same slice, identity is compared
by pointer (`t.Target == trans`), and targets are mutated in place
(`refreshLastToneTarget` reassigns `lastToneTrans.Target`).

Faithful Rust mapping: `Rc<RefCell<Transformation>>`.
- pointer identity `a == b` → `Rc::ptr_eq(&a, &b)`
- mutation `t.Target = x` → `t.borrow_mut().target = Some(x)`
- `nil` target → `Option<Rc<RefCell<Transformation>>>`

This makes the translation mechanical and line-for-line verifiable against the Go source.

### Module map (`crates/bamboo-core/src/`)
- `rules.rs` — `Mode`, `EffectType`, `Mark`, `Tone`, `Rule`, `Transformation`, flag consts (`rules_parser.go` types + `bamboo.go` consts).
- `utils.rs` — vowel/mark/tone tables and helpers (`utils.go`).
- `input_method_def.rs` — the input-method definition tables (`input_method_def.go`).
- `rules_parser.rs` — `parse_rules`, `parse_toneless_rules`, `parse_input_method` (`rules_parser.go`).
- `spelling.rs` — CVC spelling validation matrices (`spelling.go`).
- `flattener.rs` — `flatten` / `get_canvas` (`flattener.go`).
- `bamboo_utils.rs` — the transformation algorithm (`bamboo_utils.go`).
- `charset.rs` — `encode`, charset tables (`encoder.go` + generated `charset_def`).
- `engine.rs` — `Engine` struct + public API mirroring `IEngine` (`bamboo.go`).
- `lib.rs` — re-exports; `#[cfg(test)]` ports of the three Go test files.

### Public API (mirrors Go `IEngine`)
```rust
pub trait IEngine {
    fn set_flag(&mut self, flag: u32);
    fn get_input_method(&self) -> &InputMethod;
    fn process_key(&mut self, key: char, mode: Mode);
    fn process_string(&mut self, s: &str, mode: Mode);
    fn get_processed_string(&self, mode: Mode) -> String;
    fn is_valid(&self, input_is_full_complete: bool) -> bool;
    fn can_process_key(&self, key: char) -> bool;
    fn remove_last_char(&mut self, refresh_last_tone_target: bool);
    fn restore_last_word(&mut self, to_vietnamese: bool);
    fn reset(&mut self);
}
```
`Mode` and flags are bitflags matching the Go `iota` values exactly.

### Testing
Port `bamboo_test.go`, `utils_test.go`, `rules_parser_test.go` to `#[test]` functions.
Success criterion: `cargo test -p bamboo-core` green, all original assertions preserved.

## Subsequent sub-projects
Each of crates 2–6 gets its own spec + plan + implementation cycle once the crate below it
is green. The IBus/platform layers (highest risk, hardest to auto-test) come last and are
verified by running against a live IBus daemon.
