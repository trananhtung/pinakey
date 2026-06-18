//! Actor that owns the (non-`Send`) [`EngineCore`] on a dedicated thread.
//!
//! `bamboo-core` uses `Rc<RefCell<..>>` internally, so the engine cannot be shared across threads
//! or held by an async zbus interface (which requires `Send + Sync`). We run the engine on its own
//! thread and talk to it over channels; the returned [`EngineHandle`] is a cheap `Send + Sync`
//! sender.

use std::sync::mpsc::{channel, Sender};
use std::thread;

use bamboo_config::Config;

use crate::core::{Action, EngineCore};

enum Command {
    ProcessKey {
        keyval: u32,
        keycode: u32,
        state: u32,
        reply: Sender<(bool, Vec<Action>)>,
    },
    Reset,
    SetWmClass(String),
    Shutdown,
}

#[derive(Clone)]
pub struct EngineHandle {
    tx: Sender<Command>,
}

impl EngineHandle {
    pub fn spawn(config: Config) -> EngineHandle {
        let (tx, rx) = channel::<Command>();
        thread::spawn(move || {
            let mut core = EngineCore::new(config);
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    Command::ProcessKey {
                        keyval,
                        keycode,
                        state,
                        reply,
                    } => {
                        let result = core.process_key_event(keyval, keycode, state);
                        let _ = reply.send(result);
                    }
                    Command::Reset => core.reset_preeditor(),
                    Command::SetWmClass(c) => core.set_wm_class(c),
                    Command::Shutdown => break,
                }
            }
        });
        EngineHandle { tx }
    }

    pub fn process_key(&self, keyval: u32, keycode: u32, state: u32) -> (bool, Vec<Action>) {
        let (reply, rx) = channel();
        if self
            .tx
            .send(Command::ProcessKey {
                keyval,
                keycode,
                state,
                reply,
            })
            .is_err()
        {
            return (false, Vec::new());
        }
        rx.recv().unwrap_or((false, Vec::new()))
    }

    pub fn reset(&self) {
        let _ = self.tx.send(Command::Reset);
    }

    pub fn set_wm_class(&self, class: String) {
        let _ = self.tx.send(Command::SetWmClass(class));
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(Command::Shutdown);
    }
}
