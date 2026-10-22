use rusteal_runtime::runtime::profiling::{self, CpuScope};
use rusteal_runtime::runtime::{stat_scope, trace_scope};

use crate::runner::{Context, Outcome};

pub const TRACE_SCOPE: &str = "RustealTraceProbe";

fn busy_work() -> u64 {
    (0..20_000u64).fold(0, |sum, n| sum.wrapping_add(n * n))
}

pub fn profiling_scopes_run(_context: &Context) -> Outcome {
    let total = {
        trace_scope!(TRACE_SCOPE);
        stat_scope!("RustealStatProbe");
        let _memory = profiling::memory_tag("Rusteal/Probe");
        let _named = CpuScope::new("RustealTraceProbe.Named");

        busy_work()
    };

    Outcome::Done(if total > 0 {
        Ok(())
    } else {
        Err("the work inside the scopes did not run".to_string())
    })
}

pub fn localized_text_keeps_its_source(_context: &Context) -> Outcome {
    let greeting = rusteal_runtime::runtime::loctext!(
        "RustealTests",
        "Greeting",
        "Hello from a Rust loctext"
    );

    Outcome::Done(if greeting.to_string() == "Hello from a Rust loctext" {
        Ok(())
    } else {
        Err(format!("the text is {:?}", greeting.to_string()))
    })
}
