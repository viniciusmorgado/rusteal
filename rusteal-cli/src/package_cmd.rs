use std::fs;
use std::path::{Path, PathBuf};

use rusteal_codegen::config::{find_plugins, find_uproject};

use crate::build_cmd::{self, HostPlatform};
use crate::optimizations::{self, Snapshot, Switch};
use crate::platform::{self, Platform};
use crate::static_link;
use crate::xcode_shim;

pub fn target_platform(name: Option<&str>) -> Platform {
    let name = name.unwrap_or(HostPlatform::CURRENT.ubt_platform);
    platform::find(name, &platform::search_dirs()).unwrap_or_else(|e| fail(&e))
}

pub fn run_package(
    root: &Path,
    engine: &Path,
    output: &Path,
    configuration: &str,
    switch: Switch,
    link_statically: bool,
    target: &Platform,
    server: bool,
) {
    let host = HostPlatform::CURRENT.ubt_platform;

    if server && engine.join(INSTALLED_BUILD).is_file() {
        fail(&format!(
            "a dedicated server is built with an engine built from source: {} is an installed \
             build ({INSTALLED_BUILD}), which has no server target. Package the game without \
             --server, or point rusteal at a source build of the engine.",
            engine.display()
        ));
    }

    if !link_statically && (!target.is_host(host) || target.dynamic_library.is_none()) {
        fail(&format!(
            "{} is not this machine's platform ({host}): a game for it carries its Rust code \
             linked in. Package it with --static.",
            target.name
        ));
    }

    eprintln!("rusteal package: the libraries, release profile");
    build_cmd::run_build(root, engine, None, 1, true, None);

    let uproject =
        find_uproject(root).unwrap_or_else(|| fail(&format!("no .uproject in {}", root.display())));

    let uproject = build_cmd::canonical_no_prefix(&uproject);

    if server {
        match ensure_server_target(root, &uproject) {
            Ok(Some(written)) => eprintln!(
                "rusteal package: wrote the dedicated server's target, {}",
                written.display()
            ),
            Ok(None) => {}
            Err(e) => fail(&e),
        }
    }

    let platform = &HostPlatform::CURRENT;
    let uat = engine.join(platform.uat_script);

    if !uat.exists() {
        fail(&format!("UAT not found at {}", uat.display()));
    }

    fs::create_dir_all(output)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", output.display())));

    let output = build_cmd::canonical_no_prefix(output);

    eprintln!(
        "rusteal package: BuildCookRun, {configuration}, {}",
        target.name
    );

    if link_statically {
        let cargo_args = target.cargo_args();

        let build = static_link::Target {
            rust_target: target.rust_target.as_deref(),
            archive_name: &target.static_archive,
            cargo_args: &cargo_args,
            rustflags: &target.rustflags,
            toolchain: target.toolchain.as_deref(),
            env: &target.env,
        };

        let mut artifact = static_link::build(root, &build).unwrap_or_else(|e| fail(&e));

        for lib in &target.system_libraries {
            if !artifact.system_libraries.contains(lib) {
                artifact.system_libraries.push(lib.clone());
            }
        }

        let installed =
            static_link::install(root, &artifact, &target.name).unwrap_or_else(|e| fail(&e));

        eprintln!(
            "rusteal package: the Rust libraries are linked into the game from {}{}",
            installed.display(),
            if artifact.system_libraries.is_empty() {
                String::new()
            } else {
                format!(" (with {})", artifact.system_libraries.join(", "))
            }
        );

        static_link::set_switch(root, true).unwrap_or_else(|e| fail(&e));
    }

    let shim = xcode_shim::for_package().unwrap_or_else(|e| fail(&e));
    let env: Vec<(&str, &Path)> = shim
        .iter()
        .map(|shim| ("DEVELOPER_DIR", shim.developer_dir()))
        .collect();

    let snapshot = Snapshot::take(root);
    optimizations::report("package", root, switch);

    let mut uat_args = vec![
        uat.to_str().unwrap().to_string(),
        "BuildCookRun".to_string(),
        format!("-project={}", uproject.display()),
        format!("-platform={}", target.name),
        format!("-clientconfig={configuration}"),
        "-build".to_string(),
        "-cook".to_string(),
        "-stage".to_string(),
        "-pak".to_string(),
        "-archive".to_string(),
        format!("-archivedirectory={}", output.display()),
        "-nop4".to_string(),
        "-unattended".to_string(),
        "-utf8output".to_string(),
    ];

    if server {
        uat_args.extend(server_uat_args(&target.name, configuration));
    }

    uat_args.extend(target.uat_args.iter().cloned());
    let uat_args: Vec<&str> = uat_args.iter().map(String::as_str).collect();
    let code = build_cmd::run_cmd_code_env(&uat_args, &env);

    snapshot.restore();
    drop(shim);

    if link_statically {
        static_link::set_switch(root, false).unwrap_or_else(|e| fail(&e));
    }

    if code != 0 {
        std::process::exit(code);
    }

    if link_statically {
        let staged = static_link::staged_dynamic_libraries(&output);

        for library in &staged {
            eprintln!(
                "Warning: {} was staged, though the game links the Rust code statically: its \
                 plugin's Build.cs still adds it (see `Rusteal.AddRustLibrary`)",
                library.display()
            );
        }
    }

    eprintln!(
        "\nrusteal package: done, the {} is in {}",
        if server { "dedicated server" } else { "game" },
        output.display()
    );
}

const INSTALLED_BUILD: &str = "Engine/Build/InstalledBuild.txt";

pub fn server_target_text(project: &str) -> String {
    let target = format!("{project}Server");

    format!(
        "using UnrealBuildTool;\n\npublic class {target}Target : TargetRules\n{{\n\
         \tpublic {target}Target(TargetInfo Target) : base(Target)\n\t{{\n\
         \t\tType = TargetType.Server;\n\
         \t\tDefaultBuildSettings = BuildSettingsVersion.Latest;\n\
         \t\tIncludeOrderVersion = EngineIncludeOrderVersion.Latest;\n\
         \t\tExtraModuleNames.Add(\"{project}\");\n\t}}\n}}\n"
    )
}

fn ensure_server_target(root: &Path, uproject: &Path) -> Result<Option<PathBuf>, String> {
    let project = uproject
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("{} has no project name", uproject.display()))?;
    let source = root.join("Source");

    if !source.join(format!("{project}.Target.cs")).is_file() {
        return Err(format!(
            "a dedicated server needs the project's C++ targets, and {} has no {project}.Target.cs",
            source.display()
        ));
    }

    let path = source.join(format!("{project}Server.Target.cs"));

    if path.is_file() {
        return Ok(None);
    }

    fs::write(&path, server_target_text(project))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;

    Ok(Some(path))
}

pub fn server_uat_args(platform: &str, configuration: &str) -> Vec<String> {
    vec![
        "-server".to_string(),
        "-noclient".to_string(),
        format!("-serverplatform={platform}"),
        format!("-serverconfig={configuration}"),
    ]
}

const PLUGIN_FILTER: &[&str] = &[
    "/rusteal.toml",
    "/Rust/...",
    "-/Rust/target/...",
    "/Binaries/.../*rusteal_*",
];

pub fn run_plugin_package(
    root: &Path,
    engine: &Path,
    name: &str,
    output: &Path,
    precompiled: bool,
    standalone: bool,
) {
    let plugins = find_plugins(root);

    let Some(plugin) = plugins.iter().find(|p| p.name.eq_ignore_ascii_case(name)) else {
        let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();

        fail(&format!(
            "no Rusteal plugin '{name}' in {}/Plugins (Rusteal plugins: {}).",
            root.display(),
            if names.is_empty() {
                "none".to_string()
            } else {
                names.join(", ")
            }
        ));
    };

    eprintln!(
        "rusteal plugin package: the {} library, release profile",
        plugin.name
    );

    build_cmd::run_build(root, engine, None, 1, true, Some(&plugin.name));

    if standalone {
        crate::standalone::run_standalone_package(engine, plugin, output);
        return;
    }

    let target = output.join(&plugin.name);

    if target.exists() {
        fs::remove_dir_all(&target)
            .unwrap_or_else(|e| fail(&format!("cannot replace {}: {e}", target.display())));
    }

    if precompiled {
        package_precompiled(root, engine, &plugin.dir, &target);
        return;
    }

    let copied = copy_plugin(&plugin.dir, &target);

    eprintln!(
        "\nrusteal plugin package: done, {copied} files in {}",
        target.display()
    );

    eprintln!(
        "  Copy it into another Rusteal project's Plugins/ (the project must be at Rusteal {}),",
        env!("CARGO_PKG_VERSION")
    );

    eprintln!("  list it in the .uproject and run `rusteal build --all` there.");
}

fn excluded(rel: &Path) -> bool {
    let rel = rel.to_string_lossy().replace('\\', "/");
    let first = rel.split('/').next().unwrap_or_default();

    if matches!(first, "Intermediate" | "Saved" | "DerivedDataCache") {
        return true;
    }

    if rel.starts_with("Rust/target/") || rel == "Rust/target" || rel.contains("/obj/") {
        return true;
    }

    let file = rel.rsplit('/').next().unwrap_or_default();

    rel.starts_with("Binaries/")
        && (file.contains("_hot_") || file.ends_with(".debug") || file.ends_with(".pdb"))
}

fn package_precompiled(root: &Path, engine: &Path, plugin_dir: &Path, target: &Path) {
    let name = plugin_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let uat = engine.join(HostPlatform::CURRENT.uat_script);

    if !uat.exists() {
        fail(&format!("UAT not found at {}", uat.display()));
    }

    let rusteal = root.join("Plugins/Rusteal/Rusteal.uplugin");

    if !rusteal.is_file() {
        fail(&format!(
            "{} not found: run `rusteal setup` first.",
            rusteal.display()
        ));
    }

    ensure_plugin_filter(&plugin_dir.join("Config/FilterPlugin.ini"));

    fs::create_dir_all(target)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", target.display())));

    let descriptor = build_cmd::canonical_no_prefix(&plugin_dir.join(format!("{name}.uplugin")));
    let target = build_cmd::canonical_no_prefix(target);
    let rusteal = build_cmd::canonical_no_prefix(&rusteal);

    eprintln!(
        "\nrusteal plugin package: BuildPlugin, {} binaries for {}",
        name,
        HostPlatform::CURRENT.ubt_platform
    );

    let uat = uat.to_string_lossy();
    let plugin_arg = format!("-Plugin={}", descriptor.display());
    let package_arg = format!("-Package={}", target.display());
    let dependency_arg = format!("-Dependencies={}", rusteal.display());
    let platform_arg = format!("-TargetPlatforms={}", HostPlatform::CURRENT.ubt_platform);

    let mut args = vec![
        uat.as_ref(),
        "BuildPlugin",
        plugin_arg.as_str(),
        package_arg.as_str(),
        dependency_arg.as_str(),
        platform_arg.as_str(),
    ];

    if engine.join("Engine/Build/InstalledBuild.txt").exists() {
        args.push("-Rocket");
    }

    build_cmd::run_cmd(&args);

    eprintln!(
        "\nrusteal plugin package: done, {name} precompiled in {}",
        target.display()
    );

    eprintln!(
        "  Copy it into another Rusteal project's Plugins/ (the project must be at Rusteal {}) \
         and list it in the .uproject: `rusteal build` uses its binaries as they are.",
        env!("CARGO_PKG_VERSION")
    );
}

fn ensure_plugin_filter(path: &Path) {
    let mut text = fs::read_to_string(path).unwrap_or_default();

    if !text.contains("[FilterPlugin]") {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }

        text.push_str("[FilterPlugin]\n");
    }

    let missing: Vec<&str> = PLUGIN_FILTER
        .iter()
        .copied()
        .filter(|rule| !text.lines().any(|line| line.trim() == *rule))
        .collect();

    if missing.is_empty() {
        return;
    }

    if !text.ends_with('\n') {
        text.push('\n');
    }

    for rule in &missing {
        text.push_str(rule);
        text.push('\n');
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", parent.display())));
    }

    fs::write(path, text)
        .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", path.display())));
    eprintln!(
        "  {}: the package takes the Rust library, its sources and rusteal.toml",
        path.display()
    );
}

pub fn run_plugin_merge(into: &Path, from: &[PathBuf]) {
    let name = descriptor_name(into)
        .unwrap_or_else(|| fail(&format!("{} holds no .uplugin", into.display())));

    let version = descriptor_field(&into.join(format!("{name}.uplugin")), "VersionName");
    let mut copied = 0;

    for package in from {
        if descriptor_name(package).as_deref() != Some(name.as_str()) {
            fail(&format!("{} is not a package of {name}", package.display()));
        }

        let other = descriptor_field(&package.join(format!("{name}.uplugin")), "VersionName");

        if other != version {
            fail(&format!(
                "{} is {name} {}, and {} is {name} {}: merge packages of one version",
                package.display(),
                other.unwrap_or_default(),
                into.display(),
                version.clone().unwrap_or_default()
            ));
        }

        copied += merge_platform_files(package, into);
    }

    let platforms: Vec<String> = fs::read_dir(into.join("Binaries"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();

    eprintln!(
        "rusteal plugin merge: {copied} files added to {}; its binaries are for {}",
        into.display(),
        platforms.join(", ")
    );
}

fn descriptor_name(package: &Path) -> Option<String> {
    fs::read_dir(package)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "uplugin"))
        .and_then(|path| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
}

fn descriptor_field(descriptor: &Path, field: &str) -> Option<String> {
    let text = fs::read_to_string(descriptor).ok()?;
    let json: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    json[field].as_str().map(str::to_string)
}

fn merge_platform_files(from: &Path, into: &Path) -> usize {
    let mut copied = 0;
    let mut stack = vec![from.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let rel = path.strip_prefix(from).unwrap();
            let per_platform = rel.starts_with("Binaries") || rel.starts_with("Intermediate/Build");
            let dest = into.join(rel);

            if !per_platform || dest.exists() {
                continue;
            }

            fs::create_dir_all(dest.parent().unwrap())
                .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", dest.display())));

            fs::copy(&path, &dest)
                .unwrap_or_else(|e| fail(&format!("cannot copy {}: {e}", path.display())));

            copied += 1;
        }
    }

    copied
}

fn copy_plugin(from: &Path, to: &Path) -> usize {
    let mut copied = 0;
    let mut stack: Vec<PathBuf> = vec![from.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir)
            .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", dir.display())));

        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(from).unwrap();

            if excluded(rel) {
                continue;
            }

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let dest = to.join(rel);

            fs::create_dir_all(dest.parent().unwrap())
                .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", dest.display())));

            fs::copy(&path, &dest)
                .unwrap_or_else(|e| fail(&format!("cannot copy {}: {e}", path.display())));

            copied += 1;
        }
    }

    copied
}

fn fail(message: &str) -> ! {
    eprintln!("Error: {message}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packages_from_several_platforms_merge_their_binaries() {
        let root = std::env::temp_dir().join(format!("rusteal-merge-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);

        let package = |dir: &str, platform: &str, version: &str| {
            let package = root.join(dir);
            fs::create_dir_all(package.join("Binaries").join(platform)).unwrap();
            fs::create_dir_all(package.join("Intermediate/Build").join(platform)).unwrap();

            fs::write(
                package.join("Inventory.uplugin"),
                format!("{{\"VersionName\": \"{version}\"}}"),
            )
            .unwrap();

            fs::write(
                package.join("Binaries").join(platform).join("lib"),
                platform,
            )
            .unwrap();
            fs::write(
                package.join("Intermediate/Build").join(platform).join("a"),
                "",
            )
            .unwrap();
            fs::write(package.join("rusteal.toml"), platform).unwrap();
            package
        };

        let linux = package("linux", "Linux", "1.0.0");
        let windows = package("windows", "Win64", "1.0.0");
        merge_platform_files(&windows, &linux);

        assert!(linux.join("Binaries/Win64/lib").is_file());
        assert!(linux.join("Intermediate/Build/Win64/a").is_file());
        assert_eq!(
            fs::read_to_string(linux.join("rusteal.toml")).unwrap(),
            "Linux"
        );
        assert_eq!(descriptor_name(&linux).as_deref(), Some("Inventory"));

        assert_eq!(
            descriptor_field(&linux.join("Inventory.uplugin"), "VersionName").as_deref(),
            Some("1.0.0")
        );

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_dedicated_server_gets_a_server_target() {
        let dir =
            std::env::temp_dir().join(format!("rusteal-server-target-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Source")).unwrap();
        let uproject = dir.join("Arena.uproject");

        assert!(
            ensure_server_target(&dir, &uproject)
                .unwrap_err()
                .contains("Arena.Target.cs")
        );

        fs::write(dir.join("Source/Arena.Target.cs"), "").unwrap();
        let written = ensure_server_target(&dir, &uproject).unwrap().unwrap();
        let text = fs::read_to_string(&written).unwrap();

        assert!(written.ends_with("Source/ArenaServer.Target.cs"));
        assert!(text.contains("public class ArenaServerTarget : TargetRules"));
        assert!(text.contains("Type = TargetType.Server;"));
        assert!(text.contains("ExtraModuleNames.Add(\"Arena\");"));
        assert!(ensure_server_target(&dir, &uproject).unwrap().is_none());

        assert_eq!(
            server_uat_args("Linux", "Shipping"),
            [
                "-server",
                "-noclient",
                "-serverplatform=Linux",
                "-serverconfig=Shipping"
            ]
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn packages_leave_build_output_out() {
        for rel in [
            "Intermediate/Build/x.o",
            "Rust/target/release/libinventory.so",
            "Binaries/Linux/librusteal_Inventory_hot_3.so",
            "Binaries/Linux/libUnrealEditor-Inventory.debug",
            "Source/Inventory/obj/x",
        ] {
            assert!(excluded(Path::new(rel)), "{rel} kept");
        }

        for rel in [
            "Inventory.uplugin",
            "rusteal.toml",
            "Source/Inventory/Generated/RustealFuncIds.h",
            "Rust/bindings/src/lib.rs",
            "Rust/inventory/src/lib.rs",
            "Binaries/Linux/librusteal_Inventory.so",
            "Binaries/Linux/libUnrealEditor-Inventory.so",
            "Content/Items/DA_Apple.uasset",
        ] {
            assert!(!excluded(Path::new(rel)), "{rel} left out");
        }
    }
}
