// The hand-written extensions in manual/ only compile inside a generated
// `bindings` crate, next to the modules they extend (`crate::core_ue`,
// `crate::engine`, ...). This test generates one from a small reflection
// fixture — tests/fixtures/uht/, just the types manual/ uses, taken from a
// UE 5.8.2 project — and runs `cargo check` on it, so a change that breaks
// manual/ fails here instead of in a game project.
//
// A second case turns on Enhanced Input, the module every game template uses,
// and checks the C++ wrappers its types need (they compile only in a UE build).
// A third adds a game crate whose `#[uclass]` declares every kind of
// `#[uproperty]` and `#[ufunction]` parameter and binds input actions, so the
// macros' expansion and manual/input_ext.rs are type-checked against real bindings.
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
    // Enhanced Input, with the Engine parents of its classes.
    "Subsystem",
    "LocalPlayerSubsystem",
    "EnhancedInputSubsystemInterface",
    "EnhancedInputLocalPlayerSubsystem",
    "DataAsset",
    "PrimaryDataAsset",
    "InputAction",
    "InputMappingContext",
    "InputModifier",
    "PlayerInput",
    "EnhancedPlayerInput",
    "PlayerMappableInputConfig",
    "InputComponent",
    "EnhancedInputComponent",
    // What manual/input_ext.rs calls: the subsystem lookup and the value reader.
    // `Class` is the type of the `TSubclassOf` parameter of the lookup.
    "Field",
    "Struct",
    "Class",
    "Controller",
    "PlayerController",
    "BlueprintFunctionLibrary",
    "SubsystemBlueprintLibrary",
    "EnhancedInputLibrary",
    // The touch controls the Third Person player controller spawns.
    "Visual",
    "Widget",
    "UserWidget",
];

/// Enums manual/ names that no class or struct above references.
const ENUMS: &[&str] = &["ETriggerEvent", "EInputActionValueType"];

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
    // The hit result's vectors (manual/net_quantize.rs).
    "Vector_NetQuantize",
    "Vector_NetQuantize10",
    "Vector_NetQuantize100",
    "Vector_NetQuantizeNormal",
    // Enhanced Input.
    "InputActionValue",
    "ModifyContextOptions",
    "EnhancedActionKeyMapping",
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
EnhancedInput = { module = "enhanced_input", feature = "enhanced-input" }
"#;

/// RUSTEAL_TOML with the Enhanced Input feature on.
fn rusteal_toml_enhanced_input() -> String {
    RUSTEAL_TOML.replace(r#""umg"]"#, r#""umg", "enhanced-input"]"#)
}

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

/// A project with nothing but what codegen reads, generated with `rusteal_toml`,
/// and `cargo check`ed. Returns the project directory.
fn generate_and_check(case: &str, rusteal_toml: &str) -> PathBuf {
    let work = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("manual_compiles");
    let project = work.join(case);
    if project.exists() {
        std::fs::remove_dir_all(&project).unwrap();
    }

    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("rusteal.toml"), rusteal_toml).unwrap();
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

    let output = cargo_check(&project, "bindings");
    assert!(
        output.status.success(),
        "the generated bindings, manual/ included, do not compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    project
}

/// `cargo check` one package of a generated project's Rust workspace.
fn cargo_check(project: &Path, package: &str) -> std::process::Output {
    let work = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("manual_compiles");
    Command::new(env!("CARGO"))
        .arg("check")
        .arg("--manifest-path")
        .arg(project.join("Rust/Cargo.toml"))
        .arg("--target-dir")
        .arg(work.join("target"))
        .args(["-p", package, "--all-features"])
        .output()
        .expect("run cargo check")
}

#[test]
fn manual_compiles_against_generated_bindings() {
    generate_and_check("project", RUSTEAL_TOML);
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn enhanced_input_bindings() {
    let project = generate_and_check("enhanced_input", &rusteal_toml_enhanced_input());
    let cpp = project.join("Plugins/Rusteal/Source/Rusteal/Generated");
    let rust = project.join("Rust/bindings/src/enhanced_input");

    // A UFUNCTION declared on an interface is generated on the classes
    // implementing it, and called through the interface.
    let subsystem = read(&cpp.join("RustealFunc_enhanced_input_EnhancedInputLocalPlayerSubsystem.cpp"));
    assert!(subsystem.contains("static_cast<IEnhancedInputSubsystemInterface*>(Self)->AddMappingContext("));
    assert!(read(&rust.join("enhanced_input_local_player_subsystem.rs")).contains("fn add_mapping_context("));

    // A protected `_Implementation` is reached through a using-declaration.
    let modifier = read(&cpp.join("RustealFunc_enhanced_input_InputModifier.cpp"));
    assert!(modifier.contains("using UInputModifier::ModifyRaw_Implementation;"));
    assert!(modifier.contains("(Self->*&FAccess::ModifyRaw_Implementation)("));

    // A returned container is written as the function's own return type, and a
    // struct the function's header only forward-declares gets its header included.
    let config = read(&cpp.join("RustealFunc_enhanced_input_PlayerMappableInputConfig.cpp"));
    assert!(config.contains("std::decay_t<decltype(Self->GetMappingContexts())>"));
    assert!(config.contains("#include \"EnhancedActionKeyMapping.h\""));

    // Turning the module off removes its directory, not only its `mod` line.
    std::fs::write(project.join("rusteal.toml"), RUSTEAL_TOML).unwrap();
    rusteal_codegen::run_generate(&project);
    assert!(!rust.exists(), "enhanced_input/ left behind after the module was turned off");
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

    let mut enum_names: BTreeSet<String> = ENUMS.iter().map(|e| e.to_string()).collect();
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

/// A game class with one `#[uproperty]` of each supported kind, `#[ufunction]`s
/// taking and returning each supported kind, and code using every accessor the
/// macros generate, the Enhanced Input helpers and the touch controls ones.
const UCLASS_GAME: &str = r#"
use bindings::engine::{Actor, ActorExt, Controller, Pawn, PawnExt, PlayerController, SceneComponent};
use bindings::enhanced_input::{
    ETriggerEvent, EnhancedInputLocalPlayerSubsystemExt, FInputActionValue, InputAction,
    InputMappingContext,
};
use bindings::prelude::*;
use bindings::umg::{UserWidget, UserWidgetExt};
use rusteal_runtime::runtime::input::should_display_touch_interface;
use rusteal_runtime::runtime::{
    FName, OutRef, OwnedStruct, RustealResult, SubclassOf, UObjectRef, UStructRef, UeArray,
};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = Pawn)]
pub struct Probe {
    #[component(attach = "root_component")]
    arm: SceneComponent,
    #[component(attach = "arm", socket = "Tip")]
    tip: SceneComponent,
    #[uproperty(EditAnywhere, default = 2.5)]
    speed: f32,
    #[uproperty(EditAnywhere, BlueprintReadWrite)]
    jump_action: UObjectRef<InputAction>,
    #[uproperty(EditAnywhere, BlueprintReadOnly)]
    fixed_action: UObjectRef<InputAction>,
    #[uproperty(EditAnywhere)]
    pawn_class: SubclassOf<Pawn>,
    #[uproperty(EditAnywhere)]
    contexts: UeArray<UObjectRef<InputMappingContext>>,
    #[uproperty(EditAnywhere)]
    classes: UeArray<SubclassOf<Actor>>,
    #[uproperty(EditAnywhere)]
    weights: UeArray<f32>,
    #[uproperty(EditAnywhere)]
    widget_class: SubclassOf<UserWidget>,
    #[uproperty]
    widget: UObjectRef<UserWidget>,
    #[uproperty(EditDefaultsOnly, default = true)]
    b_can_dash: bool,
    #[uproperty(VisibleAnywhere)]
    seen: UObjectRef<InputAction>,
    #[uproperty(EditAnywhere, category = "Platform")]
    target: OwnedStruct<FVector>,
    #[uproperty(EditAnywhere)]
    tag: FName,
    #[uproperty(EditAnywhere)]
    label: String,
    #[uproperty(EditAnywhere, default = ETriggerEvent::Started)]
    trigger: ETriggerEvent,
    #[uproperty(VisibleAnywhere, name = "NPC", category = "Context")]
    npc: UObjectRef<Pawn>,
    #[component(attach = "arm", name = "Collision Check Box")]
    collision_check_box: SceneComponent,
}

#[uclass_impl]
impl Probe {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.set_speed(1.0);
        let _ = self.arm()?;
        let _ = self.tip()?;
        Ok(())
    }

    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        let _ = self.bind_input();
    }

    #[ufunction(BlueprintCallable)]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        let axis = value.axis2d();
        self.set_speed(axis.x as f32 + value.axis1d() as f32 + value.axis3d().z as f32);
        let _ = value.get_bool();
    }

    #[ufunction(BlueprintCallable)]
    fn take_objects(&mut self, action: UObjectRef<InputAction>, class: SubclassOf<Pawn>) -> UObjectRef<InputAction> {
        self.set_pawn_class(class);
        action
    }

    #[ufunction(BlueprintCallable)]
    fn pick_class(&mut self, index: i32) -> SubclassOf<Actor> {
        self.classes().get(index as usize).unwrap_or_default()
    }

    #[ufunction(BlueprintPure)]
    fn can_dash(&self) -> bool {
        self.b_can_dash()
    }

    #[ufunction(Override, name = "K2_OnBecomeViewTarget")]
    fn on_become_view_target(&mut self, pc: UObjectRef<PlayerController>) {
        self.set_trail(true);
        let _ = self.pick_target(pc);
        self.set_b_can_dash(self.count_targets(2) > 0);
    }

    #[ufunction(BlueprintImplementableEvent)]
    fn set_trail(&self, b_enabled: bool) {}

    #[ufunction(BlueprintImplementableEvent)]
    fn pick_target(&self, pc: UObjectRef<PlayerController>) -> UObjectRef<Actor> {}

    #[ufunction(BlueprintImplementableEvent)]
    fn count_targets(&self, max: i32) -> i32 {}

    #[ufunction(Override, name = "BlueprintUpdateCamera")]
    fn update_camera(
        &mut self,
        _camera_target: UObjectRef<Actor>,
        new_camera_location: UStructRef<FVector>,
        _new_camera_rotation: UStructRef<FRotator>,
        new_camera_fov: OutRef<f32>,
    ) -> bool {
        new_camera_location.set_x(self.target().as_ref().get_x());
        new_camera_fov.set(new_camera_fov.get() + 1.0);
        self.set_target(&OwnedStruct::new());
        self.set_tag(FName::new(&self.label()));
        self.set_label("probe");
        self.set_trigger(ETriggerEvent::Completed);
        let _ = (self.trigger(), self.npc(), self.collision_check_box());
        true
    }
}

pub fn worlds(probe: UObjectRef<Probe>, class: SubclassOf<Pawn>) -> RustealResult<UObjectRef<Pawn>> {
    let actor = probe.upcast_to::<Actor>();
    actor.checked()?.on_destroyed().add_ufunction(&probe, "PickClass")?;
    let world = probe.get_world()?;
    world.spawn_actor_of_class(class, &OwnedStruct::new())
}

pub fn upcasts(probe: UObjectRef<Probe>, pc: UObjectRef<PlayerController>) -> (UObjectRef<Actor>, UObjectRef<Pawn>) {
    let _controller: UObjectRef<Controller> = pc.upcast_to::<Controller>();
    let _actor_class: SubclassOf<Actor> = SubclassOf::<PlayerController>::base().upcast_to::<Actor>();
    (pc.upcast_to::<Actor>(), probe.upcast_to::<Pawn>())
}

#[uclass(parent = Actor)]
pub struct Plain {}

#[uclass_impl]
impl Plain {
    #[class_defaults]
    fn class_defaults(&mut self) {}
}

impl Probe {
    fn bind_input(&self) -> RustealResult<()> {
        let me = self.as_ref();
        bind_action(&me, self.jump_action(), ETriggerEvent::Started, "Jump")?;
        bind_action(&me, self.jump_action(), ETriggerEvent::Completed, "StopJumping")?;
        bind_action(&me, self.jump_action(), ETriggerEvent::Triggered, "Move")
    }
}

pub fn add_contexts(controller: UObjectRef<PlayerController>, contexts: UeArray<UObjectRef<InputMappingContext>>) -> RustealResult<()> {
    let subsystem = enhanced_input_subsystem(controller)?.checked()?;
    for context in contexts.to_vec()? {
        subsystem.add_mapping_context(context, 0, &OwnedStruct::new());
    }
    Ok(())
}

/// Generated class references are `SubclassOf<T>` (TSubclassOf), not objects.
pub fn use_generated_classes(p: &Probe) -> RustealResult<()> {
    let pawn = p.as_ref().checked()?;
    let class: SubclassOf<Controller> = pawn.get_ai_controller_class();
    pawn.set_ai_controller_class(class);
    Ok(())
}

/// The touch controls, as the Third Person template's player controller spawns them.
pub fn spawn_touch_controls(p: &Probe, controller: UObjectRef<PlayerController>) -> RustealResult<()> {
    if should_display_touch_interface() {
        let widget = create_widget_of_class(&controller, p.widget_class())?;
        widget.checked()?.add_to_player_screen(Some(0));
        p.set_widget(widget);
    }
    let _: RustealResult<UObjectRef<UserWidget>> = create_widget(&controller);
    Ok(())
}

pub fn use_accessors(p: &Probe) {
    p.set_speed(p.speed() * 2.0);
    let action: UObjectRef<InputAction> = p.jump_action();
    p.set_jump_action(action);
    let _: UObjectRef<InputAction> = p.fixed_action();
    let class: SubclassOf<Pawn> = p.pawn_class();
    p.set_pawn_class(class);
    let contexts: UeArray<UObjectRef<InputMappingContext>> = p.contexts();
    let _ = contexts.len();
    let _ = p.classes().to_vec();
    let _ = p.weights().push(&1.0);
}
"#;

#[test]
fn uclass_types_compile() {
    let project = generate_and_check("uclass", &rusteal_toml_enhanced_input());
    let rust = project.join("Rust");

    // A game crate next to the bindings, as `rusteal new` lays it out.
    let checkout = crate_dir().join("..").canonicalize().unwrap();
    let manifest = read(&rust.join("Cargo.toml"))
        .replace(r#"members = ["bindings"]"#, r#"members = ["bindings", "game"]"#)
        + &format!(
            "rusteal-runtime = {{ path = {:?} }}\n",
            checkout.join("rusteal-runtime")
        );
    std::fs::write(rust.join("Cargo.toml"), manifest).unwrap();
    std::fs::create_dir_all(rust.join("game/src")).unwrap();
    std::fs::write(
        rust.join("game/Cargo.toml"),
        "[package]\n\
         name = \"game\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\
         \n\
         [dependencies]\n\
         rusteal-runtime = { workspace = true }\n\
         bindings = { path = \"../bindings\" }\n",
    )
    .unwrap();
    std::fs::write(rust.join("game/src/lib.rs"), UCLASS_GAME).unwrap();

    let output = cargo_check(&project, "game");
    assert!(
        output.status.success(),
        "a #[uclass] with every #[uproperty] and #[ufunction] kind does not compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
