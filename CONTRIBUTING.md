# Contributing

Thanks for helping with PinaKey. This guide covers the local workflow, the quality gates CI
enforces, and how to regenerate the generated data.

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

A few spots intentionally keep a non-idiomatic shape that mirrors the reference algorithm (so the
behaviour stays easy to compare against it). In those cases, add a **targeted**
`#[allow(clippy::...)]` with a comment explaining why — see `flattener.rs`, `spelling.rs`,
`transform_utils.rs` for the pattern. Do not blanket-allow lints at the crate level.

## Project layout

See [ARCHITECTURE.md](ARCHITECTURE.md) for the crate dependency graph and the two load-bearing
design decisions (pointer aliasing → `Rc<RefCell>`, and the engine actor thread). In short:

- Put **pure transformation logic** in `pinakey-core`. It has no I/O and no sibling dependencies.
- Put **transport-independent IBus behaviour** in `pinakey-ibus::core`, returning `Action`s so it
  stays unit-testable without a live daemon. Keep `pinakey-ibus::dbus` a thin translation layer.
- Keep files focused; if a module starts doing several unrelated things, split it.

## Tests

- `pinakey-core` is covered by the behavioural suite in `crates/pinakey-core/tests/`. When you add
  transformation behaviour, add a test alongside.
- Test the **pure logic**, not the D-Bus/display plumbing (which CI can't run). If you add IBus
  behaviour, express it as `core::Action`s and test those.

## Regenerating charset tables

`crates/pinakey-core/src/charset_def.rs` is generated — never edit it by hand. The legacy charset
data originates from the upstream reference project; to regenerate after a change to it:

```sh
git clone https://github.com/BambooEngine/bamboo-core /tmp/bamboo-src
BAMBOO_GO_SRC=/tmp/bamboo-src python3 tools/gen_charset.py
cargo fmt --all          # the generator emits compact output; fmt normalises it
git diff                 # review the change
```

The generator is deterministic: regenerating from the same source and running `cargo fmt` yields a
byte-identical file.

## Not yet implemented

The default Preedit input mode works end-to-end. Larger follow-ups (backspace-correction modes +
key injection, emoji/hex lookup tables, shortcuts, property menu, dictionary spell-check, a
graphical setup UI) are listed in [README.md](README.md#not-yet-implemented-follow-ups).
Each needs a live IBus daemon + display to test fully.
