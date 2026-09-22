// The hand-written extensions in manual/ only compile inside a generated
// `bindings` crate, next to the modules they extend (`crate::core_ue`,
// `crate::engine`, ...). This test generates one from a small reflection
// fixture — tests/fixtures/uht/, just the types manual/ uses, taken from a
// UE 5.8.2 project — and runs `cargo check` on it, so a change that breaks
// manual/ fails here instead of in a game project.
//
// Regenerating the fixture (a new engine version, a new exporter, a new type
// used by manual/):
//
//   RUSTEAL_UHT_DIR=<project>/Intermediate/Rusteal/uht \
//     cargo test -p rusteal-codegen --test manual_compiles -- --ignored regenerate_fixture

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Classes manual/ extends or calls into, with their parents.
const CLASSES: &[&str] = &[
    "Object",
    "Actor",
    "Pawn",
    "World",
    "ActorComponent",
    "SceneComponent",
];

/// Structs manual/ extends.
const STRUCTS: &[&str] = &[
    "Vector",
    "Vector2D",
    "Vector4",
    "Quat",
    "Rotator",
    "Transform",
    "LinearColor",
    "Color",
    "Plane",
    "Box2D",
    "Key",
];

const RUSTEAL_TOML: &str = r#"[project]
crate = "probe"

[codegen]
features = ["core", "engine", "input", "slate", "umg"]

[codegen.modules]
CoreUObject = { module = "core_ue", feature = "core" }
Engine = { module = "engine", feature = "engine" }
InputCore = { module = "input_core", feature = "input" }
SlateCore = { module = "slate_core", feature = "slate" }
Slate = { module = "slate", feature = "slate" }
UMG = { module = "umg", feature = "umg" }
"#;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_dir() -> PathBuf {
    crate_dir().join("tests/fixtures/uht")
}

/// The workspace's glam requirement, so the probe builds with the same one.
fn workspace_glam() -> String {
    let manifest = std::fs::read_to_string(crate_dir().join("../Cargo.toml")).unwrap();
    let manifest: toml::Table = toml::from_str(&manifest).unwrap();
    manifest["workspace"]["dependencies"]["glam"]
        .as_str()
        .expect("glam = \"x.y.z\" in [workspace.dependencies]")
        .to_string()
}

fn copy_fixture(to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(fixture_dir()).unwrap() {
        let path = entry.unwrap().path();
        std::fs::copy(&path, to.join(path.file_name().unwrap())).unwrap();
    }
}

#[test]
fn manual_compiles_against_generated_bindings() {
    let work = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("manual_compiles");
    let project = work.join("project");
    if project.exists() {
        std::fs::remove_dir_all(&project).unwrap();
    }

    // A project with nothing but what codegen reads.
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("rusteal.toml"), RUSTEAL_TOML).unwrap();
    copy_fixture(&project.join("Intermediate/Rusteal/uht"));

    rusteal_codegen::run_generate(&project);

    // The workspace a game project would have, depending on this checkout.
    let checkout = crate_dir().join("..").canonicalize().unwrap();
    let workspace = format!(
        "[workspace]\n\
         members = [\"bindings\"]\n\
         resolver = \"3\"\n\
         \n\
         [workspace.dependencies]\n\
         rusteal-core = {{ path = {core:?} }}\n\
         rusteal-ffi = {{ path = {ffi:?} }}\n\
         glam = {glam:?}\n",
        core = checkout.join("rusteal-core"),
        ffi = checkout.join("rusteal-ffi"),
        glam = workspace_glam(),
    );
    std::fs::write(project.join("Rust/Cargo.toml"), workspace).unwrap();

    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--manifest-path")
        .arg(project.join("Rust/Cargo.toml"))
        .arg("--target-dir")
        .arg(work.join("target"))
        .args(["-p", "bindings", "--all-features"])
        .output()
        .expect("run cargo check");
    assert!(
        output.status.success(),
        "the generated bindings, manual/ included, do not compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Every `enum_name` referenced anywhere inside `value`.
fn referenced_enums(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                if key == "enum_name"
                    && let Value::String(name) = v
                    && !name.is_empty()
                {
                    out.insert(name.clone());
                }
                referenced_enums(v, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|v| referenced_enums(v, out)),
        _ => {}
    }
}

/// The records under `key` in `file` whose name is in `names`,
/// in the order of `names`.
fn select(file: &Value, key: &str, names: &[&str]) -> Vec<Value> {
    let records = file[key].as_array().unwrap();
    names
        .iter()
        .map(|name| {
            records
                .iter()
                .find(|r| r["name"] == *name)
                .unwrap_or_else(|| panic!("{key}: {name} not in the reflection data"))
                .clone()
        })
        .collect()
}

#[test]
#[ignore = "rewrites the fixture from a project's reflection JSON (RUSTEAL_UHT_DIR)"]
fn regenerate_fixture() {
    let source = PathBuf::from(
        std::env::var("RUSTEAL_UHT_DIR").expect("RUSTEAL_UHT_DIR=<project>/Intermediate/Rusteal/uht"),
    );
    let read = |name: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(source.join(name)).unwrap()).unwrap()
    };
    let classes = read("rusteal_classes.json");
    let structs = read("rusteal_structs.json");
    let enums = read("rusteal_enums.json");

    let classes = select(&classes, "classes", CLASSES);
    let structs = select(&structs, "structs", STRUCTS);

    let mut enum_names = BTreeSet::new();
    classes.iter().chain(&structs).for_each(|r| referenced_enums(r, &mut enum_names));
    let enums: Vec<Value> = enums["enums"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["name"].as_str().is_some_and(|n| enum_names.contains(n)))
        .cloned()
        .collect();

    let out = fixture_dir();
    std::fs::create_dir_all(&out).unwrap();
    let write = |name: &str, key: &str, records: Vec<Value>| {
        let mut file = serde_json::Map::new();
        file.insert(key.to_string(), Value::Array(records));
        let text = serde_json::to_string_pretty(&Value::Object(file)).unwrap() + "\n";
        std::fs::write(out.join(name), text).unwrap();
    };
    write("rusteal_classes.json", "classes", classes);
    write("rusteal_structs.json", "structs", structs);
    write("rusteal_enums.json", "enums", enums);
}
