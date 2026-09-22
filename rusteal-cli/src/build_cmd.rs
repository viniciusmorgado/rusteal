// Build command: 5-step build pipeline replacing tools/build.py.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use rusteal_codegen::config::{ProjectConfig, ProjectLayout};

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
/// `project_root` holds the .uproject and `rusteal.toml`; `engine_path` is the
/// UE root. `step` runs only that step (1-5), `from` starts from it; the two
/// are mutually exclusive.
pub fn run_build(project_root: &Path, engine_path: &Path, step: Option<u8>, from: u8) {
    // Validate step/from
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
    let ctx = BuildContext::new(&config, project_root, engine_path);

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
    layout: ProjectLayout,
    crate_name: String,
    /// Extra features for `cargo build` (from `[project].features`).
    features: Vec<String>,
}

impl BuildContext {
    fn new(config: &ProjectConfig, project_root: &Path, engine_path: &Path) -> Self {
        BuildContext {
            engine_path: engine_path.to_path_buf(),
            layout: ProjectLayout::new(project_root),
            crate_name: config.project.crate_name.clone(),
            features: config.project.features.clone(),
        }
    }

    /// The directory holding the .uproject.
    fn project_path(&self) -> &Path {
        &self.layout.root
    }

    /// Find the .uproject file and derive the Editor target name.
    fn editor_target(&self) -> String {
        let uproject = self.uproject_path();
        let stem = uproject
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        format!("{stem}Editor")
    }

    /// Full path to the .uproject file.
    fn uproject_path(&self) -> PathBuf {
        self.layout.uproject().unwrap_or_else(|| {
            eprintln!("Error: no .uproject found in {}", self.project_path().display());
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
                fs::copy(&src, uht_input.join(name)).unwrap_or_else(|e| {
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
            uht_input.display()
        );
    }

    /// Step 2: Run codegen (in-process).
    fn step2_codegen(&self) {
        rusteal_codegen::run_generate(&self.layout.root);
    }

    /// Step 3: UE rebuild — compiles generated C++ wrappers.
    fn step3_ue_rebuild(&self) {
        self.run_ubt();
    }

    /// Step 4: cargo build --release of the game crate in the Rust workspace.
    fn step4_cargo_build(&self) {
        let manifest = self.layout.rust_workspace().join("Cargo.toml");
        let manifest_str = manifest.to_string_lossy().into_owned();
        let mut args = vec![
            "cargo",
            "build",
            "--release",
            "--manifest-path",
            &manifest_str,
            "-p",
            &self.crate_name,
        ];
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

        let src = self
            .layout
            .rust_workspace()
            .join("target/release")
            .join(&dll_filename);

        let dest_dir = self
            .project_path()
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
    let display: String = args.to_vec().join(" ");
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
