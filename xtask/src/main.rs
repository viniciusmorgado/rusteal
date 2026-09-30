// Contributor tooling for the Rusteal repository. Not published; games never use it.
//
//   cargo xtask dev-setup [--project <path> | --new <path>]
//
// Prepares a checkout for working on Rusteal. Rusteal is developed against a UE
// project: the plugins in `ue_plugin/` are built inside one, and the C++ editor
// support reads what that build generates. `dev-setup` asks for that base project
// (an existing one, or a new blank one it creates), then writes the files IDEs
// need to understand the plugin sources: compile_commands.json for clangd, and the
// exporter's .csproj.props for C# language servers. The engine path comes from
// `.env` at the repository root.

mod clangdb;
mod dev_env;

use std::ffi::OsStr;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use dev_env::{DevEnv, expand_home};

const USAGE: &str = "usage: cargo xtask dev-setup [--project <path> | --new <path>]

Reads RUSTEAL_DEV_ENGINE_ROOT from .env at the repository root (see .env.example)
or from the environment, and asks for the UE project Rusteal is developed against:

  --project <path>  an existing project, with this checkout's plugins installed
                    and built in it; it is only read, never changed
  --new <path>      a new blank project, created there by this checkout's CLI
                    (the last component of the path is the project name)

Then writes compile_commands.json (clangd) and the exporter's .csproj.props
(C# language servers), and restores rusteal.sln (the exporter and the *.Build.cs
projects).";

const BASE_PROJECT: &str = "\
Rusteal is developed against an Unreal Engine project: the plugins in ue_plugin/
are built inside one, and the C++ editor support reads what that build generates
(UHT's *.generated.h headers, UBT's compile commands).

  1) Use an existing project. It must have this checkout's plugins installed and
     built; it is only read, never changed.
  2) Create a new blank project, depending on this checkout, and build it.";

/// The UE project Rusteal is developed against.
enum Base {
    Existing(PathBuf),
    New(PathBuf),
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("dev-setup") => dev_setup(&args[1..]),
        _ => usage(),
    }
}

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2);
}

fn dev_setup(args: &[String]) {
    let repo = repo_root();
    let env = DevEnv::load(&repo);

    let base = match args {
        [] => ask_base(),
        [flag, path] if flag == "--project" => Base::Existing(expand_home(path)),
        [flag, path] if flag == "--new" => Base::New(expand_home(path)),
        _ => usage(),
    };
    let project = match base {
        Base::Existing(project) => {
            check_existing(&repo, &project);
            project
        }
        Base::New(project) => {
            create_new(&repo, &project);
            project
        }
    };
    let name = uproject_name(&project);

    let ubt_db = clangdb::generate_ubt_db(&env.engine_root, &project, &name, &repo);
    let out = repo.join("compile_commands.json");
    let count = clangdb::write_for_checkout(
        &ubt_db,
        &project.join("Plugins"),
        &repo.join("ue_plugin"),
        &out,
    );
    eprintln!("xtask: {count} plugin sources -> {}", out.display());

    let props = repo.join(
        "ue_plugin/RustealGenerator/Source/RustealExporter/RustealExporter.ubtplugin.csproj.props",
    );
    write_exporter_props(&env.engine_root, &props);
    eprintln!("xtask: EngineDir -> {}", props.display());
    restore_solution(&env.engine_root, &repo.join("rusteal.sln"));

    eprintln!("\nxtask: done. Restart the IDE's language servers to pick the files up.");
}

/// Ask for the base project on the terminal.
fn ask_base() -> Base {
    if !std::io::stdin().is_terminal() {
        fail("no terminal to ask on: pass --project <path> or --new <path>.");
    }
    eprintln!("{BASE_PROJECT}\n");
    loop {
        match prompt("Choose 1 or 2: ").as_str() {
            "1" => {
                let path = prompt("Path to the project (the directory holding its .uproject): ");
                return Base::Existing(expand_home(&path));
            }
            "2" => {
                let path = prompt(
                    "Where to create it (the new project's directory; its last component \
                     is the project name, e.g. ~/UnrealProjects/RustealDev): ",
                );
                return Base::New(expand_home(&path));
            }
            _ => {}
        }
    }
}

fn prompt(question: &str) -> String {
    eprint!("{question}");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line).unwrap_or(0) == 0 {
        fail("no answer given");
    }
    let answer = line.trim();
    // A path pasted with quotes around it.
    answer
        .strip_prefix('"')
        .and_then(|a| a.strip_suffix('"'))
        .unwrap_or(answer)
        .to_string()
}

/// An existing project must have this checkout's plugins installed and built.
fn check_existing(repo: &Path, project: &Path) {
    let name = uproject_name(project);
    let fix = format!(
        "Install and build them from this checkout:\n  \
         cargo run -p rusteal -- setup \"{p}\"\n  \
         cargo run -p rusteal -- build \"{p}\"",
        p = project.display()
    );
    if !project.join("Plugins/Rusteal").is_dir() {
        fail(&format!("{name} has no Rusteal plugin. {fix}"));
    }
    if !project.join("Plugins/Rusteal/Intermediate/Build").is_dir() {
        fail(&format!("the Rusteal plugin in {name} has not been built. {fix}"));
    }
    let differing = differing_headers(repo, project);
    if let Some(first) = differing.first() {
        eprintln!(
            "xtask: warning: {} plugin header(s) in {name} differ from this checkout \
             (e.g. {first}); UHT's generated headers there will not match the checkout's. {fix}",
            differing.len()
        );
    }
}

/// Plugin headers of this checkout that are missing or different in the project.
fn differing_headers(repo: &Path, project: &Path) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for path in entries.flatten().map(|e| e.path()) {
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "h") {
                out.push(path);
            }
        }
    }
    let checkout = repo.join("ue_plugin");
    let mut headers = Vec::new();
    walk(&checkout, &mut headers);
    headers
        .iter()
        .filter_map(|header| {
            let rel = header.strip_prefix(&checkout).ok()?;
            let copy = std::fs::read(project.join("Plugins").join(rel)).ok();
            (copy != std::fs::read(header).ok()).then(|| rel.display().to_string())
        })
        .collect()
}

/// Create a new blank project with this checkout's CLI.
fn create_new(repo: &Path, project: &Path) {
    let name = project
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let valid = name.len() <= 20
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric());
    if !valid {
        fail(&format!(
            "the last component of the path is the project name: letters and digits, \
             starting with a letter, 20 characters at most (got {}).",
            project.display()
        ));
    }
    if project.exists() {
        fail(&format!(
            "{} already exists; to use it, choose an existing project (--project).",
            project.display()
        ));
    }
    let parent = project.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", parent.display())));

    eprintln!("xtask: creating the blank project {}", project.display());
    rusteal(
        repo,
        &[
            OsStr::new("new"),
            OsStr::new(&name),
            OsStr::new("--dir"),
            parent.as_os_str(),
            OsStr::new("--runtime-path"),
            repo.as_os_str(),
        ],
    );
}

/// The project's name: the stem of its only `.uproject`.
fn uproject_name(project: &Path) -> String {
    let uprojects: Vec<PathBuf> = std::fs::read_dir(project)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", project.display())))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "uproject"))
        .collect();
    match uprojects.as_slice() {
        [uproject] => uproject
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        [] => fail(&format!("{} has no .uproject.", project.display())),
        _ => fail(&format!("{} has more than one .uproject.", project.display())),
    }
}

/// The repository root: this crate lives in `<root>/xtask`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits inside the repository")
        .to_path_buf()
}

/// Run this checkout's CLI, as `cargo run -p rusteal -- <args>`.
fn rusteal(repo: &Path, args: &[&OsStr]) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(repo.join("Cargo.toml"))
        .args(["--package", "rusteal", "--"])
        .args(args)
        .status()
        .unwrap_or_else(|e| fail(&format!("cannot run cargo: {e}")));
    if !status.success() {
        fail("the rusteal command above failed");
    }
}

/// The exporter's `.csproj` reads `EngineDir` from this file (see its comment).
fn write_exporter_props(engine_root: &Path, props: &Path) {
    let engine_dir = engine_root.join("Engine");
    let engine_dir = engine_dir.to_string_lossy().replace('\\', "/");
    let text = format!(
        "<Project>\n  <PropertyGroup>\n    <EngineDir>{engine_dir}</EngineDir>\n  </PropertyGroup>\n</Project>\n"
    );
    std::fs::write(props, text)
        .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", props.display())));
}

/// `dotnet restore` the solution (the exporter and the `*.Build.cs` projects), so a C#
/// language server can load them right away. Uses the .NET SDK bundled with the engine,
/// the one UBT builds the exporter with, or `dotnet` from PATH. Not fatal: language
/// servers restore on their own too.
fn restore_solution(engine_root: &Path, solution: &Path) {
    let dotnet = bundled_dotnet(engine_root).unwrap_or_else(|| PathBuf::from("dotnet"));
    let ok = Command::new(&dotnet)
        .args(["restore", "--verbosity", "quiet"])
        .arg(solution)
        .status()
        .is_ok_and(|s| s.success());
    if ok {
        eprintln!("xtask: restored {}", solution.display());
    } else {
        eprintln!(
            "xtask: warning: `{} restore` failed; the C# language server will restore the \
             projects itself.",
            dotnet.display()
        );
    }
}

/// `Engine/Binaries/ThirdParty/DotNet/<version>/<os>-<arch>/dotnet` for this host.
fn bundled_dotnet(engine_root: &Path) -> Option<PathBuf> {
    let os = match std::env::consts::OS {
        "windows" => "win",
        "macos" => "osx",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    let exe = if cfg!(windows) { "dotnet.exe" } else { "dotnet" };
    let root = engine_root.join("Engine/Binaries/ThirdParty/DotNet");
    let mut versions: Vec<PathBuf> =
        std::fs::read_dir(root).ok()?.flatten().map(|e| e.path()).collect();
    versions.sort();
    versions
        .iter()
        .rev()
        .map(|v| v.join(format!("{os}-{arch}")).join(exe))
        .find(|p| p.is_file())
}

fn fail(message: &str) -> ! {
    eprintln!("xtask: error: {message}");
    std::process::exit(1);
}
