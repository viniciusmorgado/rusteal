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

        if !name.ends_with(".rs") || name == "std_fallback.rs" {
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
