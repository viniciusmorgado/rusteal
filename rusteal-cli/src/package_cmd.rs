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
) {
    let host = HostPlatform::CURRENT.ubt_platform;

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
        "\nrusteal package: done, the game is in {}",
        output.display()
    );
}

pub fn run_plugin_package(root: &Path, engine: &Path, name: &str, output: &Path) {
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

    let target = output.join(&plugin.name);

    if target.exists() {
        fs::remove_dir_all(&target)
            .unwrap_or_else(|e| fail(&format!("cannot replace {}: {e}", target.display())));
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
