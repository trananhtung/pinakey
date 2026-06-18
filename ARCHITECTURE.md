# Architecture

`rust-ibus-bamboo` is a Cargo workspace of six crates. Dependencies flow strictly bottom-up;
no crate depends on one above it, so each layer can be understood and tested in isolation.

```
                 ┌───────────────────┐
                 │   ibus-bamboo     │  binary: arg parsing, tokio runtime
                 │   (src/main.rs)   │
                 └─────────┬─────────┘
                           │
                 ┌─────────▼─────────┐
                 │    bamboo-ibus    │  IBus engine: D-Bus transport + key-handling logic
                 └──┬─────────┬───┬──┘
          ┌─────────┘         │   └──────────────┐
          │                   │                  │
┌─────────▼───────┐ ┌─────────▼──────┐ ┌─────────▼────────┐
│  bamboo-config  │ │  bamboo-emoji  │ │  bamboo-platform │  X11/Wayland integration
└─────────┬───────┘ └─────────┬──────┘ └──────────────────┘
          │                   │
          └─────────┬─────────┘
                    │
          ┌─────────▼─────────┐
          │    bamboo-core    │  transformation engine (no I/O, no deps on siblings)
          └───────────────────┘
```

## Crates

| Crate | Responsibility | Key dependencies |
|-------|----------------|------------------|
| `bamboo-core` | Telex/VNI/VIQR transformation, spelling validation, charset encoding. Pure logic, single-threaded, no I/O. | `once_cell`, `regex` |
| `bamboo-config` | Load/save JSON config (field-compatible with the Go version), feature flags, config paths. | `bamboo-core`, `serde`, `dirs` |
| `bamboo-emoji` | Emoji trie lookup and macro table. | `serde` |
| `bamboo-ibus` | The IBus engine: transport-independent key-handling (`core`) plus the D-Bus protocol surface (`dbus`, behind the `dbus` feature). | the three crates above, `bamboo-platform`, `zbus` |
| `bamboo-platform` | Focused-window class detection (X11). Wayland introspection and XTest key injection are follow-ups. | `x11rb` |
| `ibus-bamboo` | The binary. Parses args (`--version`, `--ibus`) and starts the embedded engine. | `bamboo-ibus`, `tokio` |

## Two design decisions worth knowing

### 1. Pointer aliasing → `Rc<RefCell<Transformation>>`

The Go engine stores `[]*Transformation` where each element's `Target` field is an aliased pointer
to another element in the same slice, and the algorithm relies on **pointer identity** and
**in-place mutation**. The Rust port reproduces this exactly:

- `Rc<RefCell<Transformation>>` (aliased: `TransRef`) — shared, mutable nodes.
- `Rc::ptr_eq` — pointer identity comparison.
- `Rc::as_ptr(..) as usize` — a stable key for the `appendingMap` (Go keyed it by pointer).
- `borrow_mut()` — in-place mutation.

This is why `bamboo-core` is single-threaded: `Rc`/`RefCell` are not `Send`/`Sync`.

### 2. Non-`Send` engine vs. `Send + Sync` D-Bus → actor thread

`zbus` requires interface objects to be `Send + Sync`, but `bamboo-core` is `Rc`-based and cannot
cross threads. The engine therefore runs on its **own dedicated thread** behind a channel-based
actor, `bamboo-ibus::EngineHandle`. The handle is `Send + Sync` and forwards key events / reset /
window-class updates over an `mpsc` channel; the engine thread owns the non-`Send` state.

## Data flow for a keystroke

```
IBus daemon ──ProcessKeyEvent──▶ dbus::BambooEngine
                                      │ EngineHandle.process_key(keyval, keycode, state)
                                      ▼
                              engine thread: core::EngineCore.process_key_event
                                      │ returns (handled: bool, Vec<Action>)
                                      ▼
                              dbus::apply_actions  ──emits IBus signals──▶ IBus daemon
```

`core::Action` (`CommitText`, `UpdatePreedit`, `HidePreedit`, …) is transport-independent, so the
full Preedit-mode behaviour is unit-tested in `bamboo-ibus` **without** a live IBus daemon. The
D-Bus layer's only job is to translate `Action`s into IBus signals.

## Testing strategy

- `bamboo-core` is verified against the **upstream Go gold-standard test suite**, ported verbatim
  into `crates/bamboo-core/tests/` (`transformation.rs`, `utils.rs`, `rules_parser.rs`). These run
  against the public API as an external consumer.
- `bamboo-ibus::core`, `bamboo-config`, `bamboo-emoji`, and `bamboo-platform::parse_wm_class` have
  unit tests for their pure logic.
- The D-Bus and live-display paths cannot be exercised in CI (no IBus daemon / display); they are
  compile-checked and kept thin so the tested `core` carries the behaviour.

## Generated data

`crates/bamboo-core/src/charset_def.rs` (~2,100 entries) is **generated** from upstream
`charset_def.go` by `tools/gen_charset.py`. Do not edit it by hand. See
[CONTRIBUTING.md](CONTRIBUTING.md#regenerating-charset-tables).

See [README.md](README.md) for build instructions and the list of not-yet-ported features.
