// Setup command: extracts embedded UE plugin files into a UE project,
// generates starter config, registers plugins in .uproject, and writes C++ stubs.

use std::fs;
use std::path::Path;

use rusteal_codegen::config::find_uproject;

include!(concat!(env!("OUT_DIR"), "/plugin_files.rs"));

const CSPROJ_PROPS_TEMPLATE: &str = r#"<Project>
  <PropertyGroup>
    <EngineDir>{engine_path}</EngineDir>
  </PropertyGroup>
</Project>
"#;

/// Starter `rusteal.toml`. `{crate}` is replaced at runtime.
const PROJECT_CONFIG_TEMPLATE: &str = r##"# Rusteal project configuration. Versioned with the project.
#
# The layout is fixed by convention:
#   Rust/                                       Cargo workspace (game crate + bindings)
#   Rust/bindings/                              generated bindings crate (versioned)
#   Plugins/Rusteal/Source/Rusteal/Generated/   generated C++ wrappers (versioned)
#   Intermediate/Rusteal/uht/                   reflection JSON (build output)
#
# The engine location is per machine: `rusteal` asks for it once and keeps it
# in its own configuration directory.

[project]
# The cdylib package in Rust/ deployed as the Rusteal library.
crate = "{crate}"

[codegen]
# UE modules to generate bindings for, by feature (see [codegen.modules]).
# `engine` needs `input`, `slate` and `umg`: the runtime plugin links them.
features = ["core", "engine", "input", "slate", "umg"]

[codegen.modules]
# UE package = { module = "rust module", feature = "cargo feature" }
CoreUObject = { module = "core_ue", feature = "core" }
Engine = { module = "engine", feature = "engine" }
PhysicsCore = { module = "physics_core", feature = "physics-core" }
InputCore = { module = "input_core", feature = "input" }
SlateCore = { module = "slate_core", feature = "slate" }
Slate = { module = "slate", feature = "slate" }
UMG = { module = "umg", feature = "umg" }
Niagara = { module = "niagara", feature = "niagara" }
GameplayAbilities = { module = "gameplay_abilities", feature = "gameplay-abilities" }
LevelSequence = { module = "level_sequence", feature = "level-sequence" }
CinematicCamera = { module = "cinematic_camera", feature = "cinematic" }
MovieScene = { module = "movie_scene", feature = "movie" }
MovieSceneTracks = { module = "movie_scene_tracks", feature = "movie" }

[codegen.blocklist]
# Reflected types and functions the generated code cannot handle yet.
classes = [
    "BlueprintTypeConversions",
    "InstancedStaticMeshComponent",
    "InstancedSkinnedMeshComponent",
    "HierarchicalInstancedStaticMeshComponent",
    "VisualLoggerKismetLibrary",
    "PluginBlueprintLibrary",
    "MeshVertexPainterKismetLibrary",
    "FieldNotificationLibrary",
    "WorldPartitionBlueprintLibrary",
    "BlueprintMapLibrary",
    "BlueprintSetLibrary",
    "KismetArrayLibrary",
    "MaterialExpressionDataDrivenShaderPlatformInfoSwitch",
]
structs = [
    "ConstraintInstanceAccessor",
]
functions = [
    # UFUNCTIONs the engine does not export (no *_API on the declaration): the
    # generated wrapper compiles but does not link. Found with UE 5.8.2; keep sorted.
    "AnimMontage.IsValidAdditiveSlot",
    "GameplayStatics.BlueprintSuggestProjectileVelocity",
    "KismetSystemLibrary.GetEnumTopLevelAssetPath",
    "KismetSystemLibrary.GetStructTopLevelAssetPath",
    "KismetSystemLibrary.RaiseScriptError",
    "KismetSystemLibrary.StackTrace",
    "ListView.SetReturnFocusToSelection",
    "ListViewBase.CreateDragDropOperation",
    "MaterialInstanceConstant.K2_GetScalarParameterValue",
    "MaterialInstanceConstant.K2_GetTextureCollectionParameterValue",
    "MaterialInstanceConstant.K2_GetTextureParameterValue",
    "MaterialInstanceConstant.K2_GetVectorParameterValue",
    "ParticleSystem.ContainsEmitterType",
    "PhysicsConstraintComponent.IsProjectionEnabled",
    "PlanarReflection.OnInterpToggle",
    "SceneCapture2D.OnInterpToggle",
    "SoundConcurrency.SetMaxCount",
    "SplineComponent.PopulateFromLegacy",
    "SpotLight.SetInnerConeAngle",
    "SpotLight.SetOuterConeAngle",
    "Texture.Blueprint_GetBuiltTextureSize",
    "Texture.Blueprint_GetMemorySize",
    "Texture.Blueprint_GetTextureSourceDiskAndMemorySize",
    "Texture.Blueprint_GetTextureSourceIdString",
    "Texture.ComputeTextureSourceChannelMinMax",
    "Texture2D.Blueprint_GetSizeX",
    "Texture2D.Blueprint_GetSizeY",
    "WidgetAnimation.UnbindAllFromAnimationFinished",
    "WidgetAnimation.UnbindAllFromAnimationStarted",
    "WidgetAnimationPlayCallbackProxy.CreatePlayAnimationProxyObject",
    "WidgetAnimationPlayCallbackProxy.CreatePlayAnimationTimeRangeProxyObject",
    "WidgetAnimationPlayCallbackProxy.NewPlayAnimationProxyObject",
    "WidgetAnimationPlayCallbackProxy.NewPlayAnimationTimeRangeProxyObject",
]
"##;

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
    let content = PROJECT_CONFIG_TEMPLATE.replace("{crate}", &crate_name);
    fs::write(&config_path, content)
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
