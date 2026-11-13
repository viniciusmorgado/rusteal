use std::path::{Path, PathBuf};

const STD_SERVICES: &[(&str, &str)] = &[
    (
        "std::thread",
        "threads of its own: use rusteal_runtime::runtime::task (spawn, run_on_game_thread)",
    ),
    (
        "std::time::Instant",
        "the OS clock: use rusteal_runtime::runtime::time::Instant",
    ),
    (
        "std::time::SystemTime",
        "the OS clock: use rusteal_runtime::runtime::time, or the engine's date functions",
    ),
];

const THREADED_CRATES: &[(&str, &str)] = &[
    (
        "rayon-core",
        "starts a pool of OS threads unless its work runs in rusteal_runtime::rayon's pool, on the engine's threads",
    ),
    ("tokio", "starts OS threads and uses the OS's I/O"),
    ("async-std", "starts OS threads and uses the OS's I/O"),
    ("smol", "starts OS threads and uses the OS's I/O"),
    ("threadpool", "starts OS threads"),
    ("mio", "uses the OS's I/O"),
];

pub fn enabled(root: &Path) -> bool {
    std::fs::read_to_string(root.join("rusteal.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
        .and_then(|config| config.get("consoles")?.get("check")?.as_bool())
        .unwrap_or(false)
}

pub fn source_findings(file: &Path, text: &str) -> Vec<String> {
    let mut findings = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let code = line.split("//").next().unwrap_or_default();

        for (pattern, advice) in STD_SERVICES {
            let (module, item) = pattern.rsplit_once("::").unwrap();

            let matches = code.contains(pattern)
                || (module != "std"
                    && code.contains(&format!("{module}::{{"))
                    && code.contains(item));

            if matches {
                findings.push(format!(
                    "{}:{}: {pattern} uses {advice}",
                    file.display(),
                    index + 1
                ));
            }
        }
    }

    findings
}

pub fn lock_findings(lock: &Path, text: &str) -> Vec<String> {
    let Ok(lockfile) = text.parse::<toml::Table>() else {
        return Vec::new();
    };

    let names: Vec<&str> = lockfile
        .get("package")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
        .filter_map(|p| p.get("name")?.as_str())
        .collect();

    THREADED_CRATES
        .iter()
        .filter(|(name, _)| names.contains(name))
        .map(|(name, why)| {
            format!(
                "{}: the crate {name} {why}; on a console it needs those services from the \
                 platform's port of std",
                lock.display()
            )
        })
        .collect()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();

        if path.is_dir() {
            if name != "target" && !name.ends_with("bindings") && !name.starts_with('.') {
                rust_files(&path, out);
            }
        } else if name.ends_with(".rs") {
            out.push(path);
        }
    }
}

pub fn check(workspaces: &[PathBuf]) -> Vec<String> {
    let mut findings = Vec::new();

    for workspace in workspaces {
        let mut files = Vec::new();
        rust_files(workspace, &mut files);
        files.sort();

        for file in files {
            if let Ok(text) = std::fs::read_to_string(&file) {
                findings.extend(source_findings(&file, &text));
            }
        }

        let lock = workspace.join("Cargo.lock");

        if let Ok(text) = std::fs::read_to_string(&lock) {
            findings.extend(lock_findings(&lock, &text));
        }
    }

    findings
}

pub fn report(root: &Path, workspaces: &[PathBuf]) {
    if !enabled(root) {
        return;
    }

    let findings = check(workspaces);

    if findings.is_empty() {
        eprintln!("  console check: nothing uses the operating system's threads or clock directly");
        return;
    }

    eprintln!(
        "  console check: {} use(s) of what a console's std may not have ([consoles] check in rusteal.toml):",
        findings.len()
    );

    for finding in findings {
        eprintln!("  warning: {finding}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_threads_and_clocks_are_found_in_code_not_comments() {
        let text = "use std::time::{Duration, Instant};\n\
                    use rusteal_runtime::runtime::time::Instant as EngineInstant;\n\
                    fn f() {\n    std::thread::spawn(|| {});\n    // std::thread::sleep is fine here\n}\n\
                    let now = std::time::SystemTime::now();\n";

        let findings = source_findings(Path::new("src/lib.rs"), text);

        assert_eq!(findings.len(), 3, "{findings:#?}");
        assert!(findings[0].starts_with("src/lib.rs:1: std::time::Instant"));
        assert!(findings[1].starts_with("src/lib.rs:4: std::thread"));
        assert!(findings[2].starts_with("src/lib.rs:7: std::time::SystemTime"));
    }

    #[test]
    fn crates_that_start_threads_are_named() {
        let lock = "[[package]]\nname = \"rayon-core\"\nversion = \"1.12.1\"\n\n\
                    [[package]]\nname = \"glam\"\nversion = \"0.34.1\"\n";

        let findings = lock_findings(Path::new("Cargo.lock"), lock);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].contains("rayon-core starts a pool of OS threads"));
        assert!(findings[0].contains("rusteal_runtime::rayon"));
    }

    #[test]
    fn the_check_is_off_unless_asked() {
        let dir =
            std::env::temp_dir().join(format!("rusteal-console-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("rusteal.toml"), "[project]\ncrate = \"g\"\n").unwrap();
        assert!(!enabled(&dir));

        std::fs::write(
            dir.join("rusteal.toml"),
            "[project]\ncrate = \"g\"\n\n[consoles]\ncheck = true\n",
        )
        .unwrap();
        assert!(enabled(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
