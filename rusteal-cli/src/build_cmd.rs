// Build command: 5-step build pipeline replacing tools/build.py.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use rusteal_codegen::config::RustealConfig;

/// Canonicalize a path, stripping the `\\?\` extended-length prefix that
/// Windows adds. UBT's .NET XML parser chokes on that prefix.
fn canonical_no_prefix(path: &Path) -> PathBuf {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    #[cfg(windows)]
    {
        // `canonicalize()` returns `\\?\C:\...` on Windows; strip the prefix.
        let s = abs.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }
    abs
}

/// Run the build pipeline.
///
/// `config_path` is the path to rusteal.config.toml.
/// `step` runs only that step (1-5). `from` starts from that step (1-5).
/// `step` and `from` are mutually exclusive.
pub fn run_build(config_path: &Path, step: Option<u8>, from: u8) {
    // Validate step/from
    if step.is_some() && from != 1 {
        eprintln!("Error: --step and --from are mutually exclusive.");
        std::process::exit(1);
    }
    if let Some(s) = step {
        if !(1..=5).contains(&s) {
            eprintln!("Error: --step must be 1-5, got {s}");
            std::process::exit(1);
        }
    }
    if !(1..=5).contains(&from) {
        eprintln!("Error: --from must be 1-5, got {from}");
        std::process::exit(1);
    }

    // Load config
    let config_str = fs::read_to_string(config_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", config_path.display()));
    let config: RustealConfig = toml::from_str(&config_str)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", config_path.display()));

    // Resolve paths relative to config file directory
    let config_parent = config_path.parent().unwrap_or(Path::new("."));
    let config_dir = if config_parent.as_os_str().is_empty() {
        Path::new(".").canonicalize()
    } else {
        config_parent.canonicalize()
    }
    .unwrap_or_else(|e| panic!("Failed to canonicalize config dir: {e}"));

    let ctx = BuildContext::new(&config, &config_dir, config_path);

    // Determine which steps to run
    let steps: Vec<u8> = if let Some(s) = step {
        vec![s]
    } else {
        (from..=5).collect()
    };

    let step_descs: &[&str] = &[
        "",
        "UE build (UHT export to JSON)",
        "rusteal-codegen (JSON -> Rust + C++)",
        "UE rebuild (compile C++ wrappers)",
        "cargo build --release (cdylib)",
        "Copy the shared library to plugin Binaries",
    ];

    let total = steps.len();
    let overall_start = Instant::now();

    eprintln!("\n{}", "=".repeat(60));
    eprintln!("  Rusteal build pipeline");
    if step.is_some() {
        eprintln!("  Running step {} only", steps[0]);
    } else if from > 1 {
        eprintln!("  Running steps {from}-5");
    } else {
        eprintln!("  Running all {total} steps");
    }
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
            1 => ctx.step1_ue_build(),
            2 => ctx.step2_codegen(),
            3 => ctx.step3_ue_rebuild(),
            4 => ctx.step4_cargo_build(),
            5 => ctx.step5_copy_dll(),
            _ => unreachable!(),
        }

        let elapsed = step_start.elapsed().as_secs_f64();
        eprintln!("  Step {step_num} completed in {elapsed:.1}s\n");
    }

    let overall = overall_start.elapsed().as_secs_f64();
    eprintln!("{}", "=".repeat(60));
    eprintln!("  All steps completed in {overall:.1}s");
    eprintln!("{}", "=".repeat(60));
}

/// What differs between the host platforms the pipeline runs on. Chosen at compile time from
/// the platform `rusteal-cli` itself was built for, which is also the platform UE builds the
/// editor for.
struct HostPlatform {
    /// UBT entry script, relative to the engine root.
    ubt_script: &'static str,
    /// Platform name as UBT expects it (`Build.bat <Target> Win64 ...`).
    ubt_platform: &'static str,
    /// Prefix and extension of a shared library (`libx.so`, `x.dll`, `libx.dylib`).
    lib_prefix: &'static str,
    lib_extension: &'static str,
}

impl HostPlatform {
    #[cfg(target_os = "windows")]
    const CURRENT: HostPlatform = HostPlatform {
        ubt_script: "Engine/Build/BatchFiles/Build.bat",
        ubt_platform: "Win64",
        lib_prefix: "",
        lib_extension: "dll",
    };

    #[cfg(target_os = "linux")]
    const CURRENT: HostPlatform = HostPlatform {
        ubt_script: "Engine/Build/BatchFiles/Linux/Build.sh",
        ubt_platform: "Linux",
        lib_prefix: "lib",
        lib_extension: "so",
    };

    #[cfg(target_os = "macos")]
    const CURRENT: HostPlatform = HostPlatform {
        ubt_script: "Engine/Build/BatchFiles/Mac/Build.sh",
        ubt_platform: "Mac",
        lib_prefix: "lib",
        lib_extension: "dylib",
    };

    /// File name of a shared library built from `crate_name`, as Cargo names it.
    fn lib_filename(&self, crate_name: &str) -> String {
        // Cargo converts hyphens to underscores in output filenames.
        format!(
            "{}{}.{}",
            self.lib_prefix,
            crate_name.replace('-', "_"),
            self.lib_extension
        )
    }

    /// File name the Rusteal plugin loads (`rusteal.dll`, `librusteal.so`, `librusteal.dylib`); must match
    /// `FRustealModule::StartupModule` in the C++ plugin.
    fn deployed_lib_filename(&self) -> String {
        format!("{}rusteal.{}", self.lib_prefix, self.lib_extension)
    }
}

/// Resolved build context with all paths pre-computed.
struct BuildContext {
    engine_path: PathBuf,
    project_path: PathBuf,
    uht_input: PathBuf,
    config_path: PathBuf,
    crate_name: String,
    config_dir: PathBuf,
    /// Path to an external crate directory (resolved, absolute).
    /// When set, `cargo build` uses `--manifest-path` instead of `-p`.
    crate_path: Option<PathBuf>,
    /// Extra features for `cargo build` (from `[build].features`).
    features: Vec<String>,
}

impl BuildContext {
    fn new(config: &RustealConfig, config_dir: &Path, config_path: &Path) -> Self {
        // Engine path (required for build)
        let engine_path = config
            .ue
            .as_ref()
            .map(|ue| PathBuf::from(&ue.engine_path))
            .unwrap_or_else(|| {
                eprintln!("Error: [ue].engine_path is required for build.");
                std::process::exit(1);
            });

        // Project path (relative to config dir)
        let project_rel = config
            .project
            .as_ref()
            .map(|p| p.path.as_str())
            .unwrap_or(".");
        let project_path = config_dir.join(project_rel);

        // UHT input path (relative to config dir)
        let uht_input = config_dir.join(&config.codegen.paths.uht_input);

        // Crate name: from config or auto-detect
        let crate_name = config
            .build
            .as_ref()
            .and_then(|b| b.crate_name.clone())
            .unwrap_or_else(|| detect_cdylib_crate(config_dir));

        let crate_path = config
            .build
            .as_ref()
            .and_then(|b| b.crate_path.as_ref())
            .map(|p| config_dir.join(p));

        let features = config
            .build
            .as_ref()
            .map(|b| b.features.clone())
            .unwrap_or_default();

        BuildContext {
            engine_path,
            project_path,
            uht_input,
            config_path: config_path.to_path_buf(),
            crate_name,
            config_dir: config_dir.to_path_buf(),
            crate_path,
            features,
        }
    }

    /// Find the .uproject file and derive the Editor target name.
    fn editor_target(&self) -> String {
        let uproject = find_uproject(&self.project_path).unwrap_or_else(|| {
            eprintln!(
                "Error: no .uproject found in {}",
                self.project_path.display()
            );
            std::process::exit(1);
        });
        let stem = uproject
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        format!("{stem}Editor")
    }

    /// Full path to the .uproject file.
    fn uproject_path(&self) -> PathBuf {
        find_uproject(&self.project_path).unwrap_or_else(|| {
            eprintln!(
                "Error: no .uproject found in {}",
                self.project_path.display()
            );
            std::process::exit(1);
        })
    }

    /// Build the project's Editor target with UBT for the host platform.
    fn run_ubt(&self) {
        let platform = &HostPlatform::CURRENT;
        let script = self.engine_path.join(platform.ubt_script);
        if !script.exists() {
            eprintln!("Error: UBT script not found at {}", script.display());
            std::process::exit(1);
        }

        let target = self.editor_target();
        let uproject = self.uproject_path();
        let uproject_abs = canonical_no_prefix(&uproject);

        run_cmd(&[
            script.to_str().unwrap(),
            &target,
            platform.ubt_platform,
            "Development",
            &format!("-Project={}", uproject_abs.display()),
        ]);
    }

    /// Step 1: UE build — triggers UHT export, then copies JSON.
    fn step1_ue_build(&self) {
        self.run_ubt();

        // Copy UHT JSON files
        let uht_intermediate = self.project_path.join(format!(
            "Plugins/RustealGenerator/Intermediate/Build/{}/UnrealEditor/Inc/RustealGenerator/UHT",
            HostPlatform::CURRENT.ubt_platform
        ));

        fs::create_dir_all(&self.uht_input).unwrap_or_else(|e| {
            panic!("Failed to create {}: {e}", self.uht_input.display())
        });

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
                fs::copy(&src, self.uht_input.join(name)).unwrap_or_else(|e| {
                    panic!("Failed to copy {}: {e}", src.display())
                });
                eprintln!("  Copied {name} ({size_kb} KB)");
                copied += 1;
            } else {
                eprintln!("  Warning: {name} not found in {}", uht_intermediate.display());
            }
        }
        eprintln!(
            "  {copied}/{} JSON files copied to {}",
            json_files.len(),
            self.uht_input.display()
        );
    }

    /// Step 2: Run codegen (in-process).
    fn step2_codegen(&self) {
        rusteal_codegen::run_generate(&self.config_path);
    }

    /// Step 3: UE rebuild — compiles generated C++ wrappers.
    fn step3_ue_rebuild(&self) {
        self.run_ubt();
    }

    /// Step 4: cargo build --release.
    fn step4_cargo_build(&self) {
        let manifest_path_str;
        let mut args = vec!["cargo", "build", "--release"];

        if let Some(ref crate_path) = self.crate_path {
            // External crate: use --manifest-path
            let manifest = crate_path.join("Cargo.toml");
            manifest_path_str = manifest.to_string_lossy().into_owned();
            args.push("--manifest-path");
            args.push(&manifest_path_str);
        } else {
            // Workspace member: use -p
            args.push("-p");
            args.push(&self.crate_name);
        }

        let features_str = self.features.join(",");
        if !features_str.is_empty() {
            args.push("--features");
            args.push(&features_str);
        }
        run_cmd(&args);
    }

    /// Step 5: Copy the built shared library to the UE plugin's Binaries.
    fn step5_copy_dll(&self) {
        let platform = &HostPlatform::CURRENT;
        let dll_filename = platform.lib_filename(&self.crate_name);

        // Search order for the built DLL:
        // 1. External crate's own target dir (when crate_path is set)
        // 2. CWD/target/release/ (workspace member, step 4 runs from CWD)
        // 3. config_dir/target/release/ (fallback)
        let external_target = self.crate_path.as_ref().map(|p| {
            p.join("target/release").join(&dll_filename)
        });
        let cwd_target = std::env::current_dir()
            .unwrap_or_default()
            .join("target/release")
            .join(&dll_filename);
        let config_target = self.config_dir.join("target/release").join(&dll_filename);

        let src = if external_target.as_ref().is_some_and(|p| p.exists()) {
            external_target.unwrap()
        } else if cwd_target.exists() {
            cwd_target
        } else {
            config_target
        };

        let dest_dir = self
            .project_path
            .join("Plugins/Rusteal/Binaries")
            .join(platform.ubt_platform);
        let deployed_name = platform.deployed_lib_filename();
        let dest = dest_dir.join(&deployed_name);

        if !src.exists() {
            eprintln!("Error: library not found at {}", src.display());
            eprintln!("  Did step 4 (cargo build) succeed?");
            std::process::exit(1);
        }

        fs::create_dir_all(&dest_dir)
            .unwrap_or_else(|e| panic!("Failed to create {}: {e}", dest_dir.display()));
        fs::copy(&src, &dest)
            .unwrap_or_else(|e| panic!("Failed to copy DLL: {e}"));

        eprintln!("  Copied {}", src.display());
        eprintln!("      -> {} (renamed to {deployed_name})", dest.display());
    }
}

/// Run an external command, printing it and exiting on failure.
fn run_cmd(args: &[&str]) {
    let display: String = args.iter().map(|a| *a).collect::<Vec<_>>().join(" ");
    let truncated = if display.len() > 200 {
        format!("{}...", &display[..197])
    } else {
        display
    };
    eprintln!("  $ {truncated}");

    let status = Command::new(args[0])
        .args(&args[1..])
        .status()
        .unwrap_or_else(|e| panic!("Failed to run {}: {e}", args[0]));

    if !status.success() {
        let code = status.code().unwrap_or(1);
        eprintln!("\n  Command failed with exit code {code}");
        std::process::exit(code);
    }
}

/// Find a .uproject file in the given directory.
fn find_uproject(dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "uproject") {
            return Some(path);
        }
    }
    None
}

/// Auto-detect the cdylib crate name from workspace Cargo.toml files.
fn detect_cdylib_crate(config_dir: &Path) -> String {
    // Walk the workspace looking for a crate with crate-type = ["cdylib"]
    let workspace_toml = config_dir.join("Cargo.toml");
    if let Ok(content) = fs::read_to_string(&workspace_toml) {
        if let Ok(doc) = content.parse::<toml::Table>() {
            // Check workspace members
            if let Some(workspace) = doc.get("workspace").and_then(|w| w.as_table()) {
                if let Some(members) = workspace.get("members").and_then(|m| m.as_array()) {
                    for member in members {
                        if let Some(member_str) = member.as_str() {
                            let member_toml = config_dir.join(member_str).join("Cargo.toml");
                            if let Some(name) = check_cdylib_crate(&member_toml) {
                                return name;
                            }
                        }
                    }
                }
            }
            // Check if this is a single-crate project
            if let Some(name) = check_cdylib_crate(&workspace_toml) {
                return name;
            }
        }
    }

    eprintln!("Warning: could not auto-detect cdylib crate name, using 'rusteal'.");
    "rusteal".to_string()
}

/// Check if a Cargo.toml defines a cdylib crate, return its name.
fn check_cdylib_crate(cargo_toml: &Path) -> Option<String> {
    let content = fs::read_to_string(cargo_toml).ok()?;
    let doc: toml::Table = content.parse().ok()?;

    // Check [lib].crate-type for "cdylib"
    let lib = doc.get("lib")?.as_table()?;
    let crate_type = lib.get("crate-type").or_else(|| lib.get("crate_type"))?;
    let types = crate_type.as_array()?;
    let is_cdylib = types.iter().any(|t| t.as_str() == Some("cdylib"));

    if !is_cdylib {
        return None;
    }

    // Get crate name: [lib].name or [package].name
    let name = lib
        .get("name")
        .and_then(|n| n.as_str())
        .or_else(|| {
            doc.get("package")
                .and_then(|p| p.as_table())
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
        })?;

    Some(name.to_string())
}
