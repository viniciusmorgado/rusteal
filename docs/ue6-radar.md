# Unreal Engine 6 radar

What Epic has announced about Unreal Engine 6, what its public development
stream shows, what in Rusteal depends on the parts that are changing, and the
open questions to check as details come out. Nothing here asks for a change
today: Rusteal targets UE 5.8, which Epic describes as the last planned UE5
release. UE6 work happens on the `ue6` branch, which takes `develop` merges and
does not go back into `develop` or `main` before UE6 ships.

**Stream checked up to:** `cd299b9d4365` (`ue6-main`, 2026-10-02), against
`release` 5.8.3 (`396c9f059903`); merge-base `f5c25e2ba290` (2026-05-01).

## Sources

| Date | Source | What it says |
|---|---|---|
| 2026-06-22 | [The road to Unreal Engine 6](https://www.unrealengine.com/news/the-road-to-ue-6) | UE5 and UEFN merge into UE6; gameplay moves to Verse and a new framework, Scene Graph; Actors and Blueprints stay in early UE6 and are deprecated later, with conversion tools; Early Access at the end of 2027, full release 12–18 months after; no UE5 release planned after 5.8 (a 5.9 stays possible); the UE6 stream is public on GitHub, "not intended to be any kind of Alpha", and the Verse implementation moved there, "not intended for general adoption". |
| 2026-06-17 | [State of Unreal 2026](https://www.unrealengine.com/news/state-of-unreal-2026-top-news-from-the-show) | Animation, itemization and gameplay abilities "landing as Verse-scriptable components"; the MCP plugin is Experimental in 5.8; 5.8 is "the last planned major release" of UE5. |
| 2026-08-20 | [Unreal Fest Seoul recap](https://www.invenglobal.com/articles/24950/next-gen-engine-unreal-engine-6-will-bring-a-shift-in-the-development-ecosystem) (secondary: press) | Scene Graph presented as the entity-component model that replaces Actors/Components; C++ for the "low-level engine core"; a 5.9 reported as planned (AI features, mobile). No Epic post confirms 5.9. |
| 2024-03-15 | [Bringing Verse transactional memory semantics to C++](https://www.unrealengine.com/tech-blog/bringing-verse-transactional-memory-semantics-to-c) | AutoRTFM: a clang/LLVM pass clones every function and instruments the "closed" clone; calling code without a clone inside a transaction aborts it; `Open`, `OnAbort`, `OnCommit` are the escape hatches. |
| 2026-09 | [verselang/book, DEVELOPMENT.md](https://github.com/verselang/book/blob/main/DEVELOPMENT.md) | Natives are declared in `*.native.verse` with `<native>` and implemented in C++ behind a VNI-generated binding; `<native_callable>` is a Verse body C++ can call; everything Verse ships as a plugin. |
| — | [Scene Graph in UEFN](https://dev.epicgames.com/documentation/fortnite/getting-started-in-scene-graph-in-fortnite) | Entities contain components and other entities; prefabs are Verse classes deriving from `entity`; "Actors and entities are not interchangeable". |
| — | [The Verse book](https://verselang.github.io/book/00_overview/) | The language: effects, failure as control flow with rollback, structured concurrency. |
| 2026-10-02 | The `ue6-main` stream (`EpicGames/UnrealEngine`, mirrored to the maintainer's fork) | See [What the UE6 stream shows](#what-the-ue6-stream-shows). |

Epic's words on the programming model: Verse "transactionalizes C++"; all Verse
functions run as atomic transactions that can be rolled back, and that extends
to C++ called from Verse, which is built with a custom LLVM compiler that makes
it transactional. Verse runs on a single thread for now.

## What the UE6 stream shows

From `ue6-main` at `cd299b9d4365`, compared with `release` (5.8.3). Facts carry
a path or a commit; inferences say so.

**Volume and direction.** 22,268 commits since the merge-base. About a fifth
share a subject with `release`, almost all from the 5.8 stabilization in April
and May; since June the stream has diverged. The busiest areas are the engine
runtime (`Runtime/Engine`, `Core`, `CoreUObject`, `Renderer`), then Scene Graph
(`Plugins/EntityFramework`, 550 commits), PCG, the new animation framework
(UAF), the editor AI toolsets (`Plugins/Experimental/Toolsets`) and the Verse
stack (`Runtime/VerseCompiler`, `CoreUObject/*/VerseVM`, `Plugins/Solaris`).
Verse commits rise every month (40 in April, 194 in September).

**UObject reflection is what Verse is built on, not something it replaces.**

- Verse classes are UClasses: `class UVerseClass : public UClass`
  (`CoreUObject/Public/VerseVM/VVMVerseClass.h:155`). Verse runs on the
  Blueprint VM by default (`bUseVerseBPVM = true`, `TargetRules.cs:1001`); a
  separate VM (`VerseCLR`) exists for the Verse tools and tests.
- Native Verse types are moving from VNI-generated code to UHT: "Porting
  VerseNative to use UHT instead of VNI" (`00a9317`, 2026-06-01); "Add the
  ability to disable VNI in a project. When disabled, all native verse types
  will assume UHT implementation" (`fea9c3a`, 2026-05-26: `SetupVerse(...,
  VerseVNI)` per module, VNI enabled by default); "Adding the ability for UHT
  to create VerseFunctionProperty" (`c8d7ca5`); "Port VerseJSON to UHT"
  (`c0717e7`).
- No replacement reflection system. The only new reflection-like code is Ness
  (`Runtime/Ness`, "New Serialization Stack Prototype"), which is
  serialization only and off by default.

**UHT stays, and its type model changed.**

- `Engine/Source/Programs/Shared/EpicGames.UHT`: 136 files changed, mostly a
  code generation refactor. The exporter plugin interface (`Utils/UhtExport.cs`,
  `IUhtExportFactory`) is unchanged.
- The types an exporter reads changed: `UhtStruct.Properties`/`Functions` are
  `IReadOnlyList`, `UhtFunction.ParameterProperties` is a
  `ReadOnlyMemory<UhtProperty>`, `FindType(...)` became `FindSourceType`,
  `SuperIdentifier`/`BaseIdentifiers` are `UhtTokenList`, the
  `UhtEngineType` helpers are C# 14 extension properties.
- New: the class specifier `NeedsCookRequiresInstanceData` (now on `AActor`
  and `UBlueprintGeneratedClass`), the property specifier `AllowSelfReference`,
  the flags `CPF_GroupedRepNotify` and `CPF_VerseOnly`, the property type
  `FVerseFunctionProperty` next to the Verse value, cell, class and string
  properties.
- CoreUObject removed the `FField` object flag APIs (`f9260d5`) and changed
  `ICppStructOps::PostLoad`.

**Scene Graph is an entity/component hierarchy of UObjects, not a data-oriented
ECS.**

- It is `Engine/Plugins/EntityFramework` (category Verse, Beta, off by
  default; modules Entity, Component, ActorEntity, EntityPrefabEditor,
  EntityStreaming, EntitySimulation, ...).
- `UObject → UBaseVerseSubobject → UBaseEntity → verse::entity` and
  `UBaseVerseSubobject → UBaseComponent → verse::component`, with 82 component
  classes (`light_component`, `physics_component`, `tag_component`, ...). A
  class deriving from `entity` is a prefab. Entities nest (`GetParent`,
  `AddEntities`). Lifecycle: OnInitialized, OnAddedToScene, OnBeginSimulation,
  OnEndSimulation, OnRemovingFromScene, OnUninitializing.
- The `U*Base*` classes are ordinary UHT UCLASSes; the `verse::*` classes and
  their API are declared in `*.native.verse` files and their glue
  (`ENTITY_DEF()`, `COMPONENT_DEF()`) is generated by VNI, which UHT does not
  see today.
- Mass sits underneath as an implementation detail (an entity gets a Mass
  representation on demand). Actors connect through a bridge
  (`ActorEntity`, `UActorEntityComponent : UActorComponent`), not a
  conversion.

**Actors and Blueprints: no deprecation on the stream yet, but it is the plan.**
Epic says they "will eventually be deprecated once the new framework is
sufficiently mature", with conversion tools; removal would follow the usual
deprecation cycle (inference), likely years after Early Access. The absence of
markers only means it has not started. No new `UE_DEPRECATED` on
`AActor`, `APawn`, `ACharacter`, `UActorComponent`, `UUserWidget` or
`UBlueprint`; the 300 `UE_DEPRECATED(6.x)` markers are elsewhere (rendering,
Core). New systems are built so they don't depend on Actor (Mover, editor
selection, AI perception). The Blueprint compiler was restructured:
`KismetCompiler` folded into `BlueprintGraph` (`e03cbcb`). No conversion
tooling found yet.

**Gameplay APIs Rusteal binds: few changes.** Enhanced Input
(`IsKeyIgnored`/`AddIgnoredKey`, 5.9; a misspelled user-settings function,
6.0), StateTree (`Compile` → `K2_CompileStateTreeWithResult`), AI Perception
listener APIs (`UE_DEPRECATED_FORGAME(6.0)`), EQS (`FDebugHelper`). They follow
the [`ue-deprecations.md`](ue-deprecations.md) process when the target moves.

**AutoRTFM is off for games by default.** `bUseAutoRTFMCompiler` defaults to
false (`TargetRules.cs:1012`) and is on only for program targets (Verse VM,
tests); it builds with `Engine/Source/Programs/EpicClang`, on Windows and
Linux. Core types carry AutoRTFM attributes (`AUTORTFM_OPEN_NO_SANITIZE
UObject()`, `AUTORTFM_OVERRIDE_MAY_DISABLE` on `PostLoad` and
`IModuleInterface`). Code the compiler did not instrument is handled with
`.aem` external mapping files (`AutoRTFM/Documentation/ExternalMappings.md`)
or `autortfm_register_open_to_closed_functions` (`AutoRTFM/Public/AutoRTFM/CAPI.h:404`); closed code
calling a function without a closed variant reports "Could not find function"
(`Private/FunctionMap.cpp:302`).

**MCP and editor toolsets are reflection-driven.** The ModelContextProtocol
plugin (already in 5.8) grew a lot; tools are static `UFUNCTION`s with
`meta=(AICallable)` on `UToolsetDefinition` subclasses, registered with
`UToolsetRegistry::RegisterToolsetClass(UClass*)` and discovered by iterating
UFunctions. About 1,000 tools, against about 300 in 5.8, including StateTree,
EQS, UMG, Input and Scene Graph toolsets.

**Module loading is unchanged** for a plugin that loads a shared library. New:
`IModuleInterface::PostStartupModuleUObjects()` (a hook once UObjects are
ready) and the plugin descriptor's `bMergeModules`.

## What Rusteal depends on

| Dependency | What Rusteal uses it for | Where | Exposure to UE6 |
|---|---|---|---|
| UObject reflection through UHT | The reflection JSON every binding is generated from | `ue_plugin/RustealGenerator` (UHT exporter), `rusteal-codegen` | Low for the approach: Verse is built on it and native Verse types move to UHT. Medium for the code: the exporter's type model changed, and Verse property types need marshalling. |
| `UFUNCTION` / `UPROPERTY` calls | The generated C++ wrappers and the Rust bindings | `Plugins/Rusteal/Source/Rusteal/Generated/`, the `bindings` crate | Low while the classes exist; few deprecations so far on the gameplay classes the templates use. |
| Actors, Components, the gameplay framework | Everything a game writes today | the game's crate, the templates | Low in early UE6, high later: Scene Graph replaces them once it matures. |
| Blueprint internals | Reification: Rust structs become `UClass`es disguised as Blueprint-generated classes (a stub `UBlueprint`, `CLASS_CompiledFromBlueprint`, `bCooked`) | `RustealReifyApiImpl.cpp`, `URustealReifiedClass.cpp` | Highest: the most coupled piece; the Blueprint compiler is being restructured and Blueprints are to be deprecated. |
| A C++ plugin loading a shared library | The FFI function table between the engine and the Rust library | `RustealModule.cpp`, `rusteal-ffi` | Low for loading (unchanged); unknown when Verse transactions call into Rust (UE6-02). |

## Open questions

Each entry is a question to answer from Epic's material or from the public UE6
stream on GitHub. Entries are numbered `UE6-NN` and never renumbered.

### UE6-01 — How is Scene Graph exposed to native code?

| | |
|---|---|
| **Why it matters** | If entities and components are reflected like UObjects today, the codegen can generate bindings for them. If they are reachable only through Verse APIs, Rusteal would need a Verse-facing binding layer. |
| **Answer so far** | Both, for now. Entities and components are UObjects (`UBaseEntity`, `UBaseComponent`, UCLASSes UHT sees), but their API (`verse::entity`, `verse::component` and 82 component classes) is declared in `*.native.verse` and glued by VNI-generated code UHT does not see. Native Verse types are moving to UHT (`00a9317`, `fea9c3a`: VNI can be turned off per module, and then types "assume UHT implementation"). |
| **Next check** | Whether `EntityFramework` switches to UHT (`SetupVerse(..., VerseVNI.Disabled)` in its Build.cs files); until then, whether the runtime reflection of `verse::*` classes (their `UClass`, `UFunction`s and properties after load) is enough to generate bindings. |
| **Status** | partly answered |
| **Last checked** | 2026-10-02 |

### UE6-02 — Can native code run inside Verse transactions?

| | |
|---|---|
| **Why it matters** | C++ called from Verse is made transactional by Epic's compiler. The Rust library is not built by it: inside a transaction, a call into Rust aborts, and a rollback would not undo Rust's own memory. |
| **Answer so far** | Not by default: closed code calling a function without a closed variant reports an error. The ways in are a wrapper built with EpicClang marked `AUTORTFM_OPEN` (Rust runs "open", with `OnAbort`/`OnCommit` compensation), `.aem` external mappings, or `autortfm_register_open_to_closed_functions`. The AutoRTFM compiler is off for game and editor targets by default (Windows and Linux only). |
| **Next check** | Whether game targets turn AutoRTFM on by default in UE6; how the BPVM (`EX_AutoRtfmTransact`) treats a `UFunction` whose native thunk is a Rust function. |
| **Status** | partly answered |
| **Last checked** | 2026-10-02 |

### UE6-03 — Does UHT stay, and in what form?

| | |
|---|---|
| **Why it matters** | The exporter is a UHT plugin; if reflection moves elsewhere, the exporter moves with it. |
| **Answer** | It stays, and becomes the reflection backbone for native Verse types too. The exporter plugin interface is unchanged; the type model it reads changed (see [What the UE6 stream shows](#what-the-ue6-stream-shows)), and new specifiers, flags and Verse property types appear. |
| **Next check** | Build `RustealGenerator` against `ue6-main` (compile only) to list what breaks. |
| **Status** | answered (2026-10-02) |
| **Last checked** | 2026-10-02 |

### UE6-04 — When do Actors and Blueprints go, and what replaces runtime class creation?

| | |
|---|---|
| **Why it matters** | Reification creates classes at runtime through Blueprint machinery. While Actors and Blueprints are in UE6 it keeps working; once they are deprecated it needs another way to register Rust-defined types. |
| **Answer so far** | Epic's plan: deprecated once Scene Graph is mature, with conversion tools, then removed (inference: after a deprecation period of some years). On the stream: no deprecation yet, no conversion tooling. The Blueprint compiler is being restructured (`KismetCompiler` into `BlueprintGraph`), and `UBlueprintGeneratedClass` gained the `NeedsCookRequiresInstanceData` specifier. |
| **Next check** | Deprecations on `AActor`/`UBlueprint`; the conversion tools; whether reification still works once the Blueprint compiler changes (build the plugin against the stream). |
| **Status** | open |
| **Last checked** | 2026-10-02 |

### UE6-05 — Is there a UE 5.9?

| | |
|---|---|
| **Why it matters** | A 5.9 would bring the removals the 5.8 deprecations announce ([`ue-deprecations.md`](ue-deprecations.md)); without it, 5.8 stays the target until UE6. |
| **Answer so far** | Epic: "not currently planning", with the option kept. Press reports from Unreal Fest Seoul (August 2026) describe it as planned; the stream already carries `UE_DEPRECATED(5.9, ...)` markers (Enhanced Input, StateTree, EQS). |
| **Status** | open |
| **Last checked** | 2026-10-02 |

### UE6-06 — Can Rust-defined types be visible to Verse?

| | |
|---|---|
| **Why it matters** | Using Verse and Rust in one project means Verse code reaching Rust classes. |
| **Answer so far** | There is no public API to declare a Verse type at runtime. Verse binds to existing UClasses with `@import_as("/Script/Module.Class")`; Rust classes are UClasses. Unverified: whether `@import_as` accepts a UClass created at runtime, or needs it at compile time (Verse compiles before the Rust library loads). |
| **Next check** | How `@import_as` resolves its class (at Verse compile time or at load); whether generating `.native.verse` files for Rust classes at build time is a route. |
| **Status** | open |
| **Last checked** | 2026-10-02 |

### UE6-07 — Can the codegen see types written in Verse?

| | |
|---|---|
| **Why it matters** | UHT reads C++ headers only. Classes written in Verse (components, prefabs) become `UVerseClass`es when the Verse code loads, so they are not in the reflection JSON the codegen reads today. |
| **Answer so far** | They are UClasses at runtime, with UFunctions and properties (inference from `VVMVerseClass.h` and `VVMUECodeGen.h`: `ConstructUVerseClass/Struct/Enum/Function`). |
| **Next check** | Whether a runtime reflection dump (from a commandlet or the editor) is a workable second source for the codegen, next to the UHT JSON. |
| **Status** | open |
| **Last checked** | 2026-10-02 |

### UE6-08 — How do Verse property types marshal?

| | |
|---|---|
| **Why it matters** | `FVerseValueProperty`, `FVerseFunctionProperty`, Verse cells and strings, and `CPF_VerseOnly` properties appear on types the codegen would bind. |
| **Next check** | Their layout and access rules; whether `CPF_VerseOnly` properties should be skipped. |
| **Status** | open |
| **Last checked** | 2026-10-02 |

## Directions

Not decisions: directions the findings point to, to revisit at each check.

- **Reflection as the common ground with Verse.** Epic is putting Verse on the
  same reflection Rusteal uses (UClass, UFunction, FProperty, UHT). If native
  Verse types move to UHT for good, Rusteal can reach Scene Graph and native
  Verse APIs the way Verse reaches them.
- **Scene Graph fits Rust.** It is composition: entities that contain
  components and other entities, with a fixed lifecycle, rather than deep
  Actor inheritance. Rust components (structs with their own data and
  lifecycle events) map onto `verse::component` more directly than onto
  `AActor` subclasses. It is not a data-oriented ECS: Mass stays underneath.
- **Verse and Rust in one project.** Verse for what it is built for
  (transactional gameplay, persistence through global maps, distribution across
  servers), Rust for systems and heavy computation outside transactions, with
  UClasses as the meeting point (UE6-02, UE6-06).
- **A gradual move.** Actors and Blueprints stay in early UE6 and Epic promises
  conversion tools, so the current model should run on UE6 Early Access while
  Scene Graph support grows on the `ue6` branch.
- **Branches.** `ue6` takes `develop` merges and stays apart until UE6 ships;
  then a `ue5.8.2` branch keeps the last UE5 version working, and `develop`/
  `main` move to UE6. An alternative kept open: freeze Rusteal on UE 5.8.x and
  start a separate project for UE6 only.
- **First technical checks, when the `ue6` branch starts changing code:** build
  `RustealGenerator` and the runtime plugin against `ue6-main`; dump the
  reflection of a Scene Graph component at runtime; try `@import_as` on a
  reified class.
- **MCP as a dev-tool route.** Editor tools are static `AICallable` UFunctions
  on registered toolset classes; Rusteal could expose its own (build, codegen,
  reification state) the same way.

## Timeline

| When | What |
|---|---|
| 2026-05 | `ue6-main` diverges from the 5.8 line (merge-base 2026-05-01). |
| 2026-06 | Unreal Fest Chicago (June 16–18); the UE6 announcement (June 22); the Verse stack and Scene Graph made public on the stream (June 10–11). |
| 2026 | UE 5.8, the last planned UE5 release; a 5.9 possible, reported as planned in August (unconfirmed by Epic). |
| End of 2027 | UE6 Early Access. |
| 2029, roughly | UE6 full release (12–18 months after Early Access). |
| Later | Actors and Blueprints deprecated, with conversion tools to Scene Graph; removed after that (inference). Watch for the first `UE_DEPRECATED` on `AActor` or `UBlueprint`: from then on, Rusteal's reification and the templates' Actor-based classes are on a clock. |

## Reviewing this file

At each new merge into `ue6-main`, when Epic publishes something new about
Verse, Scene Graph or native interop, or at each UE release: compare the
stream from **Stream checked up to** to its new tip, add the sources, answer or
update the open questions, add new ones with the next number, and update
**Stream checked up to** and **Last checked**. An answered question keeps its
number, with the answer in **Status**.
