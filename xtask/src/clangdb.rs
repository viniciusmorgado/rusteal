// compile_commands.json for the plugin sources in this checkout.
//
// UBT writes compile commands only for a UE project (`-mode=GenerateClangDatabase`),
// with the paths of the project's copy of the plugins. The entries for the plugin
// sources are re-targeted at `ue_plugin/`:
//
//   - response files (`@file.rsp`) are expanded, so the database is self-contained;
//   - the checkout's `Public/` and `Private/` go first on the include path, so its
//     headers win over the project's copy;
//   - the project's other include paths stay, for what only exists there (UHT's
//     `*.generated.h`, the generated wrappers, engine headers);
//   - the module's shared PCH, which UBT passes only to the unity files as a
//     precompiled `-include-pch X.h.gch`, is force-included as its header `X.h`:
//     clangd cannot read the engine compiler's PCH, and UE sources rely on it;
//   - `-Werror`, dependency output and `-o` are dropped.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use crate::fail;

#[cfg(target_os = "windows")]
const UBT: (&str, &str) = ("Engine/Build/BatchFiles/Build.bat", "Win64");
#[cfg(target_os = "linux")]
const UBT: (&str, &str) = ("Engine/Build/BatchFiles/Linux/Build.sh", "Linux");
#[cfg(target_os = "macos")]
const UBT: (&str, &str) = ("Engine/Build/BatchFiles/Mac/Build.sh", "Mac");

/// Run UBT's GenerateClangDatabase on the project's editor target; returns the database.
pub fn generate_ubt_db(engine_root: &Path, project: &Path, name: &str, repo: &Path) -> PathBuf {
    let (script, platform) = UBT;
    let out_dir = repo.join("target").join("xtask").join("clangdb");
    std::fs::create_dir_all(&out_dir)
        .unwrap_or_else(|e| fail(&format!("cannot create {}: {e}", out_dir.display())));
    let uproject = project.join(format!("{name}.uproject"));

    eprintln!("xtask: generating the UBT compile database");
    let status = Command::new(engine_root.join(script))
        .arg("-mode=GenerateClangDatabase")
        .arg(format!("-project={}", uproject.display()))
        .arg(format!("{name}Editor"))
        .arg(platform)
        .arg("Development")
        .arg(format!("-OutputDir={}", out_dir.display()))
        .status()
        .unwrap_or_else(|e| fail(&format!("cannot run UBT: {e}")));
    if !status.success() {
        fail("UBT could not generate the compile database");
    }
    out_dir.join("compile_commands.json")
}

/// Write the entries of the plugin sources that exist in `checkout_plugins`
/// to `out`; returns how many.
pub fn write_for_checkout(
    ubt_db: &Path,
    project_plugins: &Path,
    checkout_plugins: &Path,
    out: &Path,
) -> usize {
    let text = std::fs::read_to_string(ubt_db)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", ubt_db.display())));
    let entries: Vec<Value> = serde_json::from_str(&text)
        .unwrap_or_else(|e| fail(&format!("cannot parse {}: {e}", ubt_db.display())));
    let project_plugins = canonical(project_plugins);
    let checkout_plugins = canonical(checkout_plugins);

    let mut result = Vec::new();
    for entry in &entries {
        let (Some(file), Some(command), Some(directory)) = (
            entry["file"].as_str(),
            entry["command"].as_str(),
            entry["directory"].as_str(),
        ) else {
            continue;
        };
        let Ok(rel) = Path::new(file).strip_prefix(&project_plugins) else {
            continue;
        };
        let source = checkout_plugins.join(rel);
        if !source.exists() {
            continue; // generated sources, or a plugin the checkout does not have
        }

        // <Plugin>/Source/<Module>/...: that module in the checkout.
        let parts: Vec<_> = rel.components().collect();
        let module_dir = if parts.len() > 3 {
            checkout_plugins.join(parts[..3].iter().collect::<PathBuf>())
        } else {
            source.parent().unwrap_or(&checkout_plugins).to_path_buf()
        };

        let directory = Path::new(directory);
        let args = strip(expand(command, directory), file);
        let mut front = vec![
            format!("-I{}", module_dir.join("Public").display()),
            format!("-I{}", module_dir.join("Private").display()),
        ];
        if !args.iter().any(|a| a == "-include-pch")
            && let Some(pch) = entry["output"].as_str().and_then(|o| shared_pch(Path::new(o), directory))
        {
            front.push("-include".into());
            front.push(pch);
        }

        let mut arguments = Vec::with_capacity(args.len() + front.len() + 1);
        arguments.extend(args.first().cloned());
        arguments.extend(front);
        arguments.extend(args.into_iter().skip(1));
        arguments.push(source.display().to_string());
        result.push(json!({
            "directory": directory,
            "file": source,
            "arguments": arguments,
        }));
    }

    let count = result.len();
    let text = serde_json::to_string_pretty(&Value::Array(result)).expect("serializable") + "\n";
    std::fs::write(out, text).unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", out.display())));
    count
}

/// Command-line tokens: whitespace-separated, double quotes group and are removed.
/// No escapes, so Windows paths keep their backslashes.
fn split_args(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in text.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    tokens.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        tokens.push(current);
    }
    tokens
}

/// Tokens of `text`, with every `@file.rsp` replaced by the file's tokens.
fn expand_tokens(text: &str, directory: &Path, out: &mut Vec<String>) {
    for token in split_args(text) {
        match token.strip_prefix('@') {
            Some(rsp) => {
                let rsp = directory.join(rsp);
                match std::fs::read_to_string(&rsp) {
                    Ok(contents) => expand_tokens(&contents, directory, out),
                    Err(e) => fail(&format!(
                        "cannot read {}: {e}. Build the development project first.",
                        rsp.display()
                    )),
                }
            }
            None => out.push(token),
        }
    }
}

fn expand(command: &str, directory: &Path) -> Vec<String> {
    let mut out = Vec::new();
    expand_tokens(command, directory, &mut out);
    out
}

/// Drop the input file, the outputs and warnings-as-errors.
fn strip(args: Vec<String>, source: &str) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "-o" || arg == "-MF" {
            skip_next = true;
            continue;
        }
        if arg == source || arg == "-MD" || arg.starts_with("-MF") || arg == "-Werror" || arg == "/WX" {
            continue;
        }
        out.push(arg);
    }
    out
}

/// The shared PCH header the module's unity files use, read from their response files.
fn shared_pch(output: &Path, directory: &Path) -> Option<String> {
    let dir = output.parent()?;
    let mut rsps: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            name.starts_with("Module.") && name.ends_with(".rsp")
        })
        .collect();
    rsps.sort();
    for rsp in rsps {
        let mut tokens = Vec::new();
        expand_tokens(&std::fs::read_to_string(&rsp).ok()?, directory, &mut tokens);
        for pair in tokens.windows(2) {
            if pair[0] == "-include-pch"
                && let Some(header) = pair[1].strip_suffix(".gch")
                && Path::new(header).exists()
            {
                return Some(header.to_string());
            }
        }
    }
    None
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_args_groups_quotes_and_keeps_backslashes() {
        assert_eq!(
            split_args(r#"-include "C:\A B\x.h" -I/usr/include -MF"/p q/d""#),
            vec![r"-include", r"C:\A B\x.h", "-I/usr/include", "-MF/p q/d"]
        );
    }

    #[test]
    fn strip_drops_input_and_outputs() {
        let args = ["clang++", "-c", "a.cpp", "-MD", "-MF/x.d", "-o", "a.o", "-Werror", "-Wall"]
            .map(String::from)
            .to_vec();
        assert_eq!(strip(args, "a.cpp"), vec!["clang++", "-c", "-Wall"]);
    }
}
