use std::io::Write;
use std::sync::atomic::Ordering;

use bindings::editor_subsystem::EditorSubsystem;
use bindings::unreal_ed::EditorSubsystemBlueprintLibrary;
use rusteal_runtime::runtime::{SubclassOf, console};

use crate::subsystem::{INITIALIZED, ProbeEditorSubsystem};

type Check = fn() -> Result<(), String>;

const CHECKS: &[(&str, Check)] = &[(
    "editor_library_subsystem_initializes",
    editor_library_subsystem_initializes,
)];

fn ensure(condition: bool, what: impl Into<String>) -> Result<(), String> {
    if condition { Ok(()) } else { Err(what.into()) }
}

fn editor_library_subsystem_initializes() -> Result<(), String> {
    let class = SubclassOf::<ProbeEditorSubsystem>::base().upcast_to::<EditorSubsystem>();
    let subsystem = EditorSubsystemBlueprintLibrary::get_editor_subsystem(class);
    let probe = ProbeEditorSubsystem::from_obj(subsystem)
        .map_err(|e| format!("no ProbeEditorSubsystem: {e:?}"))?;

    ensure(
        INITIALIZED.load(Ordering::SeqCst) && probe.initializations() == 1,
        format!(
            "initialized: {}, {} initializations",
            INITIALIZED.load(Ordering::SeqCst),
            probe.initializations()
        ),
    )
}

fn append(lines: &[String]) {
    let Ok(path) = std::env::var("RUSTEAL_TEST_RESULTS") else {
        return;
    };

    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        for line in lines {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn run() {
    let lines: Vec<String> = CHECKS
        .iter()
        .map(|(name, check)| {
            match std::panic::catch_unwind(check).unwrap_or_else(|_| Err("panicked".into())) {
                Ok(()) => format!("PASS {name}"),
                Err(reason) => format!("FAIL {name}: {reason}"),
            }
        })
        .collect();

    append(&lines);
}

pub fn register() {
    if let Err(error) = console::register_command(
        "rusteal_editor_tests.run",
        "Runs the checks of Rusteal's editor libraries and appends their results.",
        |_, _| run(),
    ) {
        log::error!("[RustealProbe] cannot register rusteal_editor_tests.run: {error:?}");
    }
}
