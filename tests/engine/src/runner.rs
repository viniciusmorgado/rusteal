use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex};

use bindings::engine::{KismetSystemLibrary, World};
use rusteal_runtime::runtime::{UObjectRef, console, task};

pub struct Context {
    pub world: UObjectRef<World>,
}

pub enum Outcome {
    Done(Result<(), String>),
    Pending(Box<dyn FnMut() -> Option<Result<(), String>> + Send>),
}

pub type Check = fn(&Context) -> Outcome;

const TIMEOUT_SECONDS: f32 = 30.0;

pub fn register() {
    let registered = console::register_command(
        "rusteal_tests.run",
        "Runs Rusteal's engine tests, writes their results and quits.",
        |_, context| match context.world::<World>() {
            Ok(world) => run(Context { world }),
            Err(error) => log::error!("[RustealTests] no world: {error:?}"),
        },
    );

    if let Err(error) = registered {
        log::error!("[RustealTests] cannot register rusteal_tests.run: {error:?}");
    }
}

fn outcome(check: Check, context: &Context) -> Outcome {
    std::panic::catch_unwind(AssertUnwindSafe(|| check(context)))
        .unwrap_or_else(|_| Outcome::Done(Err("panicked".to_string())))
}

fn line(name: &str, result: &Result<(), String>) -> String {
    match result {
        Ok(()) => format!("PASS {name}"),
        Err(reason) => format!("FAIL {name}: {reason}"),
    }
}

fn run(context: Context) {
    let mut lines = Vec::new();
    let mut pending = Vec::new();

    for (name, check) in crate::checks::ALL {
        match outcome(*check, &context) {
            Outcome::Done(result) => lines.push(line(name, &result)),
            Outcome::Pending(poll) => pending.push((*name, poll)),
        }
    }

    let state = Arc::new(Mutex::new((lines, pending, 0.0f32)));
    let world = context.world;

    let handle = Arc::new(Mutex::new(None));
    let slot = handle.clone();

    let tick = task::on_tick(move |delta| {
        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
        let (lines, pending, elapsed) = &mut *state;
        *elapsed += delta;

        pending.retain_mut(|(name, poll)| match poll() {
            Some(result) => {
                lines.push(line(name, &result));
                false
            }
            None => true,
        });

        if *elapsed > TIMEOUT_SECONDS {
            for (name, _) in pending.drain(..) {
                lines.push(format!("FAIL {name}: timed out"));
            }
        }

        if pending.is_empty() {
            finish(lines, &world);

            if let Some(handle) = slot.lock().unwrap_or_else(|e| e.into_inner()).take() {
                task::TickHandle::remove(handle);
            }
        }
    });

    *handle.lock().unwrap_or_else(|e| e.into_inner()) = Some(tick);
}

fn finish(lines: &mut Vec<String>, world: &UObjectRef<World>) {
    let path = std::env::var("RUSTEAL_TEST_RESULTS")
        .unwrap_or_else(|_| "rusteal-test-results.txt".to_string());

    for line in lines.iter() {
        log::info!("[RustealTests] {line}");
    }

    lines.push("DONE".to_string());

    if let Err(error) = std::fs::write(&path, lines.join("\n") + "\n") {
        log::error!("[RustealTests] cannot write {path}: {error}");
    }

    KismetSystemLibrary::execute_console_command(world.upcast(), "quit", None);
}
