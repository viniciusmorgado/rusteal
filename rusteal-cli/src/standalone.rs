use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rusteal_codegen::config::{LibraryKind, PluginConfig, PluginLayout};

use crate::build_cmd::{self, HostPlatform};
use crate::setup;

const TEXT_EXTENSIONS: &[&str] = &["cpp", "h", "cs", "inl", "txt"];

const SKIPPED_FROM_PLUGIN: &[&str] = &[
    "Rust",
    "rusteal.toml",
    "Binaries",
    "Intermediate",
    "Saved",
    "Config/FilterPlugin.ini",
];

pub const PREBUILT_DIR: &str = "Source/ThirdParty/RustLibraries";

fn fail(message: &str) -> ! {
    eprintln!("Error: {message}");
    std::process::exit(1);
}

pub fn rename(text: &str, prefix: &str) -> String {
    text.replace("Rusteal", &format!("{prefix}Rusteal"))
        .replace("RUSTEAL", &format!("{}RUSTEAL", prefix.to_uppercase()))
}

pub fn rename_lookups(text: &str, prefix: &str) -> String {
    const START: &str = "b\"Rusteal";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(at) = rest.find(START) {
        out.push_str(&rest[..at]);
        let literal = &rest[at..];

        let Some(end) = literal.find("\\0\"") else {
            out.push_str(literal);
            return out;
        };

        let name = &literal[2..end];
        let after = &literal[end + 3..];

        let Some(args) = after.strip_prefix(".as_ptr(),") else {
            out.push_str(&literal[..end + 3]);
            rest = after;
            continue;
        };

        let spaces = args.len() - args.trim_start().len();
        let digits: String = args
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();

        if digits.parse::<usize>().ok() != Some(name.len()) {
            out.push_str(&literal[..end + 3]);
            rest = after;
            continue;
        }

        out.push_str(&format!(
            "b\"{prefix}{name}\\0\".as_ptr(),{}{}",
            &args[..spaces],
            name.len() + prefix.len()
        ));

        rest = &args[spaces + digits.len()..];
    }

    out.push_str(rest);
    out
}

fn is_text(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| TEXT_EXTENSIONS.iter().any(|t| ext == *t))
}

fn write(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", parent.display())));
    }

    fs::write(path, contents)
        .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", path.display())));
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }

    files.sort();
    files
}

pub fn rename_tree(dir: &Path, prefix: &str) {
    for path in files_under(dir) {
        let rel = path
            .strip_prefix(dir)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let renamed = dir.join(rename(&rel, prefix));

        if is_text(&path) {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));

            write(&renamed, rename(&text, prefix).as_bytes());
        } else if renamed != path {
            let bytes = fs::read(&path)
                .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));

            write(&renamed, &bytes);
        } else {
            continue;
        }

        if renamed != path {
            let _ = fs::remove_file(&path);
        }
    }
}

fn rusteal_version() -> u32 {
    let mut parts = env!("CARGO_PKG_VERSION")
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0));

    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let patch = parts.next().unwrap_or(0);
    major * 1_000_000 + minor * 1_000 + patch
}

pub fn vendor_runtime(plugin_dir: &Path, prefix: &str) {
    for (rel, contents) in setup::PLUGIN_FILES {
        let Some(rel) = rel.strip_prefix("Rusteal/") else {
            continue;
        };

        let is_runtime =
            rel.starts_with("Source/Rusteal/") || rel.starts_with("Source/RustealEditor/");

        if !is_runtime {
            continue;
        }

        let dest = plugin_dir.join(rename(rel, prefix));

        if is_text(Path::new(rel)) {
            let text = std::str::from_utf8(contents)
                .unwrap_or_else(|e| fail(&format!("{rel} is not UTF-8: {e}")));

            let mut text = rename(text, prefix);

            if rel == "Source/Rusteal/Rusteal.Build.cs" {
                text = text.replacen(
                    "PrivateDependencyModuleNames.Add(\"Projects\");",
                    &format!(
                        "PrivateDependencyModuleNames.Add(\"Projects\");\n        \
                         PublicDefinitions.Add(\"{upper}RUSTEAL_HOSTS_GAME_LIBRARY=0\");\n        \
                         PublicDefinitions.Add(\"{upper}RUSTEAL_PLUGIN_VERSION={version}\");",
                        upper = prefix.to_uppercase(),
                        version = rusteal_version()
                    ),
                    1,
                );
            }

            write(&dest, text.as_bytes());
        } else {
            write(&dest, contents);
        }
    }

    let generated = plugin_dir
        .join("Source")
        .join(format!("{prefix}Rusteal"))
        .join("Generated");

    let stubs = generated.join(".stubs");
    setup::write_cpp_stubs(&stubs, "");

    for path in files_under(&stubs) {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(&path).unwrap_or_default();
        write(
            &generated.join(rename(&name, prefix)),
            rename(&text, prefix).as_bytes(),
        );
    }

    let _ = fs::remove_dir_all(&stubs);
}

fn copy_plugin_sources(from: &Path, to: &Path) {
    for path in files_under(from) {
        let rel = path.strip_prefix(from).unwrap();
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        let skipped = SKIPPED_FROM_PLUGIN
            .iter()
            .any(|skip| rel_str == *skip || rel_str.starts_with(&format!("{skip}/")));

        if skipped {
            continue;
        }

        let bytes = fs::read(&path)
            .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));

        write(&to.join(rel), &bytes);
    }
}

fn rewrite_descriptor(descriptor: &Path, prefix: &str) {
    let text = fs::read_to_string(descriptor)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", descriptor.display())));

    let mut json: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .unwrap_or_else(|e| fail(&format!("cannot parse {}: {e}", descriptor.display())));

    let object = json
        .as_object_mut()
        .unwrap_or_else(|| fail(&format!("{} is not a JSON object", descriptor.display())));

    let mut modules = vec![
        serde_json::json!({ "Name": format!("{prefix}Rusteal"), "Type": "Editor", "LoadingPhase": "Default" }),
        serde_json::json!({ "Name": format!("{prefix}RustealEditor"), "Type": "Editor", "LoadingPhase": "Default" }),
    ];

    if let Some(existing) = object.get("Modules").and_then(serde_json::Value::as_array) {
        modules.extend(existing.iter().cloned());
    }

    object.insert("Modules".to_string(), serde_json::Value::Array(modules));

    let mut plugins: Vec<serde_json::Value> = object
        .get("Plugins")
        .and_then(serde_json::Value::as_array)
        .map(|plugins| {
            plugins
                .iter()
                .filter(|p| p["Name"] != "Rusteal" && p["Name"] != "RustealGenerator")
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    if !plugins.iter().any(|p| p["Name"] == "EnhancedInput") {
        plugins.push(serde_json::json!({ "Name": "EnhancedInput", "Enabled": true }));
    }

    object.insert("Plugins".to_string(), serde_json::Value::Array(plugins));

    let mut out = serde_json::to_string_pretty(&json).expect("serializable descriptor");
    out.push('\n');
    write(descriptor, out.as_bytes());
}

fn build_rust_library(
    root: &Path,
    plugin: &PluginLayout,
    crate_name: &str,
    bindings_dir: &str,
    prefix: &str,
) -> PathBuf {
    let work = root
        .join("Intermediate/Rusteal/Standalone")
        .join(&plugin.name);
    let workspace = work.join("Rust");
    let _ = fs::remove_dir_all(&workspace);

    for path in files_under(&plugin.rust_workspace()) {
        let rel = path.strip_prefix(plugin.rust_workspace()).unwrap();

        if rel.starts_with("target") {
            continue;
        }

        let bytes = fs::read(&path)
            .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));

        write(&workspace.join(rel), &bytes);
    }

    for path in files_under(&workspace.join(bindings_dir)) {
        if path.extension().is_some_and(|ext| ext == "rs") {
            let text = fs::read_to_string(&path).unwrap_or_default();
            let renamed = rename_lookups(&text, prefix);

            if renamed != text {
                write(&path, renamed.as_bytes());
            }
        }
    }

    let manifest = workspace.join(crate_name).join("Cargo.toml");
    let target_dir = work.join("target");

    eprintln!("  $ cargo rustc --release ({crate_name}, Rusteal's types named {prefix}Rusteal*)");

    let status = Command::new("cargo")
        .arg("rustc")
        .arg("--manifest-path")
        .arg(&manifest)
        .args(["--lib", "--crate-type", "cdylib", "--release"])
        .env("CARGO_TARGET_DIR", &target_dir)
        .status()
        .unwrap_or_else(|e| fail(&format!("cannot run cargo: {e}")));

    if !status.success() {
        fail("the standalone Rust library did not build");
    }

    target_dir
        .join("release")
        .join(HostPlatform::CURRENT.lib_filename(crate_name))
}

pub fn run_standalone_package(root: &Path, engine: &Path, plugin: &PluginLayout, output: &Path) {
    let config = PluginConfig::load(&plugin.dir).unwrap_or_else(|e| fail(&e));

    if config.plugin.kind != LibraryKind::Editor || config.editor.is_some() {
        fail(&format!(
            "{} is not an editor plugin: --standalone packages an editor-only plugin (kind = \
             \"editor\"), which other projects use in the editor and never ship in a game.",
            plugin.name
        ));
    }

    let uat = engine.join(HostPlatform::CURRENT.uat_script);

    if !uat.exists() {
        fail(&format!("UAT not found at {}", uat.display()));
    }

    let prefix = plugin.name.clone();
    let library = &config.libraries(&plugin.name)[0];
    let source = output.join(format!("{}-Source", plugin.name));
    let package = output.join(&plugin.name);

    for dir in [&source, &package] {
        if dir.exists() {
            fs::remove_dir_all(dir)
                .unwrap_or_else(|e| fail(&format!("cannot replace {}: {e}", dir.display())));
        }
    }

    eprintln!(
        "\nrusteal plugin package: {} as a standalone editor plugin",
        plugin.name
    );

    copy_plugin_sources(&plugin.dir, &source);
    rename_tree(&source.join("Source").join(&plugin.name), &prefix);
    vendor_runtime(&source, &prefix);
    rewrite_descriptor(&source.join(format!("{}.uplugin", plugin.name)), &prefix);

    let built = build_rust_library(
        root,
        plugin,
        &library.crate_name,
        library.bindings_dir,
        &prefix,
    );
    let prebuilt = source
        .join(PREBUILT_DIR)
        .join(HostPlatform::CURRENT.ubt_platform)
        .join(HostPlatform::CURRENT.deployed_lib_filename(&format!("rusteal_{}", library.module)));

    let bytes =
        fs::read(&built).unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", built.display())));

    write(&prebuilt, &bytes);

    fs::create_dir_all(&package)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", package.display())));

    let descriptor =
        build_cmd::canonical_no_prefix(&source.join(format!("{}.uplugin", plugin.name)));
    let package = build_cmd::canonical_no_prefix(&package);
    let uat = uat.to_string_lossy();
    let plugin_arg = format!("-Plugin={}", descriptor.display());
    let package_arg = format!("-Package={}", package.display());
    let platform_arg = format!("-TargetPlatforms={}", HostPlatform::CURRENT.ubt_platform);

    let mut args = vec![
        uat.as_ref(),
        "BuildPlugin",
        plugin_arg.as_str(),
        package_arg.as_str(),
        platform_arg.as_str(),
    ];

    if engine.join("Engine/Build/InstalledBuild.txt").exists() {
        args.push("-Rocket");
    }

    build_cmd::run_cmd(&args);

    eprintln!("\nrusteal plugin package: done");
    eprintln!(
        "  {}: the plugin, built, with no dependency on Rusteal",
        package.display()
    );

    eprintln!(
        "  {}: its sources with the Rust library prebuilt, no Rust sources (what Fab takes)",
        source.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rusteal_names_take_the_plugin_prefix_and_lowercase_ones_stay() {
        let text = "class RUSTEAL_API URustealReifiedClass; LoadModule(TEXT(\"RustealEditor\")); \
                    TEXT(\"rusteal_\")";

        assert_eq!(
            rename(text, "LevelTools"),
            "class LEVELTOOLSRUSTEAL_API ULevelToolsRustealReifiedClass; \
             LoadModule(TEXT(\"LevelToolsRustealEditor\")); TEXT(\"rusteal_\")"
        );
    }

    #[test]
    fn lookups_of_rusteal_types_take_the_prefix_and_their_new_length() {
        let text = "get_static_class(b\"RustealEditorSubsystem\\0\".as_ptr(), 22)\n\
                    find_class(b\"Actor\\0\".as_ptr(), 5)";

        assert_eq!(
            rename_lookups(text, "LevelTools"),
            "get_static_class(b\"LevelToolsRustealEditorSubsystem\\0\".as_ptr(), 32)\n\
             find_class(b\"Actor\\0\".as_ptr(), 5)"
        );
    }

    #[test]
    fn the_vendored_runtime_is_renamed_and_hosts_no_game_library() {
        let dir = std::env::temp_dir().join(format!("rusteal-vendor-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        vendor_runtime(&dir, "LevelTools");

        let module = dir.join("Source/LevelToolsRusteal");
        assert!(module.join("LevelToolsRusteal.Build.cs").is_file());
        assert!(
            module
                .join("Public/ULevelToolsRustealReifiedClass.h")
                .is_file()
        );
        assert!(
            module
                .join("Generated/LevelToolsRustealFillFuncTable.cpp")
                .is_file()
        );
        assert!(
            dir.join("Source/LevelToolsRustealEditor/LevelToolsRustealEditor.Build.cs")
                .is_file()
        );
        assert!(!dir.join("Source/ThirdParty").exists());

        let rules = fs::read_to_string(module.join("LevelToolsRusteal.Build.cs")).unwrap();
        assert!(
            rules.contains("public class LevelToolsRusteal : ModuleRules"),
            "{rules}"
        );
        assert!(
            rules.contains("LEVELTOOLSRUSTEAL_HOSTS_GAME_LIBRARY=0"),
            "{rules}"
        );

        let header =
            fs::read_to_string(module.join("Public/ULevelToolsRustealReifiedClass.h")).unwrap();
        assert!(
            header.contains("ULevelToolsRustealReifiedClass.generated.h"),
            "{header}"
        );
        assert!(!header.contains(" URustealReifiedClass"), "{header}");

        fs::remove_dir_all(&dir).unwrap();
    }
}
