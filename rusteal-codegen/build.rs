// Embeds the hand-written binding extensions (`manual/*.rs`) into the code
// generator, which writes them into every generated `bindings` crate.
//
// They are plain Rust files, copied verbatim: `crate::core_ue`, `crate::engine`
// and `rusteal_core::` all resolve the same way inside the generated crate.

use std::env;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manual_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("manual");
    println!("cargo:rerun-if-changed={}", manual_dir.display());

    let mut files: Vec<(String, PathBuf)> = fs::read_dir(&manual_dir)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", manual_dir.display()))
        .map(|entry| entry.expect("readable directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, path)
        })
        .collect();
    files.sort();

    let mut out = String::from("pub const MANUAL_FILES: &[(&str, &str)] = &[\n");
    for (name, path) in &files {
        let abs = path.to_str().expect("non-UTF8 path").replace('\\', "/");
        writeln!(out, "    ({name:?}, include_str!({abs:?})),").unwrap();
    }
    out.push_str("];\n");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("manual_files.rs");
    fs::write(&out_path, out)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", out_path.display()));
}
