# Architecture

PinaKey is a Cargo workspace of six crates. Dependencies flow strictly bottom-up; no crate depends
on one above it, so each layer can be understood and tested in isolation.

```
                 ┌─────────────────────┐
                 │       pinakey       │  binary: arg parsing, tokio runtime
                 │    (src/main.rs)    │
                 └──────────┬──────────┘
                            │
                 ┌──────────▼──────────┐
                 │    pinakey-ibus     │  IBus engine: D-Bus transport + key-handling logic
                 └───┬──────────┬────┬─┘
          ┌─────────┘           │    └──────────────┐
          │                     │                   │
┌─────────▼────────┐ ┌──────────▼─────┐ ┌───────────▼───────┐
│  pinakey-config  │ │  pinakey-emoji │ │  pinakey-platform │  X11/Wayland integration
└─────────┬────────┘ └──────────┬─────┘ └───────────────────┘
          │                     │
          └──────────┬──────────┘
                     │
          ┌──────────▼──────────┐
          │    pinakey-core     │  transformation engine (no I/O, no deps on siblings)
          └─────────────────────┘
```

## Crates

| Crate | Responsibility | Key dependencies |
|-------|----------------|------------------|
| `pinakey-core` | Telex/VNI/VIQR transformation, spelling validation, charset encoding. Pure logic, single-threaded, no I/O. | `once_cell`, `regex` |
| `pinakey-config` | Load/save JSON config, feature flags, config paths. | `pinakey-core`, `serde`, `dirs` |
| `pinakey-emoji` | Emoji trie lookup and macro table. | `serde` |
| `pinakey-ibus` | The IBus engine: transport-independent key-handling (`core`) plus the D-Bus protocol surface (`dbus`, behind the `dbus` feature). | the three crates above, `pinakey-platform`, `zbus` |
| `pinakey-platform` | Focused-window class detection (X11). Wayland introspection and XTest key injection are follow-ups. | `x11rb` |
| `pinakey` | The binary. Parses args (`--version`, `--ibus`) and starts the embedded engine. | `pinakey-ibus`, `tokio` |

## Two design decisions worth knowing

### 1. Pointer aliasing → `Rc<RefCell<Transformation>>`

The transformation algorithm keeps a list of `Transformation`s where each element's `target` is an
aliased pointer to another element in the same list, relying on **pointer identity** and
**in-place mutation**. PinaKey models this with:

- `Rc<RefCell<Transformation>>` (aliased: `TransRef`) — shared, mutable nodes.
- `Rc::ptr_eq` — pointer identity comparison.
- `Rc::as_ptr(..) as usize` — a stable key for the appending map.
- `borrow_mut()` — in-place mutation.

This is why `pinakey-core` is single-threaded: `Rc`/`RefCell` are not `Send`/`Sync`.

### 2. Non-`Send` engine vs. `Send + Sync` D-Bus → actor thread

`zbus` requires interface objects to be `Send + Sync`, but `pinakey-core` is `Rc`-based and cannot
cross threads. The engine therefore runs on its **own dedicated thread** behind a channel-based
actor, `pinakey-ibus::EngineHandle`. The handle is `Send + Sync` and forwards key events / reset /
window-class updates over an `mpsc` channel; the engine thread owns the non-`Send` state.

## Data flow for a keystroke

```
IBus daemon ──ProcessKeyEvent──▶ dbus::PinaKeyEngine
                                      │ EngineHandle.process_key(keyval, keycode, state)
                                      ▼
                              engine thread: core::EngineCore.process_key_event
                                      │ returns (handled: bool, Vec<Action>)
                                      ▼
                              dbus::apply_actions  ──emits IBus signals──▶ IBus daemon
```

`core::Action` (`CommitText`, `UpdatePreedit`, `HidePreedit`, …) is transport-independent, so the
full Preedit-mode behaviour is unit-tested in `pinakey-ibus` **without** a live IBus daemon. The
D-Bus layer's only job is to translate `Action`s into IBus signals.

## Testing strategy

- `pinakey-core` is covered by a behavioural test suite in `crates/pinakey-core/tests/`
  (`transformation.rs`, `utils.rs`, `rules_parser.rs`), run against the public API as an external
  consumer.
- `pinakey-ibus::core`, `pinakey-config`, `pinakey-emoji`, and `pinakey-platform::parse_wm_class`
  have unit tests for their pure logic.
- The D-Bus and live-display paths cannot be exercised in CI (no IBus daemon / display); they are
  compile-checked and kept thin so the tested `core` carries the behaviour.

## Generated data

`crates/pinakey-core/src/charset_def.rs` (~2,100 charset entries) is **generated** by
`tools/gen_charset.py`. Do not edit it by hand. See
[CONTRIBUTING.md](CONTRIBUTING.md#regenerating-charset-tables).

See [README.md](README.md) for build instructions and the list of not-yet-implemented features.
