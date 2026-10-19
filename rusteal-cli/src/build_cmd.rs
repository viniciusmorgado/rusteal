use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use rusteal_codegen::config::{
    PluginConfig, PluginLayout, PluginLibrary, ProjectConfig, ProjectLayout, find_plugins,
};

pub fn canonical_no_prefix(path: &Path) -> PathBuf {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    #[cfg(windows)]
    {
        let s = abs.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }

    abs
}

pub const RUST_STEP: u8 = 4;

pub fn run_build(
    project_root: &Path,
    engine_path: &Path,
    step: Option<u8>,
    from: u8,
    release: bool,
    plugin: Option<&str>,
) {
    if step.is_some() && from != 1 {
        eprintln!("Error: --step and --from are mutually exclusive.");
        std::process::exit(1);
    }

    if let Some(s) = step
        && !(1..=5).contains(&s)
    {
        eprintln!("Error: --step must be 1-5, got {s}");
        std::process::exit(1);
    }

    if !(1..=5).contains(&from) {
        eprintln!("Error: --from must be 1-5, got {from}");
        std::process::exit(1);
    }

    let config = ProjectConfig::load(project_root).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        std::process::exit(1);
    });

    let ctx = BuildContext::new(&config, project_root, engine_path, release, plugin);

    let steps: Vec<u8> = if let Some(s) = step {
        vec![s]
    } else {
        (from..=5).collect()
    };

    let cargo_step = format!("cargo rustc, {} profile (cdylib)", ctx.profile());

    let libraries = ctx
        .libraries
        .iter()
        .map(|library| library.label.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    let step_descs: &[&str] = &[
        "",
        "UE build (UHT export to JSON)",
        "rusteal-codegen (JSON -> Rust + C++)",
        "UE rebuild (compile C++ wrappers)",
        &cargo_step,
        "Copy the shared library to plugin Binaries",
    ];

    let total = steps.len();
    let overall_start = Instant::now();

    eprintln!("\n{}", "=".repeat(60));
    eprintln!("  Rusteal build pipeline");

    if step.is_some() {
        eprintln!("  Running step {} only", steps[0]);
    } else if from == RUST_STEP {
        eprintln!("  Running steps {RUST_STEP}-5: the Rust libraries (--all runs every step)");
    } else if from > 1 {
        eprintln!("  Running steps {from}-5");
    } else {
        eprintln!("  Running all {total} steps");
    }

    eprintln!("  Libraries: {libraries}");
    eprintln!("{}\n", "=".repeat(60));

    for (i, &step_num) in steps.iter().enumerate() {
        let step_start = Instant::now();

        eprintln!(
            "[{}/{}] Step {}: {}",
            i + 1,
            total,
            step_num,
            step_descs[step_num as usize]
        );

        eprintln!("{}", "-".repeat(60));

        match step_num {
            1 => ctx.step1_ue_build(steps.contains(&2) && steps.contains(&3)),
            2 => ctx.step2_codegen(steps.contains(&1) && steps.contains(&3)),
            3 => ctx.step3_ue_rebuild(),
            4 => ctx.step4_cargo_build(),
            5 => ctx.step5_copy_dll(),
            _ => unreachable!(),
        }

        let elapsed = step_start.elapsed().as_secs_f64();
        eprintln!("  Step {step_num} completed in {elapsed:.1}s\n");
    }

    if steps.contains(&RUST_STEP) {
        let mut workspaces: Vec<PathBuf> = ctx
            .libraries
            .iter()
            .map(|library| library.rust_workspace.clone())
            .collect();

        workspaces.dedup();

        crate::console_check::report(project_root, &workspaces);
    }

    let overall = overall_start.elapsed().as_secs_f64();
    eprintln!("{}", "=".repeat(60));
    eprintln!("  All steps completed in {overall:.1}s");
    eprintln!("{}", "=".repeat(60));
}

pub struct HostPlatform {
    ubt_script: &'static str,
    pub uat_script: &'static str,
    pub ubt_platform: &'static str,
    lib_prefix: &'static str,
    lib_extension: &'static str,
}

impl HostPlatform {
    #[cfg(target_os = "windows")]
    pub const CURRENT: HostPlatform = HostPlatform {
        ubt_script: "Engine/Build/BatchFiles/Build.bat",
        uat_script: "Engine/Build/BatchFiles/RunUAT.bat",
        ubt_platform: "Win64",
        lib_prefix: "",
        lib_extension: "dll",
    };

    #[cfg(target_os = "linux")]
    pub const CURRENT: HostPlatform = HostPlatform {
        ubt_script: "Engine/Build/BatchFiles/Linux/Build.sh",
        uat_script: "Engine/Build/BatchFiles/RunUAT.sh",
        ubt_platform: "Linux",
        lib_prefix: "lib",
        lib_extension: "so",
    };

    #[cfg(target_os = "macos")]
    pub const CURRENT: HostPlatform = HostPlatform {
        ubt_script: "Engine/Build/BatchFiles/Mac/Build.sh",
        uat_script: "Engine/Build/BatchFiles/RunUAT.sh",
        ubt_platform: "Mac",
        lib_prefix: "lib",
        lib_extension: "dylib",
    };

    pub fn lib_filename(&self, crate_name: &str) -> String {
        format!(
            "{}{}.{}",
            self.lib_prefix,
            crate_name.replace('-', "_"),
            self.lib_extension
        )
    }

    pub fn deployed_lib_filename(&self, stem: &str) -> String {
        format!("{}{stem}.{}", self.lib_prefix, self.lib_extension)
    }
}

struct Library {
    label: String,
    rust_workspace: PathBuf,
    crate_name: String,
    features: Vec<String>,
    deploy_dir: PathBuf,
    deploy_stem: String,
    bindings_crate: PathBuf,
    plugin: Option<(PluginLayout, PluginLibrary)>,
}

struct BuildContext {
    engine_path: PathBuf,
    layout: ProjectLayout,
    libraries: Vec<Library>,
    release: bool,
}

impl BuildContext {
    fn new(
        config: &ProjectConfig,
        project_root: &Path,
        engine_path: &Path,
        release: bool,
        only_plugin: Option<&str>,
    ) -> Self {
        let layout = ProjectLayout::new(project_root);
        let platform = HostPlatform::CURRENT.ubt_platform;
        let mut libraries = Vec::new();

        if only_plugin.is_none() {
            libraries.push(Library {
                label: "game".to_string(),
                rust_workspace: layout.rust_workspace(),
                crate_name: config.project.crate_name.clone(),
                features: config.project.features.clone(),
                deploy_dir: project_root.join("Plugins/Rusteal/Binaries").join(platform),
                deploy_stem: "rusteal".to_string(),
                bindings_crate: layout.bindings_crate(),
                plugin: None,
            });
        }

        let plugins = find_plugins(project_root);

        if let Some(name) = only_plugin
            && !plugins.iter().any(|p| p.name.eq_ignore_ascii_case(name))
        {
            let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();

            eprintln!(
                "Error: no Rusteal plugin '{name}' in {}/Plugins (Rusteal plugins: {}).",
                project_root.display(),
                if names.is_empty() {
                    "none".to_string()
                } else {
                    names.join(", ")
                }
            );

            std::process::exit(1);
        }

        for plugin in plugins {
            if only_plugin.is_some_and(|name| !plugin.name.eq_ignore_ascii_case(name)) {
                continue;
            }

            if plugin.is_precompiled() {
                eprintln!(
                    "  plugin {}: precompiled (\"Installed\": true), its libraries are used as \
                     they are",
                    plugin.name
                );

                continue;
            }

            let plugin_config = PluginConfig::load(&plugin.dir).unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            if let Some(editor) = &plugin_config.editor {
                refuse_runtime_dependency(
                    &plugin.rust_workspace(),
                    &editor.crate_name,
                    &plugin_config.plugin.crate_name,
                );
            }

            for library in plugin_config.libraries(&plugin.name) {
                let label = if library.module == plugin.name {
                    format!("plugin {}", plugin.name)
                } else {
                    format!("plugin {} ({})", plugin.name, library.module)
                };

                libraries.push(Library {
                    label,
                    rust_workspace: plugin.rust_workspace(),
                    crate_name: library.crate_name.clone(),
                    features: library.features.clone(),
                    deploy_dir: plugin.binaries(platform),
                    deploy_stem: plugin.library_stem_of(&library),
                    bindings_crate: plugin.library_output(&library).bindings_crate,
                    plugin: Some((PluginLayout::new(&plugin.dir), library)),
                });
            }
        }

        BuildContext {
            engine_path: engine_path.to_path_buf(),
            layout,
            libraries,
            release,
        }
    }

    fn profile(&self) -> &'static str {
        if self.release { "release" } else { "dev" }
    }

    fn target_dir(&self) -> &'static str {
        if self.release { "release" } else { "debug" }
    }

    fn project_path(&self) -> &Path {
        &self.layout.root
    }

    fn editor_target(&self) -> String {
        let uproject = self.uproject_path();

        let stem = uproject.file_stem().unwrap().to_str().unwrap();

        format!("{stem}Editor")
    }

    fn uproject_path(&self) -> PathBuf {
        self.layout.uproject().unwrap_or_else(|| {
            eprintln!(
                "Error: no .uproject found in {}",
                self.project_path().display()
            );

            std::process::exit(1);
        })
    }

    fn run_ubt(&self) {
        let code = self.run_ubt_code();

        if code != 0 {
            std::process::exit(code);
        }
    }

    fn run_ubt_code(&self) -> i32 {
        let platform = &HostPlatform::CURRENT;
        let script = self.engine_path.join(platform.ubt_script);

        if !script.exists() {
            eprintln!("Error: UBT script not found at {}", script.display());
            std::process::exit(1);
        }

        let target = self.editor_target();
        let uproject = self.uproject_path();
        let uproject_abs = canonical_no_prefix(&uproject);

        run_cmd_code(&[
            script.to_str().unwrap(),
            &target,
            platform.ubt_platform,
            "Development",
            &format!("-Project={}", uproject_abs.display()),
            "-NoHotReload",
        ])
    }

    fn step1_ue_build(&self, regenerates: bool) {
        let code = self.run_ubt_code();

        let uht_input = self.layout.uht_json();

        let uht_intermediate = self.project_path().join(format!(
            "Plugins/RustealGenerator/Intermediate/Build/{}/UnrealEditor/Inc/RustealGenerator/UHT",
            HostPlatform::CURRENT.ubt_platform
        ));

        fs::create_dir_all(&uht_input)
            .unwrap_or_else(|e| panic!("Failed to create {}: {e}", uht_input.display()));

        let json_files = [
            "rusteal_classes.json",
            "rusteal_structs.json",
            "rusteal_enums.json",
        ];

        let mut copied = 0;

        for name in &json_files {
            let src = uht_intermediate.join(name);

            if src.exists() {
                let size_kb = fs::metadata(&src).map(|m| m.len() / 1024).unwrap_or(0);

                fs::copy(&src, uht_input.join(name))
                    .unwrap_or_else(|e| panic!("Failed to copy {}: {e}", src.display()));

                eprintln!("  Copied {name} ({size_kb} KB)");
                copied += 1;
            } else {
                eprintln!(
                    "  Warning: {name} not found in {}",
                    uht_intermediate.display()
                );
            }
        }

        eprintln!(
            "  {copied}/{} JSON files copied to {}",
            json_files.len(),
            uht_input.display()
        );

        if code != 0 {
            if regenerates && copied == json_files.len() {
                eprintln!(
                    "  The build failed with the code generated before; it is generated \
                     again from the engine's reflection and built in step 3."
                );
            } else {
                std::process::exit(code);
            }
        }
    }

    fn descriptors(&self) -> Vec<PathBuf> {
        std::iter::once(self.layout.plugin_descriptor())
            .chain(
                self.libraries
                    .iter()
                    .filter_map(|l| l.plugin.as_ref().map(|(plugin, _)| plugin.descriptor())),
            )
            .collect()
    }

    fn plugin_dependencies(&self) -> Vec<String> {
        self.descriptors()
            .iter()
            .flat_map(|descriptor| {
                let json: serde_json::Value = fs::read_to_string(descriptor)
                    .ok()
                    .and_then(|text| serde_json::from_str(&text).ok())
                    .unwrap_or_default();

                json["Plugins"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| p["Name"].as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn step2_codegen(&self, reexports: bool) {
        let before = self.plugin_dependencies();
        self.generate_all();

        if reexports && self.plugin_dependencies() != before {
            eprintln!(
                "  The bindings now use engine plugins the reflection dump did not have; \
                 it is exported again and the bindings generated from it."
            );

            let header = self
                .project_path()
                .join("Plugins/RustealGenerator/Source/RustealGenerator/RustealGeneratorDummy.h");

            if let Ok(file) = fs::File::options().write(true).open(&header) {
                let _ = file.set_modified(std::time::SystemTime::now());
            }

            self.step1_ue_build(true);
            self.generate_all();
        }
    }

    fn generate_all(&self) {
        for library in &self.libraries {
            match &library.plugin {
                None => rusteal_codegen::run_generate(&self.layout.root),
                Some((plugin, plugin_library)) => rusteal_codegen::run_generate_plugin_library(
                    &self.layout.root,
                    plugin,
                    plugin_library,
                ),
            }

            crate::migrate::bindings_dependency(&library.rust_workspace, &library.crate_name);
        }
    }

    fn step3_ue_rebuild(&self) {
        self.run_ubt();
    }

    fn step4_cargo_build(&self) {
        for library in &self.libraries {
            if !library.bindings_crate.join("src/func_ids.rs").is_file() {
                eprintln!(
                    "Error: the {} library's bindings have not been generated yet: \
                     run `rusteal build --all` once.",
                    library.label
                );

                std::process::exit(1);
            }

            crate::migrate::bindings_dependency(&library.rust_workspace, &library.crate_name);
            eprintln!("  {}: {}", library.label, library.crate_name);

            let member = library
                .rust_workspace
                .join(&library.crate_name)
                .join("Cargo.toml");

            let (manifest, package) = if member.is_file() {
                (member, None)
            } else {
                (
                    library.rust_workspace.join("Cargo.toml"),
                    Some(library.crate_name.as_str()),
                )
            };

            let manifest_str = manifest.to_string_lossy().into_owned();
            let mut args = vec!["cargo", "rustc", "--manifest-path", &manifest_str];

            if let Some(package) = package {
                args.extend(["-p", package]);
            }

            args.extend(["--lib", "--crate-type", "cdylib"]);

            if self.release {
                args.push("--release");
            }

            let features_str = library.features.join(",");

            if !features_str.is_empty() {
                args.push("--features");
                args.push(&features_str);
            }

            run_cmd(&args);
        }
    }

    fn step5_copy_dll(&self) {
        let platform = &HostPlatform::CURRENT;

        for library in &self.libraries {
            let src = library
                .rust_workspace
                .join("target")
                .join(self.target_dir())
                .join(platform.lib_filename(&library.crate_name));

            let deployed_name = platform.deployed_lib_filename(&library.deploy_stem);
            let dest = library.deploy_dir.join(&deployed_name);

            if !src.exists() {
                eprintln!("Error: library not found at {}", src.display());
                eprintln!("  Did step 4 (cargo build) succeed?");
                std::process::exit(1);
            }

            fs::create_dir_all(&library.deploy_dir).unwrap_or_else(|e| {
                panic!("Failed to create {}: {e}", library.deploy_dir.display())
            });

            deploy(&src, &dest).unwrap_or_else(|e| panic!("Failed to copy DLL: {e}"));

            eprintln!("  {}: copied {}", library.label, src.display());
            eprintln!("      -> {} (renamed to {deployed_name})", dest.display());
        }
    }
}

fn deploy(src: &Path, dest: &Path) -> std::io::Result<()> {
    let mut partial = dest.as_os_str().to_owned();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    fs::copy(src, &partial)?;

    fs::rename(&partial, dest).inspect_err(|_| {
        let _ = fs::remove_file(&partial);
    })
}

pub fn run_cmd(args: &[&str]) {
    let code = run_cmd_code(args);

    if code != 0 {
        std::process::exit(code);
    }
}

pub fn run_cmd_code(args: &[&str]) -> i32 {
    run_cmd_code_env(args, &[])
}

pub fn run_cmd_code_env(args: &[&str], env: &[(&str, &Path)]) -> i32 {
    let display: String = args.to_vec().join(" ");

    let truncated = if display.len() > 200 {
        format!("{}...", &display[..197])
    } else {
        display
    };

    eprintln!("  $ {truncated}");

    let status = Command::new(args[0])
        .args(&args[1..])
        .envs(env.iter().copied())
        .status()
        .unwrap_or_else(|e| panic!("Failed to run {}: {e}", args[0]));

    if status.success() {
        return 0;
    }

    let code = status.code().unwrap_or(1);
    eprintln!("\n  Command failed with exit code {code}");
    code
}

fn depends_on(manifest_text: &str, crate_name: &str) -> bool {
    let Ok(manifest) = manifest_text.parse::<toml::Table>() else {
        return false;
    };

    ["dependencies", "build-dependencies"]
        .iter()
        .any(|section| {
            manifest
                .get(*section)
                .and_then(toml::Value::as_table)
                .is_some_and(|deps| {
                    deps.iter().any(|(name, spec)| {
                        let package = spec
                            .as_table()
                            .and_then(|t| t.get("package"))
                            .and_then(toml::Value::as_str)
                            .unwrap_or(name);

                        package == crate_name
                    })
                })
        })
}

fn refuse_runtime_dependency(workspace: &Path, editor_crate: &str, runtime_crate: &str) {
    let manifest = workspace.join(editor_crate).join("Cargo.toml");
    let text = fs::read_to_string(&manifest).unwrap_or_default();

    if depends_on(&text, runtime_crate) {
        eprintln!(
            "Error: {} depends on {runtime_crate}, the plugin's runtime crate. The editor \
             library would register the runtime library's classes a second time: reach them \
             through reflection (DynamicCall, interfaces, Blueprint classes), and move shared \
             code without Rust types into a crate of its own.",
            manifest.display()
        );

        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{depends_on, deploy};

    #[test]
    fn an_editor_crate_depending_on_the_runtime_crate_is_seen() {
        let by_name = "[dependencies]\ninventory = { path = \"../inventory\" }\n";
        let by_package =
            "[dependencies]\ninv = { package = \"inventory\", path = \"../inventory\" }\n";
        let unrelated = "[dependencies]\nrusteal-runtime = { workspace = true }\n";

        assert!(depends_on(by_name, "inventory"));
        assert!(depends_on(by_package, "inventory"));
        assert!(!depends_on(unrelated, "inventory"));
    }

    #[test]
    fn deploy_replaces_the_library_and_leaves_nothing_else() {
        let dir = std::env::temp_dir().join(format!("rusteal-deploy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let built = dir.join("libgame.so");
        let deployed = dir.join("librusteal.so");
        std::fs::write(&deployed, b"old").unwrap();
        std::fs::write(&built, b"new").unwrap();

        deploy(&built, &deployed).unwrap();

        assert_eq!(std::fs::read(&deployed).unwrap(), b"new");

        let mut names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();

        names.sort();
        assert_eq!(names, ["libgame.so", "librusteal.so"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
