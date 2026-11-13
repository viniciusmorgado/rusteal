//! Rust tests in Unreal's automation system: the Session Frontend's **Automation** tab, the
//! `Automation RunTests` console command and the engine's test reports list them with C++ ones.

use std::fmt::Display;

use crate::delegate_registry;
use crate::error::{RustealResult, check_ffi};
use crate::ffi_dispatch;

/// Where a test may run; [`Contexts::ALL`] when the test does not care.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contexts(u32);

impl Contexts {
    pub const EDITOR: Contexts = Contexts(0x1);
    pub const CLIENT: Contexts = Contexts(0x2);
    pub const SERVER: Contexts = Contexts(0x4);
    pub const COMMANDLET: Contexts = Contexts(0x8);
    pub const ALL: Contexts = Contexts(0x1 | 0x2 | 0x4 | 0x8);
}

impl std::ops::BitOr for Contexts {
    type Output = Contexts;

    fn bitor(self, other: Contexts) -> Contexts {
        Contexts(self.0 | other.0)
    }
}

/// Registers the automation test `name` (`"Game.Combat.DamageIsClamped"`; the dots make the
/// tree of the Automation tab), which runs `run` on the game thread: an `Err` fails the test with
/// its message, as [`add_error`] does. Registering a name again replaces the test, and a library's
/// tests go away when it unloads, so register them from `on_load!`.
#[track_caller]
pub fn register_test<E: Display>(
    name: &str,
    contexts: Contexts,
    mut run: impl FnMut() -> Result<(), E> + Send + 'static,
) -> RustealResult<()> {
    let callback_id = delegate_registry::register_callback(move |_| {
        if let Err(error) = run() {
            add_error(&error.to_string());
        }
    });

    check_ffi(unsafe {
        ffi_dispatch::world_automation_register_test(
            name.as_ptr(),
            name.len() as u32,
            contexts.0,
            callback_id,
        )
    })
}

/// Fails the running test with `message`, which the report shows; the test goes on.
pub fn add_error(message: &str) {
    add_message(0, message);
}

/// A warning in the running test's report.
pub fn add_warning(message: &str) {
    add_message(1, message);
}

/// A line in the running test's report.
pub fn add_info(message: &str) {
    add_message(2, message);
}

fn add_message(kind: u32, message: &str) {
    unsafe {
        ffi_dispatch::world_automation_add_message(kind, message.as_ptr(), message.len() as u32);
    }
}
