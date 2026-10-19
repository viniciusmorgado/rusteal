use std::path::Path;

use crate::project_version::{self, PINNED, PLUGINS, Pins, Scope, Version};
use crate::{build_cmd, global_config, new_cmd, setup};

const MIGRATIONS: &[(&str, &str)] = &[
    (
        "0.10.0",
        "Engine soft references are SoftObjectRef<T> and engine weak references TWeakObjectPtr<T>, \
     in properties, struct fields, function parameters and returns; they were UObjectRef<T>. \
     A soft reference keeps its path when the asset is not loaded: path(), get() for the \
     loaded object, load_synchronous() to load it. A weak one gives get() -> Option<UObjectRef<T>>, \
     None once its object is destroyed. Functions that take one take &SoftObjectRef<T> or \
     TWeakObjectPtr<T>: SoftObjectRef::new(path) or TWeakObjectPtr::from_ref(&object).",
    ),
    (
        "0.10.0",
        "Engine functions that change a struct passed by reference take &mut OwnedStruct<T>; \
         they took a *mut u8.",
    ),
    (
        "0.10.0",
        "Engine FText is Text, which keeps its localization: properties, struct fields, \
         returns and arrays give a Text (Display and to_string() for the string), and functions \
         take impl Into<Text>, so a &str or a String still works. An FText parameter with a \
         default value is a required one; pass \"\" for an empty text.",
    ),
    (
        "0.10.0",
        "Engine structs with no StaticStruct (FBox, FIntPoint, FTimespan, FPlatformUserId and the \
         rest of the engine's NoExportTypes) are OwnedStruct<T> like the others, in functions, \
         properties and arrays; functions took and returned them as a *const u8.",
    ),
];

pub fn migration_notes(from: Version, to: Version) -> Vec<(Version, &'static str)> {
    MIGRATIONS
        .iter()
        .filter_map(|(version, note)| Some((Version::parse(version)?, *note)))
        .filter(|(version, _)| *version > from && *version <= to)
        .collect()
}

pub fn run_upgrade(root: &Path) {
    let cli = Version::cli();

    let current = match project_version::read_pins(root).unwrap_or_else(|e| fail(&e)) {
        Pins::Exact(version) => version,
        Pins::Checkout(dir) => fail(&format!(
            "this project follows the Rusteal checkout at {dir}; there is no version to move.\n  \
             After updating the checkout, reinstall the plugins with its CLI:\n  \
             cargo run --manifest-path {manifest} -p rusteal -- setup {root}",
            dir = dir.display(),
            manifest = dir.join("Cargo.toml").display(),
            root = root.display(),
        )),
    };

    if current > cli {
        fail(&format!(
            "this project is at Rusteal {current}, newer than this CLI ({cli}).\n  \
             Install its version: cargo install rusteal@{current}"
        ));
    }

    if current == cli && project_version::check(root, Scope::PinsAndPlugins).is_ok() {
        eprintln!("rusteal upgrade: the project is already at {cli}.");
        return;
    }

    let engine = global_config::engine_path();

    if current == cli {
        eprintln!(
            "rusteal upgrade: the project is at {cli}, its plugins are not; reinstalling them"
        );
    } else {
        eprintln!("rusteal upgrade: {current} -> {cli}");
    }

    let workspaces = std::iter::once(root.join("Rust")).chain(
        rusteal_codegen::config::find_plugins(root)
            .into_iter()
            .map(|p| p.rust_workspace()),
    );

    for workspace in workspaces {
        let manifest_path = workspace.join("Cargo.toml");

        let manifest = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", manifest_path.display())));

        let rewritten = rewrite_glam(&rewrite_pins(&manifest, cli), new_cmd::GLAM_VERSION);

        std::fs::write(&manifest_path, rewritten)
            .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", manifest_path.display())));

        eprintln!(
            "  {}: {} pinned to ={cli}, glam {}",
            manifest_path.display(),
            PINNED.join(", "),
            new_cmd::GLAM_VERSION
        );
    }

    for plugin in PLUGINS {
        let dir = root.join("Plugins").join(plugin);

        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .unwrap_or_else(|e| fail(&format!("cannot remove {}: {e}", dir.display())));
        }
    }

    let notes = migration_notes(current, cli);

    if !notes.is_empty() {
        eprintln!("\n  Changes this upgrade asks of the project's Rust code:");

        for (version, note) in &notes {
            eprintln!("  - {version}: {note}");
        }

        eprintln!();
    }

    setup::run_setup(root, &engine);
    build_cmd::run_build(root, &engine, None, 1, false, None);

    eprintln!("\nrusteal upgrade: done, the project is at {cli}.");
}

fn fail(message: &str) -> ! {
    eprintln!("Error: {message}");
    std::process::exit(1);
}

pub fn rewrite_pins(manifest: &str, version: Version) -> String {
    let mut in_deps = false;
    let mut out = String::with_capacity(manifest.len());

    for line in manifest.split_inclusive('\n') {
        let trimmed = line.trim_start();

        if trimmed.starts_with('[') {
            in_deps = trimmed.starts_with("[workspace.dependencies]");
        }

        let pinned = if in_deps {
            PINNED.iter().find(|name| {
                trimmed
                    .strip_prefix(**name)
                    .is_some_and(|rest| rest.trim_start().starts_with('='))
            })
        } else {
            None
        };

        match pinned {
            Some(name) => {
                let indent = &line[..line.len() - trimmed.len()];
                let newline = if line.ends_with('\n') { "\n" } else { "" };
                out.push_str(&format!("{indent}{name} = \"={version}\"{newline}"));
            }
            None => out.push_str(line),
        }
    }

    out
}

pub fn rewrite_glam(manifest: &str, glam: &str) -> String {
    let mut in_deps = false;
    let mut out = String::with_capacity(manifest.len());

    for line in manifest.split_inclusive('\n') {
        let trimmed = line.trim_start();

        if trimmed.starts_with('[') {
            in_deps = trimmed.starts_with("[workspace.dependencies]");
        }

        let value = trimmed
            .strip_prefix("glam")
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix('='))
            .map(str::trim_start);

        let rewritten = match value {
            Some(value) if in_deps && value.starts_with('"') => {
                let indent = &line[..line.len() - trimmed.len()];
                let newline = if line.ends_with('\n') { "\n" } else { "" };
                Some(format!("{indent}glam = \"{glam}\"{newline}"))
            }
            Some(_) if in_deps => line.find("version").and_then(|at| {
                let open = at + line[at..].find('"')? + 1;
                let close = open + line[open..].find('"')?;
                Some(format!("{}{glam}{}", &line[..open], &line[close..]))
            }),
            _ => None,
        };

        out.push_str(rewritten.as_deref().unwrap_or(line));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_version::parse_pins;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    const EXACT: &str = r#"# comment
[workspace]
members = ["game", "bindings"]

[workspace.dependencies]
rusteal-runtime = "=0.2.1"
rusteal-core = "=0.2.1"
rusteal-ffi = "=0.2.1"
glam = "0.33.8"
"#;

    #[test]
    fn notes_are_the_versions_crossed() {
        assert!(migration_notes(v("0.10.0"), v("0.10.0")).is_empty());
        assert!(migration_notes(v("0.10.0"), v("0.12.0")).is_empty());

        let notes = migration_notes(v("0.9.0"), v("0.10.0"));
        assert_eq!(notes.len(), 4);
        assert_eq!(notes[0].0, v("0.10.0"));
    }

    #[test]
    fn rewrite_keeps_everything_but_the_pins() {
        let rewritten = rewrite_pins(EXACT, v("0.3.0"));
        assert_eq!(rewritten, EXACT.replace("=0.2.1", "=0.3.0"));
        assert_eq!(parse_pins(&rewritten), Ok(Pins::Exact(v("0.3.0"))));

        let table = EXACT.replace(
            "rusteal-core = \"=0.2.1\"",
            "rusteal-core = { version = \"=0.2.1\" }",
        );

        assert_eq!(
            rewrite_pins(&table, v("0.3.0")),
            EXACT.replace("=0.2.1", "=0.3.0")
        );

        let other = "[dependencies]\nrusteal-core = \"=0.2.1\"\n";
        assert_eq!(rewrite_pins(other, v("0.3.0")), other);
    }

    #[test]
    fn glam_follows_the_cli() {
        assert_eq!(
            rewrite_glam(EXACT, "0.34"),
            EXACT.replace("glam = \"0.33.8\"", "glam = \"0.34\"")
        );

        let table = EXACT.replace(
            "glam = \"0.33.8\"",
            "glam = { version = \"0.33.8\", features = [\"serde\"] }",
        );

        assert_eq!(
            rewrite_glam(&table, "0.34"),
            table.replace("\"0.33.8\"", "\"0.34\"")
        );

        let other = "[dependencies]\nglam = \"0.33\"\n";
        assert_eq!(rewrite_glam(other, "0.34"), other);
    }
}
