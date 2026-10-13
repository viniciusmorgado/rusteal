use std::path::Path;

const OS_SERVICES: &[&str] = &[
    "std::thread",
    "std::time::Instant",
    "std::time::SystemTime",
    "std::time::{",
];

#[test]
fn the_runtime_takes_threads_and_time_from_the_engine() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;

    for entry in std::fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();

        if !name.ends_with(".rs") || name == "std_fallback.rs" || name == "sync.rs" {
            continue;
        }

        let text = std::fs::read_to_string(&path).unwrap();
        let code = text.split("#[cfg(test)]").next().unwrap();

        for service in OS_SERVICES {
            assert!(
                !code.contains(service),
                "{name} uses {service}: the runtime takes threads and time from the engine \
                 (task.rs, time.rs), with the std versions only in std_fallback.rs"
            );
        }

        checked += 1;
    }

    assert!(checked > 20, "only {checked} files checked");
}

#[test]
fn locks_and_thread_locals_come_from_one_module() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    for entry in std::fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();

        if !name.ends_with(".rs") || name == "sync.rs" || name == "std_fallback.rs" {
            continue;
        }

        let text = std::fs::read_to_string(&path).unwrap();
        let code = text.split("#[cfg(test)]").next().unwrap();

        for line in code.lines().filter(|line| line.contains("std::sync")) {
            for primitive in ["Mutex", "RwLock", "Once", "Condvar", "Barrier"] {
                assert!(
                    !line.contains(primitive),
                    "{name} takes {primitive} from std: take it from crate::sync, which a \
                     platform without them can replace"
                );
            }
        }

        assert!(
            !code.contains("std::thread_local"),
            "{name} uses std::thread_local: use crate::sync::thread_local"
        );

        if code.contains("thread_local!") {
            assert!(
                code.contains("crate::sync::") && code.contains("thread_local}"),
                "{name} declares a thread-local without importing crate::sync::thread_local"
            );
        }
    }
}
