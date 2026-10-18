use std::path::Path;

use rusteal_codegen::config::{
    EDITOR_BINDINGS_DIR, LibraryKind, PluginConfig, PluginLayout, editor_module, find_uproject,
};

use crate::project_version::{self, Pins};
use crate::{build_cmd, new_cmd, setup, templates};

const RESERVED: &[&str] = &["Rusteal", "RustealGenerator", "RustealEditor"];

pub struct PluginNewOptions<'a> {
    pub name: &'a str,
    pub template: &'a str,
    pub root: &'a Path,
    pub engine: &'a Path,
    pub build: bool,
    pub editor: bool,
}

pub fn run_plugin_new(opts: &PluginNewOptions) {
    let next_step = create_plugin(opts.name, opts.template, opts.root, opts.engine);

    if opts.editor {
        add_editor_library(
            opts.root,
            &opts.root.join("Plugins").join(opts.name),
            opts.name,
        );
    }

    if opts.build {
        build_cmd::run_build(opts.root, opts.engine, None, 1, false, None);
    }

    eprintln!("\nrusteal plugin new: done.");

    if !opts.build {
        eprintln!("  rusteal build --all   # UE build, bindings, plugin, cargo, deploy");
    }

    eprintln!("  # {next_step}");
}

pub fn create_plugin(name: &str, template: &str, root: &Path, engine: &Path) -> String {
    validate_name(name, root);

    let (template, manifest) = templates::select_plugin(template).unwrap_or_else(|message| {
        eprint!("Error: {message}");
        std::process::exit(1);
    });

    if let Some(feature) = &manifest.feature
        && feature.plugin != name
        && templates::plugin_has_content(template)
    {
        fail(&format!(
            "the {template} feature's assets live under /{plugin}/, so its plugin is named \
             {plugin}: rusteal plugin new {plugin} --template {template}",
            plugin = feature.plugin
        ));
    }

    let dir = root.join("Plugins").join(name);

    if dir.exists() {
        fail(&format!("{} already exists.", dir.display()));
    }

    let layout = PluginLayout::new(&dir);

    eprintln!(
        "rusteal plugin new: creating {} from the {template} plugin template",
        dir.display()
    );

    let crate_name = setup::crate_name_for(name);
    let context = plugin_context(name, &crate_name, root);
    let written = templates::write_plugin_files(template, &dir, &context);
    eprintln!("rusteal plugin new: {written} files (crate {crate_name})");
    new_cmd::write_bindings_placeholder(&layout.rust_workspace().join("bindings"), &crate_name);

    setup::write_cpp_stubs(
        &layout.output(Default::default(), &crate_name).cpp_generated,
        &format!("{name}_"),
    );

    enable_in_uproject(root, name);

    if let Some(feature) = &manifest.feature {
        new_cmd::copy_shared_packs(engine, root, &feature.shared_packs);
    }

    tera::Tera::one_off(&manifest.next_step, &context, false)
        .unwrap_or_else(|e| panic!("Failed to render next_step: {e}"))
}

pub fn run_plugin_editor(name: &str, root: &Path, engine: &Path, build: bool) {
    let dir = root.join("Plugins").join(name);

    if !dir.join(rusteal_codegen::config::FILE_NAME).is_file() {
        fail(&format!(
            "{} is not a Rusteal plugin (it has no {}).",
            dir.display(),
            rusteal_codegen::config::FILE_NAME
        ));
    }

    add_editor_library(root, &dir, name);

    if build {
        build_cmd::run_build(root, engine, None, 1, false, None);
    }

    eprintln!("\nrusteal plugin editor: done.");

    if !build {
        eprintln!("  rusteal build --all   # UE build, bindings, plugins, cargo, deploy");
    }
}

pub fn add_editor_library(root: &Path, dir: &Path, plugin: &str) {
    let config = PluginConfig::load(dir).unwrap_or_else(|e| fail(&e));

    if config.plugin.kind == LibraryKind::Editor {
        fail(&format!(
            "{plugin} is an editor plugin already (kind = \"editor\"): an editor library goes \
             next to a runtime one."
        ));
    }

    if config.editor.is_some() {
        fail(&format!("{plugin} has an editor library already."));
    }

    let module = editor_module(plugin);
    let editor_crate = format!("{}-editor", config.plugin.crate_name);
    let layout = PluginLayout::new(dir);
    let workspace = layout.rust_workspace();

    validate_module(&module, root);

    for taken in [
        dir.join("Source").join(&module),
        workspace.join(&editor_crate),
        workspace.join(EDITOR_BINDINGS_DIR),
    ] {
        if taken.exists() {
            fail(&format!("{} already exists.", taken.display()));
        }
    }

    let mut context = plugin_context(plugin, &config.plugin.crate_name, root);
    context.insert("module", &module);
    context.insert("editor_crate", &editor_crate);

    let written = templates::write_editor_library_files(dir, &context);

    eprintln!(
        "rusteal plugin: adding the editor library {module} to {plugin}: {written} files \
         (crate {editor_crate})"
    );

    append_config(
        &layout.config_file(),
        &templates::render_editor_library_config(&context),
    );
    add_module(&layout.descriptor(), &module);
    add_workspace_members(&workspace.join("Cargo.toml"), &editor_crate);
    new_cmd::write_bindings_placeholder(&workspace.join(EDITOR_BINDINGS_DIR), &editor_crate);

    setup::write_cpp_stubs(
        &dir.join("Source").join(&module).join("Generated"),
        &format!("{module}_"),
    );
}

fn validate_module(module: &str, root: &Path) {
    if module.len() > 32 {
        fail(&format!(
            "'{module}' is longer than 32 characters: use a shorter plugin name for a plugin \
             with an editor library."
        ));
    }

    let project = find_uproject(root)
        .and_then(|path| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default();

    let clashes_with_plugin = root.join("Plugins").join(module).exists();

    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(module))
        || project.eq_ignore_ascii_case(module)
        || clashes_with_plugin
    {
        fail(&format!(
            "'{module}' is taken: the editor library's module would clash with Rusteal's, the \
             project's or another plugin's."
        ));
    }
}

fn append_config(path: &Path, section: &str) {
    let mut text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));

    if !text.ends_with('\n') {
        text.push('\n');
    }

    text.push_str(section);

    std::fs::write(path, text)
        .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", path.display())));
}

fn add_module(descriptor: &Path, module: &str) {
    let text = std::fs::read_to_string(descriptor)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", descriptor.display())));

    let mut json: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .unwrap_or_else(|e| fail(&format!("cannot parse {}: {e}", descriptor.display())));

    let modules = json
        .as_object_mut()
        .unwrap_or_else(|| fail(&format!("{} is not a JSON object", descriptor.display())))
        .entry("Modules")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()))
        .as_array_mut()
        .unwrap_or_else(|| {
            fail(&format!(
                "{}: Modules is not an array",
                descriptor.display()
            ))
        });

    if !modules.iter().any(|m| m["Name"] == module) {
        modules.push(serde_json::json!({
            "Name": module,
            "Type": "Editor",
            "LoadingPhase": "Default"
        }));
    }

    let mut out = serde_json::to_string_pretty(&json).expect("serializable descriptor");
    out.push('\n');

    std::fs::write(descriptor, out)
        .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", descriptor.display())));
}

fn add_workspace_members(manifest: &Path, editor_crate: &str) {
    let text = std::fs::read_to_string(manifest)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", manifest.display())));

    let members = format!("\"{editor_crate}\", \"{EDITOR_BINDINGS_DIR}\"");
    let mut done = false;

    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();

            if !done && trimmed.starts_with("members") && trimmed.trim_end().ends_with(']') {
                done = true;
                let end = line.rfind(']').expect("a closing bracket");
                format!(
                    "{}, {members}]",
                    &line[..end].trim_end().trim_end_matches(',')
                )
            } else {
                line.to_string()
            }
        })
        .collect();

    if !done {
        fail(&format!(
            "{}: add \"{editor_crate}\" and \"{EDITOR_BINDINGS_DIR}\" to [workspace] members \
             by hand (the members list is not on one line).",
            manifest.display()
        ));
    }

    lines.push(String::new());
    lines.push(format!("[profile.dev.package.{editor_crate}-bindings]"));
    lines.push("opt-level = 3".to_string());
    lines.push("debug = \"line-tables-only\"".to_string());

    let mut out = lines.join("\n");
    out.push('\n');

    std::fs::write(manifest, out)
        .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", manifest.display())));
}

fn validate_name(name: &str, root: &Path) {
    let ok = (1..=32).contains(&name.len())
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric());

    if !ok {
        fail(&format!(
            "'{name}' is not a valid plugin name (letters and digits, starting with a letter, \
             32 characters at most)."
        ));
    }

    let project = find_uproject(root)
        .and_then(|path| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default();

    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(name)) || project.eq_ignore_ascii_case(name) {
        fail(&format!(
            "'{name}' is taken: the plugin's module would clash with Rusteal's or the project's."
        ));
    }
}

fn plugin_context(plugin: &str, crate_name: &str, root: &Path) -> tera::Context {
    let mut ctx = tera::Context::new();
    ctx.insert("plugin", plugin);
    ctx.insert("crate_name", crate_name);
    ctx.insert("version", env!("CARGO_PKG_VERSION"));
    ctx.insert("glam_version", new_cmd::GLAM_VERSION);

    if let Ok(Pins::Checkout(checkout)) = project_version::read_pins(root) {
        let checkout = checkout.canonicalize().unwrap_or(checkout);

        ctx.insert(
            "runtime_path",
            &checkout.to_string_lossy().replace('\\', "/"),
        );
    }

    ctx
}

fn enable_in_uproject(root: &Path, plugin: &str) {
    let Some(path) = find_uproject(root) else {
        fail(&format!("no .uproject in {}", root.display()));
    };

    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));

    let mut json: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .unwrap_or_else(|e| fail(&format!("cannot parse {}: {e}", path.display())));

    let plugins = json
        .as_object_mut()
        .unwrap_or_else(|| fail(&format!("{} is not a JSON object", path.display())))
        .entry("Plugins")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()))
        .as_array_mut()
        .unwrap_or_else(|| fail(&format!("{}: Plugins is not an array", path.display())));

    if !plugins.iter().any(|p| p["Name"] == plugin) {
        plugins.push(serde_json::json!({ "Name": plugin, "Enabled": true }));
    }

    let mut out = serde_json::to_string_pretty(&json).expect("serializable uproject");
    out.push('\n');

    std::fs::write(&path, out)
        .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", path.display())));

    eprintln!("  {} lists {plugin}", path.display());
}

fn fail(message: &str) -> ! {
    eprintln!("Error: {message}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_editor_library_is_added_next_to_the_runtime_one() {
        let root =
            std::env::temp_dir().join(format!("rusteal-editor-library-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("Plugins/Inventory");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(root.join("Game.uproject"), "{}").unwrap();

        let context = plugin_context("Inventory", "inventory", &root);
        templates::write_plugin_files("blank", &dir, &context);
        add_editor_library(&root, &dir, "Inventory");

        for rel in [
            "Source/InventoryEditor/InventoryEditor.Build.cs",
            "Source/InventoryEditor/Private/InventoryEditorModule.cpp",
            "Source/InventoryEditor/Generated",
            "Rust/inventory-editor/Cargo.toml",
            "Rust/inventory-editor/src/lib.rs",
            "Rust/editor-bindings/Cargo.toml",
        ] {
            assert!(dir.join(rel).exists(), "{rel} missing");
        }

        let config = PluginConfig::load(&dir).unwrap();
        let libraries = config.libraries("Inventory");
        assert_eq!(libraries.len(), 2);
        assert_eq!(libraries[1].crate_name, "inventory-editor");

        let descriptor: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("Inventory.uplugin")).unwrap())
                .unwrap();

        assert_eq!(descriptor["Modules"][1]["Name"], "InventoryEditor");
        assert_eq!(descriptor["Modules"][1]["Type"], "Editor");

        let workspace: toml::Table = std::fs::read_to_string(dir.join("Rust/Cargo.toml"))
            .unwrap()
            .parse()
            .unwrap();

        let members = workspace["workspace"]["members"].as_array().unwrap();
        assert!(
            members
                .iter()
                .any(|m| m.as_str() == Some("inventory-editor"))
        );
        assert!(
            members
                .iter()
                .any(|m| m.as_str() == Some("editor-bindings"))
        );

        let module = std::fs::read_to_string(
            dir.join("Source/InventoryEditor/Private/InventoryEditorModule.cpp"),
        )
        .unwrap();

        assert!(
            module.contains("InventoryEditor_RustealFillFuncTable();"),
            "{module}"
        );

        assert!(
            module.contains(
                "RustealRegisterPluginModuleLibraryChecked(TEXT(\"Inventory\"), TEXT(\"InventoryEditor\")"
            ),
            "{module}"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }
}
