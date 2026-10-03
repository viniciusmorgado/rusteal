// rusteal-codegen: reads UHT JSON, generates Rust bindings + C++ wrappers + FuncId tables.

pub mod schema;
pub mod naming;
pub mod config;
pub mod context;
pub mod type_map;
pub mod defaults;
pub mod filter;
pub mod rust_gen;
pub mod cpp_gen;

use std::path::Path;

use crate::config::{
    CodegenConfig, LibraryOutput, PluginConfig, PluginLayout, ProjectConfig, ProjectLayout,
};
use crate::schema::{ClassesFile, EnumsFile, StructsFile};

/// Generate the game's library: the bindings crate in `Rust/bindings` and
/// the C++ wrappers compiled into the Rusteal plugin.
///
/// `project_root` is the directory holding the .uproject and `rusteal.toml`.
pub fn run_generate(project_root: &Path) {
    let config = ProjectConfig::load(project_root).unwrap_or_else(|e| {
        eprintln!("rusteal-codegen: {e}");
        std::process::exit(1);
    });
    let layout = ProjectLayout::new(project_root);
    generate_library(&layout.uht_json(), &config.codegen, &layout.game_output());
}

/// Generate a Rusteal plugin's library: the bindings crate in the plugin's
/// `Rust/bindings` and the C++ wrappers compiled into the plugin's module,
/// from the project's reflection JSON.
pub fn run_generate_plugin(project_root: &Path, plugin: &PluginLayout) {
    let config = PluginConfig::load(&plugin.dir).unwrap_or_else(|e| {
        eprintln!("rusteal-codegen: {e}");
        std::process::exit(1);
    });
    let layout = ProjectLayout::new(project_root);
    eprintln!("rusteal-codegen: plugin {}", plugin.name);
    generate_library(&layout.uht_json(), &config.codegen, &plugin.output(config.plugin.kind));
}

/// Generate one library's bindings crate and C++ wrappers from the
/// reflection JSON in `uht_input`.
pub fn generate_library(uht_input: &Path, codegen: &CodegenConfig, output: &LibraryOutput) {
    let classes_path = uht_input.join("rusteal_classes.json");
    let structs_path = uht_input.join("rusteal_structs.json");
    let enums_path = uht_input.join("rusteal_enums.json");
    // The generated crate's directory; its sources go under `src/`.
    let rust_out = output.bindings_crate.clone();
    let rust_src = rust_out.join("src");
    let cpp_out = output.cpp_generated.clone();

    eprintln!("rusteal-codegen: loading JSON...");

    // Parse JSON files
    let classes_json: ClassesFile = {
        let data = std::fs::read_to_string(&classes_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", classes_path.display()));
        serde_json::from_str(&data)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", classes_path.display()))
    };

    let structs_json: StructsFile = {
        let data = std::fs::read_to_string(&structs_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", structs_path.display()));
        serde_json::from_str(&data)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", structs_path.display()))
    };

    let enums_json: EnumsFile = {
        let data = std::fs::read_to_string(&enums_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", enums_path.display()));
        serde_json::from_str(&data)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", enums_path.display()))
    };

    eprintln!(
        "  Loaded {} classes, {} structs, {} enums",
        classes_json.classes.len(),
        structs_json.structs.len(),
        enums_json.enums.len()
    );

    // Build context
    let mut ctx = context::CodegenContext::new(
        classes_json.classes,
        structs_json.structs,
        enums_json.enums,
        codegen,
    );
    ctx.cpp_prefix = output.cpp_prefix.clone();

    eprintln!(
        "  Enabled modules: {:?}",
        ctx.enabled_modules.iter().collect::<Vec<_>>()
    );
    for (module, classes) in &ctx.module_classes {
        eprintln!("    {}: {} classes", module, classes.len());
    }
    for (module, structs) in &ctx.module_structs {
        eprintln!("    {}: {} structs", module, structs.len());
    }
    for (module, enums) in &ctx.module_enums {
        eprintln!("    {}: {} enums", module, enums.len());
    }

    // Apply filters
    eprintln!("rusteal-codegen: filtering...");
    filter::apply_filters(&mut ctx, &codegen.blocklist);

    // Build function table (assign FuncIds)
    eprintln!("rusteal-codegen: building function table...");
    build_func_table(&mut ctx);
    eprintln!("  {} functions in func_table", ctx.func_table.len());

    // Generate the Rust bindings crate: sources, extensions, prelude, manifest.
    eprintln!("rusteal-codegen: generating Rust code...");
    rust_gen::generate(&ctx, &rust_src);
    rust_gen::manual::write_manual_module(&rust_src);
    rust_gen::prelude::write_prelude(&rust_src, &ctx);
    rust_gen::cargo_toml::write_crate_files(&rust_out, &ctx, codegen);

    // Generate C++ code
    eprintln!("rusteal-codegen: generating C++ code...");
    cpp_gen::generate(&ctx, &cpp_out);

    // Generate module_deps.txt for Rusteal.Build.cs, and list the plugins
    // those modules come from in the host plugin's descriptor
    generate_module_deps(codegen, &output.host_module, &cpp_out);
    update_plugin_dependencies(codegen, &output.plugin_descriptor);

    // Post-generate verification
    eprintln!("rusteal-codegen: verifying output...");
    verify_output(&ctx, &rust_src, &cpp_out);

    eprintln!("rusteal-codegen: done!");
}

/// Generate module_deps.txt listing UE module names needed by enabled features.
/// The module the wrappers compile into is never its own dependency.
fn generate_module_deps(config: &CodegenConfig, host_module: &str, cpp_out: &Path) {
    use std::collections::BTreeSet;

    let enabled_features: std::collections::HashSet<&str> =
        config.features.iter().map(|s| s.as_str()).collect();

    // Collect UE package names whose feature is enabled.
    // "Core" is always needed (UE base) and not in the modules map.
    let mut ue_modules: BTreeSet<&str> = BTreeSet::new();
    ue_modules.insert("Core");
    for (pkg, mapping) in &config.modules {
        if enabled_features.contains(mapping.feature.as_str()) {
            ue_modules.insert(pkg.as_str());
        }
    }
    ue_modules.remove(host_module);

    let content = ue_modules.iter().copied().collect::<Vec<_>>().join("\n");

    let path = cpp_out.join("module_deps.txt");
    std::fs::write(&path, &content)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));

    eprintln!("  module_deps.txt: {:?}", ue_modules.iter().collect::<Vec<_>>());
}

/// Add the engine plugins of the enabled modules (`plugin = "StateTree"` in
/// `[codegen.modules]`) to the Rusteal plugin's descriptor: UBT wants a plugin
/// to list the plugins whose modules it links. Plugins already listed stay.
fn update_plugin_dependencies(config: &CodegenConfig, descriptor: &Path) {
    let Ok(text) = std::fs::read_to_string(descriptor) else {
        return; // no installed plugin (a codegen test): nothing to update
    };
    let mut json: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", descriptor.display()));
    let enabled: std::collections::HashSet<&str> =
        config.features.iter().map(|s| s.as_str()).collect();
    let mut wanted: Vec<&str> = config
        .modules
        .values()
        .filter(|m| enabled.contains(m.feature.as_str()))
        .filter_map(|m| m.plugin.as_deref())
        .collect();
    wanted.sort();
    wanted.dedup();

    let plugins = json
        .as_object_mut()
        .expect("a plugin descriptor is a JSON object")
        .entry("Plugins")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()))
        .as_array_mut()
        .expect("a plugin descriptor's Plugins is an array");
    let mut added = Vec::new();
    for name in wanted {
        if plugins.iter().any(|p| p["Name"] == name) {
            continue;
        }
        plugins.push(serde_json::json!({ "Name": name, "Enabled": true }));
        added.push(name);
    }
    if added.is_empty() {
        return;
    }
    let mut out = serde_json::to_string_pretty(&json).expect("serializable descriptor");
    out.push('\n');
    std::fs::write(descriptor, out)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", descriptor.display()));
    let name = descriptor.file_name().unwrap_or_default().to_string_lossy();
    eprintln!("  {name}: now depends on {added:?}");
}

/// Verify codegen output integrity.
fn verify_output(ctx: &context::CodegenContext, rust_out: &Path, cpp_out: &Path) {
    let mut errors: Vec<String> = Vec::new();

    // 1. FuncId contiguity: IDs must be 0..N-1 with no gaps
    for (i, entry) in ctx.func_table.iter().enumerate() {
        if entry.func_id != i as u32 {
            errors.push(format!(
                "FuncId gap: expected {} for module '{}' {}.{}, got {} (total functions: {})",
                i, entry.module_name, entry.class_name, entry.func_name,
                entry.func_id, ctx.func_table.len()
            ));
            break; // one error is enough to flag the issue
        }
    }

    // 2. Required Rust output files exist and are non-empty
    let rust_required = ["lib.rs", "func_ids.rs"];
    for name in &rust_required {
        let path = rust_out.join(name);
        match std::fs::metadata(&path) {
            Ok(m) if m.len() == 0 => errors.push(format!("Rust output empty: {}", path.display())),
            Err(_) => errors.push(format!("Rust output missing: {}", path.display())),
            _ => {}
        }
    }

    // 3. Per-module mod.rs exists for each enabled module
    for module_name in ctx.enabled_modules.iter() {
        let mod_path = rust_out.join(module_name).join("mod.rs");
        if !mod_path.exists() {
            errors.push(format!("Module mod.rs missing: {}", mod_path.display()));
        }
    }

    // 4. Required C++ output files exist and are non-empty
    let cpp_required = ["RustealFuncIds.h", "RustealFillFuncTable.cpp"];
    for name in &cpp_required {
        let path = cpp_out.join(name);
        match std::fs::metadata(&path) {
            Ok(m) if m.len() == 0 => errors.push(format!("C++ output empty: {}", path.display())),
            Err(_) => errors.push(format!("C++ output missing: {}", path.display())),
            _ => {}
        }
    }

    // 5. Summary stats
    let func_count = ctx.func_table.len();
    let module_count = ctx.enabled_modules.len();
    let class_count: usize = ctx.module_classes.values().map(|v| v.len()).sum();
    let struct_count: usize = ctx.module_structs.values().map(|v| v.len()).sum();
    let enum_count: usize = ctx.module_enums.values().map(|v| v.len()).sum();

    if errors.is_empty() {
        eprintln!(
            "  OK: {} modules, {} classes, {} structs, {} enums, {} functions",
            module_count, class_count, struct_count, enum_count, func_count
        );
    } else {
        eprintln!("  Verification FAILED:");
        for e in &errors {
            eprintln!("    - {e}");
        }
        std::process::exit(1);
    }
}

/// Assign deterministic FuncIds to all exportable functions.
fn build_func_table(ctx: &mut context::CodegenContext) {
    let mut entries = Vec::new();

    for (module_name, classes) in &ctx.module_classes {
        for class in classes {
            // Skip UInterface classes — their functions can't be called directly
            if class.super_class.as_deref() == Some("Interface") {
                continue;
            }
            for func in &class.funcs {
                // Skip functions with unsupported param types
                let all_supported = func.params.iter().all(|p| {
                    type_map::map_property_type(
                        &p.prop_type,
                        p.class_name.as_deref(),
                        p.struct_name.as_deref(),
                        p.enum_name.as_deref(),
                        p.enum_underlying_type.as_deref(),
                        p.meta_class_name.as_deref(),
                        p.interface_name.as_deref(),
                    )
                    .supported
                });
                if !all_supported {
                    continue;
                }
                entries.push(context::FuncEntry {
                    func_id: 0, // assigned below
                    module_name: module_name.clone(),
                    class_name: class.name.clone(),
                    func_name: func.name.clone(),
                    rust_func_name: naming::to_snake_case(&func.name),
                    func: func.clone(),
                    cpp_class_name: class.cpp_name.clone(),
                    header: class.header.clone(),
                });
            }
        }
    }

    // Sort by (module, class, func) for deterministic IDs
    entries.sort_by(|a, b| {
        a.module_name
            .cmp(&b.module_name)
            .then_with(|| a.class_name.cmp(&b.class_name))
            .then_with(|| a.func_name.cmp(&b.func_name))
    });

    // Assign sequential IDs
    for (i, entry) in entries.iter_mut().enumerate() {
        entry.func_id = i as u32;
    }

    ctx.func_table = entries;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_dependencies_follow_enabled_modules() {
        let config: crate::config::CodegenConfig = toml::from_str(
            r#"
            features = ["engine", "state-tree"]
            [modules]
            Engine = { module = "engine", feature = "engine" }
            StateTreeModule = { module = "state_tree", feature = "state-tree", plugin = "StateTree" }
            GameplayStateTreeModule = { module = "gameplay_state_tree", feature = "state-tree", plugin = "GameplayStateTree" }
            Niagara = { module = "niagara", feature = "niagara", plugin = "Niagara" }
            "#,
        )
        .unwrap();
        let dir = std::env::temp_dir().join(format!("rusteal-uplugin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let descriptor = dir.join("Rusteal.uplugin");
        std::fs::write(
            &descriptor,
            r#"{ "VersionName": "0.0.0", "Plugins": [ { "Name": "EnhancedInput", "Enabled": true } ] }"#,
        )
        .unwrap();

        update_plugin_dependencies(&config, &descriptor);
        update_plugin_dependencies(&config, &descriptor);

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&descriptor).unwrap()).unwrap();
        let names: Vec<&str> = json["Plugins"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["Name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["EnhancedInput", "GameplayStateTree", "StateTree"]);
        assert_eq!(json["VersionName"], "0.0.0");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
