// The UE plugins carry the workspace version: `VersionName` is the version
// itself, `Version` the same version as the integer UE requires there
// (major * 1_000_000 + minor * 1_000 + patch, as `rusteal_ffi::RUSTEAL_VERSION`).
// The release bump rewrites both together with Cargo.toml; this catches a
// descriptor edited out of step.

use std::path::PathBuf;

fn encode(version: &str) -> u64 {
    let parts: Vec<u64> = version.split('.').map(|p| p.parse().unwrap()).collect();
    let [major, minor, patch] = parts[..] else {
        panic!("{version} is not major.minor.patch");
    };
    major * 1_000_000 + minor * 1_000 + patch
}

#[test]
fn plugin_descriptors_carry_the_workspace_version() {
    let version = env!("CARGO_PKG_VERSION");
    let ue_plugin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ue_plugin");
    for plugin in ["Rusteal", "RustealGenerator"] {
        let path = ue_plugin.join(plugin).join(format!("{plugin}.uplugin"));
        let descriptor: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(descriptor["VersionName"], version, "{}", path.display());
        assert_eq!(descriptor["Version"], encode(version), "{}", path.display());
    }
}
