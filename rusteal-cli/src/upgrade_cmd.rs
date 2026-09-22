// rusteal upgrade: move a project to this CLI's Rusteal version.
//
// Rewrites the three pins in Rust/Cargo.toml, replaces the UE plugins
// wholesale (files a newer version dropped must not stay behind) and runs the
// build pipeline, which regenerates the bindings and the C++ wrappers.

use std::path::Path;

use crate::project_version::{self, Pins, Scope, Version, PINNED, PLUGINS};
use crate::{build_cmd, global_config, setup};

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
        eprintln!("rusteal upgrade: the project is at {cli}, its plugins are not; reinstalling them");
    } else {
        eprintln!("rusteal upgrade: {current} -> {cli}");
    }
    let manifest_path = root.join("Rust/Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", manifest_path.display())));
    std::fs::write(&manifest_path, rewrite_pins(&manifest, cli))
        .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", manifest_path.display())));
    eprintln!("  {}: {} pinned to ={cli}", manifest_path.display(), PINNED.join(", "));

    for plugin in PLUGINS {
        let dir = root.join("Plugins").join(plugin);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .unwrap_or_else(|e| fail(&format!("cannot remove {}: {e}", dir.display())));
        }
    }
    setup::run_setup(root, &engine);
    build_cmd::run_build(root, &engine, None, 1);

    eprintln!("\nrusteal upgrade: done, the project is at {cli}.");
}

fn fail(message: &str) -> ! {
    eprintln!("Error: {message}");
    std::process::exit(1);
}

/// Rewrite the pins in the text of a Rust/Cargo.toml to `=version`, line by
/// line inside `[workspace.dependencies]`, keeping everything else as it is.
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
    fn rewrite_keeps_everything_but_the_pins() {
        let rewritten = rewrite_pins(EXACT, v("0.3.0"));
        assert_eq!(rewritten, EXACT.replace("=0.2.1", "=0.3.0"));
        assert_eq!(parse_pins(&rewritten), Ok(Pins::Exact(v("0.3.0"))));

        let table = EXACT.replace("rusteal-core = \"=0.2.1\"", "rusteal-core = { version = \"=0.2.1\" }");
        assert_eq!(rewrite_pins(&table, v("0.3.0")), EXACT.replace("=0.2.1", "=0.3.0"));

        // Outside [workspace.dependencies] nothing changes.
        let other = "[dependencies]\nrusteal-core = \"=0.2.1\"\n";
        assert_eq!(rewrite_pins(other, v("0.3.0")), other);
    }
}
