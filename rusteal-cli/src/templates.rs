use std::path::Path;

use serde::Deserialize;

include!(concat!(env!("OUT_DIR"), "/template_files.rs"));
include!(concat!(env!("OUT_DIR"), "/plugin_template_files.rs"));
include!(concat!(env!("OUT_DIR"), "/project_feature_files.rs"));
include!(concat!(env!("OUT_DIR"), "/plugin_editor_library_files.rs"));

pub const DEFAULT_PLUGIN_TEMPLATE: &str = "blank";

pub const BASE_VARIANT: &str = "base";

const SETUP_TEMPLATE: &str = "blank/base";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub description: String,
    pub engine_template: String,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub default_map: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    pub next_step: String,
}

pub struct TemplateInfo {
    pub name: &'static str,
    pub variants: Vec<(&'static str, Manifest)>,
}

impl TemplateInfo {
    pub fn base(&self) -> &Manifest {
        &self.variants[0].1
    }
}

pub struct Selected {
    pub template: &'static str,
    pub variant: &'static str,
    pub manifest: Manifest,
}

impl Selected {
    pub fn dir(&self) -> String {
        format!("{}/{}", self.template, self.variant)
    }
}

pub fn raw(path: &str) -> &'static [u8] {
    TEMPLATE_FILES
        .iter()
        .find(|(file, _)| *file == path)
        .map(|(_, contents)| *contents)
        .unwrap_or_else(|| panic!("template file {path} is not embedded"))
}

pub fn render(path: &str, context: &tera::Context) -> String {
    let text = std::str::from_utf8(raw(path))
        .unwrap_or_else(|e| panic!("template file {path} is not UTF-8: {e}"));

    tera::Tera::one_off(text, context, false)
        .unwrap_or_else(|e| panic!("Failed to render {path}: {e}"))
}

pub fn render_rusteal_toml(context: &tera::Context) -> String {
    render(&format!("{SETUP_TEMPLATE}/rusteal.toml.tera"), context)
}

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
            None => templates.push(TemplateInfo {
                name,
                variants: vec![(variant, manifest)],
            }),
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

pub fn select(template: &str, variant: Option<&str>) -> Result<Selected, String> {
    let templates = available();
    let wanted = normalize(template);

    let Some(info) = templates.into_iter().find(|t| normalize(t.name) == wanted) else {
        return Err(format!(
            "there is no template '{template}'. The templates are:\n{}",
            listing()
        ));
    };

    let wanted = normalize(variant.unwrap_or(BASE_VARIANT));
    let name = info.name;

    let Some((variant, manifest)) = info
        .variants
        .into_iter()
        .find(|(v, _)| normalize(v) == wanted)
    else {
        return Err(format!(
            "the {name} template has no variant '{}'. The templates are:\n{}",
            variant.unwrap_or(BASE_VARIANT),
            listing()
        ));
    };

    Ok(Selected {
        template: name,
        variant,
        manifest,
    })
}

pub fn listing() -> String {
    let mut out = template_listing();
    let features = feature_listing();

    if !features.is_empty() {
        out.push_str("\n  features, added to any template with --feature:\n");

        for line in features.lines() {
            out.push_str(&format!("  {line}\n"));
        }
    }

    out
}

fn template_listing() -> String {
    let mut out = String::new();

    for template in available() {
        out.push_str(&format!(
            "  {:<24} {}\n",
            template.name,
            template.base().description
        ));

        for (variant, manifest) in template.variants.iter().skip(1) {
            out.push_str(&format!(
                "    --variant {variant:<12} {}\n",
                manifest.description
            ));
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

fn manifest(dir: &str) -> Option<Manifest> {
    let path = format!("{dir}/template.toml");
    let (_, contents) = TEMPLATE_FILES.iter().find(|(file, _)| *file == path)?;

    let text = std::str::from_utf8(contents).unwrap_or_else(|e| panic!("{path} is not UTF-8: {e}"));

    Some(toml::from_str(text).unwrap_or_else(|e| panic!("{path}: {e}")))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub description: String,
    pub next_step: String,
    #[serde(default)]
    pub feature: Option<Feature>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    pub plugin: String,
    #[serde(default)]
    pub shared_packs: Vec<String>,
}

pub fn plugin_templates() -> Vec<(&'static str, PluginManifest)> {
    let mut templates: Vec<(&'static str, PluginManifest)> = PLUGIN_TEMPLATE_FILES
        .iter()
        .filter_map(|(path, contents)| {
            let name = path.strip_suffix("/template.toml")?;

            if name.contains('/') {
                return None;
            }

            let text = std::str::from_utf8(contents)
                .unwrap_or_else(|e| panic!("{path} is not UTF-8: {e}"));

            Some((
                name,
                toml::from_str(text).unwrap_or_else(|e| panic!("{path}: {e}")),
            ))
        })
        .collect();

    templates.sort_by_key(|(name, _)| (*name != DEFAULT_PLUGIN_TEMPLATE, *name));
    templates
}

pub fn select_plugin(template: &str) -> Result<(&'static str, PluginManifest), String> {
    let wanted = normalize(template);

    plugin_templates()
        .into_iter()
        .find(|(name, _)| normalize(name) == wanted)
        .ok_or_else(|| {
            format!(
                "there is no plugin template '{template}'. The plugin templates are:\n{}",
                plugin_listing()
            )
        })
}

pub fn plugin_listing() -> String {
    plugin_templates()
        .iter()
        .map(|(name, manifest)| format!("  {name:<24} {}\n", manifest.description))
        .collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectFeatureManifest {
    pub description: String,
    pub next_step: String,
}

pub enum FeatureKind {
    Plugin(Feature),
    ProjectFiles(ProjectFeatureManifest),
}

pub struct FeatureInfo {
    pub name: &'static str,
    pub description: String,
    pub kind: FeatureKind,
}

pub fn project_features() -> Vec<(&'static str, ProjectFeatureManifest)> {
    let mut features: Vec<(&'static str, ProjectFeatureManifest)> = PROJECT_FEATURE_FILES
        .iter()
        .filter_map(|(path, contents)| {
            let name = path.strip_suffix("/template.toml")?;

            if name.contains('/') {
                return None;
            }

            let text = std::str::from_utf8(contents)
                .unwrap_or_else(|e| panic!("{path} is not UTF-8: {e}"));

            Some((
                name,
                toml::from_str(text).unwrap_or_else(|e| panic!("{path}: {e}")),
            ))
        })
        .collect();

    features.sort_by_key(|(name, _)| *name);
    features
}

pub fn features() -> Vec<FeatureInfo> {
    let plugins = plugin_templates()
        .into_iter()
        .filter_map(|(name, manifest)| {
            Some(FeatureInfo {
                name,
                description: manifest.description,
                kind: FeatureKind::Plugin(manifest.feature?),
            })
        });

    let files = project_features()
        .into_iter()
        .map(|(name, manifest)| FeatureInfo {
            name,
            description: manifest.description.clone(),
            kind: FeatureKind::ProjectFiles(manifest),
        });

    plugins.chain(files).collect()
}

pub fn select_feature(name: &str) -> Result<FeatureInfo, String> {
    let wanted = normalize(name);

    features()
        .into_iter()
        .find(|feature| normalize(feature.name) == wanted)
        .ok_or_else(|| {
            format!(
                "there is no feature '{name}'. The features are:\n{}",
                feature_listing()
            )
        })
}

pub fn feature_listing() -> String {
    features()
        .iter()
        .map(|feature| format!("  {:<24} {}\n", feature.name, feature.description))
        .collect()
}

pub fn write_project_feature_files(feature: &str, root: &Path, context: &tera::Context) -> usize {
    write_files(PROJECT_FEATURE_FILES, feature, root, context)
}

pub fn plugin_has_content(template: &str) -> bool {
    let prefix = format!("{template}/Content/");
    PLUGIN_TEMPLATE_FILES
        .iter()
        .any(|(path, _)| path.starts_with(&prefix))
}

pub fn write_plugin_files(template: &str, root: &Path, context: &tera::Context) -> usize {
    write_files(PLUGIN_TEMPLATE_FILES, template, root, context)
}

pub fn write_project_files(dir: &str, root: &Path, context: &tera::Context) -> usize {
    write_files(TEMPLATE_FILES, dir, root, context)
}

pub fn write_editor_library_files(root: &Path, context: &tera::Context) -> usize {
    write_files(PLUGIN_EDITOR_LIBRARY_FILES, "library", root, context)
}

pub fn render_editor_library_config(context: &tera::Context) -> String {
    let (path, contents) = PLUGIN_EDITOR_LIBRARY_FILES
        .iter()
        .find(|(path, _)| *path == "rusteal.editor.toml.tera")
        .expect("the editor library's configuration is embedded");

    let text = std::str::from_utf8(contents).unwrap_or_else(|e| panic!("{path} is not UTF-8: {e}"));

    tera::Tera::one_off(text, context, false)
        .unwrap_or_else(|e| panic!("Failed to render {path}: {e}"))
}

fn write_files(files: &[(&str, &[u8])], dir: &str, root: &Path, context: &tera::Context) -> usize {
    let prefix = format!("{dir}/");
    let mut written = 0;

    for (path, contents) in files {
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
    fn game_developers_never_write_unsafe_code() {
        let sources: Vec<_> = TEMPLATE_FILES
            .iter()
            .chain(PLUGIN_TEMPLATE_FILES)
            .filter(|(path, _)| path.ends_with(".rs") || path.ends_with(".rs.tera"))
            .collect();

        assert!(!sources.is_empty());

        for (path, contents) in &sources {
            let text = std::str::from_utf8(contents).unwrap();
            let code = text.replace("#![deny(unsafe_code)]", "");

            for forbidden in [
                "unsafe",
                "from_raw",
                "ulog!",
                "UObjectHandle",
                "UClassHandle",
            ] {
                assert!(
                    !code.contains(forbidden),
                    "{path}: game code uses `{forbidden}`, which Rusteal's safe API replaces"
                );
            }

            if path.ends_with("/src/lib.rs") || path.ends_with("/src/lib.rs.tera") {
                assert!(
                    text.starts_with("#![deny(unsafe_code)]"),
                    "{path}: a game or plugin crate starts with #![deny(unsafe_code)]"
                );
            }
        }
    }

    #[test]
    fn every_template_carries_the_default_optimizations() {
        let configs: Vec<_> = TEMPLATE_FILES
            .iter()
            .filter(|(path, _)| path.ends_with("/rusteal.toml.tera"))
            .collect();

        assert!(!configs.is_empty());

        for (path, contents) in configs {
            let text = std::str::from_utf8(contents).unwrap();

            assert_eq!(
                crate::optimizations::parse(text).unwrap(),
                Some(crate::optimizations::defaults()),
                "{path}"
            );
        }
    }

    #[test]
    fn every_template_has_a_valid_manifest() {
        let templates = available();
        let names: Vec<&str> = templates.iter().map(|t| t.name).collect();
        assert_eq!(
            names,
            [
                "ar",
                "blank",
                "first-person",
                "intro",
                "third-person",
                "top-down",
                "vehicle",
                "vr"
            ]
        );
        assert!(manifest(SETUP_TEMPLATE).is_some());

        for template in &templates {
            for (variant, manifest) in &template.variants {
                for feature in &manifest.features {
                    assert!(
                        select_feature(feature).is_ok(),
                        "{}/{variant} bundles an unknown feature {feature}",
                        template.name
                    );
                }
            }
        }
    }

    #[test]
    fn features_are_plugin_templates_with_their_own_plugin() {
        let features = features();
        assert!(features.iter().any(|f| f.name == "mannequin"));
        let mut plugins: Vec<String> = Vec::new();

        for info in &features {
            let FeatureKind::Plugin(feature) = &info.kind else {
                continue;
            };

            let name = info.name;
            assert!(
                !plugins.contains(&feature.plugin),
                "{name}: plugin {} twice",
                feature.plugin
            );
            plugins.push(feature.plugin.clone());

            for pack in &feature.shared_packs {
                assert_eq!(
                    pack.split('/').count(),
                    2,
                    "{name}: {pack} is not <Level>/<Pack>"
                );
            }
        }

        let mannequin = select_feature("Mannequin").unwrap();
        let FeatureKind::Plugin(plugin) = &mannequin.kind else {
            panic!("mannequin is a plugin feature");
        };
        assert_eq!(
            (mannequin.name, plugin.plugin.as_str()),
            ("mannequin", "Mannequin")
        );
        assert!(select_plugin("mannequin").is_ok());
        let unknown = select_feature("nope").err().unwrap();
        assert!(
            unknown.contains("mannequin") && unknown.contains("rustrover"),
            "{unknown}"
        );
        assert!(listing().contains("--feature"));
    }

    #[test]
    fn feature_names_are_unique_across_kinds() {
        let mut names: Vec<String> = features().iter().map(|f| normalize(f.name)).collect();
        let count = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), count, "two features share a name");
    }

    #[test]
    fn the_rustrover_feature_writes_its_run_configurations() {
        let rustrover = select_feature("RustRover").unwrap();
        assert!(matches!(rustrover.kind, FeatureKind::ProjectFiles(_)));

        let root = std::env::temp_dir().join(format!("rusteal-rustrover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut context = tera::Context::new();
        context.insert("project", "MyGame");
        context.insert("crate_name", "my-game");

        assert_eq!(write_project_feature_files("rustrover", &root, &context), 2);

        let watch = std::fs::read_to_string(root.join("Rust/.run/Rusteal Watch.run.xml")).unwrap();
        let test = std::fs::read_to_string(root.join("Rust/.run/Rusteal Test.run.xml")).unwrap();
        std::fs::remove_dir_all(&root).unwrap();

        assert!(watch.contains(r#"name="Rusteal Watch" type="CargoCommandRunConfiguration""#));
        assert!(watch.contains(r#"<option name="command" value="rusteal watch" />"#));
        assert!(test.contains(r#"name="Rusteal Test" type="CargoCommandRunConfiguration""#));
        assert!(test.contains(r#"<option name="command" value="test -p my-game" />"#));

        for config in [&watch, &test] {
            assert!(
                config
                    .contains(r#"<option name="workingDirectory" value="file://$PROJECT_DIR$" />"#)
            );
            assert!(config.contains(r#"<option name="requiredFeatures" value="false" />"#));
            assert!(
                config.contains(r#"<method v="2" />"#),
                "no task before launch"
            );
        }
    }

    #[test]
    fn the_vscode_feature_writes_tasks_and_debugger_configurations() {
        assert!(matches!(
            select_feature("VSCode").unwrap().kind,
            FeatureKind::ProjectFiles(_)
        ));

        let root = std::env::temp_dir().join(format!("rusteal-vscode-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut context = tera::Context::new();
        context.insert("project", "MyGame");
        context.insert("crate_name", "my-game");

        assert_eq!(write_project_feature_files("vscode", &root, &context), 3);

        let read = |name: &str| -> serde_json::Value {
            serde_json::from_str(
                &std::fs::read_to_string(root.join("Rust/.vscode").join(name)).unwrap(),
            )
            .unwrap()
        };
        let launch = read("launch.json");
        let tasks = read("tasks.json");
        let extensions = read("extensions.json");
        std::fs::remove_dir_all(&root).unwrap();

        let attach = &launch["configurations"][0];
        assert_eq!(attach["type"], "lldb");
        assert_eq!(attach["request"], "attach");
        assert_eq!(attach["pid"], "${command:pickMyProcess}");

        let commands: Vec<&str> = tasks["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|task| task["command"].as_str())
            .collect();
        assert_eq!(
            commands,
            [
                "cargo rusteal watch",
                "cargo rusteal build",
                "cargo test -p my-game"
            ]
        );
        assert!(extensions.to_string().contains("vadimcn.vscode-lldb"));
    }

    #[test]
    fn selects_templates_and_variants_by_loose_names() {
        let base = select("third-person", None).unwrap();

        assert_eq!(
            (base.template, base.variant),
            ("third-person", BASE_VARIANT)
        );

        assert_eq!(base.dir(), "third-person/base");
        let same = select("ThirdPerson", Some("BASE")).unwrap();

        assert_eq!(
            (same.template, same.variant),
            ("third-person", BASE_VARIANT)
        );

        let loose = select("third_person", None).unwrap();
        assert_eq!(loose.template, "third-person");

        let unknown = select("nope", None).err().unwrap();
        assert!(unknown.contains("blank"), "{unknown}");
        let no_variant = select("blank", Some("nope")).err().unwrap();
        assert!(no_variant.contains("no variant 'nope'"), "{no_variant}");
    }

    #[test]
    fn plugin_templates_have_valid_manifests() {
        let templates = plugin_templates();
        assert_eq!(templates[0].0, DEFAULT_PLUGIN_TEMPLATE);
        assert_eq!(select_plugin("Blank").unwrap().0, "blank");
        let unknown = select_plugin("nope").err().unwrap();
        assert!(unknown.contains("blank"), "{unknown}");
    }

    #[test]
    fn writes_plugin_files_with_rendered_paths() {
        let root =
            std::env::temp_dir().join(format!("rusteal-plugin-templates-{}", std::process::id()));

        let _ = std::fs::remove_dir_all(&root);
        let mut context = tera::Context::new();
        context.insert("plugin", "Inventory");
        context.insert("crate_name", "inventory");
        context.insert("version", "0.0.0");
        context.insert("glam_version", "0.0.0");

        write_plugin_files("blank", &root, &context);

        for rel in [
            "Inventory.uplugin",
            "rusteal.toml",
            ".gitignore",
            "Source/Inventory/Inventory.Build.cs",
            "Source/Inventory/Private/InventoryModule.cpp",
            "Rust/Cargo.toml",
            "Rust/inventory/Cargo.toml",
            "Rust/inventory/src/lib.rs",
        ] {
            assert!(root.join(rel).is_file(), "{rel} missing");
        }

        let module =
            std::fs::read_to_string(root.join("Source/Inventory/Private/InventoryModule.cpp"))
                .unwrap();

        assert!(
            module.contains("Inventory_RustealFillFuncTable();"),
            "{module}"
        );

        assert!(
            module.contains("IMPLEMENT_MODULE(FInventoryModule, Inventory)"),
            "{module}"
        );

        let config: rusteal_codegen::config::PluginConfig =
            toml::from_str(&std::fs::read_to_string(root.join("rusteal.toml")).unwrap()).unwrap();

        assert_eq!(config.plugin.crate_name, "inventory");

        let descriptor: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("Inventory.uplugin")).unwrap())
                .unwrap();

        assert_eq!(descriptor["Modules"][0]["Name"], "Inventory");
        std::fs::remove_dir_all(&root).unwrap();
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
        let hello = std::fs::read_to_string(root.join("Rust/probe/src/hello.rs")).unwrap();
        assert!(hello.contains("[Probe]"), "{hello}");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
