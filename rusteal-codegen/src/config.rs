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

/// A Rusteal plugin's `rusteal.toml`, at the plugin's root
/// (`Plugins/<Name>/rusteal.toml`): its library and the modules it binds.
#[derive(Deserialize)]
pub struct PluginConfig {
    pub plugin: PluginSection,
    pub codegen: CodegenConfig,
}

#[derive(Deserialize)]
pub struct PluginSection {
    /// Cargo package, a cdylib member of the plugin's `Rust/`.
    #[serde(rename = "crate")]
    pub crate_name: String,
    /// Extra cargo features to build it with.
    #[serde(default)]
    pub features: Vec<String>,
    /// Where the library runs: in games and the editor, or in the editor
    /// only. It is the type of the plugin's C++ module.
    #[serde(default)]
    pub kind: LibraryKind,
}

/// Where a library runs.
#[derive(Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LibraryKind {
    /// Games and the editor (a `Runtime` module).
    #[default]
    Runtime,
    /// The editor only (an `Editor` module): it may bind editor modules.
    Editor,
}

impl PluginConfig {
    /// Read `rusteal.toml` from a plugin's root.
    pub fn load(plugin_dir: &Path) -> Result<Self, String> {
        let path = plugin_dir.join(FILE_NAME);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("cannot parse {}: {e}", path.display()))
    }
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
    /// The engine plugin the UE module belongs to (`StateTree`), for a module
    /// outside the engine's own: the Rusteal plugin must list it.
    #[serde(default)]
    pub plugin: Option<String>,
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

    /// The Rusteal plugin's descriptor in the project.
    pub fn plugin_descriptor(&self) -> PathBuf {
        self.root.join("Plugins/Rusteal/Rusteal.uplugin")
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

/// Where one library's generated code goes. The game's library is compiled
/// into the Rusteal plugin; a Rusteal plugin's library into the plugin's own
/// module, with its own bindings crate.
pub struct LibraryOutput {
    /// The generated `bindings` crate.
    pub bindings_crate: PathBuf,
    /// The generated C++ wrappers, compiled into `host_module`.
    pub cpp_generated: PathBuf,
    /// The descriptor of the plugin `host_module` belongs to: it must list
    /// the engine plugins the enabled modules come from.
    pub plugin_descriptor: PathBuf,
    /// The UE module the C++ wrappers are compiled into.
    pub host_module: String,
    /// Prefix of the generated C++ symbols (see `CodegenContext::cpp_prefix`).
    pub cpp_prefix: String,
    /// Where the library runs: an editor library also gets the engine's
    /// editor-only functions.
    pub kind: LibraryKind,
}

impl ProjectLayout {
    /// The output of the game's library.
    pub fn game_output(&self) -> LibraryOutput {
        LibraryOutput {
            bindings_crate: self.bindings_crate(),
            cpp_generated: self.cpp_generated(),
            plugin_descriptor: self.plugin_descriptor(),
            host_module: "Rusteal".to_string(),
            cpp_prefix: String::new(),
            kind: LibraryKind::Runtime,
        }
    }
}

/// Where things live inside a Rusteal plugin, `Plugins/<Name>/`: the plugin
/// and its one C++ module share the name.
pub struct PluginLayout {
    /// The plugin's directory, holding `<Name>.uplugin`.
    pub dir: PathBuf,
    /// The plugin's name, which is also its C++ module's.
    pub name: String,
}

impl PluginLayout {
    pub fn new(dir: &Path) -> Self {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self { dir: dir.to_path_buf(), name }
    }

    /// The plugin's descriptor.
    pub fn descriptor(&self) -> PathBuf {
        self.dir.join(format!("{}.uplugin", self.name))
    }

    /// The plugin's `rusteal.toml`.
    pub fn config_file(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    /// Cargo workspace with the plugin's crate and its bindings.
    pub fn rust_workspace(&self) -> PathBuf {
        self.dir.join("Rust")
    }

    /// The plugin's C++ module.
    pub fn module_dir(&self) -> PathBuf {
        self.dir.join("Source").join(&self.name)
    }

    /// The deployed library's directory: the plugin's binaries for the
    /// platform (`Binaries/Linux`).
    pub fn binaries(&self, platform: &str) -> PathBuf {
        self.dir.join("Binaries").join(platform)
    }

    /// The stem of the deployed library's file name (`rusteal_<Name>`), as
    /// `RustealRegisterPluginLibrary` looks for it.
    pub fn library_stem(&self) -> String {
        format!("rusteal_{}", self.name)
    }

    /// The output of the plugin's library, of the given kind.
    pub fn output(&self, kind: LibraryKind) -> LibraryOutput {
        LibraryOutput {
            bindings_crate: self.rust_workspace().join("bindings"),
            cpp_generated: self.module_dir().join("Generated"),
            plugin_descriptor: self.descriptor(),
            host_module: self.name.clone(),
            cpp_prefix: format!("{}_", self.name),
            kind,
        }
    }
}

/// The Rusteal plugins of a project: the directories of `Plugins/` holding a
/// `rusteal.toml`, by name.
pub fn find_plugins(project_root: &Path) -> Vec<PluginLayout> {
    let Ok(entries) = std::fs::read_dir(project_root.join("Plugins")) else {
        return Vec::new();
    };
    let mut plugins: Vec<PluginLayout> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|dir| dir.join(FILE_NAME).is_file())
        .map(|dir| PluginLayout::new(&dir))
        .collect();
    plugins.sort_by(|a, b| a.name.cmp(&b.name));
    plugins
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
