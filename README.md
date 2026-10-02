<div align="center">

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/logo_nobg_orange.png" alt="Rusteal logo" width="256">

# Rusteal

[![Crates.io](https://img.shields.io/crates/v/rusteal.svg)](https://crates.io/crates/rusteal)
[![License](https://img.shields.io/crates/l/rusteal.svg)](https://github.com/viniciusmorgado/rusteal#license)
[![dependency status](https://deps.rs/crate/rusteal/latest/status.svg)](https://deps.rs/crate/rusteal)

</div>

---

**Rust bindings for Unreal Engine 5.8**

Rusteal lets you write Unreal Engine gameplay in Rust. Your Rust code compiles to a shared library that is loaded by a small UE C++ plugin. All UE API calls cross the FFI boundary through a function pointer table — no C++ compilation required during Rust iteration.

> **⚠️ Early Stage Project** — Rusteal is under active development and **not ready for production use**. APIs will change without notice, documentation is incomplete, and many UE features are not yet covered. Contributions and feedback are welcome, but please do not use this for shipping projects.

This README serves two different readers:

| You want to | Read | You need |
|---|---|---|
| **make a game** with Rusteal | [Making a game](#making-a-game) | the `rusteal` CLI from crates.io; not this repository |
| **work on Rusteal itself** (its crates, CLI or UE plugins) | [Working on Rusteal](#working-on-rusteal) | a clone of this repository |

## Acknowledgments

Rusteal is a hard fork of [**uika**](https://github.com/VioletHelianthus/uika) by [**VioletHelianthus**](https://github.com/VioletHelianthus), who designed and wrote the foundation this project stands on: the reflection-driven code generator, the UHT exporter plugin, the reification of Rust structs as `UClass`es, and the runtime. Thank you.

Rusteal does not intend to stay compatible with the original uika, unless its author wants to port the improvements back. From here on it develops the Unreal bindings independently.

## Example

[`examples/gem_collector`](https://github.com/viniciusmorgado/rusteal/tree/main/examples/gem_collector)
in this repository is a full working demo of the runtime API: copy the
directory into a project's `Rust/` and add it to the workspace members.

## Making a game

Everything from here to [Platform Support](#platform-support) is about using
Rusteal to write a game. None of it needs this repository.

### Prerequisites

The versions come from the engine's own requirements
(`Engine/Config/Windows/Windows_SDK.json` and `Engine/Config/Linux/Linux_SDK.json`
in UE 5.8.2) and from Rusteal's crates, which need Rust 1.88.

**Linux (x64)**

| Dependency | Version | Where from | What for |
|---|---|---|---|
| A C toolchain | any | `build-essential` (Debian, Ubuntu), `gcc` (Fedora), `base-devel` (Arch) | the linker Rust uses (`cc`) |
| Rust | stable, 1.88 or newer | [rustup](https://rustup.rs) | installing the `rusteal` CLI, building your game crate |
| Unreal Engine | 5.8 | Epic's Linux build ([unrealengine.com/linux](https://www.unrealengine.com/linux)) or built from source | the game |

The engine brings the rest: the clang 20.1.8 toolchain UBT compiles with
(`v26_clang-20.1.8-rockylinux8`) and the .NET 10 SDK that runs UBT and UHT.

**Windows (x64)**

| Dependency | Version | Where from | What for |
|---|---|---|---|
| Visual Studio | 2022 17.8 or newer, or 2026 18.0 or newer | [visualstudio.microsoft.com](https://visualstudio.microsoft.com) | the C++ compiler UBT uses, and the linker Rust uses |
| Rust | stable, 1.88 or newer, `x86_64-pc-windows-msvc` | [rustup](https://rustup.rs) (`rustup-init.exe`) | installing the `rusteal` CLI, building your game crate |
| Unreal Engine | 5.8 | the Epic Games Launcher | the game |

In the Visual Studio Installer, the workloads and components the engine asks for:

- workloads **Desktop development with C++**, **Game development with C++**
  (with its Unreal Engine components) and **.NET desktop development**;
- **MSVC v143 x64/x86 build tools 14.44** for Visual Studio 2022 (14.50 for
  2026). The engine refuses 14.39 to 14.43, 14.44 before 14.44.35211 and 14.50
  before 14.50.35723;
- **Windows 11 SDK 10.0.22621** (10.0.19041 at least);
- **.NET Framework 4.6.2 targeting pack**.

Install Visual Studio before Rust: rustup uses its build tools for the MSVC
toolchain. The engine brings the .NET SDK for UBT and UHT.

### Install

```bash
cargo install rusteal
```

That installs the `rusteal` binary; the engine plugins and the project
templates travel inside it. The first command that needs the engine asks where
it is and keeps the answer in `~/.config/rusteal/config.toml` — the only
per-machine setting Rusteal has.

### A new project

```bash
rusteal new MyGame --dir ~/Projects
```

`rusteal new` creates a UE project from one of the [templates](#templates),
`blank` unless `--template` names another, installs the Rusteal plugins, writes
the Rust workspace and runs the whole build pipeline. With `blank`, open
`MyGame.uproject`, drop a `HelloActor` into the level and press Play: the
Output Log shows `[MyGame] Hello from Rust`.

It does not run `git init` — versioning is your call — but it does write a
`.gitignore` covering the Unreal and Rust build output, so a later `git init`
picks up the right files.

Two flags: `--no-build` stops after writing the project, leaving the pipeline
for a later `rusteal build`; `--runtime-path` makes the project depend on a
Rusteal checkout instead of the published crates, which is for working on
Rusteal itself — see [Working on Rusteal](#working-on-rusteal).

### Templates

```bash
rusteal new MyGame --template third-person
rusteal new MyGame --template third-person --variant combat
```

A template is one of the engine's C++ templates with its gameplay written in
Rust: the C++ classes are gone, the Rust crate in `Rust/` takes their place,
and their Blueprint children, with the same names, have Rust parents.

The engine's templates come with variants, genre starting points the C++
version puts all in one project. Here each is its own project: `--variant`
picks one, and without it the project is the template alone (`base`). A
variant project is the template plus that variant, as the engine's Blueprint
templates make it, and opens and plays the variant's level.

| Template | Variant | What it is |
|---|---|---|
| `blank` (default) | | The engine's Blank template and a `HelloActor` in Rust. |
| `first-person` | | The engine's First Person template: its character, camera manager, game mode and player controller in Rust, playable as it comes (keyboard, mouse, gamepad, touch). |
| `first-person` | `horror` | A dark level explored with a flashlight and a sprint that runs on stamina, its meter on screen. |
| `third-person` | | The engine's Third Person template: its character, game mode and player controller in Rust, playable as it comes (keyboard, mouse, gamepad, touch). |
| `third-person` | `combat` | Melee combat: combo and charged attacks, enemies run by a StateTree, spawners, checkpoints, damageable props. |
| `third-person` | `platforming` | Double jump, wall jump, coyote time, a dash, local multiplayer and respawn. |
| `third-person` | `side-scrolling` | A side view platformer: soft platforms, jump pads, moving platforms, pickups with a counter and an NPC run by a StateTree. |

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/third-person.webp" alt="The third-person template in play" width="640">

Names are matched loosely (`side-scrolling`, `SideScrolling`), and `rusteal
new` with an unknown template or variant lists them all.

### An existing project

```bash
rusteal setup /path/to/YourProject   # plugins + starter rusteal.toml
rusteal build                        # from anywhere inside the project
```

`setup` does not write the Rust workspace; create it as below, or copy the
layout `rusteal new` produces.

### Versions

A project is tied to one Rusteal version: `Rust/Cargo.toml` pins
`rusteal-runtime`, `rusteal-core` and `rusteal-ffi` to it (`"=x.y.z"`), and the
plugins in `Plugins/` carry it. Before doing anything, `build`, `generate` and
`setup` check it against the CLI's own version (`rusteal --version`):

- the CLI is newer: `rusteal upgrade` moves the project to it — the pins, the
  plugins (`Plugins/Rusteal` and `Plugins/RustealGenerator` are replaced
  wholesale) and a full build;
- the project is newer: install the CLI it uses, `cargo install rusteal@x.y.z`.

The plugin checks the library as well: a `librusteal` built against another
version is refused at load, with both versions in the Output Log.

### Project layout

Everything lives in one repository; the only thing outside it is the engine path
in the CLI's own configuration.

```
YourProject/
├── YourProject.uproject
├── rusteal.toml                # project configuration (versioned)
├── Source/YourProject/         # the C++ module UBT needs
├── Plugins/Rusteal/            # runtime plugin + generated C++ wrappers
├── Plugins/RustealGenerator/   # UHT exporter
├── Intermediate/Rusteal/uht/   # reflection JSON (build output)
└── Rust/
    ├── Cargo.toml              # workspace
    ├── your-game/              # your code (cdylib)
    └── bindings/               # generated by `rusteal build`, committed
```

**`Rust/Cargo.toml`** — `rusteal new` pins the version of the CLI that created
the project:
```toml
[workspace]
members = ["your-game", "bindings"]
resolver = "3"

[workspace.dependencies]
rusteal-runtime = "=x.y.z"
rusteal-core = "=x.y.z"
rusteal-ffi = "=x.y.z"
glam = "0.33"
```

**`Rust/your-game/Cargo.toml`:**
```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
rusteal-runtime = { workspace = true }
bindings = { path = "../bindings" }
glam = { workspace = true }
```

**`Rust/your-game/src/lib.rs`:**
```rust
rusteal_runtime::entry!();

mod my_game;
```

The runtime types come from `rusteal_runtime::prelude`, the engine types from
the generated crate's own `bindings::prelude`.

## Build Pipeline

The CLI orchestrates a 5-step build:

| Step | Command | What it does |
|------|---------|-------------|
| 1 | UE Build | Compiles UE project, triggers RustealGenerator → JSON reflection data |
| 2 | Codegen | Reads JSON → generates the `bindings` crate + C++ wrappers |
| 3 | UE Rebuild | Compiles the generated C++ wrappers into the UE module |
| 4 | Cargo Build | `cargo build` of your cdylib crate, with the dev profile (`--release`: the release profile) |
| 5 | Deploy | Copies the library to `Plugins/Rusteal/Binaries/<Platform>/` (`rusteal.dll`, `librusteal.so`) |

Commands take the project directory, or find it by walking up from the current
one:
```bash
# Full build
rusteal build

# Rust-only rebuild (skip the UE steps)
rusteal build --from 4

# Codegen and everything after
rusteal build --from 2

# Just regenerate the bindings crate and the C++ wrappers
rusteal generate /path/to/YourProject

# The game library for shipping
rusteal build --release
```

### Development and release builds

The game library is built with Cargo's dev profile by default, which is what
every build above does while you work on the game, and with the release
profile when `--release` is given, which is the build to ship. The two want
opposite things, and the templates set them up for that in the workspace's
`Rust/Cargo.toml`, the only place Cargo reads profiles from (in the game
crate's own `Cargo.toml` they are ignored with a warning):

- **dev** (iterating): recompiling after a change must be fast, and the game
  must still run well in the editor. Only the game crate, the one that
  changes, is barely optimized; the runtime crates and the generated
  bindings, which compile once, are fully optimized. A change to the game
  crate rebuilds in about a second.
- **release** (shipping): the fastest library possible, however long it
  takes to build. The whole program is optimized as one unit (link-time
  optimization, a single codegen unit), which also drops the parts of the
  bindings the game does not use.

The settings follow Bevy's recommendations for game projects
([Bevy setup](https://bevy.org/learn/quick-start/getting-started/setup/)). The
Rusteal crates and the `rusteal` binary are not affected: these profiles only
shape the game's library.

## Key Concepts

### Object References

```rust
// Lightweight handle (8 bytes, Copy). May become invalid if UE GCs the object.
let actor: UObjectRef<Actor> = world.spawn_actor(&transform)?;

// RAII GC root. Prevents garbage collection until dropped.
let pinned: Pinned<Actor> = actor.pin()?;

// Checked access — verifies the object is still alive before use.
let checked = actor.checked()?;
checked.k2_get_actor_location();

// Upcast to any ancestor, checked at compile time (a Character is an Actor);
// downcast with `cast::<T>()`, checked at runtime.
let as_object: UObjectRef<Object> = actor.upcast_to::<Object>();
```

### Defining UE Classes

```rust
#[uclass(parent = Actor)]
pub struct MyActor {
    #[component(root)]
    root: SceneComponent,

    #[component(attach = "root")]
    mesh: StaticMeshComponent,

    #[uproperty(BlueprintReadWrite, default = 100)]
    health: i32,

    // Assets and classes, assigned in a Blueprint child
    #[uproperty(EditAnywhere)]
    pickup_sound: UObjectRef<SoundBase>,
    #[uproperty(EditAnywhere)]
    projectile_class: SubclassOf<Actor>,
    #[uproperty(EditAnywhere)]
    materials: UeArray<UObjectRef<MaterialInterface>>,

    // Rust-only field (not exposed to UE)
    internal_state: Vec<String>,
}

#[uclass_impl]
impl MyActor {
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) { /* ... */ }

    #[ufunction(BlueprintCallable)]
    fn take_damage(&mut self, amount: i32) {
        self.set_health(self.health() - amount);
    }
}
```

A `#[uproperty]` is `bool`, `i32`, `i64`, `u8`, `f32`, `f64`, an object
(`UObjectRef<T>`), a class (`SubclassOf<T>`, UE's `TSubclassOf<T>`), a
`UeArray` of any of those or of `FName`s, a struct (`OwnedStruct<FVector>`), an `FName`, a
`String` (`FString`) or an engine enum (`ECollisionChannel`). Each gets a
getter named after the field and a `set_` setter (`BlueprintReadOnly` is
about Blueprints; the class's own code writes it, as C++ does); an array's
getter returns a view of the array inside the object, changed in place, and
a struct's a copy. `default = ...` is for the scalar
types and enums; the others are set in `#[class_defaults]` or a Blueprint
child, as the engine's templates do. `EditAnywhere`, `EditDefaultsOnly`,
`VisibleAnywhere`, `BlueprintReadWrite` and `BlueprintReadOnly` are UE's
specifiers.

The UE name is the field's in PascalCase, with a `b_` prefix as UE's bool
`b` (`b_force_touch_controls` is `bForceTouchControls`, as the bindings name
UE's own); `name = "NPC"` gives another, and `category = "..."` the property's
category. Matching the C++ names keeps the values a Blueprint saved when its
parent moves from C++ to Rust.

A `#[component]` is created with the object. `attach` names its parent: a
component the class declares before it, or an inherited one by its field-style
name (`root_component`, `mesh`); `socket` names a socket on that parent:

```rust
#[component(attach = "root_component")]
camera_boom: SpringArmComponent,
#[component(attach = "camera_boom", socket = "SpringEndpoint")]
follow_camera: CameraComponent,
```

A component's subobject is named after its field too, or as
`#[component(name = "Collision Check Box")]` says: a Blueprint child's
changes to an inherited component are kept by that name.

What a C++ constructor sets on inherited properties goes in a
`#[class_defaults]` method of the `#[uclass_impl]` block. It runs once, on the
class default object, which every instance and Blueprint child starts from:

```rust
#[class_defaults]
fn class_defaults(&mut self) -> RustealResult<()> {
    let me = self.as_ref().checked()?;
    me.set_use_controller_rotation_yaw(false);
    me.get_character_movement().checked()?.set_max_walk_speed(500.0);
    self.camera_boom()?.checked()?.set_target_arm_length(400.0);
    Ok(())
}
```

A Rust class's parent may be a Rust class, as a C++ class's may be a C++
class: `#[uclass(parent = FirstPersonCharacter)]` makes a class with the
parent's properties, components and functions, whose components may attach to
the parent's by their field names (`attach = "first_person_camera_component"`)
and whose class defaults start from the parent's. An event both override runs
the child's code, which calls the parent's as C++ calls `Super::`:

```rust
#[ufunction(Override)]
fn receive_begin_play(&mut self) {
    if let Ok(mut parent) = FirstPersonCharacter::from_obj(self.as_ref()) {
        parent.receive_begin_play(); // pub(crate) in the parent's impl
    }
    // ...
}
```

In the generated bindings, an engine class reference (`TSubclassOf<T>`, such
as a game mode's `DefaultPawnClass`) is a `SubclassOf<T>`, not an object:
`get_default_pawn_class()` returns `SubclassOf<Pawn>`, and
`SubclassOf::<MyPawn>::base().upcast()` passes a Rust class where a parent's is
expected.

Static functions, such as a function library's (`UGameplayStatics`,
`UKismetSystemLibrary`), are called on the class, as in C++:
`GameplayStatics::get_player_controller(world, 0)`.

Private engine properties that Blueprint can read (`ACharacter`'s `Mesh`,
`CharacterMovement` and `CapsuleComponent`) have getters, and no setters when
they are read-only.

A `#[ufunction]` takes the scalar types, objects (`UObjectRef<T>`), classes
(`SubclassOf<T>`) and structs (`UStructRef<T>`, a reference into the call's
parameters; `to_owned()` keeps a copy), and returns a scalar, an object or a
class. It is `BlueprintCallable` unless it says otherwise:

- `BlueprintPure`: callable without execution pins;
- `Override`: Rust code for an engine event of a parent class, taking what it
  takes (an enum as its type or `u8`, an out struct as a `UStructRef` written
  in place, a scalar out parameter as an `OutRef<T>`). A C++ virtual has its
  event: `Landed` is `OnLanded`, `Tick` is `ReceiveTick`, `EndPlay` is
  `ReceiveEndPlay`, `OnPossess` is `ReceivePossess`;
- `BlueprintImplementableEvent`: an event a Blueprint child implements; the
  method's body is empty, and calling it runs the Blueprint's graph. It
  takes structs as `&OwnedStruct<T>`.

`name = "K2_OnMovementModeChanged"` gives the UE name when the method's name
in PascalCase is not it.

### Defining UE Structs

A data table's rows, or a struct a class's properties hold, can be declared in
Rust too, as UHT's `USTRUCT()`:

```rust
#[ustruct]
pub struct WeaponTableRow {
    /// Mesh to display on the pickup
    #[uproperty(EditAnywhere)]
    static_mesh: SoftObjectRef<StaticMesh>,

    /// Weapon class to grant on pickup
    #[uproperty(EditAnywhere)]
    weapon_to_spawn: SubclassOf<ShooterWeapon>,
}
```

Every field is a `#[uproperty]` of the kinds a class's are (no `default`: a
struct starts zeroed), since the struct's memory is UE's: a data table's, an
object property's, an `OwnedStruct`'s. The type itself is a marker, like the
bindings' engine structs, and the fields are read and written through the
generated `WeaponTableRowExt` trait on `UStructRef<WeaponTableRow>` and
`OwnedStruct<WeaponTableRow>`. A data table's row is found as C++'s
`FindRow<T>`, by the table and row name or by an `FDataTableRowHandle`:

```rust
let Some(row) = self.weapon_type().get_row::<WeaponTableRow>() else { return };
let mesh = row.static_mesh().load_synchronous()?;
self.set_weapon_class(row.weapon_to_spawn());
```

`SoftObjectRef<T>` is UE's `TSoftObjectPtr<T>`: an object by path, loaded on
demand (`load_synchronous`), or `get` if it already is.

### Input

With the `enhanced-input` feature (on in new projects), input works as in the
engine's C++ templates: a Blueprint child assigns the input actions and mapping
contexts, the player controller adds the contexts, and the pawn binds the
actions to its functions once its input component exists, in
`receive_restarted` (C++ does it in `SetupPlayerInputComponent`):

```rust
use bindings::enhanced_input::{ETriggerEvent, FInputActionValue, InputAction};
use bindings::prelude::*; // bind_action, enhanced_input_subsystem, InputActionValueExt

#[uclass(parent = Character)]
pub struct MyCharacter {
    #[uproperty(EditAnywhere)]
    move_action: UObjectRef<InputAction>,
}

#[uclass_impl]
impl MyCharacter {
    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        let me = self.as_ref();
        let _ = bind_action(&me, self.move_action(), ETriggerEvent::Triggered, "Move");
    }

    #[ufunction(BlueprintCallable)]
    fn r#move(&mut self, value: UStructRef<FInputActionValue>) {
        let axis = value.axis2d(); // Value.Get<FVector2D>()
        // ...
    }
}
```

`bind_action` names the function by its UE name (`fn r#move` is `Move`), which
takes nothing or an `FInputActionValue`; engine functions work too (`"Jump"`).
`enhanced_input_subsystem(controller)` is where a player controller adds its
mapping contexts.

Touch controls follow the same template: `runtime::input::should_display_touch_interface()`
is the C++ `SVirtualJoystick::ShouldDisplayTouchInterface()` (a Slate check,
not in reflection), and `create_widget_of_class(&controller, class)` spawns a
widget whose class a Blueprint child assigns, as `CreateWidget<T>(this, Class)`
does.

### Delegates

An engine delegate (`OnActorBeginOverlap`, `OnDestroyed`, a component's
`OnComponentBeginOverlap`) takes a Rust closure, or a `#[ufunction]` by name
as C++'s `AddDynamic` does: once, however many times it is bound.

```rust
// A closure; dropping the binding unbinds it, detach() keeps it for as long
// as the actor lives.
actor.checked()?.on_destroyed().add(move |destroyed| { /* ... */ })?.detach();

// A #[ufunction] of this class taking the delegate's parameters.
me.checked()?.on_actor_begin_overlap().add_ufunction(&me, "BeginOverlap")?;
```

### Dynamic Calls

For Blueprint-defined functions or APIs not covered by generated bindings:

```rust
let mut call = DynamicCall::new(&actor, "SetMobility")?;
call.set("NewMobility", 2u8)?;
call.call()?;
```

### Hot Reload

During development, rebuild your Rust library and reload without restarting the editor:

```bash
rusteal build --from 4
```

Then in the UE console:
```
Rusteal.Reload
```

Function implementations update immediately. Adding/removing `uproperty` or `ufunction` requires an editor restart.

## Platform Support

| Platform | Status |
|----------|--------|
| Linux (x64) | Supported (Unreal Engine 5.8.2) |
| Windows (x64) | Supported |
| macOS | Not yet tested |

## Working on Rusteal

This part is for changing Rusteal itself: its crates, the `rusteal` CLI and the
UE plugins in `ue_plugin/`. To make a game, none of it is needed; see
[Making a game](#making-a-game).

### Prerequisites

Everything in [Making a game › Prerequisites](#prerequisites), plus:

| Dependency | Where from | What for |
|---|---|---|
| Git | the distribution's package (`git`); on Windows, [Git for Windows](https://git-scm.com/download/win) | cloning the repository |
| [clangd](https://clangd.llvm.org) (optional) | the distribution's package or LLVM; Zed downloads it on its own | C++ support in the editor, for `ue_plugin/` |
| [.NET 10 SDK](https://dotnet.microsoft.com/download) (optional) | Microsoft, or the distribution's package (`dotnet-sdk-10.0`) | C# support in the editor, for the exporter in `ue_plugin/RustealGenerator/` |

### Development environment

The settings that depend on the machine live in `.env` at the repository root,
which is not versioned; `.env.example` lists them:

| Variable | What it is |
|---|---|
| `RUSTEAL_DEV_ENGINE_ROOT` | The Unreal Engine root, the directory holding `Engine/` |

Linux:

```bash
git clone https://github.com/viniciusmorgado/rusteal.git
cd rusteal
cp .env.example .env    # then fill it in
cargo xtask dev-setup
```

Windows (PowerShell):

```powershell
git clone https://github.com/viniciusmorgado/rusteal.git
cd rusteal
Copy-Item .env.example .env    # then fill it in
cargo xtask dev-setup
```

Rusteal is developed against a UE project: the plugins in `ue_plugin/` are built
inside one, and the C++ editor support reads what that build generates (UHT's
`*.generated.h` headers and UBT's compile commands). `dev-setup` asks which one:

1. **An existing project.** It must have this checkout's plugins installed and
   built (`cargo run -p rusteal -- setup <project>`, then `build`); `dev-setup`
   only reads it, never changes it, and stops with those commands if they are
   missing. Plugin headers that differ from the checkout's are reported.
2. **A new blank project**, created at the path you give with
   `rusteal new --runtime-path` on this checkout, and built. The last component
   of the path is the project name.

To skip the question, pass the answer: `cargo xtask dev-setup --project <path>`
or `cargo xtask dev-setup --new <path>`.

Then it writes:

- **`compile_commands.json`** at the repository root, for clangd: the compile
  commands UBT gives for that project, pointed at `ue_plugin/`;
- **the exporter's `.csproj.props`**, so C# language servers resolve the
  engine's `EpicGames.*` assemblies, and a `dotnet restore` of `rusteal.sln`
  with the .NET SDK bundled with the engine. The solution has two projects, both
  for IDEs only: the exporter (`ue_plugin/RustealGenerator/Source/RustealExporter/`),
  and `ide/RustealRules/`, which gives the plugins' `*.Build.cs` files the context
  UBT compiles them with. Without that project a language server analyzes those
  files with no references at all: nothing resolves, and every `using` is
  reported as unnecessary. UBT compiles the `Build.cs` files without implicit
  usings, so a `using System.IO;` there is required, and the project makes the
  editor agree.

Run it again after adding a source file to the plugin, or after changing a
header that declares a `UCLASS` or `USTRUCT` and rebuilding it in the project:
a generated header only matches the header it was made from.

`.env` is for working on Rusteal only:

| | Working on Rusteal | Making a game |
|---|---|---|
| Engine path from | `.env` | the `rusteal` CLI, which asks once and keeps it in `~/.config/rusteal/config.toml` (the platform's config directory elsewhere) |
| Read by | `cargo xtask` only | the `rusteal` CLI |
| Writes the exporter's `.csproj.props` | `cargo xtask dev-setup`, in this checkout | `rusteal setup`, in the game project |

The `rusteal` CLI never reads `.env`, so it does not change which engine a game
builds with.

### Testing a change

The workspace builds, tests and lints without the engine:

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --no-deps
```

Anything that touches the engine is tested in a UE project of your choice,
driven by this checkout's CLI (`cargo run -p rusteal --`, from the repository
root). A new project that depends on this checkout:

```bash
cargo run -p rusteal -- new Probe --dir /tmp --runtime-path .
```

After changing the runtime, the macros or a game crate:

```bash
cargo run -p rusteal -- build /tmp/Probe --from 4
```

After changing the UE plugins in `ue_plugin/`, reinstall them into the project
and build it:

```bash
cargo run -p rusteal -- setup /tmp/Probe
cargo run -p rusteal -- build /tmp/Probe
```

In the editor, `Rusteal.Reload` swaps the library in without restarting; adding
or removing a `uproperty`/`ufunction` still needs a restart.

A project made with `--runtime-path` follows that checkout: the CLI acts on it
only when built from the same checkout, so drive it with `cargo run -p rusteal --`
from there. After pulling, `setup` reinstalls the plugins at the checkout's
version.

### `--runtime-path`

It decides where the generated project takes Rusteal from:

| | without it | `--runtime-path <checkout>` |
|---|---|---|
| `Rust/Cargo.toml` says | `rusteal-runtime = "=x.y.z"` | `{ path = "<checkout>/rusteal-runtime" }` |
| Crates come from | crates.io | the local checkout |
| A change in the runtime reaches the game | on the next published version | on the next build |
| The project builds on another machine | yes | no (the path is this machine's) |

So: without it for a real game, with it while working on Rusteal — a change in
`rusteal-core` shows up in the project you test with on the next `build --from 4`,
with nothing to publish in between.

### What lives where

| Crate | Role |
|---|---|
| `rusteal-ue-flags` | UE reflection flag constants |
| `rusteal-ffi` | `#[repr(C)]` types and the API table: the Rust ↔ C++ contract |
| `rusteal-core` | the safe runtime: object references, lifecycle, containers, math |
| `rusteal-macros` | `#[uclass]`, `#[uclass_impl]` and their attributes |
| `rusteal-runtime` | what a game depends on; re-exports the above |
| `rusteal-codegen` | reflection JSON → the `bindings` crate and the C++ wrappers; owns `manual/` |
| `rusteal-cli` | the `rusteal` binary; owns `templates/` and embeds `ue_plugin/` |

Changing the C++ plugin means running `cargo run -p rusteal -- sync-plugin`
before publishing, which refreshes the snapshot the binary embeds.

### Adding a template

A template is a directory under `rusteal-cli/templates/`, one directory per
variant inside it, `base` being the template without `--variant`; they are
embedded in the binary when it is built, and nothing else registers them. A
variant directory mirrors the root of the project it creates:

- `template.toml` names the engine template the project starts from
  (`engine_template`), the paths of it to leave out (`exclude`: the C++
  gameplay the template replaces, the other variants), the level to open
  when it is not the engine template's own (`default_map`), a one-line
  `description` and the `next_step` printed at the end;
- every other file is written into the project over the engine template's:
  `*.tera` files are rendered with [Tera](https://keats.github.io/tera/) and
  lose the extension, the rest (Blueprints, meshes) is copied as is;
- `{{ variable }}` works in folder and file names too:
  `Rust/{{crate_name}}/src/lib.rs.tera`.

The context is the same for every template: `project`, `crate_name`,
`version`, `glam_version` and, with `--runtime-path`, `runtime_path`.

Each variant is complete on its own. A new one starts as a copy of the
closest existing one (a variant from its template's `base`) and is changed
from there, never layered on top of it. Its Blueprints come from a project
where they were made and played, saved with the engine version Rusteal
targets. A variant with something to see has a screenshot in
`assets/templates/`, shown in [Templates](#templates).

Engine APIs Rusteal uses that Unreal has deprecated are tracked in
[`docs/ue-deprecations.md`](https://github.com/viniciusmorgado/rusteal/blob/main/docs/ue-deprecations.md): what, since which UE
version, until when and where. A new engine version means checking its
warnings against that list.

What Rusteal still lacks to write Unreal's Third Person template entirely in
Rust, in the order the template needs it, is mapped in
[`docs/third-person-gaps.md`](https://github.com/viniciusmorgado/rusteal/blob/main/docs/third-person-gaps.md).

What Unreal Engine 6 changes for Rusteal — Verse, Scene Graph, the end of
Blueprints — and the open questions to check as Epic publishes details are in
[`docs/ue6-radar.md`](https://github.com/viniciusmorgado/rusteal/blob/main/docs/ue6-radar.md).

Commits are small — one per fix.

### Releases and versions

Everything Rusteal ships carries one version: the seven crates (they inherit
`[workspace.package].version` through `version.workspace = true`), the two UE
plugins and the FFI contract between them.

**Where it comes from.** The release is cut from a `develop` → `main` PR. The
`prepare-release` workflow takes the last released version from crates.io
(`max_version` of the `rusteal` crate, the one source that cannot drift) and
bumps it by the commits the PR adds: `BREAKING CHANGE:` → major, `feat:` →
minor, anything else → patch. It never reads the version from the repository's
files. If nothing that goes into a package changed since the last tag, it skips
the release.

**Where it is written.** The same bump commit writes it everywhere at once:

| File | Field | 0.2.1 becomes |
|---|---|---|
| `Cargo.toml` | `[workspace.package].version` and the internal `{ version, path }` dependencies | `0.2.1` |
| `ue_plugin/*/*.uplugin` | `VersionName` | `"0.2.1"` |
| `ue_plugin/*/*.uplugin` | `Version` | `2001` |

Tags (`v0.2.1`), crates.io and the manifests only ever use `major.minor.patch`.

**The integer form.** Two places cannot hold `0.2.1`: the `.uplugin` `Version`
field, which UE requires to be an integer, and `RustealApiTable::version`, a
`u32`. Both use the same encoding, `major * 1_000_000 + minor * 1_000 + patch`
(`rusteal_ffi::encode_version`). It is always derived from the version, never
a source of it. Minor and patch must stay below 1000, or the number would be
ambiguous; `encode_version` refuses such a version at compile time, so it cannot
slip through.

**How it is kept in line.**

- `rusteal-cli/tests/plugin_version.rs` fails if either `.uplugin` disagrees with
  the workspace version, so a descriptor edited by hand turns CI red.
- At load, the plugin compares its own version with the library's
  `rusteal_version()` export and refuses a library built against another one;
  `rusteal_runtime::init` checks the table's `version` again.
- Before `build`, `generate` and `setup`, the CLI compares the project's pins
  and plugins with its own version (see [Versions](#versions)).

`Cargo.lock` is not versioned: every build would rewrite the internal versions
in it. A dependency release that breaks the workspace shows up in CI, where it
gets fixed. `cargo install rusteal --locked` still works, because cargo packs a
lock file of its own into the published crate.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/viniciusmorgado/rusteal/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](https://github.com/viniciusmorgado/rusteal/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
