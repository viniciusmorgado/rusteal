use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::fail;

pub struct DevEnv {
    pub engine_root: PathBuf,
}

impl DevEnv {
    pub fn load(repo: &Path) -> Self {
        let values = match std::fs::read_to_string(repo.join(".env")) {
            Ok(text) => parse(&text),
            Err(_) => HashMap::new(),
        };

        let key = "RUSTEAL_DEV_ENGINE_ROOT";

        let value = std::env::var(key)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| values.get(key).cloned().filter(|v| !v.trim().is_empty()))
            .unwrap_or_else(|| {
                fail(&format!(
                    "{key} is not set. Copy .env.example to .env at the repository root and \
                     fill it in."
                ))
            });

        let engine_root = expand_home(&value);

        if !engine_root.join("Engine").is_dir() {
            fail(&format!(
                "{key} must be the engine root, the directory holding Engine/ (got {}).",
                engine_root.display()
            ));
        }

        DevEnv { engine_root }
    }
}

fn parse(text: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();

    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line);

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let value = value.trim();

        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);

        values.insert(key.trim().to_string(), value.to_string());
    }

    values
}

pub fn expand_home(value: &str) -> PathBuf {
    let home = || dirs::home_dir().unwrap_or_else(|| fail("this system has no home directory"));

    if value == "~" {
        home()
    } else if let Some(rest) = value
        .strip_prefix("~/")
        .or_else(|| value.strip_prefix("~\\"))
    {
        home().join(rest)
    } else {
        PathBuf::from(value)
    }
}
