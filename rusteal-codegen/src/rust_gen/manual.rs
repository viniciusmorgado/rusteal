// Hand-written binding extensions: glam conversions, FKey helpers, world and
// widget wrappers.
//
// They live in this crate's `manual/` directory, are embedded at build time and
// written verbatim into every generated `bindings` crate, where they compile
// against the generated modules (`crate::core_ue`, `crate::engine`, ...).

use std::path::Path;

include!(concat!(env!("OUT_DIR"), "/manual_files.rs"));

/// Write `src/manual/`, removing whatever else a previous run left behind.
pub fn write_manual_module(src_dir: &Path) {
    let manual_dir = src_dir.join("manual");
    if let Ok(entries) = std::fs::read_dir(&manual_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if !MANUAL_FILES.iter().any(|(file, _)| name == *file) {
                let path = entry.path();
                let removed = if path.is_dir() {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
                removed.unwrap_or_else(|e| panic!("Failed to clean {}: {e}", path.display()));
            }
        }
    }
    std::fs::create_dir_all(&manual_dir)
        .unwrap_or_else(|e| panic!("Failed to create {}: {e}", manual_dir.display()));

    for (name, contents) in MANUAL_FILES {
        crate::write_if_changed(&manual_dir.join(name), contents)
            .unwrap_or_else(|e| panic!("Failed to write manual/{name}: {e}"));
    }
    eprintln!("  manual/: {} files", MANUAL_FILES.len());
}

/// The Cargo features `manual/` gates its modules on (`#[cfg(feature = "...")]`).
/// A project enables only some of them, so the generated manifest declares all
/// of them as expected `cfg` values.
pub fn gating_features() -> std::collections::BTreeSet<&'static str> {
    const MARKER: &str = "feature = \"";
    let mut features = std::collections::BTreeSet::new();
    for (_, contents) in MANUAL_FILES {
        let mut rest = *contents;
        while let Some(at) = rest.find(MARKER) {
            rest = &rest[at + MARKER.len()..];
            if let Some(end) = rest.find('"') {
                features.insert(&rest[..end]);
                rest = &rest[end..];
            }
        }
    }
    features
}
