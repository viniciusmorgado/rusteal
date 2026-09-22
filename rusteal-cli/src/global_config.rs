// Per-machine configuration: where the Unreal Engine is.
//
// Lives in the user's configuration directory (`~/.config/rusteal/config.toml`
// on Linux, the platform equivalent elsewhere) and is written on first use.
// It is the only state a Rusteal project cannot carry in its repository.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub engine: EngineSection,
}

#[derive(Deserialize, Serialize, Default)]
pub struct EngineSection {
    /// Unreal Engine root: the directory holding `Engine/` and `Templates/`.
    #[serde(default)]
    pub path: String,
}

/// `<config dir>/rusteal/config.toml`.
pub fn path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| {
        eprintln!("Error: this system has no configuration directory.");
        std::process::exit(1);
    });
    base.join("rusteal").join("config.toml")
}

pub fn load() -> GlobalConfig {
    let path = path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return GlobalConfig::default();
    };
    toml::from_str(&text).unwrap_or_else(|e| {
        eprintln!("Error: cannot parse {}: {e}", path.display());
        std::process::exit(1);
    })
}

pub fn save(config: &GlobalConfig) {
    let path = path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .unwrap_or_else(|e| panic!("Failed to create {}: {e}", dir.display()));
    }
    let text = toml::to_string(config).expect("serializable config");
    std::fs::write(&path, text)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", path.display()));
}

/// The engine root, asking for it — and saving it — the first time.
pub fn engine_path() -> PathBuf {
    let mut config = load();
    if config.engine.path.trim().is_empty() {
        config.engine.path = ask_engine_path();
        save(&config);
        eprintln!("  Saved to {}", path().display());
    }
    let engine = PathBuf::from(&config.engine.path);
    if !is_engine_root(&engine) {
        eprintln!(
            "Error: {} is not an Unreal Engine root (no Engine/Build/BatchFiles).",
            engine.display()
        );
        eprintln!("  Fix [engine].path in {}.", path().display());
        std::process::exit(1);
    }
    engine
}

fn ask_engine_path() -> String {
    if !std::io::stdin().is_terminal() {
        eprintln!("Error: no engine configured.");
        eprintln!(
            "  Set [engine].path in {} to your Unreal Engine root.",
            path().display()
        );
        std::process::exit(1);
    }
    loop {
        eprint!("Unreal Engine root (the directory with Engine/ and Templates/): ");
        std::io::stderr().flush().ok();
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
            std::process::exit(1);
        }
        let answer = line.trim().trim_end_matches(['/', '\\']).to_string();
        if is_engine_root(Path::new(&answer)) {
            return answer;
        }
        eprintln!("  {answer} has no Engine/Build/BatchFiles; try again.");
    }
}

fn is_engine_root(dir: &Path) -> bool {
    dir.join("Engine/Build/BatchFiles").is_dir()
}
