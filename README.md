# rust-ibus-bamboo

A pure-Rust port of [BambooEngine/ibus-bamboo](https://github.com/BambooEngine/ibus-bamboo),
the Vietnamese IBus input method engine (originally Go + cgo).

No cgo: the IBus protocol is implemented over [`zbus`](https://crates.io/crates/zbus) and X11
integration over [`x11rb`](https://crates.io/crates/x11rb).

## Workspace layout

| Crate | Ported from | Status |
|-------|-------------|--------|
| `bamboo-core` | `bamboo-core/*` | ✅ Complete — Telex/VNI/VIQR transformation, spelling, charset. All 47 upstream Go tests pass. |
| `bamboo-config` | `config/*` | ✅ Complete — JSON config (field-compatible), flags, paths. |
| `bamboo-emoji` | `emoji.go`, `trie.go`, `mactab.go` | ✅ Complete — emoji trie + macro table. Upstream emoji tests pass. |
| `bamboo-ibus` | `engine*.go`, `ibus_const.go`, goibus | ✅ Preedit mode + full IBus D-Bus transport (zbus). |
| `bamboo-platform` | `x11*.{go,c}`, `wl_*.go` | ◐ X11 WM_CLASS detection. Wayland + XTest injection are follow-ups. |
| `ibus-bamboo` (bin) | `main.go` | ✅ Builds; `--version` and `--ibus` embedded mode. |

The faithful port of the transformation engine (`bamboo-core`) was the priority: it is verified
against the upstream Go test suite, mapping Go's aliased `*Transformation` pointers to
`Rc<RefCell<Transformation>>` (pointer identity → `Rc::ptr_eq`, mutation → `borrow_mut`).

## Building

```sh
cargo build --workspace          # all crates + binary
cargo test --workspace           # 62 tests
cargo fmt --all --check          # formatting gate (CI-enforced)
cargo clippy --workspace --all-targets -- -D warnings   # lint gate
./target/debug/ibus-bamboo --version
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for the crate dependency graph and design rationale, and
[CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow and how to regenerate data tables.

## Install & use (Linux / IBus)

The engine installs as **`BambooRs`** ("Bamboo (Rust)"), side-by-side with any existing Go
`ibus-bamboo` — they use distinct IBus names so both can be present.

```sh
cargo build --release -p ibus-bamboo
! bash tools/install.sh      # copies the component XML to /usr/share/ibus/component (needs sudo),
                             # installs the binary under ~/.local/lib, refreshes IBus,
                             # and adds BambooRs to your GNOME input sources
```

Then press **Super+Space** to switch to *Bamboo (Rust)* and type Telex (e.g. `vieetj` → `việt`).
Remove it any time with `bash tools/uninstall.sh` (the Go engine is left untouched).

> IBus only scans `/usr/share/ibus/component` on most setups, so the component XML needs root;
> the engine binary itself stays in your home directory. A live end-to-end check is in
> `cargo run -p bamboo-ibus --example smoketest`.

## Architecture notes

- `bamboo-core` is `Rc`-based (single-threaded). Because zbus interfaces must be `Send + Sync`,
  the engine runs on a dedicated thread behind a channel-based actor (`bamboo-ibus::EngineHandle`).
- The Preedit-mode key-handling logic (`bamboo-ibus::core`) is transport-independent: it returns
  a list of `Action`s (commit / update-preedit / hide), so the full IME behaviour is unit-tested
  without a live IBus daemon. The D-Bus layer translates `Action`s into IBus signals.

## Not yet ported (follow-ups)

The default Preedit input mode works end-to-end. These upstream features remain to be ported and
are tracked as future sub-projects (each can't be auto-tested without a live IBus daemon + display):

- Backspace-correction input modes (`engine_backspace.go`) and XTest / Wayland key injection.
- Emoji and hexadecimal lookup tables (`engine_emoji.go`, `engine_hexadecimal.go`).
- Shortcut keys, property menu (`prop.go`), dictionary-based spell check.
- The GTK setup UI (`ui/`) and the Go standalone component-registration path.

See `docs/superpowers/specs/2026-06-18-rust-ibus-bamboo-design.md` for the full design.
