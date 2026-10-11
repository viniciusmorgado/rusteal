<div align="center">

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/logo.png" alt="Rusteal logo" width="256">

# Rusteal

[![Crates.io](https://img.shields.io/crates/v/rusteal.svg)](https://crates.io/crates/rusteal)
[![License](https://img.shields.io/crates/l/rusteal.svg)](https://github.com/viniciusmorgado/rusteal#license)
[![dependency status](https://deps.rs/crate/rusteal/latest/status.svg)](https://deps.rs/crate/rusteal)

</div>

---

**Rust bindings for Unreal Engine 5.8**

Rusteal lets you write Unreal Engine games and plugins in Rust: gameplay, reusable gameplay systems, editor tools. Your Rust code compiles to a shared library that is loaded by a small UE C++ plugin. All UE API calls cross the FFI boundary through a function pointer table — no C++ compilation required during Rust iteration.

> **⚠️ Early Stage Project** — Rusteal is under active development and **not ready for production use**. APIs will change without notice, documentation is incomplete, and many UE features are not yet covered. Contributions and feedback are welcome, but please do not use this for shipping projects.

---

## Why use Rusteal

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/mascot.png" alt="The Rusteal mascot" width="280" align="right">

**Use every core.**

The gameplay that weighs on a frame (AI for hundreds of
agents, simulations, pathfinding, procedural generation) spreads across every
core with `rayon` or threads of your own, and the compiler guarantees the
threads never race. The results reach Unreal's objects back on the game
thread, through `task::spawn`.

**Iterate in seconds.**

Your gameplay is a library of its own: Cargo rebuilds
your crate alone, without the Unreal Build Tool, and `Rusteal.Reload` swaps
the new code into the running editor.

**Test without the engine.**

Combat rules, inventories, economies and AI
written as plain Rust run under `cargo test`, with property tests and
benchmarks, without opening Unreal.

**Crash less.**

The borrow checker rules out use-after-free and data races in
your code, and a panic is caught at the FFI boundary and logged instead of
taking the editor down.

**Stay deterministic, share the rules.**

Fixed-point math and seeded random
numbers give the same simulation on every machine, the base of rollback and
lockstep netcode. The same rules crate can run in a dedicated Rust server, a
balancing tool or the web, with Unreal as the presentation.

**Bring the Rust ecosystem.**

crates.io is one line away: `serde` for saves
and configs, `rapier` for deterministic physics, `ggrs` for rollback,
`wasmtime` or `rhai` for sandboxed mods, `ort` for local inference.

**Feel at home on both sides.**

`#[uclass]`, `#[uproperty]` and
`#[ufunction]` speak Unreal's language, so anyone who knows the engine knows
where things go. Everything around them is Rust: structs and traits, `Result`
for errors, pattern matching, Cargo, rust-analyzer and clippy.

## Future migration to UE6

We are aware of the deep changes Unreal Engine 6 makes to the engine's
foundations: gameplay moving to Verse, Scene Graph replacing Actors and
Components, and Blueprints being deprecated once the new framework matures. We
follow them in Epic's own UE6 development stream (`ue6-main` in Epic's Unreal
Engine repository), and Rusteal's adaptation happens on the
[`ue6`](https://github.com/viniciusmorgado/rusteal/tree/ue6) branch, which
stays apart from `main` until UE6 ships.

So far, adapting Rusteal to UE6 looks feasible, with limited impact: Verse is
being built on the same UObject reflection and Unreal Header Tool that Rusteal
generates its bindings from. What changes, what Rusteal depends on and the open
questions are tracked in the
[UE6 radar](https://github.com/viniciusmorgado/rusteal/blob/ue6/docs/ue6-radar.md), on the `ue6` branch.

## Getting started

Making a game or a plugin? Read the Rusteal game developer's handbook:

**[The game developer's handbook](https://viniciusmorgado.github.io/game.rusteal.github.io/)** — making games and plugins with Rusteal: installing it, the templates, writing gameplay in Rust, iterating with hot reload, packaging.

Want to contribute to Rusteal itself? Read the Rusteal developer's handbook:

**[The developer's handbook](https://viniciusmorgado.github.io/dev.rusteal.github.io/)** — how Rusteal works inside, and working on it: setting up a checkout, testing a change, templates, releases.

## Acknowledgments

Rusteal is a hard fork of [**uika**](https://github.com/VioletHelianthus/uika) by [**VioletHelianthus**](https://github.com/VioletHelianthus), who designed and wrote the foundation this project stands on: the reflection-driven code generator, the UHT exporter plugin, the reification of Rust structs as `UClass`es, and the runtime. Thank you.

Rusteal does not intend to stay compatible with the original uika, unless its author wants to port the improvements back. From here on it develops the Unreal bindings independently.

## Making a game

Rusteal needs Rust 1.88 or newer and Unreal Engine 5.8, on Linux or Windows ([Prerequisites](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/prerequisites.html)). The CLI carries the engine plugins and the templates:

```bash
cargo install rusteal
rusteal new MyGame --template third-person
```

`rusteal new` creates the project, installs the plugins, writes the Rust workspace and builds it all; open `MyGame.uproject` and press Play ([Your first project](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/first_project.html), [Installing the CLI](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/install.html), [Adding Rusteal to an existing project](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/existing_project.html)).

### Templates

Each of the engine's templates, with its gameplay written in Rust. `--variant` picks one of a template's genre starting points ([Starting from a template](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/templates.html)).

#### `blank`

The engine's Blank template and a `HelloActor` in Rust, the default. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/blank.html).

#### `first-person`

The First Person template: its character, camera manager, game mode and player controller in Rust, playable with keyboard, mouse, gamepad or touch. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/first-person.html).

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/first-person.png" alt="The first-person template in play" width="800">

#### `first-person` `horror`

A dark level explored with a flashlight, and a sprint that runs on stamina, its meter on screen. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/first-person.html#horror).

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/first-person-horror.png" alt="The first-person template's horror variant in play" width="800">

#### `first-person` `shooter`

An arena shooter: weapon pickups from a data table, projectiles and grenades, team scores, and NPCs run by a StateTree that sense, snipe and shoot. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/first-person.html#shooter).

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/first-person-shooter.png" alt="The first-person template's shooter variant in play" width="800">

#### `third-person`

The Third Person template: its character, game mode and player controller in Rust, playable with keyboard, mouse, gamepad or touch. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/third-person.html).

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/third-person.png" alt="The third-person template in play" width="800">

#### `third-person` `combat`

Melee combat: combo and charged attacks, enemies run by a StateTree, spawners, checkpoints, damageable props. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/third-person.html#combat).

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/third-person-combat.png" alt="The third-person template's combat variant in play" width="800">

#### `third-person` `platforming`

Double jump, wall jump, coyote time, a dash, local multiplayer and respawn. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/third-person.html#platforming).

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/third-person-platforming.png" alt="The third-person template's platforming variant in play" width="800">

#### `third-person` `side-scrolling`

A side view platformer: soft platforms, jump pads, moving platforms, pickups with a counter and an NPC run by a StateTree. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/third-person.html#side-scrolling).

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/third-person-sidescrolling.png" alt="The third-person template's side-scrolling variant in play" width="800">

#### `top-down`

The Top Down template: its character, game mode and point and click player controller in Rust, playable with mouse or touch. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/top-down.html).

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/topdown.png" alt="The top-down template in play" width="800">

#### `top-down` `strategy`

A real-time strategy start: units selected with clicks, boxes and touches, sent along the navigation mesh by EnvQueries. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/top-down.html#strategy).

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/topdown-strategy.png" alt="The top-down template's strategy variant in play" width="800">

#### `top-down` `twin-stick`

A twin stick shooter: projectiles, dashes and area attacks against waves of enemies run by a StateTree, pickups and a combo score. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/top-down.html#twin-stick).

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/templates/topdown-twinstick.png" alt="The top-down template's twin-stick variant in play" width="800">

#### `vehicle`

The Vehicle template: a sports car and an offroad car on Chaos Vehicles, with their game mode, player controller and HUD in Rust; variants `offroad` and `time-trial`. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/vehicle.html).

#### `intro`

The Intro To Unreal template: its guided tour level, made of the four gameplay features below, and its first person character in Rust. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/intro.html).

#### `ar`

The Handheld AR template: touch gestures to place an object on a detected plane, then move, rotate and scale it, and its styled menus in Rust. It runs in the editor. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/ar.html).

#### `vr`

The Virtual Reality template on OpenXR: teleport, snap turn, hand poses, grabbing, a pistol, a 3D menu, passthrough and a spectator camera in Rust. [Read more](https://viniciusmorgado.github.io/game.rusteal.github.io/templates/vr.html).

### Features

Gameplay any template can take, added with `--feature` as a Rusteal plugin, or as project files for a tool ([Adding features](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/features.html)):

```bash
rusteal new MyGame --template third-person --feature mannequin --feature rustrover
```

- **[`mannequin`](https://viniciusmorgado.github.io/game.rusteal.github.io/features/mannequin.html)**: mannequins that copy a character's movement, and one whose hand reaches for a target.
- **[`checkpoints`](https://viniciusmorgado.github.io/game.rusteal.github.io/features/checkpoints.html)**: spawn points that save the player's progress and bring them back to the last one.
- **[`interactables`](https://viniciusmorgado.github.io/game.rusteal.github.io/features/interactables.html)**: a trigger and what it drives (lights, a door it unlocks, confetti), a level sequence trigger, a seeded MetaSound and dangling physics blocks.
- **[`tour`](https://viniciusmorgado.github.io/game.rusteal.github.io/features/tour.html)**: the Intro To Unreal tour pieces: changing texts, station titles, animated keys, gizmo demos, spline paths and editor hints.
- **[`rustrover`](https://viniciusmorgado.github.io/game.rusteal.github.io/features/rustrover.html)**: the **Rusteal Watch** and **Rusteal Test** RustRover run configurations, as project files.

### Everything else

| Topic | In the handbook |
|---|---|
| The project's files, its `rusteal.toml` and its version | [Project layout](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/layout.html), [Versions and upgrades](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/versions.html), [Binding more of the engine](https://viniciusmorgado.github.io/game.rusteal.github.io/cli/rusteal_toml.html) |
| The build pipeline and its profiles | [Building](https://viniciusmorgado.github.io/game.rusteal.github.io/cli/build.html), [Development and release builds](https://viniciusmorgado.github.io/game.rusteal.github.io/cli/profiles.html) |
| Classes, properties, components, events, functions, interfaces | [Declaring a class](https://viniciusmorgado.github.io/game.rusteal.github.io/classes/uclass.html) and the *Classes* chapter |
| Structs, data tables and enums | [Structs and data tables](https://viniciusmorgado.github.io/game.rusteal.github.io/classes/structs.html), [Enums](https://viniciusmorgado.github.io/game.rusteal.github.io/classes/enums.html) |
| Object references, calling the engine, delegates, dynamic calls | [Object references](https://viniciusmorgado.github.io/game.rusteal.github.io/engine_api/object_refs.html) and the *Engine API* chapter |
| Errors across Rust and Unreal | [Errors](https://viniciusmorgado.github.io/game.rusteal.github.io/engine_api/errors.html) |
| Widgets, input, cameras, AI, vehicles, VR, AR... | [Gameplay recipes](https://viniciusmorgado.github.io/game.rusteal.github.io/recipes/index.html) |
| Hot reload, `rusteal watch`, console commands, tests | [Hot reload](https://viniciusmorgado.github.io/game.rusteal.github.io/iteration/hot_reload.html), [rusteal watch](https://viniciusmorgado.github.io/game.rusteal.github.io/iteration/watch.html), [Testing without the engine](https://viniciusmorgado.github.io/game.rusteal.github.io/iteration/testing.html) |
| Subsystems, project settings, console variables, background work, data assets | [Subsystems](https://viniciusmorgado.github.io/game.rusteal.github.io/systems/subsystems.html) and the *Systems* chapter |
| Packaging a game, per-platform engine settings | [Packaging a game](https://viniciusmorgado.github.io/game.rusteal.github.io/shipping/packaging.html), [Platform optimizations](https://viniciusmorgado.github.io/game.rusteal.github.io/cli/optimizations.html) |
| Every command and attribute | [CLI reference](https://viniciusmorgado.github.io/game.rusteal.github.io/reference/cli.html), [Attributes](https://viniciusmorgado.github.io/game.rusteal.github.io/reference/attributes.html) |

## Making a plugin

A plugin is a Rust library of its own inside a project's `Plugins/`: a gameplay system, or an editor tool (an editor subsystem, menu entries, asset validators). `rusteal plugin new <Name> --template <blank|editor|inventory|feature>` creates one, and `rusteal plugin package` ships it to other projects ([Plugins](https://viniciusmorgado.github.io/game.rusteal.github.io/plugins/overview.html), [A new plugin](https://viniciusmorgado.github.io/game.rusteal.github.io/plugins/new_plugin.html), [Editor tools](https://viniciusmorgado.github.io/game.rusteal.github.io/plugins/editor_tools.html), [Packaging a plugin](https://viniciusmorgado.github.io/game.rusteal.github.io/plugins/packaging.html)).

## Platform support

| Platform | Status |
|----------|--------|
| Linux (x64) | Supported |
| Windows (x64) | Supported |
| macOS | Not yet tested |

Rusteal does not build for Android, iOS or consoles yet; that work is under way ([Known limits](https://viniciusmorgado.github.io/game.rusteal.github.io/reference/limits.html)).

## Working on Rusteal

Changing Rusteal itself, its crates, the CLI or the UE plugins, takes a clone of this repository: [Setting up a checkout](https://viniciusmorgado.github.io/dev.rusteal.github.io/contributing/environment.html), [Testing a change](https://viniciusmorgado.github.io/dev.rusteal.github.io/contributing/testing.html), [How Rusteal works](https://viniciusmorgado.github.io/dev.rusteal.github.io/intro/introduction.html), [Template directories](https://viniciusmorgado.github.io/dev.rusteal.github.io/templates/directories.html) and [Releases and versions](https://viniciusmorgado.github.io/dev.rusteal.github.io/contributing/releases.html) are in the developer's handbook.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/viniciusmorgado/rusteal/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](https://github.com/viniciusmorgado/rusteal/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
