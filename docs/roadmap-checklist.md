# Road to 1.0: checklist

Every milestone of [the road to 1.0](roadmap-1.0.md) split into its tasks, with
what is done. Status as of 2026-10-22, on Linux with UE 5.8.3; the counts in
parentheses are the headless harness checks (`cargo xtask engine-test`) passing
at that point. Items marked *manual* need a run by hand on a platform or in a
project the harness does not cover.

## M0 — Plugins for every project + Fab (≈0.10)

### Launcher: Rusteal on Fab

- [x] CP1: RUST category and stub conversion when a project opens
- [x] CP2: Rust and CLI checks, Cancel, Retry, stub restore
- [x] CP3: the 17 templates and variants
- [x] CP3: `rusteal launcher install` / `remove`
- [x] CP3: documentation in both handbooks
- [x] CP3: Fab submission notes
- [x] End-to-end test of one variant
- [ ] Validate on Windows (manual, not blocking)
- [ ] Validate on macOS (manual, not blocking)
- [ ] Questions to Fab support (on hold: nothing goes to Fab for now)
- [ ] Fab submission (on hold)

### Editor-only plugins for projects without Rusteal

- [x] S1: C++ runtime renamable by prefix, the copy compiles in BuildPlugin
- [x] S2: `rusteal plugin package --standalone` (no Rust sources)
- [x] S3: works in pure C++ and pure Blueprint projects
- [x] S3: installed in `Engine/Plugins/Marketplace`
- [x] S3: coexists with a Rusteal project's Rusteal and with another standalone plugin
- [x] S3: packaged game with no Rust at all
- [x] S4: documented in both handbooks, for each audience
- [ ] Fab's answer on precompiled Rust libraries (on hold)

### Distributing Rusteal plugins

- [x] Precompiled packages with BuildPlugin
- [x] Several platforms in one package
- [x] Runtime and editor library in the same plugin

### Documentation

- [x] Both plugin kinds in both handbooks, each for its audience

## M1 — The type system, closed (≈0.11)

### T0: harness

- [x] `cargo xtask engine-test`: test project, windowless Unreal, PASS/FAIL
- [x] Base tests: classes, properties, functions, delegates, components, tasks
- [x] `cargo xtask templates-check` (every template against bindings with every feature)

### T1: bugs

- [x] UTF-8 from Rust read past its end (found by the harness)
- [x] Engine `TScriptInterface` (functions and properties; the I-prefixed name hid them all)
- [x] Strings without the 512-byte cut, and in UTF-8 (not ANSI)
- [x] `rusteal build` recovers from old wrappers that no longer compile (found by the harness)
- [x] Engine soft and weak references with their own types (`SoftObjectRef`, `TWeakObjectPtr`; note in `rusteal upgrade`)
- [x] Templates compile against the new bindings (17 game, 7 plugin)
- [x] Reflection dump redone when a feature brings a new plugin (found in validation)

### T2: engine types in the codegen

- [x] `TSoftClassPtr` as `SoftClassRef`
- [x] `TOptional` as `Option<T>`
- [x] `TLazyObjectPtr` as `TWeakObjectPtr`; `TFieldPath`, `FUtf8String` and `VCell` on the accepted list
- [x] `FInstancedStruct` (make/get/set/reset); `TSubScriptStructOf` unused in the engine
- [x] `FText` as `Text` (localization kept; gathering Rust text moved to M8)
- [x] Engine functions that take delegates (Rust closure)
- [x] No skipped functions by type (1 left, on the roadmap's accepted list)
- [x] Engine structs really copied and destroyed (leak fixed, memory freed); by reference as `&mut OwnedStruct`
- [x] Delegate listeners survive GC; the closure is freed with its proxy (found by the harness)
- [x] Every engine struct is an `OwnedStruct` (190 raw pointers removed)
- [x] Build with every feature: C++ links and Rust compiles without warnings

### T3: properties in Rust

- [x] Every integer type, `Text`, `TMap` (`UeMap`), `TSet` (`UeSet`)
- [x] Weak, soft class, interface (`ScriptInterface`), arrays of String, Text, soft and weak
- [x] `meta(...)` with any key, Category, doc comment as ToolTip, String/FName/Text defaults
- [x] Containers without stack overflow or the 4 KB cut (found in T3)
- [x] `#[ustruct]` with the same types (deep copy)

### T4: functions in Rust

- [x] Any type as parameter and return value, struct returns, out params
- [x] BlueprintNativeEvent, CallInEditor, category, `meta(...)`, doc comment as ToolTip
- [x] Static functions (Blueprint Function Libraries)
- [x] `#[udelegate]` single-cast and sparse, strings and arrays
- [x] Inherited events with any type; unknown arguments rejected
- [x] Overrides lost array, soft, map and set parameters (found in T4)
- [x] Templates compile with the new macros (`templates-check`)

### T5: documentation

- [x] Handbooks: types, events, functions, delegates, versions; developer handbook
- [x] Limits and roadmap (M1 done on Linux)
- [x] Migration notes in `rusteal upgrade` (6 in 0.10)
- [x] Harness: every property type in both directions (47 checks)
- [ ] Windows and macOS (manual)

## M2 — Async and events (≈0.12)

### T0: game-thread executor

- [x] One executor per library, driven by the tick
- [x] `spawn_local`, `JoinHandle`, `abort`, `yield_now`, `next_frame`
- [x] `run_blocking` on top of `task::spawn`
- [x] Cancellation when the owner dies and on unload

### T1: time and timers

- [x] `timer::set_timer` with a closure, `TimerHandle` (clear, pause, remaining)
- [x] `timer::sleep` (world time, stops while paused) and `sleep_real`
- [x] `k2_set_timer_delegate` leak fixed (hand-written version)
- [x] `templates-check`

### T2: engine latent functions as async

- [x] UE 5.8 deprecation fixes brought in (0 warnings in the plugin)
- [x] Latent proxy in C++, `LatentCall` in Rust, cancelled when dropped
- [x] Codegen: a latent function becomes a Future, outputs on the heap until it ends
- [x] Delay, LoadAsset, MoveComponentTo and cancellation in the harness (52/52)
- [x] `templates-check` and a full TemplDump build

### T3: async loading

- [x] `SoftObjectRef::load_async` and `SoftClassRef::load_async` (class checked)
- [x] `loading::load_all` and `loading::load_primary_asset`
- [x] Cancelled when the future is dropped, failure without hanging (53/53)

### T4: events as Futures, latent async fn

- [x] `.next()` on every multicast delegate (engine and `#[udelegate]`)
- [x] Blueprint async actions with await (save game in the harness)
- [x] `#[ufunction] async fn` as a latent node; abort and a destroyed owner do not fire it
- [x] `task::AbortHandle` (cancels from any thread)
- [x] `templates-check`

### T5: native delegates and HTTP

- [x] `core_events`, `world_events`, `game_mode_events`, `on_request_finished` (add and next)
- [x] `http::get/post/request(..).send().await`, local server in the harness (57/57)
- [x] Harness again and `templates-check`

### T6: input and delegates by closure and method

- [x] `bind_action` with a closure (`ActionBinding`), `bind_action_by_name`
- [x] `method(self, Self::x)` for input, delegates and timers (`RustObject`)
- [x] Note in `rusteal upgrade` (8 in 0.10)
- [x] Harness 58/58 and `templates-check`

### T7: templates without names

- [x] 213 by-name bindings migrated (no `#[ufunction]` signature changed)
- [x] intro: `async fn` with `load_async` + `sleep` + `http` (configurable URL)
- [x] `templates-check` 24/24
- [x] Full TemplDump build

### T8: documentation

- [x] Gamedev handbook: Async page, timers, delegates, input, recipes, reference
- [x] Developer handbook: decisions D24–D30 and the runtime
- [x] Repository roadmap (gap rows)
- [x] `bind_action_instance` (value, times and action) in the harness

## M3 — Native extension points (≈0.13)

### T0: shim generator

- [x] TOML format, C++ signature parser, emitter
- [x] Curated versioned list (`RustealShims.h/.cpp`) and a golden test
- [x] The project's `[[codegen.shims]]`, generated before the export
- [x] Harness check (60/60)

### T1: actor and component virtuals, tick functions

- [x] Actor, Pawn, Character, PlayerController, GameModeBase
- [x] ActorComponent and SceneComponent (TickComponent, InitializeComponent)
- [x] `tick::add_tick_function`, groups and prerequisites
- [x] Harness 61/61, `templates-check` 24/24, TemplDump build

### T2: UMG and AnimInstance

- [x] `URustealUserWidget` Native* (OnInitialized, PreConstruct, Construct, Destruct, Tick)
- [x] `URustealAnimInstance`, thread-safe update on a worker (checked by the harness)
- [x] `#[ufunction(ThreadSafe)]` with the `<Class>ThreadSafe` view; the others refuse off the game thread
- [x] The C++ side's current library per thread (`thread_local`)

### T3: custom movement and commandlets

- [x] PhysCustom, GetMaxSpeed, GetMaxAcceleration, OnMovementModeChanged
- [x] `subobject_class(CharMoveComp = ...)` in `#[uclass]`
- [x] `URustealCommandlet` run by the xtask (exit code)

### T4: C++-only interfaces

- [x] Interfaces in the generator; `RustealTaggedActor` with `IGameplayTagAssetInterface`
- [x] `UStructRef::assign`
- [x] Harness 66/66
- [x] TemplDump build and `templates-check`

### T5: documentation

- [x] Gamedev handbook: Engine virtuals page, versions, limits, attributes
- [x] Developer handbook: decisions D31–D37, reification, runtime, contribution task
- [x] Repository roadmap: M3 status and gap rows

### Windows and macOS

- [ ] Validate (manual)

## M4 — Editor extensibility (≈0.14)

### T0: editor library in the harness

- [x] `RustealProbeEditor` and checks in a headless editor

### T1: Blutility

- [x] Rust EditorUtilityObject run by the EditorUtilitySubsystem
- [x] Editor Python: properties by UE name or snake_case, functions through `call_method` (check)

### T2: editor virtual bases

- [x] `URustealFactory` (hand-written C++): create and import
- [x] Asset created and file imported by a Rust factory, from Python (74/74)

### T3: Slate from Rust

- [x] Widget builder, attributes and events by closure
- [x] Tabs (nomad tab) opened in the headless editor (76/76)

### T4: Details customizations

- [x] `details::customize_class` and `details::view` (77/77)

### T5: documentation

- [x] Handbooks: "Editor UI", "Assets and editor utilities", decisions D42–D45
- [x] Repository roadmap and plugin roadmap
- [x] TemplDump, `templates-check` 24/24 and `static-test` 5/5

### Manual

- [ ] A Fab plugin with no UI C++ (done-when criterion); look and feel; an EUW in a tab; Windows and macOS

## M5 — Gameplay frameworks (≈0.15)

### T0: gameplay tags declared in Rust

- [x] `declare_gameplay_tag`, `gameplay_tag`; hierarchy matching

### T1: bases per feature and mapped arguments

- [x] `event_params`/`event_args`, defaults and static functions in the generator
- [x] Curated bases per feature, added to the project's own

### T2: GAS

- [x] `RustealAttributeSet` (Pre/PostAttributeChange, Pre/PostGameplayEffectExecute, AttributeOf)
- [x] Rust GameplayEffect applied, health clamped
- [x] Rust GameplayAbility granted and activated
- [x] Fix: out parameters through ProcessEvent (crash with `FGameplayEffectSpec`)

### T3: documentation

- [x] Handbooks: Gameplay tags, GAS, limits, versions; decisions D46–D49
- [x] Roadmap and validation (80/80, templates 24/24, static 5/5)

### Manual / later

- [ ] Lyra slice (done-when criterion); CommonUI, MVVM, Game Features, Smart Objects, PCG, MetaSound, EOS

## M6 — Libraries that see each other (≈0.16)

### Decisions

- [x] D1-B queue between libraries, D2-A bindings read the plugin's code, D3-B chained reload when the shape changes

### T0: Rust plugin in the harness

- [x] `RustealProbe` created by the xtask, two libraries in the build
- [x] Base check: the plugin's class registered and called dynamically

### T1: bindings for a plugin's classes

- [x] Reading the plugin's code (`syn`), module path and `type_id`
- [x] `bindings::plugins::<plugin>`: properties, functions, delegates
- [x] Harness check (68/68)

### T2: parent in another library

- [x] Queue of deferred classes and retry across libraries
- [x] Parent event override, Rust data from both libraries
- [x] Check: a game class derives from the plugin (69/69)

### T3: chained reload

- [x] Plugin and game reload in one batch when the parent's shape changes
- [x] Fix: the hot reload hard link returned the old instance (Linux)
- [x] Test in a headless editor: plugin reload, game class reparented (70/70)

### T4: static build

- [x] Static package with a game deriving from the plugin (`static-test` 5/5)
- [x] Fixes: packaged game include, `rusteal-*` crates in the static build, `target/` outside the workspace

### T5: documentation

- [x] Handbooks: "A plugin's classes in the game" page, decisions D38–D41, tests
- [x] Repository roadmap
- [x] TemplDump build and `templates-check` 24/24

### Manual

- [ ] Editor with real projects; reload in the editor; Windows and macOS

## M7 — Multiplayer (≈0.17)

### T0: network harness

- [x] Dedicated server + 2 PIE clients, one windowless editor (81/81)

### T1: replicated properties

- [x] Replicated, ReplicatedUsing (previous value), conditions (82/82)

### T2: RPCs

- [x] Server, Client, NetMulticast, Reliable, WithValidation (6 network checks)

### T3: engine RPCs

- [x] Engine Server RPCs in the bindings, linkable RPCs only; u32 fix in the codegen (87/87)

### T4: dedicated server

- [x] `rusteal package --server` (dedicated server; validating it needs a source-built engine)
- [x] Push model (dirty marks) and real Iris: 3 network variants, 101/101
- [x] Replicated subobjects: Rust component and `RustealReplicatedObject`
- [ ] `--server` with a source-built engine (manual)

### T5: multiplayer template

- [ ] Needs content authored in the editor and a playtest (manual)

### T6: documentation

- [x] Handbooks: Multiplayer, attributes, CLI, limits, versions; decisions D50–D55; roadmap
- [x] Validation: 101/101, templates 24/24, static 5/5

### Manual / later

- [ ] Lyra over the network; Windows and macOS

## M8 — Platforms and tooling (≈0.18)

### T0: Insights

- [x] `trace_scope!`, `stat_scope!`, `memory_tag`; scope seen in headless Insights (105/105)

### T1: UE Automation

- [x] `automation::register_test`; headless Automation RunTests, report read back

### T2: gatherable text

- [x] `loctext!` gathered by GatherText (manifest with the Rust entry)

### T3: debugging

- [x] vscode feature (tasks, CodeLLDB/Visual Studio attach); symbols through `CARGO_PROFILE_RELEASE_DEBUG`
- [ ] A breakpoint on each desktop platform (manual)

### T4: performance

- [x] Costs measured and published (binding 2.3 ns, reflection 928 ns); no zero-copy views (D60)

### T5: documentation

- [x] Handbooks: Profiling, Automation, Debugging, Performance, text; decisions D56–D60; roadmap
- [x] Validation: 108/108, templates 24/24, static 5/5

### Manual / later

- [ ] Windows CI, Windows and macOS, Android, iOS, ARM64 on hardware

## M9 — Stabilization, 1.0.0-rc (≈0.19)

- [ ] API freeze
- [ ] Compatibility within 1.x: fingerprint and table order
- [ ] Semver, deprecation and MSRV policy
- [ ] Hot reload of structs and enums; Pinned and GC
- [ ] Escape hatch to C++ documented and tested
- [ ] Noir Blue and the Lyra slice without workarounds
- [ ] One month of RC without a breaking change

## 1.0.0

- [ ] The 1.0.0 cut (a `BREAKING CHANGE` commit)

## Parallel tracks

- [ ] Consoles, phase 2 (depends on SDK access)
- [ ] UE6 on the `ue6` branch
- [ ] README: Rusteal Extensions (ECS plugin)
- [ ] README: commercial games and plugins
