use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "rusteal.toml";

#[derive(Deserialize)]
pub struct ProjectConfig {
    pub project: ProjectSection,
    pub codegen: CodegenConfig,
}

#[derive(Deserialize)]
pub struct ProjectSection {
    #[serde(rename = "crate")]
    pub crate_name: String,
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Deserialize)]
pub struct PluginConfig {
    pub plugin: PluginSection,
    pub codegen: CodegenConfig,
}

#[derive(Deserialize)]
pub struct PluginSection {
    #[serde(rename = "crate")]
    pub crate_name: String,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub kind: LibraryKind,
}

#[derive(Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LibraryKind {
    #[default]
    Runtime,
    Editor,
}

impl PluginConfig {
    pub fn load(plugin_dir: &Path) -> Result<Self, String> {
        let path = plugin_dir.join(FILE_NAME);

        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

        toml::from_str(&text).map_err(|e| format!("cannot parse {}: {e}", path.display()))
    }
}

#[derive(Deserialize, Clone)]
pub struct CodegenConfig {
    pub features: Vec<String>,
    pub modules: HashMap<String, ModuleMapping>,
    #[serde(default)]
    pub blocklist: Blocklist,
}

#[derive(Deserialize, Clone)]
pub struct ModuleMapping {
    pub module: String,
    pub feature: String,
    #[serde(default)]
    pub plugin: Option<String>,
    #[serde(default)]
    pub classes: Option<Vec<String>>,
}

#[derive(Deserialize, Default, Clone)]
pub struct Blocklist {
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub structs: Vec<String>,
    #[serde(default)]
    pub functions: Vec<String>,
}

const RUSTEAL_INTERNAL_CLASSES: &[&str] = &[
    "RustealReifiedClass",
    "RustealReifiedFunction",
    "RustealDelegateProxy",
];

impl CodegenConfig {
    pub fn for_library(&self, kind: LibraryKind) -> CodegenConfig {
        let mut config = self.clone();

        let core = config
            .features
            .first()
            .cloned()
            .unwrap_or_else(|| "core".to_string());

        let mut add = |package: &str, module: &str| {
            config
                .modules
                .entry(package.to_string())
                .or_insert_with(|| ModuleMapping {
                    module: module.to_string(),
                    feature: core.clone(),
                    plugin: None,
                    classes: None,
                });
        };

        add("Rusteal", "rusteal");

        if kind == LibraryKind::Editor {
            add("RustealEditor", "rusteal_editor");
        }

        for class in RUSTEAL_INTERNAL_CLASSES {
            if !config.blocklist.classes.iter().any(|c| c == class) {
                config.blocklist.classes.push(class.to_string());
            }
        }

        config
    }
}

impl Blocklist {
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
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(FILE_NAME);

        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

        toml::from_str(&text).map_err(|e| format!("cannot parse {}: {e}", path.display()))
    }
}

pub struct ProjectLayout {
    pub root: PathBuf,
}

impl ProjectLayout {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn uproject(&self) -> Option<PathBuf> {
        find_uproject(&self.root)
    }

    pub fn rust_workspace(&self) -> PathBuf {
        self.root.join("Rust")
    }

    pub fn bindings_crate(&self) -> PathBuf {
        self.rust_workspace().join("bindings")
    }

    pub fn cpp_generated(&self) -> PathBuf {
        self.root.join("Plugins/Rusteal/Source/Rusteal/Generated")
    }

    pub fn plugin_descriptor(&self) -> PathBuf {
        self.root.join("Plugins/Rusteal/Rusteal.uplugin")
    }

    pub fn uht_json(&self) -> PathBuf {
        self.root.join("Intermediate/Rusteal/uht")
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join(FILE_NAME)
    }
}

pub struct LibraryOutput {
    pub bindings_crate: PathBuf,
    pub cpp_generated: PathBuf,
    pub plugin_descriptor: PathBuf,
    pub host_module: String,
    pub cpp_prefix: String,
    pub kind: LibraryKind,
}

impl ProjectLayout {
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

pub struct PluginLayout {
    pub dir: PathBuf,
    pub name: String,
}

impl PluginLayout {
    pub fn new(dir: &Path) -> Self {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        Self {
            dir: dir.to_path_buf(),
            name,
        }
    }

    pub fn descriptor(&self) -> PathBuf {
        self.dir.join(format!("{}.uplugin", self.name))
    }

    pub fn config_file(&self) -> PathBuf {
        self.dir.join(FILE_NAME)
    }

    pub fn rust_workspace(&self) -> PathBuf {
        self.dir.join("Rust")
    }

    pub fn module_dir(&self) -> PathBuf {
        self.dir.join("Source").join(&self.name)
    }

    pub fn binaries(&self, platform: &str) -> PathBuf {
        self.dir.join("Binaries").join(platform)
    }

    pub fn library_stem(&self) -> String {
        format!("rusteal_{}", self.name)
    }

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

pub fn find_uproject(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "uproject"))
}

pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let start = start.canonicalize().ok()?;

    start
        .ancestors()
        .find(|dir| find_uproject(dir).is_some())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codegen() -> CodegenConfig {
        toml::from_str(
            r#"
            features = ["core", "engine"]
            [modules]
            Engine = { module = "engine", feature = "engine" }
            [blocklist]
            classes = ["Foo"]
            "#,
        )
        .unwrap()
    }

    #[test]
    fn libraries_bind_rusteal_modules() {
        let runtime = codegen().for_library(LibraryKind::Runtime);
        assert_eq!(runtime.modules["Rusteal"].module, "rusteal");
        assert_eq!(runtime.modules["Rusteal"].feature, "core");
        assert!(!runtime.modules.contains_key("RustealEditor"));

        assert!(
            runtime
                .blocklist
                .classes
                .iter()
                .any(|c| c == "RustealDelegateProxy")
        );

        assert!(runtime.blocklist.classes.iter().any(|c| c == "Foo"));

        let editor = codegen().for_library(LibraryKind::Editor);
        assert_eq!(editor.modules["RustealEditor"].module, "rusteal_editor");
    }
}
