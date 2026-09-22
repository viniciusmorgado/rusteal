// A project's Rusteal version, checked before the CLI acts on it.
//
// The generated bindings (manual/ included) and the UE plugins come from the
// CLI; the runtime crates come from the pins in the project's Rust/Cargo.toml.
// All of them are tied to one version, so the CLI refuses to build or
// generate for a project at another one and says how to line them up.

use std::fmt;
use std::path::{Path, PathBuf};

/// The crates a project pins, all to the same version.
pub const PINNED: [&str; 3] = ["rusteal-runtime", "rusteal-core", "rusteal-ffi"];

/// The plugins a project carries, all at the same version.
pub const PLUGINS: [&str; 2] = ["Rusteal", "RustealGenerator"];

/// `major.minor.patch`, nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(u32, u32, u32);

impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.');
        let mut next = || -> Option<u32> {
            let part = parts.next()?;
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            part.parse().ok()
        };
        let version = Version(next()?, next()?, next()?);
        parts.next().is_none().then_some(version)
    }

    /// The version of this CLI.
    pub fn cli() -> Self {
        Self::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is major.minor.patch")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Where a project takes the runtime crates from.
#[derive(Debug, PartialEq, Eq)]
pub enum Pins {
    /// `"=X.Y.Z"` on all three crates.
    Exact(Version),
    /// A path into one Rusteal checkout on all three (`rusteal new --runtime-path`).
    Checkout(PathBuf),
}

/// Read the pins from the text of a project's Rust/Cargo.toml.
pub fn parse_pins(manifest: &str) -> Result<Pins, String> {
    let manifest: toml::Table =
        toml::from_str(manifest).map_err(|e| format!("cannot parse it: {e}"))?;
    let deps = manifest
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table())
        .ok_or("it has no [workspace.dependencies]")?;

    let mut pins = Vec::new();
    for name in PINNED {
        let value = deps.get(name).ok_or(format!("{name} is missing"))?;
        let requirement = match value {
            toml::Value::String(s) => Some(s.as_str()),
            toml::Value::Table(t) => t.get("version").and_then(|v| v.as_str()),
            _ => None,
        };
        let path = value.as_table().and_then(|t| t.get("path")).and_then(|p| p.as_str());
        let pin = match (requirement, path) {
            (_, Some(path)) => {
                let crate_dir = Path::new(path);
                let checkout = crate_dir
                    .parent()
                    .ok_or(format!("{name}: path {path} has no parent"))?;
                Pins::Checkout(checkout.to_path_buf())
            }
            (Some(req), None) => {
                let version = req
                    .strip_prefix('=')
                    .and_then(Version::parse)
                    .ok_or(format!("{name} = \"{req}\" is not an exact \"=X.Y.Z\""))?;
                Pins::Exact(version)
            }
            (None, None) => return Err(format!("{name} has neither a version nor a path")),
        };
        pins.push(pin);
    }

    let first = pins.remove(0);
    if pins.iter().any(|p| *p != first) {
        return Err(format!("{} do not all point at the same version", PINNED.join(", ")));
    }
    Ok(first)
}

/// What a project's pins mean for this CLI.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Matches,
    /// The project is at an older version than the CLI.
    ProjectOlder(Version),
    /// The project is at a newer version than the CLI.
    ProjectNewer(Version),
    /// The project depends on a checkout this CLI was not built from.
    OtherCheckout(PathBuf),
}

/// `cli_checkout`: the checkout this CLI was built from, when it was.
pub fn compare(pins: &Pins, cli: Version, cli_checkout: Option<&Path>) -> Verdict {
    match pins {
        Pins::Exact(v) if *v == cli => Verdict::Matches,
        Pins::Exact(v) if *v < cli => Verdict::ProjectOlder(*v),
        Pins::Exact(v) => Verdict::ProjectNewer(*v),
        Pins::Checkout(dir) => {
            let same = cli_checkout
                .zip(dir.canonicalize().ok())
                .is_some_and(|(cli, dir)| cli == dir);
            if same { Verdict::Matches } else { Verdict::OtherCheckout(dir.clone()) }
        }
    }
}

/// The directory this CLI was built from. For an installed binary that is a
/// directory in cargo's registry, which no project points at.
fn cli_checkout() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.canonicalize().ok()
}

/// The message for a verdict other than [`Verdict::Matches`].
fn explain(verdict: &Verdict, cli: Version) -> Option<String> {
    Some(match verdict {
        Verdict::Matches => return None,
        Verdict::ProjectOlder(p) => format!(
            "this project is at Rusteal {p} and the CLI at {cli}.\n  \
             Move the project to {cli}:       rusteal upgrade\n  \
             or use the CLI it was made with: cargo install rusteal@{p}"
        ),
        Verdict::ProjectNewer(p) => format!(
            "this project is at Rusteal {p}, newer than this CLI ({cli}).\n  \
             Install its version: cargo install rusteal@{p}"
        ),
        Verdict::OtherCheckout(dir) => format!(
            "this project depends on the Rusteal checkout at {dir}.\n  \
             Run the CLI built from it: cargo run --manifest-path {manifest} -p rusteal -- <command>",
            dir = dir.display(),
            manifest = dir.join("Cargo.toml").display(),
        ),
    })
}

/// Read and check the pins of the project at `root`.
pub fn read_pins(root: &Path) -> Result<Pins, String> {
    let path = root.join("Rust/Cargo.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "cannot read {}: {e}. The project needs the Rust workspace `rusteal new` writes.",
            path.display()
        )
    })?;
    let pins = parse_pins(&text).map_err(|e| {
        format!(
            "{}: {e}.\n  rusteal-runtime, rusteal-core and rusteal-ffi must all be pinned to one \
             exact version (\"=X.Y.Z\") or all point at one Rusteal checkout by path.",
            path.display()
        )
    })?;
    // Cargo resolves a relative path from the manifest's directory.
    Ok(match pins {
        Pins::Checkout(dir) if dir.is_relative() => Pins::Checkout(root.join("Rust").join(dir)),
        pins => pins,
    })
}

/// Which parts of a project to check.
pub enum Scope {
    /// The pins only: `setup` is about to install this CLI's plugins anyway.
    Pins,
    /// The pins and the installed plugins: `build`, `generate`.
    PinsAndPlugins,
}

/// Check the project at `root` against this CLI; the error is the full message.
pub fn check(root: &Path, scope: Scope) -> Result<(), String> {
    let cli = Version::cli();
    let pins = read_pins(root)?;
    if let Some(message) = explain(&compare(&pins, cli, cli_checkout().as_deref()), cli) {
        return Err(message);
    }
    if let Scope::PinsAndPlugins = scope {
        for plugin in PLUGINS {
            let path = root.join("Plugins").join(plugin).join(format!("{plugin}.uplugin"));
            let installed = std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
                .and_then(|d| d["VersionName"].as_str().map(str::to_string));
            if installed.as_deref() != Some(&cli.to_string()) {
                return Err(format!(
                    "Plugins/{plugin} is at {}, the project at {cli}.\n  \
                     Reinstall the plugins: rusteal setup {}",
                    installed.as_deref().unwrap_or("an unknown version"),
                    root.display()
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    const EXACT: &str = r#"# comment
[workspace]
members = ["game", "bindings"]

[workspace.dependencies]
rusteal-runtime = "=0.2.1"
rusteal-core = "=0.2.1"
rusteal-ffi = "=0.2.1"
glam = "0.33.8"
"#;

    #[test]
    fn versions_are_three_numbers() {
        assert_eq!(Version::parse("0.2.1"), Some(Version(0, 2, 1)));
        assert_eq!(Version::parse("12.0.345"), Some(Version(12, 0, 345)));
        for bad in ["0.2", "0.2.1.0", "0.2.1-alpha", "^0.2.1", "a.b.c", "0..1", ""] {
            assert_eq!(Version::parse(bad), None, "{bad}");
        }
        assert!(v("0.10.0") > v("0.9.9"));
    }

    #[test]
    fn exact_pins() {
        assert_eq!(parse_pins(EXACT), Ok(Pins::Exact(v("0.2.1"))));
        let table = EXACT.replace("rusteal-core = \"=0.2.1\"", "rusteal-core = { version = \"=0.2.1\" }");
        assert_eq!(parse_pins(&table), Ok(Pins::Exact(v("0.2.1"))));
    }

    #[test]
    fn checkout_pins() {
        let manifest = r#"[workspace.dependencies]
rusteal-runtime = { path = "/src/rusteal/rusteal-runtime" }
rusteal-core = { path = "/src/rusteal/rusteal-core" }
rusteal-ffi = { path = "/src/rusteal/rusteal-ffi" }
"#;
        assert_eq!(parse_pins(manifest), Ok(Pins::Checkout(PathBuf::from("/src/rusteal"))));
    }

    #[test]
    fn rejected_pins() {
        for (from, to) in [
            ("rusteal-ffi = \"=0.2.1\"", "rusteal-ffi = \"0.2.1\""),
            ("rusteal-ffi = \"=0.2.1\"", "rusteal-ffi = \"^0.2.1\""),
            ("rusteal-ffi = \"=0.2.1\"", "rusteal-ffi = \"=0.2\""),
            ("rusteal-ffi = \"=0.2.1\"", "rusteal-ffi = \"=0.2.0\""),
            ("rusteal-ffi = \"=0.2.1\"", ""),
            ("rusteal-ffi = \"=0.2.1\"", "rusteal-ffi = { path = \"/src/rusteal/rusteal-ffi\" }"),
        ] {
            let manifest = EXACT.replace(from, to);
            assert!(parse_pins(&manifest).is_err(), "{to:?} accepted");
        }
        assert!(parse_pins("[workspace]\nmembers = []\n").is_err());
    }

    #[test]
    fn compare_exact() {
        let cli = v("0.3.0");
        assert_eq!(compare(&Pins::Exact(v("0.3.0")), cli, None), Verdict::Matches);
        assert_eq!(compare(&Pins::Exact(v("0.2.1")), cli, None), Verdict::ProjectOlder(v("0.2.1")));
        assert_eq!(compare(&Pins::Exact(v("0.3.1")), cli, None), Verdict::ProjectNewer(v("0.3.1")));
    }

    #[test]
    fn compare_checkout() {
        let here = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().canonicalize().unwrap();
        let pins = Pins::Checkout(here.clone());
        let cli = v("0.2.1");
        assert_eq!(compare(&pins, cli, Some(&here)), Verdict::Matches);
        assert_eq!(compare(&pins, cli, None), Verdict::OtherCheckout(here.clone()));
        assert_eq!(
            compare(&pins, cli, Some(Path::new("/elsewhere"))),
            Verdict::OtherCheckout(here)
        );
    }
}
