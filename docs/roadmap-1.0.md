# Road to 1.0

What Rusteal lacks to cover Unreal Engine as a game programmer uses it, and the
milestones that take it from 0.9 to a stable 1.0. Written in October 2026 against
Rusteal 0.9.0 and UE 5.8.3, from the code, the two handbooks, the other tracking
documents in `docs/`, and a survey of the other language layers for Unreal
(UnrealSharp, Angelscript, UnLua, PuerTS, unreal-rust).

## What "full coverage" means

No language layer for Unreal covers 100% of the engine, C++ included: parts of
the engine are templates, non-UObject types and render-thread code that only C++
reaches. Angelscript, which has shipped AAA games on consoles, needs an engine
fork and still leaves native extension points to C++; UnrealSharp says it is
"not production ready yet" and has no Linux, mobile or console support. None of
them publishes a definition of 1.0. Rusteal defines its own, in four tiers:

| Tier | What it is | For 1.0 |
|---|---|---|
| **1. Use parity** | Everything Blueprint reaches through reflection can be called, read and written from Rust: functions, properties of every kind, dynamic delegates, latent and async nodes. | Complete |
| **2. Declaration parity** | What C++ declares with `UCLASS`/`USTRUCT`/`UENUM`/`UINTERFACE`, Rust declares too: every property type, every common `UPROPERTY`/`UFUNCTION` specifier and metadata, replication, RPCs, native events, static functions, config. | Complete |
| **3. Native extension points** | C++ virtuals with no Blueprint event, template APIs and non-UObject types, reached through generated or hand-written shims (the pattern the subsystem bases already use). | The ones shipped games use: actor/component/widget/anim virtuals, GAS, async loading, native delegates, gameplay tags, Game Features. |
| **4. Engine internals** | Rendering (scene view extensions, global shaders, RDG, scene proxies), custom AnimNode/RigUnit/Mass/Niagara/MetaSound nodes on worker threads, Iris serializers, the replication graph. | After 1.0. A documented C++ escape hatch in the same project covers them. |

**1.0 is:** tiers 1 and 2 complete; tier 3 for the systems listed above; tier 4
reachable from a project's C++ module, documented and tested; every gap left
written down with its workaround; and a stability promise (below) that a game in
production can rely on.

### Why now, and on UE 5.8

UE 5.8 is the last planned UE5 release, and UE6 Early Access is announced for
the end of 2027 ([UE6 radar](https://github.com/viniciusmorgado/rusteal/blob/ue6/docs/ue6-radar.md)).
Games started on 5.8 in 2026 and 2027 will ship on 5.8. That makes **1.x the UE
5.8 long-term line**: 1.0 on 5.8, maintained while games ship on it; UE6 support
is a later major version, prepared on the `ue6` branch. Reaching 1.0 before UE6
Early Access gives those games a stable base.

### Where Rusteal stands

Ahead of the other layers on: Linux, macOS and Windows editors; static linking
for consoles and iOS (.NET cannot, which is UnrealSharp's blocker); the console
groundwork; Rust's own tooling (rust-analyzer, Cargo, tests without the engine).
It is the only reflection-driven Rust binding with active work. Behind them on:
multiplayer, async/latent code, native extension points and editor UI.

## The gaps

The full map, by area. **Status:** done, partial, missing. The milestone column
refers to the next section.

### Types and declarations (tiers 1 and 2)

| Gap | Status | Milestone |
|---|---|---|
| Engine `TScriptInterface` functions and properties: the dump named the native `I` class, so all were dropped, and `get_object` refused an `FInterfaceProperty` | done | M1 |
| Engine strings cut at 512 bytes, converted to ANSI, and a cut could split a UTF-8 character | done | M1 |
| Engine structs holding a `TArray` (or other owning members) read by copy into `OwnedStruct::from_bytes`, never destroyed: each read leaks | bug | M1 |
| Engine `TSoftObjectPtr` and `TWeakObjectPtr` properties typed as strong `UObjectRef`: an unloaded soft reference read as null, its path lost; now `SoftObjectRef<T>` and `TWeakObjectPtr<T>` | done | M1 |
| A feature whose modules come from an engine plugin got empty bindings: UHT did not run again after the plugin was added | done | M1 |
| `TSoftClassPtr`, `TLazyObjectPtr`, `TOptional`, `TFieldPath` engine properties dropped | missing | M1 |
| `FText` passed through `ToString`, losing its localization data; now `Text`, the engine's `FText` | done | M1 |
| `Text::localized` in Rust code is not read by the engine's text gatherer: its texts are translated only when the localization data has their namespace and key (String Tables work) | missing | M8 |
| Engine structs Rust read were shallow copies: arrays freed under Rust (function returns) or leaked (properties, containers) | done | M1 |
| Engine `TSoftClassPtr` properties and functions dropped; now `SoftClassRef<T>` | done | M1 |
| `FInstancedStruct` opaque, `TSubScriptStructOf` (new in 5.8) not mapped | partial | M1 |
| Engine functions taking a delegate dropped | missing | M1 |
| `#[uproperty]`: `TMap`, `TSet`, `FText`, `i8`/`i16`/`u16`/`u32`/`u64`, weak, soft class, interface and delegate properties, arrays of strings and soft references | missing | M1 |
| `#[uproperty]` metadata (`ClampMin`, `EditCondition`, `Units`, `ToolTip`...), any `meta` | missing | M1 |
| `#[ufunction]` parameters: `String`, `FName`, `FText`, arrays, maps, sets, soft references | missing | M1 |
| `#[ufunction]` returning a struct, several outputs (out parameters) outside `Override` | missing | M1 |
| `BlueprintNativeEvent` declared in Rust, `CallInEditor`, `Category`, function metadata | missing | M1 |
| Static functions: Blueprint function libraries in Rust | missing | M1 |
| `#[udelegate]`: single-cast and sparse delegates, strings and arrays in signatures | partial | M1 |
| Override events whose parent signature has strings, arrays or soft references (`CopyParamsFromParent` drops them) | partial | M1 |
| Unknown `#[uclass]`/`#[component]` arguments ignored instead of refused | sharp edge | M1 |

### Multiplayer

| Gap | Status | Milestone |
|---|---|---|
| `Replicated`, `ReplicatedUsing`, lifetime conditions on Rust properties | missing | M7 |
| `Server`/`Client`/`NetMulticast` RPCs, `Reliable`, `WithValidation`, `BlueprintAuthorityOnly` | missing | M7 |
| The exporter drops the engine's own `Server` functions | missing | M7 |
| Dedicated server builds (`rusteal package --server`), client-only stripping | missing | M7 |
| Push model, replicated subobjects | missing | M7 |
| Iris (production-ready in 5.8 for licensees): registration, compatibility of Rust classes | missing | M7 |
| `FFastArraySerializer`, custom `NetSerialize` | missing | after 1.0 (C++ escape hatch) |

### Async, the game loop and events

| Gap | Status | Milestone |
|---|---|---|
| Rust `async` on the game thread: an executor driven by the engine's tick, `async fn` as a Blueprint latent node | missing | M2 |
| Calling engine latent functions (`Delay`, `LoadAsset`...) from Rust | untested | M2 |
| Async Blueprint actions (`UBlueprintAsyncActionBase`) written in Rust | missing | M2 |
| Async asset loading: `FStreamableManager`, Asset Manager primary assets, soft class loading | missing (sync only) | M2 |
| Timers with closures (`FTimerManager`), not by function name | partial | M2 |
| Native (non-dynamic) engine delegates: `FCoreDelegates`, `FWorldDelegates`, `OnRequestFinished` | missing | M2 |
| Input actions and delegates bound to a method or a closure, checked at compile time, not by name | partial | M2 |
| `FTickFunction`: tick groups and prerequisites | missing | M3 |

### Native extension points (tier 3)

| Gap | Status | Milestone |
|---|---|---|
| A general shim generator: C++ bases that re-expose chosen virtuals as events, generated from a list, as the subsystem bases are written by hand today | missing | M3 |
| Actor and component virtuals without a Blueprint event: `PostInitializeComponents`, `TickComponent`, `PostLoad`, `IsNetRelevantFor`... | missing | M3 |
| UMG `NativeConstruct`/`NativeTick`/`NativeOn*`, `NativeUpdateAnimation`, `NativeThreadSafeUpdateAnimation` | missing | M3 |
| C++-only interfaces (`CannotImplementInterfaceInBlueprint`) | missing | M3 |
| Character movement custom modes, `FSavedMove_Character` prediction | missing | M3 (modes), after 1.0 (saved moves) |
| Commandlets | missing | M3 |

### Gameplay frameworks

| Gap | Status | Milestone |
|---|---|---|
| Gameplay Ability System: attribute sets (`FGameplayAttributeData`, `PreAttributeChange`, `PostGameplayEffectExecute`), ability tasks, effect contexts, execution calculations | bindings only | M5 |
| Gameplay tags declared in Rust (the `UE_DEFINE_GAMEPLAY_TAG` registration), `IGameplayTagAssetInterface`, helpers | bindings only | M5 |
| CommonUI (activatable widgets, stacks, input routing) | not listed | M5 |
| UMG ViewModel (MVVM): `FieldNotify` on Rust properties | missing | M5 |
| Game Features and modular gameplay: `UGameFeatureAction`, component manager extension handlers | not listed | M5 |
| Smart Objects, World Partition (`WorldPartitionBlueprintLibrary` blocklisted), Data Layers, level instances | not listed | M5 |
| StateTree native tasks (structs with virtuals), Behavior Tree native nodes | Blueprint bases only | M5 (via Blueprint bases), after 1.0 (native) |
| PCG Blueprint elements, MetaSound Builder, Quartz, Niagara Data Channels | not listed / untested | M5 |
| Mass entity processors, custom Niagara data interfaces, MetaSound nodes, AnimNode, Control Rig units | missing | after 1.0 |
| Online: EOS (its SDK is a C API, which Rust calls directly), the Online Subsystem through its Blueprint proxies | missing | M5 (EOS sample), after 1.0 (Online Services v2) |

### Plugins and the ecosystem

| Gap | Status | Milestone |
|---|---|---|
| Editor-only plugins for C++/Blueprint projects without Rusteal (`rusteal plugin package --standalone`) | done; Fab on hold | M0 |
| Rusteal as a precompiled engine plugin on Fab, with a Rust option in the project browser | missing | M0 |
| Precompiled plugin packages (`BuildPlugin`), several platforms in one package | done | M0 |
| Engine-wide installs of Rusteal plugins | missing | M0 |
| A plugin with both a runtime and an editor library | done | M0 |
| The game using a plugin's Rust types; a Rust parent class in another library | missing | M6 |
| A library built against another Rusteal patch version | missing (exact match) | M9 |

### Editor extensibility

| Gap | Status | Milestone |
|---|---|---|
| Slate from Rust (windows, tabs, panels): a builder API over hand-written bindings | missing | M4 |
| Details customizations | missing | M4 |
| Editor Utility Widgets with a Rust parent, scripted asset and actor actions | missing | M4 |
| New asset types: factories, asset definitions, thumbnails | missing | M4 |
| Interchange pipelines and translators (parsers from crates.io are a Rust strength) | untested | M4 |
| Editor modes, Scriptable Tools | untested | M4 |
| Custom K2 nodes, asset and graph editors | missing | after 1.0 |
| Editor Python sees UE names for Rust properties | sharp edge | M4 |

### Platforms

| Gap | Status | Milestone |
|---|---|---|
| Windows: `--static` validated, hot reload tested, a CI job | partial | M8 |
| macOS: `--static` validated; Intel untested | partial | M8 |
| Linux ARM64: run on hardware, not only cross-linked | partial | M8 |
| Linux dedicated server | missing | M7 |
| Android | missing | M8 |
| iOS (static only, no hot reload) | missing | M8 |
| Consoles, phase 2: bring-up with each SDK in a private platform pack | blocked on access | parallel track |

### Production tooling

| Gap | Status | Milestone |
|---|---|---|
| An automated headless engine test harness in the repository (it exists only as a documented method and outside fixtures) | missing | M1 (first), grows every milestone |
| Rust tests registered as UE Automation tests, functional tests, Gauntlet | missing | M8 |
| Debugging: launch and attach recipes (CodeLLDB, Visual Studio, Xcode), mixed C++/Rust stacks, symbols in packaged builds | missing | M8 |
| Unreal Insights trace scopes, stats and LLM tags from Rust | missing | M8 |
| Localization: gatherable text from Rust (a `loctext!` the gather step reads, or string tables) | missing | M8 |
| Measured call overhead against Blueprint and C++, zero-copy array views | missing | M8 |

### Stability

| Gap | Status | Milestone |
|---|---|---|
| The API table has no fingerprint of its own; function pointer order is untested | sharp edge | M9 |
| Libraries must match the plugin's version exactly | by design today | M9 |
| No API stability or deprecation policy (the README says APIs change without notice) | missing | M9 |
| UE deprecations do not reach the bindings as `#[deprecated]` | missing | M9 |
| Hot reload of a changed `#[ustruct]`/`#[uenum]`, Rust-private fields across a reload, `-game` | missing | M9 |
| `Pinned` not reference counted; a `UObjectRef` in a private field invisible to the GC | sharp edge | M9 |
| The game thread is a convention, not asserted | sharp edge | M9 |
| UE deprecations UE-DEP-001 and UE-DEP-002 open | open | M9 |

## Milestones

Each milestone is one or more releases. The release pipeline computes the
version from the commits (`feat` → minor), so the versions below are an
estimate, not a promise. Sizes: **S** weeks, **M** one to two months, **L**
two to four months, for one developer.

Every milestone ends the same way: its features have tests in the headless
harness (M1 onward), both handbooks describe them, `docs/` tracking entries are
closed, and at least one real project (the Fab plugin or Noir Blue) uses them.

### M0 — Plugins for every project (≈0.10, L)

**Status (2026-10-18):** in progress. The launcher is done on Linux: the Rust
category in the New Project dialog with every template and variant, the setup
on first open with its checks and retries, `rusteal launcher install`, both
handbooks, and the submission notes in [fab-submission.md](fab-submission.md).
A runtime plugin can also have an editor library (`rusteal plugin new
--editor`, `rusteal plugin editor`), and a plugin can be packaged precompiled
(`rusteal plugin package --precompiled`, `rusteal plugin merge`), and an editor
plugin can be packaged for any project, Blueprint and C++ ones included, with a
renamed copy of the runtime inside it (`rusteal plugin package --standalone`),
checked in a Blueprint, a C++ and a Rusteal project with two such plugins at once
and a packaged game that carries none of it. Left: Windows and macOS, and the Fab
submission (on hold).

The next step already planned: two kinds of plugins.

- **Editor-only plugins** that a C++ or Blueprint project without Rusteal can
  install: precompiled, nothing Rust shipped with the game, a narrower API.
  The first commercial use is the maintainer's own Fab plugin.
- **Shippable Rusteal plugins**, as today, packaged with `BuildPlugin` for every
  configuration and several platforms at once.
- **Rusteal on Fab** as a precompiled engine plugin, with a Rust option next to
  Blueprint and C++ in the project browser, the CLI underneath; it lives
  alongside `cargo install rusteal`.
- A runtime and an editor library in one plugin.

**Done when:** the Fab plugin installs into a plain C++ project and a Blueprint
project from the marketplace layout (`Engine/Plugins/Marketplace`), works in
the editor, and a packaged game of that project contains no Rust; both
handbooks document both kinds, each for its own reader.

### M1 — The type system, closed (≈0.11, L)

Tiers 1 and 2 for types, plus the test harness everything after it needs.

- **Harness:** a headless engine test runner in the repository, run by
  `cargo xtask`, with the property and function round trips as its first
  tests. (A CI job for it needs its own decision: UE is not on GitHub's
  runners.)
- **Bugs:** `TScriptInterface`, strings without the 512-byte cut, soft and weak
  properties with their own types.
- **Engine types:** `TSoftClassPtr`, `TLazyObjectPtr`, `TOptional`,
  `TFieldPath`, `FInstancedStruct`, `TSubScriptStructOf`, `FText` with its
  localization data, functions taking delegates.
- **Rust declarations:** every property type above as `#[uproperty]`, `TMap`
  and `TSet` included; any type as a `#[ufunction]` parameter or return,
  structs and out parameters included; `meta` and `Category`;
  `BlueprintNativeEvent`, `CallInEditor`; static functions (function
  libraries); single-cast and sparse `#[udelegate]`s; unknown arguments refused.

**Done when:** every `FProperty` kind round-trips, in both directions, in the
harness; the default modules' bindings contain no `// Skipped:` for a type
reason that is not listed as accepted; the Classes and Types pages of the game
developer's handbook have no "not yet" row left for types.

**Accepted, not bound by type** (measured with every feature on, UE 5.8.3, after
the engine types): one function is `// Skipped:`
(`AbilitySystemBlueprintLibrary.BindEventWrapperToAnyOfGameplayTagsChanged`, a
delegate next to a container), and the filter leaves out
`KismetSystemLibrary.SetFieldPathPropertyByName` (`TFieldPath`),
`KismetTextLibrary.EditTextSourceString` (an `FText` changed in place), about
sixteen functions whose delegate returns a value, writes an argument or takes an
array (`SoundSubmix.AddSpectralAnalysisDelegate`), and the `FUtf8String` and
`VCell` properties of Verse's classes. `TSubScriptStructOf` is in no engine
property or function. Everything else left out has a cause other than its types:
a module the project does not bind, wildcard functions (`KismetArrayLibrary`,
`BlueprintMapLibrary`), functions the engine does not export, network RPCs (M7).

### M2 — Async and events (≈0.12, M)

Where Rust can be better than C++ and Blueprint, not only equal.

- A game-thread executor driven by the engine's tick; `async fn` exported as a
  Blueprint latent node; cancellation when the object goes away.
- Engine latent functions awaitable from Rust; async Blueprint actions in Rust.
- Async loading: soft objects and classes, `FStreamableManager`, primary
  assets.
- Timers with closures; native engine delegates; input actions and delegates
  bound to methods and closures, checked at compile time.

**Done when:** a sequence of loads, delays and an RPC reads as one `async fn`
in a template; nothing in the templates binds by a function's name.

### M3 — Native extension points (≈0.13, L)

- A shim generator: a list of virtuals in `rusteal.toml` (or a curated
  default) becomes C++ bases that call Rust, as the subsystem bases do.
- The curated set: actor and component lifecycle virtuals, `TickComponent` and
  tick functions, UMG `Native*`, the animation instance's native and
  thread-safe updates (with the threading rules they need), character
  movement custom modes, commandlets.
- C++-only interfaces.

**Done when:** each virtual in the curated set is overridden from Rust in the
harness, and the animation update runs off the game thread safely.

### M4 — Editor extensibility (≈0.14, L)

- Slate from Rust: a builder API over hand-written bindings for the common
  widgets, attributes and delegates.
- Details customizations, Editor Utility Widgets with a Rust parent, scripted
  asset and actor actions, factories and new asset types, Interchange
  pipelines, editor modes and Scriptable Tools.
- Editor Python names for Rust properties.

**Done when:** the maintainer's Fab plugin (M0) needs no C++ of its own for
its UI.

### M5 — Gameplay frameworks (≈0.15, L)

Each a feature in `rusteal.toml`, with a page in the gamedev handbook and a
sample.

- GAS: attribute sets, abilities, ability tasks, effect contexts, executions
  (UnrealSharp needed a separate shim plugin for this; Rusteal's comes from M3).
- Gameplay tags declared in Rust; CommonUI; MVVM `FieldNotify`; Game Features
  and modular gameplay; Smart Objects; World Partition and Data Layers;
  StateTree and Behavior Tree through their Blueprint bases; PCG Blueprint
  elements; MetaSound Builder and Quartz.
- An EOS sample over its C SDK.

**Done when:** a **Lyra slice** runs with its gameplay in Rust: a hero with
GAS abilities and attributes, an experience loaded as a Game Feature, CommonUI
menus, in single player (M7 takes it over the network). Lyra exercises almost
every framework above; it is the objective gate for this milestone.

### M6 — Libraries that see each other (≈0.16, M)

- Generated Rust bindings for another library's Rust classes, from their
  reflection, so the game uses a plugin's types directly.
- A Rust parent class in another library.

**Done when:** the game crate `use`s a plugin's class and derives from it, in
dynamic and static builds, across a hot reload of either library.

### M7 — Multiplayer (≈0.17, L)

What Blueprint can do over the network, from Rust. Angelscript and UnrealSharp
define their networking at this level; so does Rusteal 1.0.

- `Replicated`, `ReplicatedUsing` (with the previous value), every lifetime
  condition; the reified classes report their replicated properties as
  Blueprint-generated classes do.
- RPCs: `Server`, `Client`, `NetMulticast`, `Reliable`/unreliable,
  `WithValidation`; the exporter keeps the engine's `Server` functions.
- Push model, replicated subobjects; Iris compatibility checked.
- Dedicated server: `rusteal package --server`, Linux server target, code that
  is stripped from clients.
- A multiplayer template (or variant) with a listen and a dedicated server.

**Done when:** two clients and a dedicated server, run headless, agree on
replicated state and RPCs in the harness; the template and the Lyra slice
(M5) play over a network.

### M8 — Platforms and production tooling (≈0.18, L)

- Windows CI job (needs its own permission), `--static` and hot reload
  validated on Windows and macOS, Linux ARM64 run on hardware.
- Android and iOS (static only).
- Debugging recipes and IDE launch configurations, packaged symbols; Insights
  scopes, stats, LLM tags; UE Automation and functional tests from Rust;
  gatherable text; published performance numbers.

**Done when:** a template packages and runs on every listed platform; a Rust
breakpoint is hit in the editor on each desktop; a trace shows Rust scopes.

### M9 — Stabilization, 1.0.0-rc (≈0.19 → 1.0.0-rc.N, M)

- **API freeze:** the public API of `rusteal-runtime`, the macros' attributes,
  `rusteal.toml` and the CLI's commands reviewed and frozen; what will change is
  removed or marked unstable before 1.0.
- **Compatibility within 1.x:** the API table gets its own fingerprint and
  order tests; a library built against 1.x.y loads into any 1.x.z with the same
  table (patch tolerance at least); `rusteal upgrade` migrates every 0.x
  project.
- **Policy:** semver for the crates, a deprecation window of one minor version,
  UE deprecations carried into the bindings as `#[deprecated]`, a Rust MSRV
  policy, the supported engine versions (5.8 for 1.x).
- **Sharp edges:** struct and enum hot reload, opt-in persistence of private
  fields, reference-counted `Pinned`, GC-visible private references, a debug
  assertion of the game thread, UE-DEP-001/002 closed.
- **Docs:** the escape hatch to C++ for tier 4, tested; every remaining limit
  in Known limits with its workaround.

**Done when:** Noir Blue and the Lyra slice build on the release candidate
with no Rusteal-side workaround; one release candidate goes a month without a
breaking change; the Known limits page lists only tier 4 and accepted items.

### 1.0.0

Cut from the last release candidate. The major bump is a decision of the
maintainer: the pipeline reaches 1.0.0 only from a `BREAKING CHANGE:` commit.

## Parallel tracks

- **Consoles, phase 2:** steps 5 to 8 of [Consoles](https://viniciusmorgado.github.io/dev.rusteal.github.io/platforms/consoles.html),
  whenever a project gets SDK access. Not a 1.0 gate: access is outside the
  project's control. The structure is ready since 0.9.
- **UE6:** the `ue6` branch keeps following the stream; UE6 support is 2.0.
- **Games:** what a game needs and Rusteal lacks is filed as it is found, each
  entry numbered (`GAME-GAP-NN`, in the format of
  [third-person-gaps.md](third-person-gaps.md)), and goes into the next
  milestone it fits, or ahead of it when it blocks the game. Edge cases are
  expected; this is how they are handled.
- **Showcase in the README:** two new sections, each added to the README only
  when its first entry is released, linked to its repository or store page:
  - **Rusteal Extensions**, plugins that extend Rusteal itself. The first one
    planned is an ECS gameplay plugin for Rusteal, for gameplay written in a
    data-oriented style in Unreal Engine, developed in parallel.
  - **Commercial Games and Plugins built with Rusteal**. The first one expected
    is the maintainer's Fab editor plugin (M0).

## Order and why

The order follows the projects that drive Rusteal now: the maintainer's Fab
editor plugin, its first commercial use, and Noir Blue, a single-player
shooter. Neither needs multiplayer, so multiplayer comes after what they use,
but before the API freeze, because it changes how classes are declared.

M0 first: it is already the next step, and it opens Fab. M1 before everything
else: every later milestone needs the harness and the full type system (async
nodes, editor tools, GAS and replication all pass types the macros refuse
today). M2 and M3 next: a single-player game hits async loading, closures
instead of function names and native virtuals early. M4 then gives the Fab
plugin its UI in Rust. M2 and M3 unlock M5: GAS needs native virtuals and
async tasks. M6 matters most to the plugin ecosystem. M7 brings the network
for the games that come after. M8 can start any time on its own and partly
run alongside. M9 last.

If a milestone has to move for a game, the dependencies that matter are: M1
before every other; M3 before M5's GAS; M0 before M4's goal; M5 before M7's
Lyra check. Milestones from M2 to M7 can otherwise trade places when a game
needs one sooner.

## Sources

- The repository: `rusteal-codegen/src/{type_map,filter}.rs`,
  `rusteal-macros/src/{uclass,uclass_impl,prop_type,udelegate}.rs`,
  `ue_plugin/RustealGenerator/Source/RustealExporter/RustealExport.cs`,
  `ue_plugin/Rusteal/Source/Rusteal/Private/*ApiImpl.cpp`; the other documents
  in `docs/`.
- The handbooks: Known limits, Known issues and sharp edges, Consoles,
  Decisions.
- Epic, UE 5.8: [release notes](https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-5-8-release-notes),
  [Iris](https://dev.epicgames.com/documentation/en-us/unreal-engine/iris-replication-system-in-unreal-engine),
  [RPCs](https://dev.epicgames.com/documentation/en-us/unreal-engine/remote-procedure-calls-in-unreal-engine),
  [Gameplay Ability System](https://dev.epicgames.com/documentation/en-us/unreal-engine/gameplay-ability-system-for-unreal-engine),
  [UMG ViewModel](https://dev.epicgames.com/documentation/en-us/unreal-engine/umg-viewmodel-for-unreal-engine),
  [asset management](https://dev.epicgames.com/documentation/en-us/unreal-engine/asset-management-in-unreal-engine),
  [feature maturity levels](https://www.unrealengine.com/en-US/blog/experimental-early-access-and-beta-features-in-unreal-engine).
- Other layers: [Angelscript development status](https://angelscript.hazelight.se/project/development-status/)
  and [networking](https://angelscript.hazelight.se/scripting/networking-features/),
  [UnrealSharp](https://github.com/UnrealSharp/UnrealSharp) and
  [UnrealSharpGAS](https://github.com/UnrealSharp/UnrealSharpGAS),
  [UnLua](https://github.com/Tencent/UnLua), [PuerTS](https://github.com/Tencent/puerts),
  [unreal-rust](https://github.com/MaikKlein/unreal-rust).
