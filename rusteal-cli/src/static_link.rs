use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rusteal_codegen::config::{
    LibraryKind, PluginConfig, ProjectConfig, find_plugins, find_uproject,
};

use crate::project_version::{self, Pins};

pub const SWITCH: &str = "Plugins/Rusteal/Source/Rusteal/Generated/static_link.txt";
pub const THIRD_PARTY: &str = "Plugins/Rusteal/Source/ThirdParty/RustealStatic/lib";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticModule {
    pub name: String,
    pub package: String,
    pub manifest: PathBuf,
    pub features: Vec<String>,
}

impl StaticModule {
    pub fn lib_name(&self) -> String {
        self.package.replace('-', "_")
    }
}

pub struct Artifact {
    pub archive: PathBuf,
    pub system_libraries: Vec<String>,
}

pub struct Target<'a> {
    pub rust_target: Option<&'a str>,
    pub archive_name: &'a str,
    pub cargo_args: &'a [String],
    pub rustflags: &'a [String],
    pub toolchain: Option<&'a str>,
    pub env: &'a BTreeMap<String, String>,
}

pub fn modules(root: &Path) -> Result<Vec<StaticModule>, String> {
    let config = ProjectConfig::load(root)?;

    let project = find_uproject(root)
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .ok_or_else(|| format!("no .uproject in {}", root.display()))?;

    let mut modules = vec![StaticModule {
        name: project,
        package: config.project.crate_name.clone(),
        manifest: member_manifest(&root.join("Rust"), &config.project.crate_name),
        features: config.project.features.clone(),
    }];

    for plugin in find_plugins(root) {
        let plugin_config = PluginConfig::load(&plugin.dir)?;

        for library in plugin_config.libraries(&plugin.name) {
            if library.kind == LibraryKind::Editor {
                continue;
            }

            modules.push(StaticModule {
                name: library.module,
                manifest: member_manifest(&plugin.rust_workspace(), &library.crate_name),
                package: library.crate_name,
                features: library.features,
            });
        }
    }

    Ok(modules)
}

fn member_manifest(workspace: &Path, crate_name: &str) -> PathBuf {
    workspace.join(crate_name).join("Cargo.toml")
}

pub fn dir(root: &Path) -> PathBuf {
    root.join("Intermediate/Rusteal/Static")
}

fn runtime_dependency(pins: &Pins) -> String {
    match pins {
        Pins::Exact(version) => {
            format!(
                "rusteal-runtime = {{ version = \"={version}\", features = [\"static-link\"] }}"
            )
        }
        Pins::Checkout(dir) => format!(
            "rusteal-runtime = {{ path = {:?}, features = [\"static-link\"] }}",
            dir.join("rusteal-runtime")
                .to_string_lossy()
                .replace('\\', "/")
        ),
    }
}

pub fn render_workspace(modules: &[StaticModule], pins: &Pins) -> String {
    let mut out = String::from(
        "# Written by `rusteal package --static` on every run.\n\n\
         [workspace]\nmembers = [\"archive\", \"manifest\"]\nresolver = \"3\"\n\n\
         [workspace.dependencies]\n",
    );

    out.push_str(&runtime_dependency(pins));
    out.push('\n');

    for module in modules {
        let crate_dir = module.manifest.parent().unwrap_or(Path::new("."));

        out.push_str(&format!(
            "{} = {{ path = {:?}, features = [{}] }}\n",
            module.package,
            crate_dir.to_string_lossy().replace('\\', "/"),
            module
                .features
                .iter()
                .map(|f| format!("{f:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    out.push_str(
        "\n[profile.release]\nlto = \"fat\"\ncodegen-units = 1\n\n\
         [profile.dev]\ndebug = false\n",
    );

    out
}

pub fn engine_allocator(root: &Path) -> bool {
    std::fs::read_to_string(root.join("rusteal.toml"))
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
        .and_then(|config| config.get("static")?.get("engine_allocator")?.as_bool())
        .unwrap_or(true)
}

fn render_member(
    package: &str,
    lib: bool,
    modules: &[StaticModule],
    engine_allocator: bool,
) -> String {
    let mut out = format!(
        "[package]\nname = \"{package}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n"
    );

    if lib {
        out.push_str("[lib]\ncrate-type = [\"staticlib\"]\n\n");
    }

    if engine_allocator {
        out.push_str(
            "[dependencies]\nrusteal-runtime = { workspace = true, features = [\"engine-allocator\"] }\n",
        );
    } else {
        out.push_str("[dependencies]\nrusteal-runtime = { workspace = true }\n");
    }

    for module in modules {
        out.push_str(&format!("{} = {{ workspace = true }}\n", module.package));
    }

    out
}

pub fn render_archive_lib(
    modules: &[StaticModule],
    owners: &BTreeMap<String, Vec<String>>,
    expected: &BTreeMap<String, Vec<String>>,
    engine_allocator: bool,
) -> String {
    let mut out = String::new();

    if engine_allocator {
        out.push_str(
            "#[global_allocator]\nstatic ALLOCATOR: rusteal_runtime::EngineAllocator = rusteal_runtime::EngineAllocator;\n\n",
        );
    }

    out.push_str("rusteal_runtime::static_modules! {\n");

    for module in modules {
        let quote = |items: &[String]| {
            items
                .iter()
                .map(|item| format!("{item:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let crates = owners.get(&module.name).cloned().unwrap_or_default();
        let entries = expected.get(&module.name).cloned().unwrap_or_default();

        out.push_str(&format!(
            "    {:?} => {} {{\n        crates: [{}],\n        expected: [{}],\n    }},\n",
            module.name,
            module.lib_name(),
            quote(&crates),
            quote(&entries)
        ));
    }

    out.push_str("}\n");
    out
}

fn render_manifest_main(modules: &[StaticModule]) -> String {
    let linked: Vec<String> = modules
        .iter()
        .map(|m| format!("&{}::__RUSTEAL_MODULE", m.lib_name()))
        .collect();

    format!(
        "fn main() {{\n    let _linked = [{}];\n\n    for entry in rusteal_runtime::manifest() {{\n        println!(\"{{entry}}\");\n    }}\n}}\n",
        linked.join(", ")
    )
}

pub fn ownership(
    metadata: &serde_json::Value,
    modules: &[StaticModule],
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata has no packages")?;

    let local: BTreeMap<&str, (&str, String)> = packages
        .iter()
        .filter(|p| p["source"].is_null())
        .filter_map(|p| {
            let id = p["id"].as_str()?;
            let name = p["name"].as_str()?;

            let lib = p["targets"].as_array()?.iter().find_map(|t| {
                let kinds = t["kind"].as_array()?;

                kinds
                    .iter()
                    .any(|k| matches!(k.as_str(), Some("lib" | "rlib" | "cdylib" | "staticlib")))
                    .then(|| t["name"].as_str().unwrap_or(name).replace('-', "_"))
            })?;

            Some((id, (name, lib)))
        })
        .collect();

    let nodes: BTreeMap<&str, Vec<&str>> = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or("cargo metadata has no resolve graph")?
        .iter()
        .filter_map(|node| {
            let deps = node["deps"]
                .as_array()?
                .iter()
                .filter_map(|d| d["pkg"].as_str())
                .collect();

            Some((node["id"].as_str()?, deps))
        })
        .collect();

    let roots: BTreeMap<&str, &str> = modules
        .iter()
        .filter_map(|module| {
            let id = local
                .iter()
                .find(|(_, (name, _))| *name == module.package)
                .map(|(id, _)| *id)?;

            Some((id, module.name.as_str()))
        })
        .collect();

    let ignored = |name: &str| name.starts_with("rusteal-") || name.ends_with("-bindings");
    let mut owner_of: BTreeMap<String, &str> = BTreeMap::new();
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for module in modules {
        let Some((&root, _)) = roots.iter().find(|(_, name)| **name == module.name) else {
            return Err(format!(
                "{}'s crate {} is not in the static build",
                module.name, module.package
            ));
        };

        let mut stack = vec![root];
        let mut seen = BTreeSet::new();

        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }

            let Some((name, lib)) = local.get(id) else {
                continue;
            };

            if ignored(name) || (id != root && roots.contains_key(id)) {
                continue;
            }

            if let Some(other) = owner_of.get(lib)
                && *other != module.name
            {
                return Err(format!(
                    "the crate {name} is used by both {other} and {}: a crate with Rust types \
                     belongs to one library",
                    module.name
                ));
            }

            owner_of.insert(lib.clone(), &module.name);
            owners
                .entry(module.name.clone())
                .or_default()
                .push(lib.clone());

            if let Some(deps) = nodes.get(id) {
                stack.extend(deps.iter().copied());
            }
        }
    }

    for crates in owners.values_mut() {
        crates.sort();
        crates.dedup();
    }

    Ok(owners)
}

pub fn group_expected(
    entries: &[String],
    owners: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    let mut expected: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for entry in entries {
        let krate = entry.split(':').nth(1).unwrap_or_default();

        if let Some((module, _)) = owners
            .iter()
            .find(|(_, crates)| crates.iter().any(|c| c == krate))
        {
            expected
                .entry(module.clone())
                .or_default()
                .push(entry.clone());
        }
    }

    expected
}

pub fn parse_native_libs(output: &str) -> Vec<String> {
    let Some(line) = output
        .lines()
        .rev()
        .find_map(|line| line.split_once("native-static-libs:").map(|(_, libs)| libs))
    else {
        return Vec::new();
    };

    let mut libs = Vec::new();
    let mut tokens = line.split_whitespace();

    while let Some(token) = tokens.next() {
        let lib = if let Some(name) = token.strip_prefix("-l") {
            name.to_string()
        } else if token == "-framework" {
            match tokens.next() {
                Some(framework) => format!("framework:{framework}"),
                None => continue,
            }
        } else if token.ends_with(".lib") {
            token.to_string()
        } else {
            continue;
        };

        if !libs.contains(&lib) {
            libs.push(lib);
        }
    }

    libs
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }

    if fs::read_to_string(path).is_ok_and(|old| old == contents) {
        return Ok(());
    }

    fs::write(path, contents).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn cargo(target: &Target, toolchain_args: bool) -> Command {
    let mut command = Command::new("cargo");

    if toolchain_args && let Some(toolchain) = target.toolchain {
        command.arg(format!("+{toolchain}"));
    }

    command
}

pub fn build(root: &Path, target: &Target) -> Result<Artifact, String> {
    let modules = modules(root)?;
    let dir = dir(root);
    let pins = project_version::read_pins(root)?;

    for module in &modules {
        crate::migrate::crate_types(&module.manifest);
    }

    eprintln!(
        "rusteal package --static: aggregating {} into one static library",
        modules
            .iter()
            .map(|m| format!("{} ({})", m.name, m.package))
            .collect::<Vec<_>>()
            .join(", ")
    );

    write(&dir.join("Cargo.toml"), &render_workspace(&modules, &pins))?;
    let allocator = engine_allocator(root);

    write(
        &dir.join("archive/Cargo.toml"),
        &render_member("rusteal-static", true, &modules, allocator),
    )?;

    write(
        &dir.join("manifest/Cargo.toml"),
        &render_member("rusteal-static-manifest", false, &modules, false),
    )?;

    if allocator {
        eprintln!("  Rust allocates through the engine's FMemory ([static] engine_allocator)");
    }

    write(
        &dir.join("manifest/src/main.rs"),
        &render_manifest_main(&modules),
    )?;

    let lock = dir.join("Cargo.lock");
    let game_lock = root.join("Rust/Cargo.lock");

    if !lock.exists() && game_lock.exists() {
        fs::copy(&game_lock, &lock)
            .map_err(|e| format!("cannot copy {}: {e}", game_lock.display()))?;
    }

    let empty = BTreeMap::new();

    if !dir.join("archive/src/lib.rs").exists() {
        write(
            &dir.join("archive/src/lib.rs"),
            &render_archive_lib(&modules, &empty, &empty, allocator),
        )?;
    }

    let manifest_path = dir.join("Cargo.toml");
    let target_dir = dir.join("target");

    let metadata = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest_path)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;

    if !metadata.status.success() {
        return Err("cargo metadata failed on the static build".to_string());
    }

    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout)
        .map_err(|e| format!("cannot read cargo metadata: {e}"))?;

    let owners = ownership(&metadata, &modules)?;

    for module in &modules {
        eprintln!(
            "  {}: crates {}",
            module.name,
            owners
                .get(&module.name)
                .map(|c| c.join(", "))
                .unwrap_or_default()
        );
    }

    eprintln!("  listing the registrations every library must link");

    let listed = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "-p",
            "rusteal-static-manifest",
            "--manifest-path",
        ])
        .arg(&manifest_path)
        .arg("--target-dir")
        .arg(&target_dir)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("cannot run the registration manifest: {e}"))?;

    if !listed.status.success() {
        return Err("the registration manifest did not build or run".to_string());
    }

    let entries: Vec<String> = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .map(str::to_string)
        .filter(|line| !line.is_empty())
        .collect();

    let expected = group_expected(&entries, &owners);

    for module in &modules {
        eprintln!(
            "  {}: {} registrations",
            module.name,
            expected.get(&module.name).map_or(0, Vec::len)
        );
    }

    write(
        &dir.join("archive/src/lib.rs"),
        &render_archive_lib(&modules, &owners, &expected, allocator),
    )?;

    eprintln!("  building the static library (release, fat LTO)");

    let mut command = cargo(target, true);

    command
        .args([
            "rustc",
            "-p",
            "rusteal-static",
            "--lib",
            "--release",
            "--manifest-path",
        ])
        .arg(&manifest_path)
        .arg("--target-dir")
        .arg(&target_dir);

    if let Some(triple) = target.rust_target {
        command.args(["--target", triple]);
    }

    command.args(target.cargo_args);
    command.args(["--", "--print=native-static-libs"]);

    if !target.rustflags.is_empty() {
        command.env("RUSTFLAGS", target.rustflags.join(" "));
    }

    command.envs(target.env);

    let output = command
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("cannot run cargo: {e}"))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    eprint!("{stderr}");

    if !output.status.success() {
        return Err("the static library did not build".to_string());
    }

    let profile_dir = match target.rust_target {
        Some(triple) => target_dir.join(target_name(triple)).join("release"),
        None => target_dir.join("release"),
    };

    let archive = profile_dir.join(target.archive_name);

    if !archive.is_file() {
        return Err(format!("no static library at {}", archive.display()));
    }

    Ok(Artifact {
        archive,
        system_libraries: parse_native_libs(&stderr),
    })
}

fn target_name(triple: &str) -> String {
    Path::new(triple)
        .file_stem()
        .filter(|_| triple.ends_with(".json"))
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| triple.to_string())
}

pub fn install(root: &Path, artifact: &Artifact, ue_platform: &str) -> Result<PathBuf, String> {
    let dest_dir = root.join(THIRD_PARTY).join(ue_platform);

    fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("cannot create {}: {e}", dest_dir.display()))?;

    let name = artifact
        .archive
        .file_name()
        .ok_or("the static library has no file name")?
        .to_string_lossy()
        .into_owned();

    let dest = dest_dir.join(&name);

    fs::copy(&artifact.archive, &dest)
        .map_err(|e| format!("cannot copy {}: {e}", artifact.archive.display()))?;

    let mut link = format!("archive={name}\n");

    for lib in &artifact.system_libraries {
        link.push_str(&format!("system={lib}\n"));
    }

    write(&dest_dir.join("link.txt"), &link)?;
    Ok(dest)
}

pub fn set_switch(root: &Path, on: bool) -> Result<(), String> {
    write(&root.join(SWITCH), if on { "1\n" } else { "0\n" })
}

pub fn staged_dynamic_libraries(output: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![output.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_lowercase();

            let dynamic = [".so", ".dll", ".dylib"]
                .iter()
                .any(|ext| name.ends_with(ext));

            if dynamic && name.contains("rusteal") && !name.contains("unrealeditor") {
                found.push(path);
            }
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(name: &str, package: &str) -> StaticModule {
        StaticModule {
            name: name.to_string(),
            package: package.to_string(),
            manifest: PathBuf::from(format!("/p/{package}/Cargo.toml")),
            features: vec!["fast".to_string()],
        }
    }

    #[test]
    fn the_workspace_links_every_library_with_the_project_runtime() {
        let modules = [module("ModLab", "mod-lab"), module("Alpha", "alpha")];
        let exact = render_workspace(
            &modules,
            &Pins::Exact(project_version::Version::parse("0.9.0").unwrap()),
        );

        assert!(
            exact.contains(
                "rusteal-runtime = { version = \"=0.9.0\", features = [\"static-link\"] }"
            )
        );
        assert!(exact.contains("mod-lab = { path = \"/p/mod-lab\", features = [\"fast\"] }"));
        assert!(exact.contains("alpha = { path = \"/p/alpha\""));
        assert!(exact.contains("lto = \"fat\""));

        let checkout = render_workspace(&modules, &Pins::Checkout(PathBuf::from("/src/rusteal")));
        assert!(checkout.contains("rusteal-runtime = { path = \"/src/rusteal/rusteal-runtime\""));
    }

    #[test]
    fn the_archive_names_each_library_and_what_it_owns() {
        let modules = [module("ModLab", "mod-lab")];
        let owners = BTreeMap::from([("ModLab".to_string(), vec!["mod_lab".to_string()])]);

        let expected =
            BTreeMap::from([("ModLab".to_string(), vec!["class:mod_lab:Item".to_string()])]);

        let lib = render_archive_lib(&modules, &owners, &expected, false);
        assert!(!lib.contains("global_allocator"));
        assert!(lib.contains("\"ModLab\" => mod_lab {"));
        assert!(lib.contains("crates: [\"mod_lab\"]"));
        assert!(lib.contains("expected: [\"class:mod_lab:Item\"]"));

        let lib = render_archive_lib(&modules, &owners, &expected, true);
        assert!(lib.starts_with("#[global_allocator]"));

        let archive = render_member("rusteal-static", true, &modules, true);
        assert!(archive.contains("features = [\"engine-allocator\"]"));
        assert!(!render_member("m", false, &modules, false).contains("engine-allocator"));
    }

    fn metadata() -> serde_json::Value {
        let package = |id: &str, name: &str, source: Option<&str>| {
            serde_json::json!({
                "id": id, "name": name, "source": source,
                "targets": [{ "kind": ["cdylib", "rlib"], "name": name }],
            })
        };

        let node = |id: &str, deps: &[&str]| {
            serde_json::json!({
                "id": id,
                "deps": deps.iter().map(|d| serde_json::json!({ "pkg": d })).collect::<Vec<_>>(),
            })
        };

        serde_json::json!({
            "packages": [
                package("game", "my-game", None),
                package("helpers", "game-helpers", None),
                package("game-b", "my-game-bindings", None),
                package("plugin", "alpha", None),
                package("plugin-b", "alpha-bindings", None),
                package("runtime", "rusteal-runtime", None),
                package("glam", "glam", Some("registry+https://github.com/rust-lang/crates.io-index")),
            ],
            "resolve": { "nodes": [
                node("game", &["helpers", "game-b", "runtime", "glam"]),
                node("helpers", &["runtime"]),
                node("plugin", &["plugin-b", "runtime"]),
                node("game-b", &[]), node("plugin-b", &[]), node("runtime", &[]), node("glam", &[]),
            ]},
        })
    }

    #[test]
    fn a_library_owns_its_crate_and_its_local_helpers() {
        let modules = [module("MyGame", "my-game"), module("Alpha", "alpha")];
        let owners = ownership(&metadata(), &modules).unwrap();

        assert_eq!(owners["MyGame"], ["game_helpers", "my_game"]);
        assert_eq!(owners["Alpha"], ["alpha"]);

        let expected = group_expected(
            &[
                "class:my_game:Item".to_string(),
                "class:game_helpers:Door".to_string(),
                "class:alpha:Item".to_string(),
                "hook:alpha".to_string(),
            ],
            &owners,
        );

        assert_eq!(
            expected["MyGame"],
            ["class:my_game:Item", "class:game_helpers:Door"]
        );
        assert_eq!(expected["Alpha"], ["class:alpha:Item", "hook:alpha"]);
    }

    #[test]
    fn a_crate_shared_by_two_libraries_is_refused() {
        let mut metadata = metadata();
        metadata["resolve"]["nodes"][2] = serde_json::json!({
            "id": "plugin", "deps": [{ "pkg": "helpers" }, { "pkg": "runtime" }],
        });

        let modules = [module("MyGame", "my-game"), module("Alpha", "alpha")];
        let error = ownership(&metadata, &modules).unwrap_err();
        assert!(error.contains("game-helpers"), "{error}");
    }

    #[test]
    fn native_libraries_come_from_rustc() {
        let linux = "warning: x\nnote: native-static-libs: -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -lgcc_s\n";
        assert_eq!(
            parse_native_libs(linux),
            ["gcc_s", "util", "rt", "pthread", "m", "dl", "c"]
        );

        let windows = "note: native-static-libs: kernel32.lib advapi32.lib /defaultlib:msvcrt";
        assert_eq!(parse_native_libs(windows), ["kernel32.lib", "advapi32.lib"]);

        let mac = "note: native-static-libs: -framework CoreFoundation -liconv -lSystem";
        assert_eq!(
            parse_native_libs(mac),
            ["framework:CoreFoundation", "iconv", "System"]
        );
        assert!(parse_native_libs("nothing").is_empty());
    }

    #[test]
    fn only_rusteal_dynamic_libraries_are_reported() {
        let dir = std::env::temp_dir().join(format!("rusteal-staged-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Game/Plugins/Rusteal/Binaries/Linux")).unwrap();
        fs::write(
            dir.join("Game/Plugins/Rusteal/Binaries/Linux/librusteal.so"),
            b"",
        )
        .unwrap();
        fs::write(dir.join("Game/libsteam_api.so"), b"").unwrap();

        let found = staged_dynamic_libraries(&dir);
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("librusteal.so"));

        fs::remove_dir_all(&dir).unwrap();
    }
}
