// The files `rusteal setup` and `rusteal new` write into a project, embedded
// at build time from `templates/`.
//
// `templates/<template>/<variant>/` is what `rusteal new --template <template>
// --variant <variant>` writes on top of the engine template its
// `template.toml` names; `base` is the variant without `--variant`. Each one
// is complete on its own: a new template or variant starts as a copy of the
// closest one, changed, not as a layer over it. A variant mirrors the
// project's root: `*.tera` files are rendered with Tera (the extension
// dropped) and everything else is copied as is, and `{{ variable }}` works in
// folder and file names too (`Rust/{{crate_name}}/src/lib.rs.tera`). Every
// variant gets the same context, so adding one is adding a directory.

use std::path::Path;

use serde::Deserialize;

include!(concat!(env!("OUT_DIR"), "/template_files.rs"));

/// The variant `rusteal new` uses without `--variant`; every template has it.
pub const BASE_VARIANT: &str = "base";

/// The template `rusteal setup` takes the starter `rusteal.toml` from.
const SETUP_TEMPLATE: &str = "blank/base";

/// A variant's `template.toml`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// One line for the list of templates.
    pub description: String,
    /// The engine template (`Templates/<name>`) the project starts from.
    pub engine_template: String,
    /// Paths of the engine template to leave out, relative to it;
    /// `%TEMPLATENAME%` stands for its name, and a folder leaves out
    /// everything under it.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// The level the project opens and plays (`/Game/...`), when it is not
    /// the engine template's own.
    #[serde(default)]
    pub default_map: Option<String>,
    /// Printed when the project is ready (Tera, with the files' context).
    pub next_step: String,
}

/// A template with its variants, `base` first.
pub struct TemplateInfo {
    pub name: &'static str,
    pub variants: Vec<(&'static str, Manifest)>,
}

impl TemplateInfo {
    /// The `base` variant's manifest.
    pub fn base(&self) -> &Manifest {
        &self.variants[0].1
    }
}

/// The template and variant `rusteal new` was asked for.
pub struct Selected {
    pub template: &'static str,
    pub variant: &'static str,
    pub manifest: Manifest,
}

impl Selected {
    /// The variant's directory under `templates/`.
    pub fn dir(&self) -> String {
        format!("{}/{}", self.template, self.variant)
    }
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

/// The templates `rusteal new` offers, by name, each with its variants.
pub fn available() -> Vec<TemplateInfo> {
    let mut templates: Vec<TemplateInfo> = Vec::new();
    for (path, _) in TEMPLATE_FILES {
        let Some(dir) = path.strip_suffix("/template.toml") else {
            continue;
        };
        let Some((name, variant)) = dir.split_once('/') else {
            continue;
        };
        if variant.contains('/') {
            continue;
        }
        let manifest = manifest(dir).expect("listed variant has a manifest");
        match templates.iter_mut().find(|t| t.name == name) {
            Some(template) => template.variants.push((variant, manifest)),
            None => templates.push(TemplateInfo { name, variants: vec![(variant, manifest)] }),
        }
    }
    for template in &mut templates {
        template
            .variants
            .sort_by_key(|(variant, _)| (*variant != BASE_VARIANT, *variant));
        assert_eq!(
            template.variants[0].0, BASE_VARIANT,
            "template {} has no {BASE_VARIANT} variant",
            template.name
        );
    }
    templates
}

/// The template and variant named on the command line: names are matched
/// ignoring case, `-` and `_` (`side-scrolling`, `SideScrolling`). No variant
/// is `base`. The error lists what
/// there is.
pub fn select(template: &str, variant: Option<&str>) -> Result<Selected, String> {
    let templates = available();
    let wanted = normalize(template);
    let Some(info) = templates.into_iter().find(|t| normalize(t.name) == wanted) else {
        return Err(format!("there is no template '{template}'. The templates are:\n{}", listing()));
    };
    let wanted = normalize(variant.unwrap_or(BASE_VARIANT));
    let name = info.name;
    let Some((variant, manifest)) =
        info.variants.into_iter().find(|(v, _)| normalize(v) == wanted)
    else {
        return Err(format!(
            "the {name} template has no variant '{}'. The templates are:\n{}",
            variant.unwrap_or(BASE_VARIANT),
            listing()
        ));
    };
    Ok(Selected { template: name, variant, manifest })
}

/// The templates and their variants, one line each, for error messages.
pub fn listing() -> String {
    let mut out = String::new();
    for template in available() {
        out.push_str(&format!("  {:<24} {}\n", template.name, template.base().description));
        for (variant, manifest) in template.variants.iter().skip(1) {
            out.push_str(&format!("    --variant {variant:<12} {}\n", manifest.description));
        }
    }
    out
}

fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

/// The manifest of the variant in `dir` (`<template>/<variant>`), if there is one.
fn manifest(dir: &str) -> Option<Manifest> {
    let path = format!("{dir}/template.toml");
    let (_, contents) = TEMPLATE_FILES.iter().find(|(file, _)| *file == path)?;
    let text = std::str::from_utf8(contents)
        .unwrap_or_else(|e| panic!("{path} is not UTF-8: {e}"));
    Some(toml::from_str(text).unwrap_or_else(|e| panic!("{path}: {e}")))
}

/// Write the variant's files (`dir`, as [`Selected::dir`]) into `root`, over
/// what the engine template put there. Returns the number of files written.
pub fn write_project_files(dir: &str, root: &Path, context: &tera::Context) -> usize {
    let prefix = format!("{dir}/");
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
        let templates = available();
        let names: Vec<&str> = templates.iter().map(|t| t.name).collect();
        assert_eq!(names, ["blank", "first-person", "third-person"]);
        assert!(manifest(SETUP_TEMPLATE).is_some());
    }

    #[test]
    fn selects_templates_and_variants_by_loose_names() {
        let base = select("third-person", None).unwrap();
        assert_eq!((base.template, base.variant), ("third-person", BASE_VARIANT));
        assert_eq!(base.dir(), "third-person/base");
        let same = select("ThirdPerson", Some("BASE")).unwrap();
        assert_eq!((same.template, same.variant), ("third-person", BASE_VARIANT));
        let loose = select("third_person", None).unwrap();
        assert_eq!(loose.template, "third-person");

        let unknown = select("nope", None).err().unwrap();
        assert!(unknown.contains("blank"), "{unknown}");
        let no_variant = select("blank", Some("nope")).err().unwrap();
        assert!(no_variant.contains("no variant 'nope'"), "{no_variant}");
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

        write_project_files("blank/base", &root, &context);

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
