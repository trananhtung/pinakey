# Contributing

Thanks for helping with the Rust port of ibus-bamboo. This guide covers the local workflow,
the quality gates CI enforces, and how to regenerate the generated data.

## Prerequisites

- A recent stable Rust toolchain (`rustup` recommended) with `rustfmt` and `clippy`:
  ```sh
  rustup component add rustfmt clippy
  ```
- System libraries for the X11 / D-Bus paths (Debian/Ubuntu names):
  ```sh
  sudo apt-get install -y libxcb1-dev libdbus-1-dev pkg-config
  ```
- Python 3 (only to regenerate charset tables).

## Everyday workflow

```sh
cargo build --workspace          # build all crates + the binary
cargo test --workspace           # run all tests
cargo fmt --all                  # format
cargo clippy --workspace --all-targets   # lint
```

## Quality gates (must pass before merge)

CI (`.github/workflows/ci.yml`) runs these with warnings denied. Run them locally first:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Formatting is pinned by `rustfmt.toml`. Workspace-wide lints live under `[workspace.lints]` in the
root `Cargo.toml`; each crate opts in with `[lints] workspace = true`.

### Deviating from a clippy lint

This is a **port**: where code intentionally mirrors the upstream Go source (so the two stay
diff-comparable), keeping that shape is more valuable than clippy's idiomatic rewrite. In those few
cases, add a **targeted** `#[allow(clippy::...)]` with a comment explaining which Go source it
mirrors — see `flattener.rs`, `spelling.rs`, `bamboo_utils.rs` for the pattern. Do not blanket-allow
lints at the crate level.

## Project layout

See [ARCHITECTURE.md](ARCHITECTURE.md) for the crate dependency graph and the two load-bearing
design decisions (pointer aliasing → `Rc<RefCell>`, and the engine actor thread). In short:

- Put **pure transformation logic** in `bamboo-core`. It has no I/O and no sibling dependencies.
- Put **transport-independent IBus behaviour** in `bamboo-ibus::core`, returning `Action`s so it
  stays unit-testable without a live daemon. Keep `bamboo-ibus::dbus` a thin translation layer.
- Keep files focused; if a module starts doing several unrelated things, split it.

## Tests

- `bamboo-core` is verified against the upstream Go gold-standard suite, ported verbatim into
  `crates/bamboo-core/tests/`. When porting more upstream behaviour, port its Go test alongside.
- Test the **pure logic**, not the D-Bus/display plumbing (which CI can't run). If you add IBus
  behaviour, express it as `core::Action`s and test those.

## Regenerating charset tables

`crates/bamboo-core/src/charset_def.rs` is generated — never edit it by hand. To regenerate after an
upstream change:

```sh
git clone https://github.com/BambooEngine/bamboo-core /tmp/bamboo-core-src
BAMBOO_GO_SRC=/tmp/bamboo-core-src python3 tools/gen_charset.py
cargo fmt --all          # the generator emits compact output; fmt normalises it
git diff                 # review the change
```

The generator is deterministic: regenerating from the same source and running `cargo fmt` yields a
byte-identical file.

## Not-yet-ported features

The default Preedit input mode works end-to-end. Larger follow-ups (backspace-correction modes +
key injection, emoji/hex lookup tables, shortcuts, property menu, dictionary spell-check, the GTK
setup UI, standalone component registration) are listed in [README.md](README.md#not-yet-ported-follow-ups).
Each needs a live IBus daemon + display to test fully.
