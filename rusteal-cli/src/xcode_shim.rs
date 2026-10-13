// UE 5.8 runs a build's actions through UBA, which starts the `xcodebuild`
// that finalizes a Mac game's .app (ApplePostBuildSync) without a stdin, and
// Xcode 27 then cannot launch the scheme's pre-action: "Failed to finalize the
// .app with Xcode". `rusteal package` points DEVELOPER_DIR at a mirror of
// Xcode's Developer directory, symbolic links but for an `xcodebuild` that
// runs the real one with /dev/null as its stdin. Remove it once the engine
// gives xcodebuild a stdin; see the developer handbook, Platform
// optimizations.

use std::fs;
use std::path::{Path, PathBuf};

pub const FIRST_AFFECTED_XCODE: u32 = 27;

pub struct XcodeShim {
    root: PathBuf,
    developer_dir: PathBuf,
}

impl XcodeShim {
    pub fn developer_dir(&self) -> &Path {
        &self.developer_dir
    }
}

impl Drop for XcodeShim {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn xcode_major(version: &str) -> Option<u32> {
    let line = version.lines().next()?.trim();
    let number = line.strip_prefix("Xcode ")?;
    number.split('.').next()?.parse().ok()
}

#[cfg(target_os = "macos")]
pub fn for_package() -> Result<Option<XcodeShim>, String> {
    use std::process::Command;

    let output = |program: &str, args: &[&str]| {
        Command::new(program)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };

    let Some(major) = output("xcodebuild", &["-version"])
        .as_deref()
        .and_then(xcode_major)
    else {
        return Ok(None);
    };

    if major < FIRST_AFFECTED_XCODE {
        return Ok(None);
    }

    let developer_dir = output("xcode-select", &["-p"])
        .map(PathBuf::from)
        .ok_or("xcode-select -p failed: make Xcode the active developer directory")?;

    let root = std::env::temp_dir().join(format!("rusteal-xcode-{}", std::process::id()));

    if root.to_string_lossy().contains(' ') {
        return Err(format!(
            "{} has a space: the Unreal Build Tool runs xcodebuild from there unquoted",
            root.display()
        ));
    }

    let shim = build(&developer_dir, &root)
        .map_err(|e| format!("cannot mirror {}: {e}", developer_dir.display()))?;

    eprintln!(
        "rusteal package: Xcode {major}, building through {} so that xcodebuild has a stdin \
         (Unreal Engine 5.8 starts it without one, and Xcode {FIRST_AFFECTED_XCODE} fails)",
        shim.developer_dir.display()
    );

    Ok(Some(shim))
}

#[cfg(not(target_os = "macos"))]
pub fn for_package() -> Result<Option<XcodeShim>, String> {
    Ok(None)
}

#[cfg(unix)]
pub fn build(developer_dir: &Path, root: &Path) -> std::io::Result<XcodeShim> {
    let Some(contents) = developer_dir.parent() else {
        return Err(std::io::Error::other(
            "the developer directory has no parent",
        ));
    };

    let _ = fs::remove_dir_all(root);
    let shim = XcodeShim {
        root: root.to_path_buf(),
        developer_dir: root.join("XcodeShim.app/Contents/Developer"),
    };

    let mirrored = [
        (
            contents.to_path_buf(),
            shim.developer_dir.parent().unwrap().to_path_buf(),
            "Developer",
        ),
        (
            developer_dir.to_path_buf(),
            shim.developer_dir.clone(),
            "usr",
        ),
        (
            developer_dir.join("usr"),
            shim.developer_dir.join("usr"),
            "bin",
        ),
        (
            developer_dir.join("usr/bin"),
            shim.developer_dir.join("usr/bin"),
            "xcodebuild",
        ),
    ];

    for (from, to, except) in mirrored {
        fs::create_dir_all(&to)?;

        for entry in fs::read_dir(from)? {
            let entry = entry?;

            if entry.file_name() != except {
                std::os::unix::fs::symlink(entry.path(), to.join(entry.file_name()))?;
            }
        }
    }

    let quote = |path: &Path| format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"));

    let script = format!(
        "#!/bin/sh\n# Written by rusteal package (rusteal-cli/src/xcode_shim.rs).\n\
         DEVELOPER_DIR={} exec {} \"$@\" </dev/null\n",
        quote(developer_dir),
        quote(&developer_dir.join("usr/bin/xcodebuild"))
    );

    let wrapper = shim.developer_dir.join("usr/bin/xcodebuild");
    fs::write(&wrapper, script)?;

    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))?;

    Ok(shim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_major_version_comes_from_the_first_line() {
        assert_eq!(xcode_major("Xcode 27.0\nBuild version 27A266a\n"), Some(27));
        assert_eq!(xcode_major("Xcode 26.1.1\nBuild version 17B100"), Some(26));
        assert_eq!(xcode_major("xcode-select: error"), None);
    }

    #[cfg(unix)]
    #[test]
    fn the_mirror_links_everything_but_xcodebuild() {
        let dir = std::env::temp_dir().join(format!("rusteal-shim-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let developer = dir.join("Xcode.app/Contents/Developer");

        for sub in ["usr/bin", "usr/lib", "Platforms", "Toolchains"] {
            fs::create_dir_all(developer.join(sub)).unwrap();
        }

        fs::write(dir.join("Xcode.app/Contents/Info.plist"), "").unwrap();
        fs::write(developer.join("usr/bin/xcodebuild"), "real").unwrap();
        fs::write(developer.join("usr/bin/xcrun"), "").unwrap();

        let root = dir.join("shim");
        let shim = build(&developer, &root).unwrap();
        let mirror = shim.developer_dir().to_path_buf();

        assert!(mirror.ends_with("XcodeShim.app/Contents/Developer"));

        for linked in [
            "../Info.plist",
            "Platforms",
            "Toolchains",
            "usr/lib",
            "usr/bin/xcrun",
        ] {
            let path = mirror.join(linked);
            assert!(
                fs::symlink_metadata(&path).unwrap().is_symlink(),
                "{linked}"
            );
        }

        let wrapper = mirror.join("usr/bin/xcodebuild");
        assert!(!fs::symlink_metadata(&wrapper).unwrap().is_symlink());

        let script = fs::read_to_string(&wrapper).unwrap();
        assert!(script.contains("</dev/null"), "{script}");
        assert!(script.contains(&format!(
            "'{}'",
            developer.join("usr/bin/xcodebuild").display()
        )));

        drop(shim);
        assert!(!root.exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
