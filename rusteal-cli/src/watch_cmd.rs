// `rusteal watch`: rebuild the Rust libraries whenever their sources change.
//
// Polls the Rust workspaces of the game and of each Rusteal plugin (their
// `.rs` and `.toml` files, not `target/` or the generated `bindings/`) and,
// once the changes have settled, runs `rusteal build` (steps 4 and 5) as a
// child process, so a failed build reports its errors and the watch goes on.
// The editor reloads each library as soon as it is deployed again
// (Rusteal.AutoReload, docs/hot-reload.md).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

use rusteal_codegen::config::{find_plugins, ProjectLayout};

/// How often the sources are looked at.
const POLL: Duration = Duration::from_millis(400);
/// How long they must stay unchanged before a build: an editor saving several
/// files, or a formatter rewriting them, makes one build.
const SETTLE: Duration = Duration::from_millis(300);

/// Every watched file, with what tells it changed.
type Snapshot = BTreeMap<PathBuf, (SystemTime, u64)>;

pub fn run_watch(root: &Path, plugin: Option<&str>) -> ! {
    let workspaces = workspaces(root, plugin);
    if workspaces.is_empty() {
        eprintln!("Error: no Rust workspace to watch in {}.", root.display());
        std::process::exit(1);
    }
    eprintln!("rusteal watch: watching");
    for workspace in &workspaces {
        eprintln!("  {}", workspace.display());
    }
    eprintln!("Each change builds and deploys the libraries (rusteal build); Ctrl+C stops.");

    let mut built = snapshot(&workspaces);
    loop {
        std::thread::sleep(POLL);
        let mut current = snapshot(&workspaces);
        if current == built {
            continue;
        }
        // Until two looks in a row agree.
        loop {
            std::thread::sleep(SETTLE);
            let again = snapshot(&workspaces);
            if again == current {
                break;
            }
            current = again;
        }
        let changes = changed(&built, &current);
        eprintln!("\nrusteal watch: {}", describe(&changes));
        build(root, plugin);
        // What this build saw: a file saved while it ran builds again.
        built = current;
    }
}

/// The Rust workspaces of the game and the Rusteal plugins, or of the one
/// plugin given.
fn workspaces(root: &Path, plugin: Option<&str>) -> Vec<PathBuf> {
    let mut workspaces = Vec::new();
    if plugin.is_none() {
        workspaces.push(ProjectLayout::new(root).rust_workspace());
    }
    for layout in find_plugins(root) {
        if plugin.is_none_or(|name| layout.name.eq_ignore_ascii_case(name)) {
            workspaces.push(layout.rust_workspace());
        }
    }
    workspaces.retain(|workspace| workspace.is_dir());
    workspaces
}

/// The `.rs` and `.toml` files of the workspaces, outside build output and
/// the generated bindings, which `rusteal build --all` writes.
fn snapshot(workspaces: &[PathBuf]) -> Snapshot {
    let mut files = Snapshot::new();
    for workspace in workspaces {
        collect(workspace, &mut files);
    }
    files
}

fn collect(dir: &Path, files: &mut Snapshot) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if !(name == "target" || name == "bindings" || name.starts_with('.')) {
                collect(&path, files);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs" || ext == "toml")
            && let Ok(meta) = entry.metadata()
        {
            files.insert(path, (meta.modified().unwrap_or(SystemTime::UNIX_EPOCH), meta.len()));
        }
    }
}

/// The files added, changed or removed between two snapshots.
fn changed(before: &Snapshot, after: &Snapshot) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = after
        .iter()
        .filter(|(path, stamp)| before.get(*path) != Some(stamp))
        .map(|(path, _)| path.clone())
        .collect();
    paths.extend(before.keys().filter(|path| !after.contains_key(*path)).cloned());
    paths.sort();
    paths
}

fn describe(changes: &[PathBuf]) -> String {
    let names: Vec<String> = changes
        .iter()
        .take(3)
        .map(|path| path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
        .collect();
    match changes.len() {
        0 => "sources changed".to_string(),
        n if n <= 3 => format!("{} changed", names.join(", ")),
        n => format!("{} and {} more changed", names.join(", "), n - 3),
    }
}

/// `rusteal build` in a child process: its failure is reported, not fatal.
fn build(root: &Path, plugin: Option<&str>) {
    let start = Instant::now();
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("rusteal"));
    let mut command = Command::new(exe);
    command.arg("build").arg(root);
    if let Some(plugin) = plugin {
        command.args(["--plugin", plugin]);
    }
    match command.status() {
        Ok(status) if status.success() => {
            eprintln!("rusteal watch: built in {:.1}s, waiting for changes", start.elapsed().as_secs_f64());
        }
        Ok(_) => eprintln!("rusteal watch: the build failed (above), waiting for changes"),
        Err(e) => eprintln!("rusteal watch: could not run rusteal build: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_workspace(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rusteal-watch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["game/src", "target/debug", "bindings/src"] {
            std::fs::create_dir_all(dir.join(sub)).unwrap();
        }
        std::fs::write(dir.join("Cargo.toml"), "[workspace]").unwrap();
        std::fs::write(dir.join("game/src/lib.rs"), "mod a;").unwrap();
        dir
    }

    #[test]
    fn sources_are_watched_build_output_and_bindings_are_not() {
        let dir = temp_workspace("scope");
        std::fs::write(dir.join("target/debug/out.rs"), "").unwrap();
        std::fs::write(dir.join("bindings/src/lib.rs"), "").unwrap();
        std::fs::write(dir.join("game/src/notes.txt"), "").unwrap();
        let files = snapshot(std::slice::from_ref(&dir));
        let names: Vec<_> = files.keys().map(|p| p.strip_prefix(&dir).unwrap().to_path_buf()).collect();
        assert_eq!(names, [PathBuf::from("Cargo.toml"), PathBuf::from("game/src/lib.rs")]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn edits_additions_and_removals_are_changes() {
        let dir = temp_workspace("changes");
        let before = snapshot(std::slice::from_ref(&dir));
        std::fs::write(dir.join("game/src/lib.rs"), "mod a; mod b;").unwrap();
        std::fs::write(dir.join("game/src/b.rs"), "").unwrap();
        std::fs::remove_file(dir.join("Cargo.toml")).unwrap();
        let after = snapshot(std::slice::from_ref(&dir));
        let names: Vec<_> = changed(&before, &after)
            .iter()
            .map(|p| p.strip_prefix(&dir).unwrap().to_path_buf())
            .collect();
        assert_eq!(
            names,
            [PathBuf::from("Cargo.toml"), PathBuf::from("game/src/b.rs"), PathBuf::from("game/src/lib.rs")]
        );
        assert!(changed(&after, &after).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_few_changes_are_named() {
        let paths: Vec<PathBuf> = ["a.rs", "b.rs", "c.rs", "d.rs"].iter().map(PathBuf::from).collect();
        assert_eq!(describe(&paths[..1]), "a.rs changed");
        assert_eq!(describe(&paths), "a.rs, b.rs, c.rs and 1 more changed");
    }
}
