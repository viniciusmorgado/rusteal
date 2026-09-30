// The files `rusteal setup` and `rusteal new` write into a project, embedded
// at build time from `templates/`.
//
// `templates/<name>/` is what `rusteal new --template <name>` writes on top of
// the engine template its `template.toml` names. Each one is complete on its
// own: a new template starts as a copy of the closest one, changed, not as a
// layer over it. A template mirrors the project's root: `*.tera` files are
// rendered with Tera (the extension dropped) and everything else is copied as
// is, and `{{ variable }}` works in folder and file names too
// (`Rust/{{crate_name}}/src/lib.rs.tera`). Every template gets the same
// context, so adding one is adding a directory.

use std::path::Path;

use serde::Deserialize;

include!(concat!(env!("OUT_DIR"), "/template_files.rs"));

/// The template `rusteal setup` takes the starter `rusteal.toml` from.
const SETUP_TEMPLATE: &str = "blank";

/// A template's `template.toml`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// One line for `rusteal new --help` and the list of templates.
    pub description: String,
    /// The engine template (`Templates/<name>`) the project starts from.
    pub engine_template: String,
    /// Paths of the engine template to leave out, relative to it;
    /// `%TEMPLATENAME%` stands for its name, and a folder leaves out
    /// everything under it.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Printed when the project is ready (Tera, with the files' context).
    pub next_step: String,
}

/// An embedded file's contents, by its path under `templates/`.
pub fn raw(path: &str) -> &'static [u8] {
    TEMPLATE_FILES
        .iter()
        .find(|(file, _)| *file == path)
        .map(|(_, contents)| *contents)
        .unwrap_or_else(|| panic!("template file {path} is not embedded"))
}

/// Render an embedded Tera file (a path under `templates/`).
pub fn render(path: &str, context: &tera::Context) -> String {
    let text = std::str::from_utf8(raw(path))
        .unwrap_or_else(|e| panic!("template file {path} is not UTF-8: {e}"));
    tera::Tera::one_off(text, context, false)
        .unwrap_or_else(|e| panic!("Failed to render {path}: {e}"))
}

/// The starter `rusteal.toml` `rusteal setup` writes into an existing project.
pub fn render_rusteal_toml(context: &tera::Context) -> String {
    render(&format!("{SETUP_TEMPLATE}/rusteal.toml.tera"), context)
}

/// The templates `rusteal new` offers, with their manifests, by name.
pub fn available() -> Vec<(&'static str, Manifest)> {
    TEMPLATE_FILES
        .iter()
        .filter_map(|(path, _)| path.strip_suffix("/template.toml"))
        .filter(|name| !name.contains('/'))
        .map(|name| (name, manifest(name).expect("listed template has a manifest")))
        .collect()
}

/// The manifest of the template `name`, if there is one.
pub fn manifest(name: &str) -> Option<Manifest> {
    let path = format!("{name}/template.toml");
    let (_, contents) = TEMPLATE_FILES.iter().find(|(file, _)| *file == path)?;
    let text = std::str::from_utf8(contents)
        .unwrap_or_else(|e| panic!("{path} is not UTF-8: {e}"));
    Some(toml::from_str(text).unwrap_or_else(|e| panic!("{path}: {e}")))
}

/// Write the template's files into `root`, over what the engine template put
/// there. Returns the number of files written.
pub fn write_project_files(template: &str, root: &Path, context: &tera::Context) -> usize {
    let prefix = format!("{template}/");
    let mut written = 0;
    for (path, contents) in TEMPLATE_FILES {
        let Some(rel) = path.strip_prefix(&prefix) else {
            continue;
        };
        if rel == "template.toml" {
            continue;
        }
        write_file(root, rel, contents, context);
        written += 1;
    }
    written
}

fn write_file(root: &Path, rel: &str, contents: &[u8], context: &tera::Context) {
    let rel = render_path(rel, context);
    let (rel, rendered) = match rel.strip_suffix(".tera") {
        Some(stripped) => {
            let text = std::str::from_utf8(contents)
                .unwrap_or_else(|e| panic!("template file {rel} is not UTF-8: {e}"));
            let text = tera::Tera::one_off(text, context, false)
                .unwrap_or_else(|e| panic!("Failed to render {rel}: {e}"));
            (stripped.to_string(), Some(text))
        }
        None => (rel, None),
    };
    let target = root.join(&rel);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("Failed to create {}: {e}", parent.display()));
    }
    let bytes = rendered.as_deref().map(str::as_bytes).unwrap_or(contents);
    std::fs::write(&target, bytes)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", target.display()));
}

/// `{{ variable }}` in folder and file names.
fn render_path(rel: &str, context: &tera::Context) -> String {
    if !rel.contains("{{") {
        return rel.to_string();
    }
    tera::Tera::one_off(rel, context, false)
        .unwrap_or_else(|e| panic!("Failed to render the path {rel}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_has_a_valid_manifest() {
        let names: Vec<&str> = available().iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["blank", "third-person"]);
        assert!(names.contains(&SETUP_TEMPLATE));
    }

    #[test]
    fn writes_template_files_with_rendered_paths() {
        let root = std::env::temp_dir().join(format!("rusteal-templates-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut context = tera::Context::new();
        context.insert("project", "Probe");
        context.insert("crate_name", "probe");
        context.insert("version", "0.0.0");
        context.insert("glam_version", "0.0.0");

        write_project_files("blank", &root, &context);

        for rel in [
            ".gitignore",
            "rusteal.toml",
            "Rust/Cargo.toml",
            "Rust/probe/Cargo.toml",
            "Rust/probe/src/lib.rs",
            "Rust/probe/src/hello.rs",
        ] {
            assert!(root.join(rel).is_file(), "{rel} missing");
        }
        assert!(!root.join("template.toml").exists());
        let lib = std::fs::read_to_string(root.join("Rust/probe/src/lib.rs")).unwrap();
        assert!(lib.contains("Probe"), "{lib}");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
