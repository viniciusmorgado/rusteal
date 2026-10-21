use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use crate::dev_env::DevEnv;
use crate::{fail, repo_root};

const PROJECT: &str = "RustealTests";
const CRATE: &str = "rusteal-tests";
const TIMEOUT: Duration = Duration::from_secs(600);

fn run(command: &mut Command, what: &str, log: &Path) {
    let file = std::fs::File::create(log)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", log.display())));

    let status = command
        .stdout(file.try_clone().expect("a log file handle"))
        .stderr(file)
        .status()
        .unwrap_or_else(|e| fail(&format!("cannot run {what}: {e}")));

    if !status.success() {
        fail(&format!(
            "{what} failed; its output is in {}",
            log.display()
        ));
    }
}

fn editor(engine: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        engine.join("Engine/Binaries/Win64/UnrealEditor.exe")
    } else if cfg!(target_os = "macos") {
        engine.join("Engine/Binaries/Mac/UnrealEditor.app/Contents/MacOS/UnrealEditor")
    } else {
        engine.join("Engine/Binaries/Linux/UnrealEditor")
    }
}

fn copy_sources(from: &Path, to: &Path) {
    let _ = std::fs::remove_dir_all(to);
    std::fs::create_dir_all(to)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", to.display())));

    for entry in std::fs::read_dir(from)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", from.display())))
        .flatten()
    {
        std::fs::copy(entry.path(), to.join(entry.file_name()))
            .unwrap_or_else(|e| fail(&format!("cannot copy {}: {e}", entry.path().display())));
    }
}

fn wait(child: &mut Child) -> bool {
    let started = Instant::now();

    while started.elapsed() < TIMEOUT {
        if let Ok(Some(_)) = child.try_wait() {
            return true;
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    let _ = child.kill();
    let _ = child.wait();
    false
}

const PROJECT_SHIMS: &str = r#"
[[codegen.shims]]
class = "RustealProbeActor"
parent = "AActor"
include = ["GameFramework/Actor.h"]
virtuals = [
    "FVector GetVelocity() const",
    "void GetActorEyesViewPoint(FVector &OutLocation, FRotator &OutRotation) const",
]
"#;

fn declare_project_shims(project: &Path) {
    let config = project.join("rusteal.toml");
    let text = std::fs::read_to_string(&config)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", config.display())));

    let base = text.split("\n[[codegen.shims]]").next().unwrap_or(&text);
    let wanted = format!("{}\n{}", base.trim_end(), PROJECT_SHIMS);

    if wanted != text {
        std::fs::write(&config, wanted)
            .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", config.display())));
    }
}

pub fn engine_test() {
    let repo = repo_root();
    let env = DevEnv::load(&repo);
    let work = repo.join("target/engine-test");
    let project = work.join(PROJECT);
    let logs = work.join("logs");
    std::fs::create_dir_all(&logs)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", logs.display())));

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let cli = repo.join("target/debug").join(if cfg!(windows) {
        "rusteal.exe"
    } else {
        "rusteal"
    });

    eprintln!("engine-test: building the CLI");
    run(
        Command::new(&cargo)
            .args(["build", "-p", "rusteal"])
            .current_dir(&repo),
        "cargo build -p rusteal",
        &logs.join("cli.log"),
    );

    if project.join(format!("{PROJECT}.uproject")).is_file() {
        eprintln!("engine-test: updating {}", project.display());
        run(
            Command::new(&cli)
                .arg("setup")
                .arg(&project)
                .stdin(std::process::Stdio::null()),
            "rusteal setup",
            &logs.join("setup.log"),
        );
    } else {
        eprintln!("engine-test: creating {}", project.display());
        run(
            Command::new(&cli)
                .args(["new", PROJECT, "--no-build"])
                .arg("--dir")
                .arg(&work)
                .arg("--runtime-path")
                .arg(&repo)
                .arg("--engine")
                .arg(&env.engine_root)
                .stdin(std::process::Stdio::null()),
            "rusteal new",
            &logs.join("new.log"),
        );
    }

    declare_project_shims(&project);

    copy_sources(
        &repo.join("tests/engine/src"),
        &project.join("Rust").join(CRATE).join("src"),
    );

    if project
        .join("Intermediate/Rusteal/uht/rusteal_classes.json")
        .is_file()
    {
        eprintln!("engine-test: generating the bindings with this CLI");
        run(
            Command::new(&cli)
                .arg("generate")
                .arg(&project)
                .stdin(std::process::Stdio::null()),
            "rusteal generate",
            &logs.join("generate.log"),
        );
    }

    eprintln!("engine-test: building the project (UBT, bindings, cargo)");
    run(
        Command::new(&cli)
            .args(["build", "--all"])
            .arg(&project)
            .stdin(std::process::Stdio::null()),
        "rusteal build --all",
        &logs.join("build.log"),
    );

    let results = work.join("results.txt");
    let _ = std::fs::remove_file(&results);

    eprintln!("engine-test: running the tests in a game, without a window");
    let mut child = Command::new(editor(&env.engine_root))
        .arg(project.join(format!("{PROJECT}.uproject")))
        .args([
            "-game",
            "-nullrhi",
            "-unattended",
            "-nosplash",
            "-nosound",
            "-ExecCmds=rusteal_tests.run",
        ])
        .env("RUSTEAL_TEST_RESULTS", &results)
        .stdout(std::fs::File::create(logs.join("editor.log")).expect("a log file"))
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap_or_else(|e| fail(&format!("cannot start the editor: {e}")));

    if !wait(&mut child) {
        fail(&format!(
            "the editor did not finish in {} seconds; its log is in {}",
            TIMEOUT.as_secs(),
            project.join(format!("Saved/Logs/{PROJECT}.log")).display()
        ));
    }

    let text = std::fs::read_to_string(&results).unwrap_or_default();

    if !text.lines().any(|line| line == "DONE") {
        fail(&format!(
            "the tests did not finish; the editor's log is in {}",
            project.join(format!("Saved/Logs/{PROJECT}.log")).display()
        ));
    }

    let failed = text.lines().filter(|line| line.starts_with("FAIL")).count();
    let passed = text.lines().filter(|line| line.starts_with("PASS")).count();

    for line in text.lines().filter(|line| *line != "DONE") {
        eprintln!("  {line}");
    }

    eprintln!("engine-test: {passed} passed, {failed} failed");

    if failed > 0 {
        std::process::exit(1);
    }
}
