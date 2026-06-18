# PinaKey

**PinaKey** is a Vietnamese input method engine (IME) for Linux/IBus, written in pure Rust —
Telex / VNI / VIQR typing with no cgo. The IBus protocol is implemented over
[`zbus`](https://crates.io/crates/zbus) and X11 integration over
[`x11rb`](https://crates.io/crates/x11rb).

## The name

**PinaKey** honours **Francisco de Pina** (1585–1625), the Portuguese Jesuit who first
systematically romanised Vietnamese in Thanh Chiêm – Hội An and laid the foundations of
**chữ Quốc Ngữ** — the script every Vietnamese keyboard types today. He taught Vietnamese to
Alexandre de Rhodes and is too often forgotten behind him; this engine is a small tribute.
The **"Key"** suffix marks it as a bộ gõ (keyboard / input method).

> PinaKey tham khảo ý tưởng từ **Bamboo** (bộ gõ ibus-bamboo).

## Workspace layout

| Crate | Responsibility | Status |
|-------|----------------|--------|
| `pinakey-core` | Telex/VNI/VIQR transformation, spelling, charset encoding. | ✅ Complete — 47 transformation tests pass. |
| `pinakey-config` | JSON config, feature flags, config paths. | ✅ Complete. |
| `pinakey-emoji` | Emoji trie + macro table. | ✅ Complete. |
| `pinakey-ibus` | Preedit-mode engine logic + full IBus D-Bus transport (zbus). | ✅ Complete. |
| `pinakey-platform` | X11 (XWayland) focused-window class detection. | ◐ Wayland-native + XTest injection are follow-ups. |
| `pinakey` (bin) | The engine binary: `--version` and `--ibus` embedded mode. | ✅ |

The transformation engine (`pinakey-core`) is the heart of the project and is covered by a
behavioural test suite mapping aliased `*Transformation` pointers to `Rc<RefCell<Transformation>>`
(pointer identity → `Rc::ptr_eq`, mutation → `borrow_mut`).

## Building

```sh
cargo build --workspace          # all crates + binary
cargo test --workspace           # 62 tests
cargo fmt --all --check          # formatting gate (CI-enforced)
cargo clippy --workspace --all-targets -- -D warnings   # lint gate
./target/debug/pinakey --version
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the crate dependency graph and design rationale, and
[CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow and how to regenerate data tables.

## Install & use (Linux / IBus)

```sh
cargo build --release -p pinakey
! bash tools/install.sh      # copies the component XML to /usr/share/ibus/component (needs sudo),
                             # installs the binary + icon under ~/.local/lib/pinakey, refreshes
                             # IBus, and adds PinaKey to your GNOME input sources
```

Then press **Super+Space** to switch to *PinaKey — Bộ gõ tiếng Việt* and type Telex
(e.g. `vieetj` → `việt`). Remove it any time with `bash tools/uninstall.sh`.

> IBus only scans `/usr/share/ibus/component` on most setups, so the component XML needs root;
> the engine binary itself stays in your home directory. A live end-to-end check is in
> `cargo run -p pinakey-ibus --example smoketest`.

## Architecture notes

- `pinakey-core` is `Rc`-based (single-threaded). Because zbus interfaces must be `Send + Sync`,
  the engine runs on a dedicated thread behind a channel-based actor (`pinakey-ibus::EngineHandle`).
- The Preedit-mode key-handling logic (`pinakey-ibus::core`) is transport-independent: it returns
  a list of `Action`s (commit / update-preedit / hide), so the full IME behaviour is unit-tested
  without a live IBus daemon. The D-Bus layer translates `Action`s into IBus signals.

## Not yet implemented (follow-ups)

The default Preedit input mode works end-to-end. These features remain (each needs a live IBus
daemon + display to test fully):

- Backspace-correction input modes and XTest / Wayland key injection.
- Emoji and hexadecimal lookup tables.
- Shortcut keys, property menu, dictionary-based spell check.
- A graphical setup UI.

See `docs/superpowers/specs/2026-06-18-pinakey-design.md` for the full design.
