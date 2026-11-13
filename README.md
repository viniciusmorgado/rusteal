<div align="center">

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/logo.png" alt="Rusteal logo" width="256">

# Rusteal

[![Crates.io](https://img.shields.io/crates/v/rusteal.svg)](https://crates.io/crates/rusteal)
[![License](https://img.shields.io/crates/l/rusteal.svg)](https://github.com/viniciusmorgado/rusteal#license)
[![dependency status](https://deps.rs/crate/rusteal/latest/status.svg)](https://deps.rs/crate/rusteal)

</div>

---

**Rust bindings for Unreal Engine 5.8**

Rusteal lets you write Unreal Engine games and plugins in Rust: gameplay, reusable gameplay systems, editor tools. Your Rust code compiles to a shared library that is loaded by a small UE C++ plugin. All UE API calls cross the FFI boundary through a function pointer table, no C++ compilation required during Rust iteration.

> **⚠️ Early Stage Project** — Rusteal is under active development and **not ready for production use**. APIs will change without notice, documentation is incomplete, and many UE features are not yet covered. Contributions and feedback are welcome, but please do not use this for shipping projects.

---

## Why use Rusteal

<img src="https://raw.githubusercontent.com/viniciusmorgado/rusteal/HEAD/assets/mascot.png" alt="The Rusteal mascot" width="180" align="right">

When gameplay becomes too complex or demanding for Blueprint graphs, the usual next step is C++. Rusteal makes Rust an alternative for that transition, bringing native performance with compiler checks against common memory and concurrency errors. Because your gameplay builds independently of Unreal’s C++ pipeline and reloads into the running editor, moving to native code also preserves a quick cycle of making changes and trying them in the game. Your Rust classes remain available to Blueprints, so designers can keep building on that logic as it grows. You also gain access to Rust’s ecosystem of crates.

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

### Making a game or a plugin

<a href="https://viniciusmorgado.github.io/game.rusteal.github.io/"><b>The Rusteal game developer's handbook</b></a> is where everything about using Rusteal lives, from installing the CLI to shipping the game.

| Chapter | What you find there |
|---|---|
| [Getting started](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/prerequisites.html) | The prerequisites on Linux, Windows and macOS, installing the CLI, your first project, creating one from the editor's New Project dialog, its layout, upgrades, and adding Rusteal to an existing project. |
| [Templates](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/templates.html) | Every engine template with its gameplay written in Rust (`first-person`, `third-person`, `top-down`, `vehicle`, `vr`, `ar`, `intro`), each variant with its screenshot, controls and classes. |
| [Features](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/features.html) | Gameplay any template can take with `--feature`: copycat mannequins, checkpoints, triggers and doors, the tour pieces, and the RustRover setup. |
| [The rusteal CLI](https://viniciusmorgado.github.io/game.rusteal.github.io/cli/overview.html) | The build pipeline, binding more of the engine, development and release builds, platform optimizations. |
| [The iteration loop](https://viniciusmorgado.github.io/game.rusteal.github.io/iteration/hot_reload.html) | Hot reload, `rusteal watch`, console commands, logging and tests without the engine. |
| [Classes](https://viniciusmorgado.github.io/game.rusteal.github.io/classes/uclass.html) and [the engine API](https://viniciusmorgado.github.io/game.rusteal.github.io/engine_api/object_refs.html) | Declaring classes, properties, components, events, functions, interfaces, structs and enums; calling the engine, spawning, assets, timers, traces, delegates and errors. |
| [Gameplay recipes](https://viniciusmorgado.github.io/game.rusteal.github.io/recipes/index.html) | Movement, cameras, animation, combat, projectiles, pickups, physics, UI, AI, vehicles, VR, AR, saving and more, taken from the templates. |
| [Systems](https://viniciusmorgado.github.io/game.rusteal.github.io/systems/subsystems.html) and [plugins](https://viniciusmorgado.github.io/game.rusteal.github.io/plugins/overview.html) | Subsystems, project settings, console variables, background work, data assets; gameplay plugins, editor tools and packaging them. |
| [Shipping](https://viniciusmorgado.github.io/game.rusteal.github.io/shipping/packaging.html) and [reference](https://viniciusmorgado.github.io/game.rusteal.github.io/reference/cli.html) | Packaging the game; every command, attribute and type, the known limits and troubleshooting. |

### Contributing to Rusteal

<a href="https://viniciusmorgado.github.io/dev.rusteal.github.io/"><b>The Rusteal developer's handbook</b></a> explains how Rusteal works inside and how to change it: its crates, the `rusteal` CLI and the Unreal Engine plugins.

| Chapter | What you find there |
|---|---|
| [How Rusteal works](https://viniciusmorgado.github.io/dev.rusteal.github.io/intro/introduction.html) | From the engine's reflection to the bindings, the boundary between Rust and C++, Rust classes the engine sees, hot reload, games and plugins. |
| [Setting up a checkout](https://viniciusmorgado.github.io/dev.rusteal.github.io/contributing/environment.html) | The prerequisites, `.env` and `cargo xtask dev-setup` for clangd and the C# exporter. |
| [Testing a change](https://viniciusmorgado.github.io/dev.rusteal.github.io/contributing/testing.html) | The workspace checks, a test project on the checkout, and `--runtime-path`. |
| [Templates](https://viniciusmorgado.github.io/dev.rusteal.github.io/templates/porting.html) | Porting an engine template to Rust, the template directories, features. |
| [Releases and versions](https://viniciusmorgado.github.io/dev.rusteal.github.io/contributing/releases.html) | One version for everything, where it comes from, and the pipeline that publishes it. |

## Acknowledgments

Rusteal is a hard fork of [**uika**](https://github.com/VioletHelianthus/uika) by [**VioletHelianthus**](https://github.com/VioletHelianthus), who designed and wrote the foundation this project stands on: the reflection-driven code generator, the UHT exporter plugin, the reification of Rust structs as `UClass`es, and the runtime. Thank you.

Rusteal does not intend to stay compatible with the original uika, unless its author wants to port the improvements back. From here on it develops the Unreal bindings independently.

## Platform support

| Platform | Status |
|----------|--------|
| Linux (x64) | Supported |
| Windows (x64) | Supported |
| macOS (Apple silicon) | Supported |

macOS on Intel processors has not been tested.

Rusteal does not build for Android, iOS or consoles yet; that work is under way ([Known limits](https://viniciusmorgado.github.io/game.rusteal.github.io/reference/limits.html)).

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/viniciusmorgado/rusteal/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](https://github.com/viniciusmorgado/rusteal/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
