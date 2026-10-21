use std::path::PathBuf;

use rusteal_codegen::cpp_gen::shims::{self, ShimFile};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn generated() -> (String, String) {
    let text = std::fs::read_to_string(repo().join("rusteal-codegen/shims/engine.toml")).unwrap();
    let file: ShimFile = toml::from_str(&text).unwrap();

    let header = shims::generate_header(&file.shim, "RustealShims", "RUSTEAL_API").unwrap();
    let source = shims::generate_source(&file.shim, "RustealShims").unwrap();

    (header, source)
}

fn plugin_file(rel: &str) -> PathBuf {
    repo().join("ue_plugin/Rusteal/Source/Rusteal").join(rel)
}

#[test]
fn plugin_shims_match_the_curated_list() {
    let (header, source) = generated();

    assert_eq!(
        std::fs::read_to_string(plugin_file("Public/RustealShims.h")).unwrap_or_default(),
        header,
        "RustealShims.h is stale: cargo test -p rusteal-codegen --test shims -- --ignored regenerate_shims"
    );
    assert_eq!(
        std::fs::read_to_string(plugin_file("Private/RustealShims.cpp")).unwrap_or_default(),
        source,
        "RustealShims.cpp is stale: cargo test -p rusteal-codegen --test shims -- --ignored regenerate_shims"
    );
}

#[test]
#[ignore = "rewrites the plugin's shims from rusteal-codegen/shims/engine.toml"]
fn regenerate_shims() {
    let (header, source) = generated();

    std::fs::write(plugin_file("Public/RustealShims.h"), header).unwrap();
    std::fs::write(plugin_file("Private/RustealShims.cpp"), source).unwrap();
}
