// Hand-written binding extensions: glam conversions, FKey helpers, world and
// widget wrappers.
//
// They live in this crate's `manual/` directory, are embedded at build time and
// written verbatim into every generated `bindings` crate, where they compile
// against the generated modules (`crate::core_ue`, `crate::engine`, ...).

use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/manual_files.rs"));

/// Write `src/manual/`, replacing whatever a previous run left behind.
pub fn write_manual_module(src_dir: &Path) {
    let manual_dir = src_dir.join("manual");
    if manual_dir.exists() {
        std::fs::remove_dir_all(&manual_dir)
            .unwrap_or_else(|e| panic!("Failed to clean {}: {e}", manual_dir.display()));
    }
    std::fs::create_dir_all(&manual_dir)
        .unwrap_or_else(|e| panic!("Failed to create {}: {e}", manual_dir.display()));

    for (name, contents) in MANUAL_FILES {
        std::fs::write(manual_dir.join(name), contents)
            .unwrap_or_else(|e| panic!("Failed to write manual/{name}: {e}"));
    }
    eprintln!("  manual/: {} files", MANUAL_FILES.len());
}
