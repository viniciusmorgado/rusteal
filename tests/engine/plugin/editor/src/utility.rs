use std::sync::atomic::{AtomicI32, Ordering};

use bindings::blutility::EditorUtilityObject;
use rusteal_runtime::{uclass, uclass_impl};

pub static RUNS: AtomicI32 = AtomicI32::new(0);

#[uclass(parent = EditorUtilityObject)]
pub struct ProbeUtility {}

#[uclass_impl]
impl ProbeUtility {
    #[ufunction(Override)]
    fn run(&mut self) {
        RUNS.fetch_add(1, Ordering::SeqCst);
    }
}
