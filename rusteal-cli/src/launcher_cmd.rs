use std::fs;
use std::path::{Path, PathBuf};

use crate::build_cmd::{self, HostPlatform};

include!(concat!(env!("OUT_DIR"), "/launcher_files.rs"));

const LAUNCHER: &str = "RustealLauncher";

fn fail(message: &str) -> ! {
    eprintln!("Error: {message}");
    std::process::exit(1);
}

pub fn installed_dir(engine: &Path) -> PathBuf {
    engine
        .join("Engine")
        .join("Plugins")
        .join("Marketplace")
        .join(LAUNCHER)
}

fn write_sources(dir: &Path) {
    for (rel, contents) in LAUNCHER_FILES {
        let path = dir.join(rel);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", parent.display())));
        }

        fs::write(&path, contents)
            .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", path.display())));
    }
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", to.display())));

    for entry in
        fs::read_dir(from).unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", from.display())))
    {
        let entry = entry.unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", from.display())));
        let target = to.join(entry.file_name());

        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target)
                .unwrap_or_else(|e| fail(&format!("cannot copy to {}: {e}", target.display())));
        }
    }
}

pub fn run_install(engine: &Path) {
    let uat = engine.join(HostPlatform::CURRENT.uat_script);

    if !uat.exists() {
        fail(&format!("UAT not found at {}", uat.display()));
    }

    let work = std::env::temp_dir().join(format!("rusteal-launcher-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    let source = work.join(LAUNCHER);
    let package = work.join("Package");
    write_sources(&work);

    eprintln!(
        "rusteal launcher install: building the launcher for {} with {}",
        HostPlatform::CURRENT.ubt_platform,
        engine.display()
    );

    let descriptor = source.join(format!("{LAUNCHER}.uplugin"));
    let plugin_arg = format!("-Plugin={}", descriptor.display());
    let package_arg = format!("-Package={}", package.display());
    let platform_arg = format!("-TargetPlatforms={}", HostPlatform::CURRENT.ubt_platform);
    let uat_str = uat.to_string_lossy();

    let mut args = vec![
        uat_str.as_ref(),
        "BuildPlugin",
        plugin_arg.as_str(),
        package_arg.as_str(),
        platform_arg.as_str(),
    ];

    if engine.join("Engine/Build/InstalledBuild.txt").exists() {
        args.push("-Rocket");
    }

    let code = build_cmd::run_cmd_code(&args);

    if code != 0 {
        let _ = fs::remove_dir_all(&work);
        eprintln!("Error: BuildPlugin failed; the launcher was not installed.");
        std::process::exit(code);
    }

    let installed = installed_dir(engine);

    if installed.exists() {
        fs::remove_dir_all(&installed).unwrap_or_else(|e| {
            fail(&format!(
                "cannot replace {}: {e}. The engine directory must be writable.",
                installed.display()
            ))
        });
    }

    copy_tree(&package, &installed);
    let _ = fs::remove_dir_all(&work);

    eprintln!(
        "rusteal launcher install: installed in {}",
        installed.display()
    );
    eprintln!("  The New Project dialog now lists the Rust templates under RUST.");
}

pub fn run_remove(engine: &Path) {
    let installed = installed_dir(engine);

    if !installed.exists() {
        eprintln!(
            "rusteal launcher remove: no launcher in {}",
            installed.display()
        );

        return;
    }

    fs::remove_dir_all(&installed)
        .unwrap_or_else(|e| fail(&format!("cannot remove {}: {e}", installed.display())));

    eprintln!("rusteal launcher remove: removed {}", installed.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_launcher_is_embedded_with_its_templates() {
        let has = |rel: &str| LAUNCHER_FILES.iter().any(|(path, _)| *path == rel);

        assert!(has("RustealLauncher/RustealLauncher.uplugin"));
        assert!(has("RustealLauncher/Config/FilterPlugin.ini"));
        assert!(has("RustealLauncher/Templates/TemplateCategories.ini"));
        assert!(has(
            "RustealLauncher/Source/RustealLauncher/RustealLauncher.Build.cs"
        ));
    }

    #[test]
    fn every_rust_template_names_a_template_the_cli_has() {
        let mut stubs = 0;

        for (path, contents) in LAUNCHER_FILES {
            if !path.ends_with("/RustealLauncher.json") {
                continue;
            }

            let marker: serde_json::Value = serde_json::from_slice(contents).unwrap();
            assert_eq!(marker["stub"], true, "{path}");

            let template = marker["template"].as_str().unwrap();
            let variant = marker["variant"].as_str();

            crate::templates::select(template, variant).unwrap_or_else(|e| panic!("{path}: {e}"));

            stubs += 1;
        }

        let variants: usize = crate::templates::available()
            .iter()
            .map(|template| template.variants.len())
            .sum();

        assert_eq!(stubs, variants, "one Rust template per template variant");
    }
}
