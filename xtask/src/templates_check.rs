use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::dev_env::DevEnv;
use crate::{fail, repo_root};

const DUMP_PROJECT: &str = "TemplDump";
const DUMP_FILES: [&str; 3] = [
    "rusteal_classes.json",
    "rusteal_structs.json",
    "rusteal_enums.json",
];

fn log_to(path: &Path) -> (Stdio, Stdio) {
    let file = std::fs::File::create(path)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", path.display())));

    (
        Stdio::from(file.try_clone().expect("a log file handle")),
        Stdio::from(file),
    )
}

fn succeeds(command: &mut Command, log: &Path) -> bool {
    let (out, err) = log_to(log);

    command
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .status()
        .is_ok_and(|s| s.success())
}

fn first_errors(log: &Path) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains("error") && !line.contains("could not compile"))
        .take(6)
        .map(str::to_string)
        .collect()
}

fn variants(repo: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();

    for template in sorted_dirs(&repo.join("rusteal-cli/templates")) {
        for variant in sorted_dirs(&template) {
            out.push((file_name(&template), file_name(&variant)));
        }
    }

    out
}

fn plugin_templates(repo: &Path) -> Vec<String> {
    sorted_dirs(&repo.join("rusteal-cli/plugin_templates"))
        .iter()
        .map(|dir| file_name(dir))
        .collect()
}

fn sorted_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", dir.display())))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();

    dirs.sort();
    dirs
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .expect("a directory name")
        .to_string_lossy()
        .into_owned()
}

fn project_name(template: &str, variant: &str) -> String {
    let name: String = format!("T{template}{variant}")
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();

    name.chars().take(20).collect()
}

fn plugin_name(template: &str) -> String {
    let mut chars = template.chars();
    let capitalized: String = chars
        .next()
        .map(|c| c.to_ascii_uppercase())
        .into_iter()
        .chain(chars)
        .collect();

    match template {
        "blank" | "editor" => format!("P{capitalized}"),
        _ => capitalized,
    }
}

fn ensure_dump(cli: &Path, env: &DevEnv, work: &Path, logs: &Path, repo: &Path) -> PathBuf {
    let dump = work.join("full-dump");

    if DUMP_FILES.iter().all(|f| dump.join(f).is_file()) {
        return dump;
    }

    eprintln!("templates-check: exporting the engine's reflection with every feature");
    let project = work.join(DUMP_PROJECT);

    if !project.join(format!("{DUMP_PROJECT}.uproject")).is_file()
        && !succeeds(
            Command::new(cli)
                .args(["new", DUMP_PROJECT, "--template", "vr", "--no-build"])
                .arg("--dir")
                .arg(work)
                .arg("--runtime-path")
                .arg(repo)
                .arg("--engine")
                .arg(&env.engine_root),
            &logs.join("dump.new.log"),
        )
    {
        fail("rusteal new failed for the reflection project");
    }

    let config_path = project.join("rusteal.toml");
    let config = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", config_path.display())));

    let mut features: Vec<String> = config
        .lines()
        .filter_map(|line| line.trim().split_once("feature = \""))
        .filter_map(|(_, rest)| rest.split('"').next().map(str::to_string))
        .collect();

    features.sort();
    features.dedup();

    let line = format!(
        "features = [{}]",
        features
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let rewritten: Vec<String> = config
        .lines()
        .map(|l| {
            if l.starts_with("features = ") {
                line.clone()
            } else {
                l.to_string()
            }
        })
        .collect();

    std::fs::write(&config_path, rewritten.join("\n") + "\n")
        .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", config_path.display())));

    if !succeeds(
        Command::new(cli).args(["build", "--all"]).arg(&project),
        &logs.join("dump.build.log"),
    ) {
        fail(&format!(
            "the reflection project did not build; its output is in {}",
            logs.join("dump.build.log").display()
        ));
    }

    std::fs::create_dir_all(&dump)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", dump.display())));

    for file in DUMP_FILES {
        std::fs::copy(
            project.join("Intermediate/Rusteal/uht").join(file),
            dump.join(file),
        )
        .unwrap_or_else(|e| fail(&format!("cannot copy {file}: {e}")));
    }

    dump
}

fn generate_and_check(
    cli: &Path,
    project: &Path,
    dump: &Path,
    manifests: &[PathBuf],
    target: &Path,
    log: &Path,
) -> Result<(), Vec<String>> {
    let uht = project.join("Intermediate/Rusteal/uht");
    std::fs::create_dir_all(&uht)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", uht.display())));

    for file in DUMP_FILES {
        std::fs::copy(dump.join(file), uht.join(file))
            .unwrap_or_else(|e| fail(&format!("cannot copy {file}: {e}")));
    }

    if !succeeds(Command::new(cli).arg("generate").arg(project), log) {
        return Err(first_errors(log));
    }

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    for manifest in manifests {
        if !succeeds(
            Command::new(&cargo)
                .args(["check", "--message-format", "short", "--manifest-path"])
                .arg(manifest)
                .env("CARGO_TARGET_DIR", target),
            log,
        ) {
            return Err(first_errors(log));
        }
    }

    Ok(())
}

pub fn templates_check(only: &[String]) {
    let repo = repo_root();
    let env = DevEnv::load(&repo);
    let work = repo.join("target/engine-test");
    let projects = work.join("templates");
    let logs = work.join("logs/templates");
    let target = work.join("templates-target");

    std::fs::create_dir_all(&logs)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", logs.display())));

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let cli = repo.join("target/debug").join(if cfg!(windows) {
        "rusteal.exe"
    } else {
        "rusteal"
    });

    eprintln!("templates-check: building the CLI");

    if !succeeds(
        Command::new(&cargo)
            .args(["build", "-p", "rusteal"])
            .current_dir(&repo),
        &logs.join("cli.log"),
    ) {
        fail("cargo build -p rusteal failed");
    }

    let dump = ensure_dump(&cli, &env, &work, &logs, &repo);
    let wanted = |name: &str| only.is_empty() || only.iter().any(|o| o == name);
    let mut failed = 0;
    let mut passed = 0;

    let mut report = |name: &str, result: Result<(), Vec<String>>| match result {
        Ok(()) => {
            passed += 1;
            eprintln!("  PASS {name}");
        }
        Err(errors) => {
            failed += 1;
            eprintln!("  FAIL {name}");

            for line in errors {
                eprintln!("       {line}");
            }
        }
    };

    for (template, variant) in variants(&repo) {
        let name = format!("{template}:{variant}");

        if !wanted(&name) {
            continue;
        }

        let project_name = project_name(&template, &variant);
        let project = projects.join(&project_name);
        let log = logs.join(format!("{project_name}.log"));
        let _ = std::fs::remove_dir_all(&project);

        let mut new = Command::new(&cli);
        new.args(["new", &project_name, "--template", &template, "--no-build"])
            .arg("--dir")
            .arg(&projects)
            .arg("--runtime-path")
            .arg(&repo)
            .arg("--engine")
            .arg(&env.engine_root);

        if variant != "base" {
            new.args(["--variant", &variant]);
        }

        let result = if succeeds(&mut new, &log) {
            generate_and_check(
                &cli,
                &project,
                &dump,
                &[project.join("Rust/Cargo.toml")],
                &target,
                &log,
            )
        } else {
            Err(first_errors(&log))
        };

        report(&name, result);
    }

    let plugins: Vec<String> = plugin_templates(&repo)
        .into_iter()
        .filter(|t| wanted(&format!("plugin:{t}")))
        .collect();

    if !plugins.is_empty() {
        let host = projects.join("PluginHost");
        let log = logs.join("PluginHost.log");
        let _ = std::fs::remove_dir_all(&host);

        if !succeeds(
            Command::new(&cli)
                .args(["new", "PluginHost", "--no-build"])
                .arg("--dir")
                .arg(&projects)
                .arg("--runtime-path")
                .arg(&repo)
                .arg("--engine")
                .arg(&env.engine_root),
            &log,
        ) {
            fail(&format!(
                "rusteal new failed for the plugin host; see {}",
                log.display()
            ));
        }

        let mut manifests = Vec::new();

        for template in &plugins {
            let plugin = plugin_name(template);
            let plugin_log = logs.join(format!("plugin-{template}.log"));

            if succeeds(
                Command::new(&cli)
                    .args([
                        "plugin",
                        "new",
                        &plugin,
                        "--template",
                        template,
                        "--no-build",
                    ])
                    .current_dir(&host),
                &plugin_log,
            ) {
                manifests.push((
                    template,
                    host.join("Plugins").join(&plugin).join("Rust/Cargo.toml"),
                ));
            } else {
                report(
                    &format!("plugin:{template}"),
                    Err(first_errors(&plugin_log)),
                );
            }
        }

        for (template, manifest) in manifests {
            let plugin_log = logs.join(format!("plugin-{template}.log"));
            report(
                &format!("plugin:{template}"),
                generate_and_check(
                    &cli,
                    &host,
                    &dump,
                    std::slice::from_ref(&manifest),
                    &target,
                    &plugin_log,
                ),
            );
        }
    }

    eprintln!("templates-check: {passed} passed, {failed} failed");

    if failed > 0 {
        std::process::exit(1);
    }
}
