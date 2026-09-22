// Project configuration: `<Project>/rusteal.toml`, next to the .uproject.
//
// The file is versioned with the project and holds only what varies between
// projects: the game crate and the codegen module selection. Everything else
// is a fixed layout (see `ProjectLayout`). The engine location is per machine
// and lives in the CLI's own configuration, not here.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Name of the project configuration file.
pub const FILE_NAME: &str = "rusteal.toml";

/// Top-level `rusteal.toml`.
#[derive(Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectSection,
    pub codegen: CodegenConfig,
}

#[derive(Deserialize)]
pub struct ProjectSection {
    /// Cargo package, a cdylib member of `Rust/`, deployed as the Rusteal library.
    #[serde(rename = "crate")]
    pub crate_name: String,
    /// Extra cargo features to build it with.
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Deserialize)]
pub struct CodegenConfig {
    pub features: Vec<String>,
    pub modules: HashMap<String, ModuleMapping>,
    #[serde(default)]
    pub blocklist: Blocklist,
}

#[derive(Deserialize)]
pub struct ModuleMapping {
    pub module: String,
    pub feature: String,
}

#[derive(Deserialize, Default)]
pub struct Blocklist {
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub structs: Vec<String>,
    /// Function blocklist in "Class.Function" format.
    #[serde(default)]
    pub functions: Vec<String>,
}

impl Blocklist {
    /// Parse function blocklist entries into (class, function) tuples.
    pub fn function_tuples(&self) -> Vec<(String, String)> {
        self.functions
            .iter()
            .filter_map(|entry| {
                let (class, func) = entry.split_once('.')?;
                Some((class.to_string(), func.to_string()))
            })
            .collect()
    }
}

impl ProjectConfig {
    /// Read `rusteal.toml` from the project root.
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(FILE_NAME);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("cannot parse {}: {e}", path.display()))
    }
}

/// Where things live inside a Rusteal project. Fixed by convention so that the
/// config, the CLI and the plugin agree without a single path in `rusteal.toml`.
pub struct ProjectLayout {
    /// The directory holding the .uproject.
    pub root: PathBuf,
}

impl ProjectLayout {
    pub fn new(root: &Path) -> Self {
        Self { root: root.to_path_buf() }
    }

    /// The .uproject file, if the root holds one.
    pub fn uproject(&self) -> Option<PathBuf> {
        find_uproject(&self.root)
    }

    /// Cargo workspace with the game crate(s) and the bindings.
    pub fn rust_workspace(&self) -> PathBuf {
        self.root.join("Rust")
    }

    /// The generated `bindings` crate.
    pub fn bindings_crate(&self) -> PathBuf {
        self.rust_workspace().join("bindings")
    }

    /// The generated C++ wrappers, compiled into the Rusteal plugin.
    pub fn cpp_generated(&self) -> PathBuf {
        self.root.join("Plugins/Rusteal/Source/Rusteal/Generated")
    }

    /// The reflection JSON the codegen reads: build output, not versioned.
    pub fn uht_json(&self) -> PathBuf {
        self.root.join("Intermediate/Rusteal/uht")
    }

    /// The project's `rusteal.toml`.
    pub fn config_file(&self) -> PathBuf {
        self.root.join(FILE_NAME)
    }
}

/// The .uproject file in `dir`, if any.
pub fn find_uproject(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "uproject"))
}

/// Walk up from `start` to the first directory holding a .uproject.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let start = start.canonicalize().ok()?;
    start
        .ancestors()
        .find(|dir| find_uproject(dir).is_some())
        .map(Path::to_path_buf)
}
