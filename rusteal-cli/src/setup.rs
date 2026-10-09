use std::fs;
use std::path::Path;

use rusteal_codegen::config::find_uproject;

use crate::templates;

include!(concat!(env!("OUT_DIR"), "/plugin_files.rs"));

const CSPROJ_PROPS_TEMPLATE: &str = r#"<Project>
  <PropertyGroup>
    <EngineDir>{engine_path}/Engine</EngineDir>
  </PropertyGroup>
</Project>
"#;

const STUB_FUNC_IDS_H: &str = "\
// Auto-generated stub by rusteal setup. Will be overwritten by codegen.

#pragma once

#include <cstdint>

namespace RustealFuncId {

    constexpr uint32_t FUNC_COUNT = 0;

} // namespace RustealFuncId
";

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

const STUB_MODULE_DEPS: &str = "Core\nCoreUObject\nEngine";

pub fn run_setup(project_path: &Path, engine_path: &Path) {
    let plugins_dir = project_path.join("Plugins");

    if !project_path.exists() {
        eprintln!(
            "Error: project path does not exist: {}",
            project_path.display()
        );

        std::process::exit(1);
    }

    if !engine_path.join("Engine").exists() {
        eprintln!(
            "Warning: {}/Engine/ not found. Make sure the engine path is correct.",
            engine_path.display()
        );
    }

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

        if fs::read(&dest).is_ok_and(|old| old == *contents) {
            continue;
        }

        fs::write(&dest, contents)
            .unwrap_or_else(|e| panic!("Failed to write {}: {e}", dest.display()));

        written += 1;
    }

    let engine_path_str = engine_path.to_str().unwrap_or_else(|| {
        eprintln!("Error: engine path contains non-UTF8 characters");
        std::process::exit(1);
    });

    let engine_path_normalized = engine_path_str.replace('\\', "/");

    let props_content = CSPROJ_PROPS_TEMPLATE.replace("{engine_path}", &engine_path_normalized);

    let props_path = plugins_dir
        .join("RustealGenerator/Source/RustealExporter/RustealExporter.ubtplugin.csproj.props");

    if let Some(parent) = props_path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("Failed to create {}: {e}", parent.display()));
    }

    if !fs::read_to_string(&props_path).is_ok_and(|old| old == props_content) {
        fs::write(&props_path, &props_content)
            .unwrap_or_else(|e| panic!("Failed to write {}: {e}", props_path.display()));
    }

    eprintln!(
        "  Wrote {written} of {} plugin files to {} (the others were unchanged)",
        PLUGIN_FILES.len(),
        plugins_dir.display()
    );

    eprintln!("  Generated {}", props_path.display());
    eprintln!("  Engine path: {}", engine_path_normalized);

    generate_config(project_path);

    register_uproject_plugins(project_path);

    generate_cpp_stubs(project_path);

    eprintln!("rusteal setup: done!");
}

fn generate_config(project_path: &Path) {
    let config_path = project_path.join(rusteal_codegen::config::FILE_NAME);

    if config_path.exists() {
        eprintln!("  {} already exists, keeping it.", config_path.display());
        return;
    }

    let crate_name = default_crate_name(project_path);
    let mut ctx = tera::Context::new();
    ctx.insert("crate_name", &crate_name);

    fs::write(&config_path, templates::render_rusteal_toml(&ctx))
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", config_path.display()));

    eprintln!(
        "  Generated {} (crate = \"{crate_name}\")",
        config_path.display()
    );
}

pub fn default_crate_name(project_path: &Path) -> String {
    let name = find_uproject(project_path)
        .and_then(|path| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "game".to_string());

    crate_name_for(&name)
}

pub fn crate_name_for(name: &str) -> String {
    let mut out = String::new();

    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('-');
        }

        out.push(if c.is_alphanumeric() {
            c.to_ascii_lowercase()
        } else {
            '-'
        });
    }

    out
}

fn register_uproject_plugins(project_path: &Path) {
    let uproject_path = match find_uproject(project_path) {
        Some(p) => p,
        None => {
            eprintln!(
                "  Warning: no .uproject file found in {}, skipping plugin registration.",
                project_path.display()
            );

            return;
        }
    };

    let content = fs::read_to_string(&uproject_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", uproject_path.display()));

    let mut doc: serde_json::Value = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", uproject_path.display()));

    let obj = doc
        .as_object_mut()
        .expect(".uproject root must be a JSON object");

    if !obj.contains_key("Plugins") {
        obj.insert("Plugins".to_string(), serde_json::Value::Array(Vec::new()));
    }

    let plugins = obj
        .get_mut("Plugins")
        .unwrap()
        .as_array_mut()
        .expect(".uproject Plugins must be an array");

    let required = ["Rusteal", "RustealGenerator"];
    let mut added = Vec::new();

    for name in &required {
        let already = plugins
            .iter()
            .any(|p| p.get("Name").and_then(|v| v.as_str()) == Some(*name));

        if !already {
            plugins.push(serde_json::json!({
                "Name": name,
                "Enabled": true
            }));

            added.push(*name);
        }
    }

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

fn generate_cpp_stubs(project_path: &Path) {
    write_cpp_stubs(
        &project_path.join("Plugins/Rusteal/Source/Rusteal/Generated"),
        "",
    );
}

pub fn write_cpp_stubs(generated_dir: &Path, prefix: &str) {
    fs::create_dir_all(generated_dir)
        .unwrap_or_else(|e| panic!("Failed to create {}: {e}", generated_dir.display()));

    if generated_dir.join("RustealFuncIds.h").exists() {
        eprintln!(
            "  {} already has generated code, keeping it.",
            generated_dir.display()
        );

        return;
    }

    let fill_table = STUB_FILL_TABLE_CPP
        .replace(
            "void RustealFillFuncTable",
            &format!("void {prefix}RustealFillFuncTable"),
        )
        .replace(
            "void** RustealGetFuncTable",
            &format!("void** {prefix}RustealGetFuncTable"),
        )
        .replace(
            "uint32_t RustealGetFuncCount",
            &format!("uint32_t {prefix}RustealGetFuncCount"),
        );

    let files: &[(&str, &str)] = &[
        ("RustealFuncIds.h", STUB_FUNC_IDS_H),
        ("RustealFillFuncTable.cpp", &fill_table),
        ("module_deps.txt", STUB_MODULE_DEPS),
    ];

    for (name, content) in files {
        let path = generated_dir.join(name);

        fs::write(&path, content)
            .unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
    }

    eprintln!("  Generated C++ stubs in {}", generated_dir.display());
}
