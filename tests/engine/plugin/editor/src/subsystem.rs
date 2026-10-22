use std::sync::atomic::{AtomicBool, Ordering};

use bindings::rusteal_editor::RustealEditorSubsystem;
use rusteal_runtime::{uclass, uclass_impl};

pub static INITIALIZED: AtomicBool = AtomicBool::new(false);

#[uclass(parent = RustealEditorSubsystem)]
pub struct ProbeEditorSubsystem {
    #[uproperty]
    initializations: i32,
}

#[uclass_impl]
impl ProbeEditorSubsystem {
    #[ufunction(Override)]
    fn receive_initialize(&mut self) {
        self.set_initializations(self.initializations() + 1);
        INITIALIZED.store(true, Ordering::SeqCst);
    }
}
