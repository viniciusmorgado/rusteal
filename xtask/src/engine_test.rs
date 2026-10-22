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
        if entry.path().is_dir() {
            copy_sources(&entry.path(), &to.join(entry.file_name()));
            continue;
        }

        std::fs::copy(entry.path(), to.join(entry.file_name()))
            .unwrap_or_else(|e| fail(&format!("cannot copy {}: {e}", entry.path().display())));
    }
}

fn wait(child: &mut Child) -> bool {
    wait_status(child).is_some()
}

fn wait_status(child: &mut Child) -> Option<std::process::ExitStatus> {
    let started = Instant::now();

    while started.elapsed() < TIMEOUT {
        if let Ok(Some(status)) = child.try_wait() {
            return Some(status);
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    let _ = child.kill();
    let _ = child.wait();
    None
}

const COMMANDLET: &str = "VirtualsCommandlet";
const COMMANDLET_EXIT: i32 = 3;

fn run_commandlet(engine: &Path, project: &Path, logs: &Path) -> String {
    let mut child = Command::new(editor(engine))
        .arg(project.join(format!("{PROJECT}.uproject")))
        .args([
            &format!("-run={COMMANDLET}"),
            "rusteal-probe",
            "-nullrhi",
            "-unattended",
            "-nosplash",
            "-nosound",
        ])
        .stdout(std::fs::File::create(logs.join("commandlet.log")).expect("a log file"))
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap_or_else(|e| fail(&format!("cannot start the commandlet: {e}")));

    match wait_status(&mut child).map(|status| status.code()) {
        Some(Some(COMMANDLET_EXIT)) => format!("PASS rust_commandlet_runs ({COMMANDLET})"),
        Some(code) => format!(
            "FAIL rust_commandlet_runs: {COMMANDLET} exited with {code:?}, not \
             {COMMANDLET_EXIT}; its log is in {}",
            logs.join("commandlet.log").display()
        ),
        None => format!(
            "FAIL rust_commandlet_runs: {COMMANDLET} did not finish in {} seconds",
            TIMEOUT.as_secs()
        ),
    }
}

const PLUGIN: &str = "RustealProbe";
const PLUGIN_CRATE: &str = "rusteal-probe";
const PLUGIN_EDITOR_CRATE: &str = "rusteal-probe-editor";

fn add_probe_plugin(cli: &Path, project: &Path, repo: &Path, logs: &Path) {
    let plugin = project.join("Plugins").join(PLUGIN);

    if !plugin.join("rusteal.toml").is_file() {
        eprintln!("engine-test: adding the {PLUGIN} plugin");
        run(
            Command::new(cli)
                .args(["plugin", "new", PLUGIN, "--no-build", "--project"])
                .arg(project)
                .stdin(std::process::Stdio::null()),
            "rusteal plugin new",
            &logs.join("plugin.log"),
        );
    }

    if !plugin.join("Rust").join(PLUGIN_EDITOR_CRATE).is_dir() {
        eprintln!("engine-test: adding {PLUGIN}'s editor library");
        run(
            Command::new(cli)
                .args(["plugin", "editor", PLUGIN, "--no-build", "--project"])
                .arg(project)
                .stdin(std::process::Stdio::null()),
            "rusteal plugin editor",
            &logs.join("plugin-editor.log"),
        );
    }

    copy_sources(
        &repo.join("tests/engine/plugin/src"),
        &plugin.join("Rust").join(PLUGIN_CRATE).join("src"),
    );
    copy_sources(
        &repo.join("tests/engine/plugin/editor/src"),
        &plugin.join("Rust").join(PLUGIN_EDITOR_CRATE).join("src"),
    );

    let config = plugin.join("rusteal.toml");
    let text = std::fs::read_to_string(&config)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", config.display())));

    if !text.contains("\nBlutility = ") {
        let line = "Blutility = { module = \"blutility\", feature = \"editor\", classes = [\"EditorUtilityObject\", \"AssetActionUtility\", \"ActorActionUtility\", \"EditorUtilityWidget\", \"EditorUtilitySubsystem\", \"EditorUtilityLibrary\"] }";
        let updated = text.replacen(
            "\n[editor.codegen.blocklist]",
            &format!("{line}\n\n[editor.codegen.blocklist]"),
            1,
        );
        std::fs::write(&config, updated)
            .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", config.display())));
    }
}

fn library_file(stem: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}

fn run_editor(
    cargo: &str,
    engine: &Path,
    project: &Path,
    repo: &Path,
    work: &Path,
    logs: &Path,
) -> Vec<String> {
    let plugin = project.join("Plugins").join(PLUGIN);
    let source = plugin
        .join("Rust")
        .join(PLUGIN_CRATE)
        .join("src")
        .join("probe.rs");
    let deployed = plugin
        .join("Binaries")
        .join(if cfg!(target_os = "windows") {
            "Win64"
        } else if cfg!(target_os = "macos") {
            "Mac"
        } else {
            "Linux"
        })
        .join(library_file(&format!("rusteal_{PLUGIN}")));

    let reload = work.join("reload");
    std::fs::create_dir_all(&reload)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", reload.display())));
    let variant = reload.join(library_file("rusteal_probe_variant"));
    let original = reload.join(library_file("rusteal_probe_original"));

    std::fs::copy(repo.join("tests/engine/plugin/reload/probe.rs"), &source)
        .unwrap_or_else(|e| fail(&format!("cannot write the reload variant: {e}")));

    let target_dir = work.join("reload-target");
    let manifest = plugin.join("Rust").join("Cargo.toml");
    let built = Command::new(cargo)
        .args(["rustc", "--manifest-path"])
        .arg(&manifest)
        .args([
            "-p",
            PLUGIN_CRATE,
            "--lib",
            "--crate-type",
            "cdylib",
            "--target-dir",
        ])
        .arg(&target_dir)
        .stdout(std::fs::File::create(logs.join("reload-build.log")).expect("a log file"))
        .stderr(std::fs::File::create(logs.join("reload-build.err.log")).expect("a log file"))
        .status();

    std::fs::copy(repo.join("tests/engine/plugin/src/probe.rs"), &source)
        .unwrap_or_else(|e| fail(&format!("cannot restore the plugin's source: {e}")));

    if !built.is_ok_and(|status| status.success()) {
        return vec![format!(
            "FAIL chained_reload_rebuilds_the_derived_game_class: the plugin's variant did not \
             build; see {}",
            logs.join("reload-build.err.log").display()
        )];
    }

    let _ = std::fs::copy(
        target_dir.join("debug").join(library_file("rusteal_probe")),
        &variant,
    );
    let _ = std::fs::copy(&deployed, &original);

    let results = work.join("reload_results.txt");
    let _ = std::fs::remove_file(&results);

    let mut child = Command::new(editor(engine))
        .arg(project.join(format!("{PROJECT}.uproject")))
        .args([
            "-nullrhi",
            "-unattended",
            "-nosplash",
            "-nosound",
            &format!(
                "-ExecCmds=rusteal_tests.reload_prepare,Rusteal.Reload RustealProbe,rusteal_tests.reload_verify,rusteal_editor_tests.run,py {},py {},QUIT_EDITOR",
                repo.join("tests/engine/plugin/editor/python_names.py").display(),
                repo.join("tests/engine/plugin/editor/factories.py").display()
            ),
        ])
        .env("RUSTEAL_TEST_RESULTS", &results)
        .env("RUSTEAL_RELOAD_VARIANT", &variant)
        .env("RUSTEAL_RELOAD_TARGET", &deployed)
        .stdout(std::fs::File::create(logs.join("reload-editor.log")).expect("a log file"))
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap_or_else(|e| fail(&format!("cannot start the editor: {e}")));

    let finished = wait(&mut child);
    let staged = deployed.with_extension("restore");
    let _ = std::fs::copy(&original, &staged);
    let _ = std::fs::rename(&staged, &deployed);

    let text = std::fs::read_to_string(&results).unwrap_or_default();
    let lines: Vec<String> = text
        .lines()
        .filter(|line| *line != "DONE")
        .map(str::to_string)
        .collect();

    if lines.is_empty() {
        vec![format!(
            "FAIL chained_reload_rebuilds_the_derived_game_class: the editor {}; its log is in {}",
            if finished {
                "wrote no results"
            } else {
                "did not finish"
            },
            project.join(format!("Saved/Logs/{PROJECT}.log")).display()
        )]
    } else {
        lines
    }
}

fn run_net_editor(
    engine: &Path,
    project: &Path,
    repo: &Path,
    work: &Path,
    logs: &Path,
) -> Vec<String> {
    let results = work.join("net_results.txt");
    let _ = std::fs::remove_file(&results);

    let mut child = Command::new(editor(engine))
        .arg(project.join(format!("{PROJECT}.uproject")))
        .args([
            "-nullrhi",
            "-unattended",
            "-nosplash",
            "-nosound",
            &format!(
                "-ExecCmds=rusteal_tests.net_configure_pie,py {}",
                repo.join("tests/engine/net/start_pie.py").display()
            ),
        ])
        .env("RUSTEAL_NET_TEST_RESULTS", &results)
        .stdout(std::fs::File::create(logs.join("net-editor.log")).expect("a log file"))
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap_or_else(|e| fail(&format!("cannot start the editor: {e}")));

    let finished = wait(&mut child);
    let text = std::fs::read_to_string(&results).unwrap_or_default();
    let lines: Vec<String> = text
        .lines()
        .filter(|line| *line != "DONE")
        .map(str::to_string)
        .collect();

    if lines.is_empty() {
        vec![format!(
            "FAIL net_checks: the editor {}; its log is in {}",
            if finished {
                "wrote no results"
            } else {
                "did not finish"
            },
            project.join(format!("Saved/Logs/{PROJECT}.log")).display()
        )]
    } else {
        lines
    }
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

const PROJECT_FEATURES: &str = "features = [\"core\", \"engine\", \"input\", \"slate\", \"umg\", \"enhanced-input\", \"gameplay-tags\", \"gameplay-abilities\"]";

fn declare_project_shims(project: &Path) {
    let config = project.join("rusteal.toml");
    let text = std::fs::read_to_string(&config)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", config.display())));

    let base = text.split("\n[[codegen.shims]]").next().unwrap_or(&text);
    let base = base
        .lines()
        .map(|line| {
            if line.starts_with("features = [") {
                PROJECT_FEATURES
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
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
    add_probe_plugin(&cli, &project, &repo, &logs);

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

    eprintln!("engine-test: running a Rust commandlet");
    let commandlet = run_commandlet(&env.engine_root, &project, &logs);

    eprintln!(
        "engine-test: reloading the plugin and running the editor checks in an editor, without a window"
    );
    let reload = run_editor(&cargo, &env.engine_root, &project, &repo, &work, &logs);

    eprintln!(
        "engine-test: running a dedicated server and two clients in Play In Editor, without a window"
    );
    let net = run_net_editor(&env.engine_root, &project, &repo, &work, &logs);

    let lines: Vec<&str> = text
        .lines()
        .filter(|line| *line != "DONE")
        .chain(std::iter::once(commandlet.as_str()))
        .chain(reload.iter().map(String::as_str))
        .chain(net.iter().map(String::as_str))
        .collect();

    let failed = lines.iter().filter(|line| line.starts_with("FAIL")).count();
    let passed = lines.iter().filter(|line| line.starts_with("PASS")).count();

    for line in &lines {
        eprintln!("  {line}");
    }

    eprintln!("engine-test: {passed} passed, {failed} failed");

    if failed > 0 {
        std::process::exit(1);
    }
}

const STATIC_CHECKS: &[&str] = &[
    "rust_class_is_registered",
    "spawn_rust_actor",
    "plugin_class_is_reached_by_reflection",
    "game_uses_a_plugin_class_through_its_bindings",
    "game_class_derives_from_a_plugin_class",
];

fn packaged_game(package: &Path) -> Option<PathBuf> {
    let name = if cfg!(target_os = "windows") {
        format!("{PROJECT}.exe")
    } else if cfg!(target_os = "macos") {
        format!("{PROJECT}.app/Contents/MacOS/{PROJECT}")
    } else {
        format!("{PROJECT}.sh")
    };

    let direct = package.join(&name);

    if direct.exists() {
        return Some(direct);
    }

    std::fs::read_dir(package)
        .ok()?
        .flatten()
        .map(|entry| entry.path().join(&name))
        .find(|path| path.exists())
}

pub fn static_test() {
    let repo = repo_root();
    let work = repo.join("target/engine-test");
    let project = work.join(PROJECT);
    let logs = work.join("logs");
    let package = work.join("static-package");

    if !project.join(format!("{PROJECT}.uproject")).is_file() {
        fail("run `cargo xtask engine-test` first: it creates and builds the test project");
    }

    let cli = repo.join("target/debug").join(if cfg!(windows) {
        "rusteal.exe"
    } else {
        "rusteal"
    });

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    eprintln!("static-test: building the CLI and updating {PROJECT}'s plugins");
    run(
        Command::new(&cargo)
            .args(["build", "-p", "rusteal"])
            .current_dir(&repo),
        "cargo build -p rusteal",
        &logs.join("cli.log"),
    );
    run(
        Command::new(&cli)
            .arg("setup")
            .arg(&project)
            .stdin(std::process::Stdio::null()),
        "rusteal setup",
        &logs.join("setup.log"),
    );

    copy_sources(
        &repo.join("tests/engine/src"),
        &project.join("Rust").join(CRATE).join("src"),
    );
    copy_sources(
        &repo.join("tests/engine/plugin/src"),
        &project
            .join("Plugins")
            .join(PLUGIN)
            .join("Rust")
            .join(PLUGIN_CRATE)
            .join("src"),
    );

    eprintln!("static-test: packaging {PROJECT} with every Rust library linked into the game");
    let _ = std::fs::remove_dir_all(&package);
    run(
        Command::new(&cli)
            .args(["package", "--static", "--output"])
            .arg(&package)
            .arg(&project)
            .stdin(std::process::Stdio::null()),
        "rusteal package --static",
        &logs.join("static-package.log"),
    );

    let game = packaged_game(&package)
        .unwrap_or_else(|| fail(&format!("no packaged game in {}", package.display())));

    let results = work.join("static_results.txt");
    let _ = std::fs::remove_file(&results);

    eprintln!("static-test: running the packaged game, without a window");
    let mut child = Command::new(&game)
        .args([
            "-nullrhi",
            "-unattended",
            "-nosound",
            "-ExecCmds=rusteal_tests.run",
        ])
        .env("RUSTEAL_TEST_RESULTS", &results)
        .env("RUSTEAL_TEST_FILTER", STATIC_CHECKS.join(","))
        .stdout(std::fs::File::create(logs.join("static-game.log")).expect("a log file"))
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap_or_else(|e| fail(&format!("cannot start {}: {e}", game.display())));

    if !wait(&mut child) {
        fail(&format!(
            "the packaged game did not finish in {} seconds; its output is in {}",
            TIMEOUT.as_secs(),
            logs.join("static-game.log").display()
        ));
    }

    let text = std::fs::read_to_string(&results).unwrap_or_default();

    if !text.lines().any(|line| line == "DONE") {
        fail(&format!(
            "the packaged game wrote no results; its output is in {}",
            logs.join("static-game.log").display()
        ));
    }

    let failed = text.lines().filter(|line| line.starts_with("FAIL")).count();
    let passed = text.lines().filter(|line| line.starts_with("PASS")).count();

    for line in text.lines().filter(|line| *line != "DONE") {
        eprintln!("  {line}");
    }

    eprintln!("static-test: {passed} passed, {failed} failed");

    if failed > 0 || passed != STATIC_CHECKS.len() {
        std::process::exit(1);
    }
}
