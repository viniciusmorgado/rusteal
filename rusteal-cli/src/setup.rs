// Setup command: extracts embedded UE plugin files into a UE project,
// generates starter config, registers plugins in .uproject, and writes C++ stubs.

use std::fs;
use std::path::Path;

use rusteal_codegen::config::find_uproject;

use crate::templates;

include!(concat!(env!("OUT_DIR"), "/plugin_files.rs"));

const CSPROJ_PROPS_TEMPLATE: &str = r#"<Project>
  <PropertyGroup>
    <EngineDir>{engine_path}</EngineDir>
  </PropertyGroup>
</Project>
"#;

/// Stub RustealFuncIds.h — empty namespace with FUNC_COUNT = 0.
const STUB_FUNC_IDS_H: &str = "\
// Auto-generated stub by rusteal setup. Will be overwritten by codegen.

#pragma once

#include <cstdint>

namespace RustealFuncId {

    constexpr uint32_t FUNC_COUNT = 0;

} // namespace RustealFuncId
";

/// Stub RustealFillFuncTable.cpp — empty implementations.
const STUB_FILL_TABLE_CPP: &str = "\
// Auto-generated stub by rusteal setup. Will be overwritten by codegen.

#include \"RustealFuncIds.h\"

void RustealFillFuncTable() {}

static void* GRustealFuncTable[1]; // placeholder (FUNC_COUNT == 0)

void** RustealGetFuncTable() {
    return GRustealFuncTable;
}

uint32_t RustealGetFuncCount() {
    return 0;
}
";

/// Default module_deps.txt content for initial build.
const STUB_MODULE_DEPS: &str = "Core\nCoreUObject\nEngine";

pub fn run_setup(project_path: &Path, engine_path: &Path) {
    let plugins_dir = project_path.join("Plugins");

    // Validate project path looks reasonable
    if !project_path.exists() {
        eprintln!(
            "Error: project path does not exist: {}",
            project_path.display()
        );
        std::process::exit(1);
    }

    // Validate engine path
    if !engine_path.join("Engine").exists() {
        eprintln!(
            "Warning: {}/Engine/ not found. Make sure the engine path is correct.",
            engine_path.display()
        );
    }

    // --- Step 1: Write embedded plugin files ---
    eprintln!(
        "rusteal setup: writing {} plugin files...",
        PLUGIN_FILES.len()
    );

    let mut written = 0;
    for (rel_path, contents) in PLUGIN_FILES {
        let dest = plugins_dir.join(rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("Failed to create {}: {e}", parent.display()));
        }
        fs::write(&dest, contents)
            .unwrap_or_else(|e| panic!("Failed to write {}: {e}", dest.display()));
        written += 1;
    }

    // Generate .csproj.props with engine path
    let engine_path_str = engine_path.to_str().unwrap_or_else(|| {
        eprintln!("Error: engine path contains non-UTF8 characters");
        std::process::exit(1);
    });
    // Normalize to forward slashes for UBT compatibility
    let engine_path_normalized = engine_path_str.replace('\\', "/");

    let props_content = CSPROJ_PROPS_TEMPLATE.replace("{engine_path}", &engine_path_normalized);
    let props_path = plugins_dir
        .join("RustealGenerator/Source/RustealExporter/RustealExporter.ubtplugin.csproj.props");
    if let Some(parent) = props_path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("Failed to create {}: {e}", parent.display()));
    }
    fs::write(&props_path, &props_content)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", props_path.display()));

    eprintln!("  Wrote {} plugin files to {}", written, plugins_dir.display());
    eprintln!("  Generated {}", props_path.display());
    eprintln!("  Engine path: {}", engine_path_normalized);

    // --- Step 2: Starter rusteal.toml in the project ---
    generate_config(project_path);

    // --- Step 3: Register plugins in .uproject ---
    register_uproject_plugins(project_path);

    // --- Step 4: Generate C++ stubs ---
    generate_cpp_stubs(project_path);

    eprintln!("rusteal setup: done!");
}

/// Generate a starter `rusteal.config.toml` in the current working directory (if it doesn't exist).
fn generate_config(project_path: &Path) {
    let config_path = project_path.join(rusteal_codegen::config::FILE_NAME);
    if config_path.exists() {
        eprintln!("  {} already exists, keeping it.", config_path.display());
        return;
    }
    let crate_name = default_crate_name(project_path);
    let mut ctx = tera::Context::new();
    ctx.insert("crate_name", &crate_name);
    fs::write(&config_path, templates::render("rusteal.toml.tera", &ctx))
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", config_path.display()));
    eprintln!("  Generated {} (crate = \"{crate_name}\")", config_path.display());
}

/// Cargo package name derived from the project name: `MyProject` → `my-project`.
pub fn default_crate_name(project_path: &Path) -> String {
    let name = find_uproject(project_path)
        .and_then(|path| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "game".to_string());
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.push(if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' });
    }
    out
}

/// Find and update the .uproject file to include Rusteal and RustealGenerator plugins.
fn register_uproject_plugins(project_path: &Path) {
    // Find *.uproject in project directory
    let uproject_path = match find_uproject(project_path) {
        Some(p) => p,
        None => {
            eprintln!("  Warning: no .uproject file found in {}, skipping plugin registration.", project_path.display());
            return;
        }
    };

    // Read and parse JSON
    let content = fs::read_to_string(&uproject_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", uproject_path.display()));

    let mut doc: serde_json::Value = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", uproject_path.display()));

    let obj = doc
        .as_object_mut()
        .expect(".uproject root must be a JSON object");

    // Ensure "Plugins" array exists
    if !obj.contains_key("Plugins") {
        obj.insert(
            "Plugins".to_string(),
            serde_json::Value::Array(Vec::new()),
        );
    }

    let plugins = obj
        .get_mut("Plugins")
        .unwrap()
        .as_array_mut()
        .expect(".uproject Plugins must be an array");

    // Add missing plugin entries
    let required = ["Rusteal", "RustealGenerator"];
    let mut added = Vec::new();
    for name in &required {
        let already = plugins.iter().any(|p| {
            p.get("Name")
                .and_then(|v| v.as_str())
                .map_or(false, |n| n == *name)
        });
        if !already {
            plugins.push(serde_json::json!({
                "Name": name,
                "Enabled": true
            }));
            added.push(*name);
        }
    }

    // Write back formatted JSON
    let output = serde_json::to_string_pretty(&doc)
        .unwrap_or_else(|e| panic!("Failed to serialize .uproject: {e}"));
    fs::write(&uproject_path, output)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", uproject_path.display()));

    if added.is_empty() {
        eprintln!(
            "  {} already has Rusteal plugins registered.",
            uproject_path.display()
        );
    } else {
        eprintln!(
            "  Registered plugins in {}: {}",
            uproject_path.display(),
            added.join(", ")
        );
    }
}

/// Generate C++ stub files so the first UE build can link before codegen runs.
fn generate_cpp_stubs(project_path: &Path) {
    let generated_dir = project_path.join("Plugins/Rusteal/Source/Rusteal/Generated");
    fs::create_dir_all(&generated_dir)
        .unwrap_or_else(|e| panic!("Failed to create {}: {e}", generated_dir.display()));

    // Only for a project that never ran codegen: re-running setup (to update the
    // plugins, say) must not replace real wrappers with stubs.
    if generated_dir.join("RustealFuncIds.h").exists() {
        eprintln!("  {} already has generated code, keeping it.", generated_dir.display());
        return;
    }

    let files: &[(&str, &str)] = &[
        ("RustealFuncIds.h", STUB_FUNC_IDS_H),
        ("RustealFillFuncTable.cpp", STUB_FILL_TABLE_CPP),
        ("module_deps.txt", STUB_MODULE_DEPS),
    ];

    for (name, content) in files {
        let path = generated_dir.join(name);
        fs::write(&path, content)
            .unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
    }

    eprintln!(
        "  Generated C++ stubs in {}",
        generated_dir.display()
    );
}
