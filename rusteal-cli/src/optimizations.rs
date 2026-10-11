use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const BEGIN: &str =
    "; Rusteal optimizations, written by rusteal from rusteal.toml [optimizations]: begin";
pub const END: &str = "; Rusteal optimizations: end";

pub const DEFAULTS: &str = include_str!("../optimizations.toml");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    On,
    Off,
}

impl Switch {
    pub fn parse(text: &str) -> Result<Switch, String> {
        match text.to_ascii_lowercase().as_str() {
            "on" | "1" | "true" => Ok(Switch::On),
            "off" | "0" | "false" => Ok(Switch::Off),
            _ => Err(format!("'{text}' is not on/off (or 1/0)")),
        }
    }
}

pub struct Platform {
    pub key: &'static str,
    pub config_dir: &'static str,
}

#[cfg(target_os = "linux")]
pub const HOST: Platform = Platform {
    key: "linux",
    config_dir: "Linux",
};

#[cfg(target_os = "windows")]
pub const HOST: Platform = Platform {
    key: "windows",
    config_dir: "Windows",
};

#[cfg(target_os = "macos")]
pub const HOST: Platform = Platform {
    key: "mac",
    config_dir: "Mac",
};

pub const PLATFORMS: &[&str] = &["linux", "windows", "mac"];

type Section = BTreeMap<String, toml::Value>;
type IniFile = BTreeMap<String, Section>;
type PlatformSettings = BTreeMap<String, IniFile>;
pub type Optimizations = BTreeMap<String, PlatformSettings>;

#[derive(Deserialize)]
struct Manifest {
    optimizations: Option<Optimizations>,
}

pub fn parse(text: &str) -> Result<Option<Optimizations>, String> {
    let manifest: Manifest = toml::from_str(text).map_err(|e| e.to_string())?;

    if let Some(optimizations) = &manifest.optimizations {
        for platform in optimizations.keys() {
            if !PLATFORMS.contains(&platform.as_str()) {
                return Err(format!(
                    "[optimizations.{platform}]: the platforms are {}",
                    PLATFORMS.join(", ")
                ));
            }
        }

        for settings in optimizations.values() {
            for file in settings.values() {
                for section in file.values() {
                    for (key, value) in section {
                        ini_value(value).map_err(|e| format!("{key}: {e}"))?;
                    }
                }
            }
        }
    }

    Ok(manifest.optimizations)
}

pub fn defaults() -> Optimizations {
    parse(DEFAULTS)
        .expect("the default optimizations parse")
        .expect("the default optimizations have a table")
}

pub fn load(root: &Path) -> Result<Optimizations, String> {
    let path = root.join(rusteal_codegen::config::FILE_NAME);

    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(defaults());
    };

    let parsed = parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;

    Ok(parsed.unwrap_or_else(defaults))
}

fn ini_value(value: &toml::Value) -> Result<String, String> {
    match value {
        toml::Value::String(text) => Ok(text.clone()),
        toml::Value::Integer(number) => Ok(number.to_string()),
        toml::Value::Float(number) => Ok(number.to_string()),
        toml::Value::Boolean(flag) => Ok(if *flag { "True" } else { "False" }.to_string()),
        _ => Err("a value is a string, a number or a boolean".to_string()),
    }
}

pub fn ini_path(root: &Path, platform: &Platform, file: &str) -> PathBuf {
    root.join("Config")
        .join(platform.config_dir)
        .join(format!("{}{file}.ini", platform.config_dir))
}

pub struct Change {
    pub path: PathBuf,
    pub settings: usize,
}

pub fn apply(root: &Path, switch: Switch) -> Result<Vec<Change>, String> {
    let optimizations = load(root)?;
    let settings = optimizations.get(HOST.key).cloned().unwrap_or_default();

    apply_settings(root, &HOST, &settings, switch)
}

fn apply_settings(
    root: &Path,
    platform: &Platform,
    settings: &PlatformSettings,
    switch: Switch,
) -> Result<Vec<Change>, String> {
    let mut paths: BTreeMap<PathBuf, Option<&IniFile>> = BTreeMap::new();
    let dir = root.join("Config").join(platform.config_dir);

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "ini") {
                paths.insert(path, None);
            }
        }
    }

    for (file, sections) in settings {
        paths.insert(ini_path(root, platform, file), Some(sections));
    }

    let mut changes = Vec::new();

    for (path, sections) in paths {
        let old = fs::read_to_string(&path).ok();
        let had_block = old.as_deref().is_some_and(has_block);
        let user = strip_block(old.as_deref().unwrap_or_default());

        let (block, count) = match (switch, sections) {
            (Switch::On, Some(sections)) => block(&user, sections)?,
            _ => (String::new(), 0),
        };

        if block.is_empty() && !had_block {
            continue;
        }

        let new = compose(&user, &block);

        if old.as_deref().unwrap_or_default() == new {
            continue;
        }

        if new.is_empty() {
            fs::remove_file(&path).map_err(|e| format!("cannot remove {}: {e}", path.display()))?;
        } else {
            fs::create_dir_all(path.parent().unwrap())
                .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;

            fs::write(&path, &new).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        }

        changes.push(Change {
            path,
            settings: count,
        });
    }

    Ok(changes)
}

fn has_block(text: &str) -> bool {
    text.lines().any(|line| line.trim() == BEGIN)
}

pub fn strip_block(text: &str) -> String {
    let mut out = Vec::new();
    let mut inside = false;

    for line in text.lines() {
        if line.trim() == BEGIN {
            inside = true;
            continue;
        }

        if inside {
            if line.trim() == END {
                inside = false;
            }

            continue;
        }

        out.push(line);
    }

    let mut text = out.join("\n");
    let trimmed = text.trim_end().len();
    text.truncate(trimmed);
    text
}

fn user_keys(user: &str) -> Vec<(String, String)> {
    let mut keys = Vec::new();
    let mut section = String::new();

    for line in user.lines() {
        let line = line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].to_string();
        } else if !line.starts_with(';')
            && let Some((key, _)) = line.split_once('=')
        {
            keys.push((section.clone(), key.trim().to_string()));
        }
    }

    keys
}

fn block(user: &str, sections: &IniFile) -> Result<(String, usize), String> {
    let taken = user_keys(user);
    let mut body = String::new();
    let mut count = 0;

    for (section, values) in sections {
        let mut lines = String::new();

        for (key, value) in values {
            if taken.iter().any(|(s, k)| s == section && k == key) {
                continue;
            }

            lines.push_str(&format!("{key}={}\n", ini_value(value)?));
            count += 1;
        }

        if !lines.is_empty() {
            body.push_str(&format!("[{section}]\n{lines}"));
        }
    }

    if body.is_empty() {
        return Ok((String::new(), 0));
    }

    Ok((format!("{BEGIN}\n{body}{END}\n"), count))
}

fn compose(user: &str, block: &str) -> String {
    match (user.is_empty(), block.is_empty()) {
        (true, true) => String::new(),
        (true, false) => block.to_string(),
        (false, true) => format!("{user}\n"),
        (false, false) => format!("{user}\n\n{block}"),
    }
}

pub struct Snapshot(Vec<(PathBuf, Option<String>)>);

impl Snapshot {
    pub fn take(root: &Path) -> Snapshot {
        let dir = root.join("Config").join(HOST.config_dir);
        let mut files = Vec::new();

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.extension().is_some_and(|e| e == "ini") {
                    files.push((path.clone(), fs::read_to_string(&path).ok()));
                }
            }
        }

        if let Ok(optimizations) = load(root) {
            for file in optimizations
                .get(HOST.key)
                .into_iter()
                .flat_map(|s| s.keys())
            {
                let path = ini_path(root, &HOST, file);

                if !files.iter().any(|(p, _)| *p == path) {
                    files.push((path, None));
                }
            }
        }

        Snapshot(files)
    }

    pub fn restore(self) {
        for (path, text) in self.0 {
            let current = fs::read_to_string(&path).ok();

            if current == text {
                continue;
            }

            let result = match &text {
                Some(text) => fs::write(&path, text),
                None => fs::remove_file(&path),
            };

            if let Err(e) = result {
                eprintln!("Warning: cannot restore {}: {e}", path.display());
            }
        }
    }
}

pub fn report(command: &str, root: &Path, switch: Switch) {
    let changes = match apply(root, switch) {
        Ok(changes) => changes,
        Err(message) => {
            eprintln!("Error: {message}");
            std::process::exit(1);
        }
    };

    for change in changes {
        let rel = change.path.strip_prefix(root).unwrap_or(&change.path);

        if change.settings == 0 {
            eprintln!(
                "rusteal {command}: {} optimizations off, {}",
                HOST.config_dir,
                rel.display()
            );
        } else {
            eprintln!(
                "rusteal {command}: {} optimizations on, {} ({} setting{})",
                HOST.config_dir,
                rel.display(),
                change.settings,
                if change.settings == 1 { "" } else { "s" }
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINUX: Platform = Platform {
        key: "linux",
        config_dir: "Linux",
    };

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rusteal-opt-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn linux(text: &str) -> PlatformSettings {
        parse(text).unwrap().unwrap().remove("linux").unwrap()
    }

    const VULKAN: &str =
        "[optimizations.linux.Engine.ConsoleVariables]\n\"r.Vulkan.CpuWaitForFence\" = 0\n";

    #[test]
    fn the_defaults_fix_the_vulkan_fence_on_linux() {
        let defaults = defaults();
        let console = &defaults["linux"]["Engine"]["ConsoleVariables"];

        assert_eq!(
            ini_value(&console["r.Vulkan.CpuWaitForFence"]).unwrap(),
            "0"
        );
        assert_eq!(defaults.len(), 1, "only Linux has defaults");
    }

    #[test]
    fn on_writes_a_block_and_off_removes_the_file_it_created() {
        let root = scratch("create");
        let settings = linux(VULKAN);
        let ini = ini_path(&root, &LINUX, "Engine");

        let changes = apply_settings(&root, &LINUX, &settings, Switch::On).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].settings, 1);

        assert_eq!(
            fs::read_to_string(&ini).unwrap(),
            format!("{BEGIN}\n[ConsoleVariables]\nr.Vulkan.CpuWaitForFence=0\n{END}\n")
        );

        assert!(
            apply_settings(&root, &LINUX, &settings, Switch::On)
                .unwrap()
                .is_empty()
        );

        apply_settings(&root, &LINUX, &settings, Switch::Off).unwrap();
        assert!(!ini.exists());
    }

    #[test]
    fn the_user_settings_stay_and_win() {
        let root = scratch("user");
        let ini = ini_path(&root, &LINUX, "Engine");
        fs::create_dir_all(ini.parent().unwrap()).unwrap();

        let user = "[ConsoleVariables]\n; mine\nr.Vulkan.CpuWaitForFence=1\n\n[SystemSettings]\nr.setres=1280x720\n";
        fs::write(&ini, user).unwrap();

        let settings = linux(&format!("{VULKAN}\"r.Vulkan.KeepSwapChain\" = \"0\"\n"));
        apply_settings(&root, &LINUX, &settings, Switch::On).unwrap();
        let text = fs::read_to_string(&ini).unwrap();

        assert!(text.starts_with(user.trim_end()));
        assert!(text.contains("r.Vulkan.KeepSwapChain=0"));
        assert!(!text.contains("r.Vulkan.CpuWaitForFence=0"), "{text}");

        apply_settings(&root, &LINUX, &settings, Switch::Off).unwrap();
        assert_eq!(fs::read_to_string(&ini).unwrap(), user);
    }

    #[test]
    fn files_without_a_block_are_never_rewritten() {
        let root = scratch("untouched");
        let ini = ini_path(&root, &LINUX, "Game");
        fs::create_dir_all(ini.parent().unwrap()).unwrap();
        fs::write(&ini, "[A]\r\nx=1").unwrap();

        assert!(
            apply_settings(&root, &LINUX, &linux(VULKAN), Switch::On)
                .unwrap()
                .len()
                == 1
        );
        assert!(
            apply_settings(&root, &LINUX, &linux(VULKAN), Switch::Off)
                .unwrap()
                .len()
                == 1
        );
        assert_eq!(fs::read_to_string(&ini).unwrap(), "[A]\r\nx=1");
    }

    #[test]
    fn a_setting_removed_from_rusteal_toml_leaves_the_ini() {
        let root = scratch("stale");
        let ini = ini_path(&root, &LINUX, "Engine");

        apply_settings(&root, &LINUX, &linux(VULKAN), Switch::On).unwrap();
        assert!(ini.exists());

        apply_settings(&root, &LINUX, &PlatformSettings::new(), Switch::On).unwrap();
        assert!(!ini.exists());
    }

    #[test]
    fn a_project_without_the_table_gets_the_defaults() {
        let root = scratch("load");
        fs::write(root.join("rusteal.toml"), "[project]\ncrate = \"game\"\n").unwrap();
        assert_eq!(load(&root).unwrap(), defaults());

        fs::write(root.join("rusteal.toml"), "[optimizations]\n").unwrap();
        assert!(load(&root).unwrap().is_empty());
    }

    #[test]
    fn unknown_platforms_and_tables_are_refused() {
        assert!(parse("[optimizations.android.Engine.ConsoleVariables]\nx = 1\n").is_err());
        assert!(parse("[optimizations.linux.Engine.ConsoleVariables]\nx = [1]\n").is_err());
    }

    #[test]
    fn the_snapshot_puts_the_files_back() {
        let root = scratch("snapshot");
        let ini = ini_path(&root, &HOST, "Engine");
        fs::create_dir_all(ini.parent().unwrap()).unwrap();
        fs::write(&ini, "[ConsoleVariables]\nx=1\n").unwrap();

        let snapshot = Snapshot::take(&root);
        fs::write(&ini, "changed").unwrap();
        snapshot.restore();

        assert_eq!(
            fs::read_to_string(&ini).unwrap(),
            "[ConsoleVariables]\nx=1\n"
        );
    }

    #[test]
    fn switches_take_on_off_and_numbers() {
        for on in ["on", "ON", "1", "true"] {
            assert_eq!(Switch::parse(on), Ok(Switch::On));
        }

        for off in ["off", "0", "false"] {
            assert_eq!(Switch::parse(off), Ok(Switch::Off));
        }

        assert!(Switch::parse("maybe").is_err());
    }
}
