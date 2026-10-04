// `rusteal package`: the game, built, cooked and packaged for the host
// platform; `rusteal plugin package`: a Rusteal plugin, ready to go into
// another project.
//
// Both build every library they carry with the release profile first. A
// packaged game stages each library it loads (the Build.cs of the Rusteal
// plugin and of each Rusteal plugin lists it as a runtime dependency).

use std::fs;
use std::path::{Path, PathBuf};

use rusteal_codegen::config::{find_plugins, find_uproject};

use crate::build_cmd::{self, HostPlatform};

/// Package the game into `output`, with the `configuration` UE builds
/// (`Development`, `Shipping`).
pub fn run_package(root: &Path, engine: &Path, output: &Path, configuration: &str) {
    eprintln!("rusteal package: the libraries, release profile");
    build_cmd::run_build(root, engine, None, 1, true, None);

    let uproject = find_uproject(root).unwrap_or_else(|| fail(&format!("no .uproject in {}", root.display())));
    let uproject = build_cmd::canonical_no_prefix(&uproject);
    let platform = &HostPlatform::CURRENT;
    let uat = engine.join(platform.uat_script);
    if !uat.exists() {
        fail(&format!("UAT not found at {}", uat.display()));
    }
    fs::create_dir_all(output).unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", output.display())));
    let output = build_cmd::canonical_no_prefix(output);

    eprintln!("rusteal package: BuildCookRun, {configuration}, {}", platform.ubt_platform);
    build_cmd::run_cmd(&[
        uat.to_str().unwrap(),
        "BuildCookRun",
        &format!("-project={}", uproject.display()),
        &format!("-platform={}", platform.ubt_platform),
        &format!("-clientconfig={configuration}"),
        "-build",
        "-cook",
        "-stage",
        "-pak",
        "-archive",
        &format!("-archivedirectory={}", output.display()),
        "-nop4",
        "-unattended",
        "-utf8output",
    ]);
    eprintln!("\nrusteal package: done, the game is in {}", output.display());
}

/// Copy the Rusteal plugin `name` into `output/<Name>`, its library built with
/// the release profile: the plugin's sources (C++ and Rust, the generated
/// code included), content, configuration and binaries for the host
/// platform, without build output.
pub fn run_plugin_package(root: &Path, engine: &Path, name: &str, output: &Path) {
    let plugins = find_plugins(root);
    let Some(plugin) = plugins.iter().find(|p| p.name.eq_ignore_ascii_case(name)) else {
        let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
        fail(&format!(
            "no Rusteal plugin '{name}' in {}/Plugins (Rusteal plugins: {}).",
            root.display(),
            if names.is_empty() { "none".to_string() } else { names.join(", ") }
        ));
    };

    eprintln!("rusteal plugin package: the {} library, release profile", plugin.name);
    build_cmd::run_build(root, engine, None, 1, true, Some(&plugin.name));

    let target = output.join(&plugin.name);
    if target.exists() {
        fs::remove_dir_all(&target)
            .unwrap_or_else(|e| fail(&format!("cannot replace {}: {e}", target.display())));
    }
    let copied = copy_plugin(&plugin.dir, &target);
    eprintln!("\nrusteal plugin package: done, {copied} files in {}", target.display());
    eprintln!(
        "  Copy it into another Rusteal project's Plugins/ (the project must be at Rusteal {}),",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!("  list it in the .uproject and run `rusteal build --all` there.");
}

/// What a plugin package leaves out: build output, and the numbered copies
/// of the library a hot reload loads.
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
        let entries = fs::read_dir(&dir).unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", dir.display())));
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
