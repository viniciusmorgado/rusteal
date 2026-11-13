use std::path::{Path, PathBuf};

pub fn bindings_dependency(workspace: &Path, crate_name: &str) -> Vec<PathBuf> {
    let package = rusteal_codegen::config::bindings_package(crate_name);
    let mut changed = Vec::new();

    let mut rewrite = |path: PathBuf, edit: &dyn Fn(&str) -> Option<String>| {
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };

        if let Some(new) = edit(&text) {
            std::fs::write(&path, new)
                .unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));

            changed.push(path);
        }
    };

    rewrite(workspace.join("Cargo.toml"), &|text| {
        rewrite_profiles(text, &package)
    });

    let members = std::fs::read_dir(workspace)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|dir| {
            dir.is_dir()
                && dir
                    .file_name()
                    .is_some_and(|n| !n.to_string_lossy().ends_with("bindings"))
        })
        .map(|dir| dir.join("Cargo.toml"))
        .filter(|manifest| manifest.is_file());

    for manifest in members {
        rewrite(manifest, &|text| rewrite_dependency(text, &package));
    }

    for path in &changed {
        eprintln!(
            "  {}: the generated bindings are now the package `{package}`, imported as `bindings`",
            path.display()
        );
    }

    changed
}

pub fn crate_types(manifest: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(manifest) else {
        return false;
    };

    let Some(new) = rewrite_crate_types(&text) else {
        return false;
    };

    std::fs::write(manifest, new)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", manifest.display()));

    eprintln!(
        "  {}: the library is a Rust library (rlib), which the static build links; \
         `rusteal build` makes the editor's dynamic library from it",
        manifest.display()
    );

    true
}

fn rewrite_crate_types(text: &str) -> Option<String> {
    let mut changed = false;

    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            let trimmed = line.trim();

            if trimmed.starts_with("crate-type") && trimmed.contains("\"cdylib\"") {
                changed = true;
                let indent = &line[..line.len() - line.trim_start().len()];
                format!("{indent}crate-type = [\"rlib\"]")
            } else {
                line.to_string()
            }
        })
        .collect();

    changed.then(|| join(text, lines))
}

fn rewrite_dependency(text: &str, package: &str) -> Option<String> {
    let mut changed = false;

    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            let rest = line.trim_start();

            let is_bindings = rest
                .strip_prefix("bindings")
                .map(str::trim_start)
                .is_some_and(|after| after.starts_with('='));

            if is_bindings
                && line.contains('{')
                && line.contains("path")
                && !line.contains("package")
            {
                changed = true;
                line.replacen('{', &format!("{{ package = \"{package}\","), 1)
                    .replace(",  ", ", ")
            } else {
                line.to_string()
            }
        })
        .collect();

    changed.then(|| join(text, lines))
}

fn rewrite_profiles(text: &str, package: &str) -> Option<String> {
    let mut changed = false;

    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            let trimmed = line.trim();

            let is_profile = trimmed.starts_with("[profile.")
                && (trimmed.ends_with(".package.bindings]")
                    || trimmed.ends_with(".package.\"bindings\"]"));

            if is_profile {
                changed = true;
                let cut = trimmed.rfind(".package.").unwrap() + ".package.".len();
                format!("{}{package}]", &trimmed[..cut])
            } else {
                line.to_string()
            }
        })
        .collect();

    changed.then(|| join(text, lines))
}

fn join(original: &str, lines: Vec<String>) -> String {
    let mut out = lines.join("\n");

    if original.ends_with('\n') {
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dependency_names_the_package_and_keeps_its_name() {
        let crate_toml = "[dependencies]\nrusteal-runtime = { workspace = true }\nbindings = { path = \"../bindings\" }\nglam = { workspace = true }\n";

        assert_eq!(
            rewrite_dependency(crate_toml, "my-game-bindings").unwrap(),
            "[dependencies]\nrusteal-runtime = { workspace = true }\nbindings = { package = \"my-game-bindings\", path = \"../bindings\" }\nglam = { workspace = true }\n"
        );

        let migrated = rewrite_dependency(crate_toml, "x").unwrap();
        assert!(
            rewrite_dependency(&migrated, "x").is_none(),
            "migrating twice"
        );
        assert!(rewrite_dependency("[dependencies]\nbindings-extra = \"1\"\n", "x").is_none());
    }

    #[test]
    fn a_library_crate_is_a_plain_rlib() {
        for old in ["[\"cdylib\"]", "[\"cdylib\", \"rlib\"]"] {
            assert_eq!(
                rewrite_crate_types(&format!("[lib]\ncrate-type = {old}\n")).unwrap(),
                "[lib]\ncrate-type = [\"rlib\"]\n"
            );
        }

        assert!(rewrite_crate_types("[lib]\ncrate-type = [\"rlib\"]\n").is_none());
        assert!(rewrite_crate_types("[lib]\nname = \"x\"\n").is_none());
    }

    #[test]
    fn profiles_follow_the_package() {
        let workspace = "[profile.dev.package.\"*\"]\nopt-level = 3\n\n[profile.dev.package.bindings]\nopt-level = 3\n";

        assert_eq!(
            rewrite_profiles(workspace, "my-game-bindings").unwrap(),
            "[profile.dev.package.\"*\"]\nopt-level = 3\n\n[profile.dev.package.my-game-bindings]\nopt-level = 3\n"
        );

        assert!(rewrite_profiles("[profile.release]\nlto = \"thin\"\n", "x").is_none());
    }

    #[test]
    fn a_project_is_migrated_in_place() {
        let dir = std::env::temp_dir().join(format!("rusteal-bindings-dep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("my-game")).unwrap();
        std::fs::create_dir_all(dir.join("bindings")).unwrap();

        std::fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\nmembers = [\"my-game\", \"bindings\"]\n\n[profile.dev.package.bindings]\nopt-level = 3\n",
        )
        .unwrap();

        std::fs::write(
            dir.join("my-game/Cargo.toml"),
            "[dependencies]\nbindings = { path = \"../bindings\" }\n",
        )
        .unwrap();

        std::fs::write(
            dir.join("bindings/Cargo.toml"),
            "[package]\nname = \"bindings\"\n",
        )
        .unwrap();

        assert_eq!(bindings_dependency(&dir, "my-game").len(), 2);
        assert!(bindings_dependency(&dir, "my-game").is_empty());

        let game = std::fs::read_to_string(dir.join("my-game/Cargo.toml")).unwrap();
        assert!(game.contains("package = \"my-game-bindings\""));

        let bindings = std::fs::read_to_string(dir.join("bindings/Cargo.toml")).unwrap();
        assert_eq!(
            bindings, "[package]\nname = \"bindings\"\n",
            "codegen owns the bindings' manifest"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
