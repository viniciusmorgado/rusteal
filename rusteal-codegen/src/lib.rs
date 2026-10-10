pub mod config;
pub mod context;
pub mod cpp_gen;
pub mod defaults;
pub mod filter;
pub mod naming;
pub mod rust_gen;
pub mod schema;
pub mod type_map;

use std::path::Path;

use crate::config::{
    CodegenConfig, LibraryKind, LibraryOutput, PluginConfig, PluginLayout, ProjectConfig,
    ProjectLayout,
};
use crate::schema::{ClassesFile, EnumsFile, StructsFile};

pub(crate) fn write_if_changed(path: &Path, contents: &str) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|old| old == contents.as_bytes()) {
        return Ok(());
    }

    std::fs::write(path, contents)
}

pub fn run_generate(project_root: &Path) {
    let config = ProjectConfig::load(project_root).unwrap_or_else(|e| {
        eprintln!("rusteal-codegen: {e}");
        std::process::exit(1);
    });

    let layout = ProjectLayout::new(project_root);
    generate_library(&layout.uht_json(), &config.codegen, &layout.game_output());
}

pub fn run_generate_plugin(project_root: &Path, plugin: &PluginLayout) {
    let config = PluginConfig::load(&plugin.dir).unwrap_or_else(|e| {
        eprintln!("rusteal-codegen: {e}");
        std::process::exit(1);
    });

    let layout = ProjectLayout::new(project_root);
    eprintln!("rusteal-codegen: plugin {}", plugin.name);

    generate_library(
        &layout.uht_json(),
        &config.codegen,
        &plugin.output(config.plugin.kind),
    );
}

pub fn generate_library(uht_input: &Path, codegen: &CodegenConfig, output: &LibraryOutput) {
    let codegen = &codegen.for_library(output.kind);
    let classes_path = uht_input.join("rusteal_classes.json");
    let structs_path = uht_input.join("rusteal_structs.json");
    let enums_path = uht_input.join("rusteal_enums.json");

    let rust_out = output.bindings_crate.clone();
    let rust_src = rust_out.join("src");
    let cpp_out = output.cpp_generated.clone();

    eprintln!("rusteal-codegen: loading JSON...");

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

    eprintln!("rusteal-codegen: filtering...");
    let blocklist = with_unlisted_classes(&ctx, codegen);
    filter::apply_filters(&mut ctx, &blocklist, output.kind == LibraryKind::Editor);

    eprintln!("rusteal-codegen: building function table...");
    build_func_table(&mut ctx);
    eprintln!("  {} functions in func_table", ctx.func_table.len());

    eprintln!("rusteal-codegen: generating Rust code...");
    rust_gen::generate(&ctx, &rust_src);
    rust_gen::manual::write_manual_module(&rust_src);
    rust_gen::prelude::write_prelude(&rust_src, &ctx);
    rust_gen::cargo_toml::write_crate_files(&rust_out, &ctx, codegen);

    eprintln!("rusteal-codegen: generating C++ code...");
    cpp_gen::generate(&ctx, &cpp_out);

    generate_module_deps(codegen, &output.host_module, &cpp_out);
    update_plugin_dependencies(codegen, &output.plugin_descriptor);

    eprintln!("rusteal-codegen: verifying output...");
    verify_output(&ctx, &rust_src, &cpp_out);

    eprintln!("rusteal-codegen: done!");
}

fn with_unlisted_classes(
    ctx: &context::CodegenContext,
    codegen: &CodegenConfig,
) -> config::Blocklist {
    use std::collections::HashSet;

    let mut blocklist = codegen.blocklist.clone();

    for (package, mapping) in &codegen.modules {
        let Some(listed) = &mapping.classes else {
            continue;
        };

        let mut wanted: HashSet<&str> = HashSet::new();

        for name in listed {
            let mut current = Some(name.as_str());

            while let Some(class) = current.and_then(|n| ctx.classes.get(n)) {
                if &class.package != package || !wanted.insert(class.name.as_str()) {
                    break;
                }

                current = class.super_class.as_deref();
            }
        }

        for class in ctx
            .module_classes
            .get(&mapping.module)
            .into_iter()
            .flatten()
        {
            if !wanted.contains(class.name.as_str()) {
                blocklist.classes.push(class.name.clone());
            }
        }
    }

    blocklist
}

fn generate_module_deps(config: &CodegenConfig, host_module: &str, cpp_out: &Path) {
    use std::collections::BTreeSet;

    let enabled_features: std::collections::HashSet<&str> =
        config.features.iter().map(|s| s.as_str()).collect();

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

    write_if_changed(&path, &content)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));

    eprintln!(
        "  module_deps.txt: {:?}",
        ue_modules.iter().collect::<Vec<_>>()
    );
}

fn update_plugin_dependencies(config: &CodegenConfig, descriptor: &Path) {
    let Ok(text) = std::fs::read_to_string(descriptor) else {
        return;
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

fn verify_output(ctx: &context::CodegenContext, rust_out: &Path, cpp_out: &Path) {
    let mut errors: Vec<String> = Vec::new();

    for (i, entry) in ctx.func_table.iter().enumerate() {
        if entry.func_id != i as u32 {
            errors.push(format!(
                "FuncId gap: expected {} for module '{}' {}.{}, got {} (total functions: {})",
                i,
                entry.module_name,
                entry.class_name,
                entry.func_name,
                entry.func_id,
                ctx.func_table.len()
            ));

            break;
        }
    }

    let rust_required = ["lib.rs", "func_ids.rs"];

    for name in &rust_required {
        let path = rust_out.join(name);

        match std::fs::metadata(&path) {
            Ok(m) if m.len() == 0 => errors.push(format!("Rust output empty: {}", path.display())),
            Err(_) => errors.push(format!("Rust output missing: {}", path.display())),
            _ => {}
        }
    }

    for module_name in ctx.enabled_modules.iter() {
        let mod_path = rust_out.join(module_name).join("mod.rs");

        if !mod_path.exists() {
            errors.push(format!("Module mod.rs missing: {}", mod_path.display()));
        }
    }

    let cpp_required = ["RustealFuncIds.h", "RustealFillFuncTable.cpp"];

    for name in &cpp_required {
        let path = cpp_out.join(name);

        match std::fs::metadata(&path) {
            Ok(m) if m.len() == 0 => errors.push(format!("C++ output empty: {}", path.display())),
            Err(_) => errors.push(format!("C++ output missing: {}", path.display())),
            _ => {}
        }
    }

    let func_count = ctx.func_table.len();
    let module_count = ctx.enabled_modules.len();
    let class_count: usize = ctx.module_classes.values().map(|v| v.len()).sum();
    let struct_count: usize = ctx.module_structs.values().map(|v| v.len()).sum();
    let enum_count: usize = ctx.module_enums.values().map(|v| v.len()).sum();

    if errors.is_empty() {
        eprintln!(
            "  OK: {module_count} modules, {class_count} classes, {struct_count} structs, {enum_count} enums, {func_count} functions"
        );
    } else {
        eprintln!("  Verification FAILED:");

        for e in &errors {
            eprintln!("    - {e}");
        }

        std::process::exit(1);
    }
}

fn build_func_table(ctx: &mut context::CodegenContext) {
    let mut entries = Vec::new();

    for (module_name, classes) in &ctx.module_classes {
        for class in classes {
            if class.super_class.as_deref() == Some("Interface") {
                continue;
            }

            for func in &class.funcs {
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
                    func_id: 0,
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

    entries.sort_by(|a, b| {
        a.module_name
            .cmp(&b.module_name)
            .then_with(|| a.class_name.cmp(&b.class_name))
            .then_with(|| a.func_name.cmp(&b.func_name))
    });

    for (i, entry) in entries.iter_mut().enumerate() {
        entry.func_id = i as u32;
    }

    ctx.func_table = entries;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listed_classes_keep_their_ancestors_only() {
        let class = |name: &str, package: &str, parent: &str| -> schema::ClassInfo {
            serde_json::from_value(serde_json::json!({
                "name": name, "cpp_name": format!("U{name}"), "package": package,
                "header": "", "class_flags": 0, "super": parent,
            }))
            .unwrap()
        };

        let codegen: CodegenConfig = toml::from_str(
            r#"
            features = ["core", "editor"]
            [modules]
            CoreUObject = { module = "core_ue", feature = "core" }
            UnrealEd = { module = "unreal_ed", feature = "editor", classes = ["EditorActorSubsystem"] }
            "#,
        )
        .unwrap();

        let ctx = context::CodegenContext::new(
            vec![
                class("Object", "CoreUObject", ""),
                class("EditorSubsystemBase", "UnrealEd", "Object"),
                class("EditorActorSubsystem", "UnrealEd", "EditorSubsystemBase"),
                class("Factory", "UnrealEd", "Object"),
            ],
            vec![],
            vec![],
            &codegen,
        );

        let blocked = with_unlisted_classes(&ctx, &codegen).classes;
        assert_eq!(blocked, ["Factory"]);
    }

    #[test]
    fn unchanged_output_keeps_its_timestamp() {
        let dir = std::env::temp_dir().join(format!("rusteal-write-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.rs");
        write_if_changed(&path, "a").unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);

        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(old)
            .unwrap();

        write_if_changed(&path, "a").unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().modified().unwrap(), old);
        write_if_changed(&path, "b").unwrap();
        assert_ne!(std::fs::metadata(&path).unwrap().modified().unwrap(), old);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "b");
        std::fs::remove_dir_all(&dir).unwrap();
    }

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
