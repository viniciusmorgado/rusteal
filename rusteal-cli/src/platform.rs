use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicLibrary {
    pub prefix: String,
    pub extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rust_target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub build_std: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cargo_args: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rustflags: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    pub static_archive: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_library: Option<DynamicLibrary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub system_libraries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uat_args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    platform: Platform,
}

fn desktop(
    name: &str,
    rust_target: Option<&str>,
    archive: &str,
    prefix: &str,
    ext: &str,
) -> Platform {
    Platform {
        name: name.to_string(),
        rust_target: rust_target.map(str::to_string),
        toolchain: None,
        build_std: Vec::new(),
        cargo_args: Vec::new(),
        rustflags: Vec::new(),
        env: BTreeMap::new(),
        static_archive: archive.to_string(),
        dynamic_library: Some(DynamicLibrary {
            prefix: prefix.to_string(),
            extension: ext.to_string(),
        }),
        system_libraries: Vec::new(),
        uat_args: Vec::new(),
    }
}

pub fn builtins() -> Vec<Platform> {
    vec![
        desktop("Linux", None, "librusteal_static.a", "lib", "so"),
        desktop(
            "LinuxArm64",
            Some("aarch64-unknown-linux-gnu"),
            "librusteal_static.a",
            "lib",
            "so",
        ),
        desktop("Win64", None, "rusteal_static.lib", "", "dll"),
        Platform {
            uat_args: mac_package_flags(),
            ..desktop("Mac", None, "librusteal_static.a", "lib", "dylib")
        },
    ]
}

// The archive step copies Binaries/Mac/<Game>.app, which only gets the cooked
// data and the staged libraries from the package step.
fn mac_package_flags() -> Vec<String> {
    vec!["-package".to_string()]
}

impl Platform {
    pub fn parse(text: &str) -> Result<Platform, String> {
        toml::from_str::<File>(text)
            .map(|file| file.platform)
            .map_err(|e| e.to_string())
    }

    pub fn to_toml(&self) -> String {
        toml::to_string(&File {
            platform: self.clone(),
        })
        .expect("a platform serializes")
    }

    pub fn load(path: &Path) -> Result<Platform, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

        let platform =
            Self::parse(&text).map_err(|e| format!("cannot parse {}: {e}", path.display()))?;

        Ok(platform.resolved_from(path.parent().unwrap_or(Path::new("."))))
    }

    fn resolved_from(mut self, dir: &Path) -> Platform {
        if let Some(target) = &self.rust_target
            && target.ends_with(".json")
            && Path::new(target).is_relative()
        {
            self.rust_target = Some(dir.join(target).to_string_lossy().into_owned());
        }

        self
    }

    pub fn is_host(&self, host: &str) -> bool {
        self.name == host && self.rust_target.is_none()
    }

    pub fn cargo_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        if !self.build_std.is_empty() {
            args.push(format!("-Zbuild-std={}", self.build_std.join(",")));
        }

        args.extend(self.cargo_args.iter().cloned());
        args
    }
}

pub fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(paths) = std::env::var("RUSTEAL_PLATFORM_PATH") {
        dirs.extend(std::env::split_paths(&paths));
    }

    if let Some(config) = dirs::config_dir() {
        dirs.push(config.join("rusteal").join("platforms"));
    }

    dirs
}

pub fn find(name_or_file: &str, dirs: &[PathBuf]) -> Result<Platform, String> {
    let as_path = Path::new(name_or_file);

    if name_or_file.ends_with(".toml") || as_path.is_file() {
        return Platform::load(as_path);
    }

    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().is_none_or(|ext| ext != "toml") {
                continue;
            }

            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();

            if stem.eq_ignore_ascii_case(name_or_file) {
                return Platform::load(&path);
            }
        }
    }

    if let Some(platform) = builtins()
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(name_or_file))
    {
        return Ok(platform);
    }

    let known: Vec<String> = builtins().into_iter().map(|p| p.name).collect();

    Err(format!(
        "no platform '{name_or_file}': the built-in ones are {}, and no definition file \
         <name>.toml was found in {}. A platform definition is a file such as \
         `rusteal platform show Linux` prints.",
        known.join(", "),
        if dirs.is_empty() {
            "any directory".to_string()
        } else {
            dirs.iter()
                .map(|d| d.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_built_in_platform_round_trips_through_its_file() {
        for platform in builtins() {
            let text = platform.to_toml();
            assert_eq!(Platform::parse(&text).unwrap(), platform, "{text}");
        }

        let linux = builtins().remove(0);
        let text = linux.to_toml();
        assert!(text.starts_with("[platform]\nname = \"Linux\""), "{text}");
    }

    #[test]
    fn a_console_definition_names_its_toolchain_and_target() {
        let platform = Platform::parse(
            "[platform]\n\
             name = \"Console\"\n\
             rust_target = \"targets/console.json\"\n\
             toolchain = \"nightly-2026-09-01\"\n\
             build_std = [\"core\", \"alloc\", \"std\", \"panic_abort\"]\n\
             rustflags = [\"-Cpanic=abort\"]\n\
             static_archive = \"librusteal_static.a\"\n\
             system_libraries = [\"sdk_runtime\"]\n",
        )
        .unwrap()
        .resolved_from(Path::new("/packs/console"));

        assert_eq!(
            platform.rust_target.as_deref(),
            Some("/packs/console/targets/console.json")
        );

        assert!(platform.dynamic_library.is_none());
        assert_eq!(
            platform.cargo_args(),
            ["-Zbuild-std=core,alloc,std,panic_abort"]
        );
        assert!(!platform.is_host("Linux"));
    }

    #[test]
    fn the_host_platform_is_built_in_and_names_libraries_as_the_cli_does() {
        let host = crate::build_cmd::HostPlatform::CURRENT;
        let platform = find(host.ubt_platform, &[]).unwrap();
        let naming = platform.dynamic_library.as_ref().unwrap();

        assert!(platform.is_host(host.ubt_platform));

        assert_eq!(
            format!("{}rusteal.{}", naming.prefix, naming.extension),
            host.deployed_lib_filename("rusteal")
        );
    }

    #[test]
    fn unknown_fields_and_platforms_are_refused() {
        assert!(
            Platform::parse("[platform]\nname = \"X\"\nstatic_archive = \"a\"\nlib = 1\n").is_err()
        );

        let error = find("Switch3", &[]).unwrap_err();
        assert!(error.contains("Linux, LinuxArm64, Win64, Mac"), "{error}");
    }

    #[test]
    fn definition_files_are_found_by_name_and_win_over_built_ins() {
        let dir = std::env::temp_dir().join(format!("rusteal-platforms-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut custom = builtins().remove(0);
        custom.system_libraries = vec!["extra".to_string()];
        std::fs::write(dir.join("linux.toml"), custom.to_toml()).unwrap();

        let found = find("Linux", std::slice::from_ref(&dir)).unwrap();
        assert_eq!(found.system_libraries, ["extra"]);

        let by_file = find(dir.join("linux.toml").to_str().unwrap(), &[]).unwrap();
        assert_eq!(by_file, found);

        assert_eq!(
            find("win64", &[]).unwrap().static_archive,
            "rusteal_static.lib"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
