// Build script: embeds the UE plugin sources and the project templates into
// the binary, so `rusteal setup` and `rusteal new` carry everything they write.
//
// Dual-path resolution:
// - Workspace build: reads from ../../ue_plugin/ (always up-to-date)
// - crates.io build: falls back to ./ue_plugin_embed/ (committed snapshot)

use std::env;
use std::fs;
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Directories to skip when walking plugin sources.
const EXCLUDED_DIRS: &[&str] = &["Generated", "Binaries", "Intermediate", "obj"];

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Dual-path: prefer workspace ue_plugin/, fall back to ue_plugin_embed/
    let workspace_path = manifest_dir.join("..").join("ue_plugin");
    let embed_path = manifest_dir.join("ue_plugin_embed");

    let source_root = if workspace_path.join("Rusteal").exists() {
        workspace_path
    } else if embed_path.join("Rusteal").exists() {
        embed_path
    } else {
        panic!(
            "Cannot find UE plugin sources. \
             Expected either {workspace} or {embed} to contain Rusteal/",
            workspace = workspace_path.display(),
            embed = embed_path.display(),
        );
    };

    let source_root = fs::canonicalize(&source_root)
        .unwrap_or_else(|e| panic!("Failed to canonicalize {}: {e}", source_root.display()));

    // Tell Cargo to rerun if plugin sources change
    println!("cargo:rerun-if-changed={}", source_root.display());

    // Collect all eligible files
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    collect_files(&source_root, &source_root, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));

    // Generate plugin_files.rs
    let out_file = out_dir.join("plugin_files.rs");
    let mut f = fs::File::create(&out_file)
        .unwrap_or_else(|e| panic!("Failed to create {}: {e}", out_file.display()));

    writeln!(f, "pub const PLUGIN_FILES: &[(&str, &[u8])] = &[").unwrap();
    for (rel_path, abs_path) in &files {
        // Use forward slashes for the include_bytes! path (works on all platforms)
        let abs_str = abs_path.to_str().expect("non-UTF8 path").replace('\\', "/");
        writeln!(f, "    ({rel_path:?}, include_bytes!({abs_str:?})),").unwrap();
    }
    writeln!(f, "];").unwrap();

    eprintln!(
        "rusteal build.rs: embedded {} plugin files from {}",
        files.len(),
        source_root.display()
    );

    embed_templates(&manifest_dir, &out_dir);
}

/// Embed `templates/` as `TEMPLATE_FILES: &[(&str, &str)]`: the Tera templates
/// and the verbatim files `rusteal new` writes into a project.
fn embed_templates(manifest_dir: &Path, out_dir: &Path) {
    let templates_dir = manifest_dir.join("templates");
    println!("cargo:rerun-if-changed={}", templates_dir.display());

    let mut files: Vec<(String, PathBuf)> = fs::read_dir(&templates_dir)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", templates_dir.display()))
        .map(|entry| entry.expect("readable directory entry").path())
        .filter(|path| path.is_file())
        .map(|path| (path.file_name().unwrap().to_string_lossy().into_owned(), path))
        .collect();
    files.sort();

    let mut out = String::from("pub const TEMPLATE_FILES: &[(&str, &str)] = &[\n");
    for (name, path) in &files {
        let abs = path.to_str().expect("non-UTF8 path").replace('\\', "/");
        writeln!(out, "    ({name:?}, include_str!({abs:?})),").unwrap();
    }
    out.push_str("];\n");

    let out_file = out_dir.join("template_files.rs");
    fs::write(&out_file, out)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", out_file.display()));

    eprintln!("rusteal build.rs: embedded {} templates", files.len());
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", dir.display()));

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if path.is_dir() {
            if EXCLUDED_DIRS.contains(&name.as_ref()) {
                continue;
            }
            collect_files(root, &path, out);
        } else {
            // Skip .csproj.props (machine-specific)
            if name.ends_with(".csproj.props") {
                continue;
            }

            let rel = path.strip_prefix(root).unwrap();
            // Use forward slashes in relative paths
            let rel_str = rel.to_str().expect("non-UTF8 path").replace('\\', "/");
            out.push((rel_str, path.clone()));
        }
    }
}
