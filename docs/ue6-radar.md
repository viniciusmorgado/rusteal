# Unreal Engine 6 radar

What Epic has announced about Unreal Engine 6, what in Rusteal depends on the
parts that are changing, and the open questions to check as details come out.
Nothing here asks for a change today: Rusteal targets UE 5.8, which Epic
describes as the last planned UE5 release.

## Sources

| Date | Source | What it says |
|---|---|---|
| 2026-06-22 | [The road to Unreal Engine 6](https://www.unrealengine.com/news/the-road-to-ue-6) | UE5 and UEFN merge into UE6; gameplay moves to Verse and a new framework, Scene Graph; Actors and Blueprints stay in early UE6 and are deprecated later, with conversion tools; Early Access at the end of 2027, full release 12–18 months after; no UE5 release planned after 5.8 (a 5.9 stays possible). |
| — | [The Verse book](https://verselang.github.io/book/00_overview/) | The language: effects, failure as control flow with rollback, structured concurrency. Says nothing about native interop. |

Epic's words on the programming model: Verse "transactionalizes C++"; all Verse
functions run as atomic transactions that can be rolled back, and that extends
to C++ called from Verse, which is built with a custom LLVM compiler that makes
it transactional. Verse runs on a single thread for now.

## What Rusteal depends on

| Dependency | What Rusteal uses it for | Where | Exposure to UE6 |
|---|---|---|---|
| UObject reflection through UHT | The reflection JSON every binding is generated from | `ue_plugin/RustealGenerator` (UHT exporter), `rusteal-codegen` | Low while UHT exists; the codegen follows whatever reflection exports. |
| `UFUNCTION` / `UPROPERTY` calls | The generated C++ wrappers and the Rust bindings | `Plugins/Rusteal/Source/Rusteal/Generated/`, the `bindings` crate | Low while the classes exist; deprecations are tracked in [`ue-deprecations.md`](ue-deprecations.md). |
| Actors, Components, the gameplay framework | Everything a game writes today | the game's crate | High in the long run: Scene Graph replaces them once it matures. |
| Blueprint internals | Reification: Rust structs become `UClass`es disguised as Blueprint-generated classes (a stub `UBlueprint`, `CLASS_CompiledFromBlueprint`, `bCooked`) | `RustealReifyApiImpl.cpp`, `URustealReifiedClass.cpp` | Highest: the most coupled piece, and Blueprints are to be deprecated. |
| A C++ plugin loading a shared library | The FFI function table between the engine and the Rust library | `RustealModule.cpp`, `rusteal-ffi` | Unknown: depends on how native code runs alongside Verse transactions. |

## Open questions

Each entry is a question to answer from Epic's material or from the public UE6
stream on GitHub. Entries are numbered `UE6-NN` and never renumbered.

### UE6-01 — How is Scene Graph exposed to native code?

| | |
|---|---|
| **Why it matters** | If entities and components are reflected like UObjects today, the codegen can generate bindings for them. If they are reachable only through Verse APIs ("open specifications with Verse APIs"), Rusteal would need a Verse-facing binding layer — a different project. |
| **Where to look** | Scene Graph deep dives; the UE6 stream: whether Scene Graph types carry `UCLASS`/`USTRUCT` reflection and are visible to UHT. |
| **Status** | open |
| **Last checked** | 2026-09-23 |

### UE6-02 — Can native code run inside Verse transactions?

| | |
|---|---|
| **Why it matters** | C++ called from Verse is made transactional by Epic's LLVM compiler. The Rust library is not built by it: if Rust code changes game state inside a transaction, a rollback undoes the engine's side and not Rust's memory. Either there is a way to call non-transactional native code (an escape hatch), or the Rust side has to go through the same transformation — rustc uses LLVM too, but whether that is possible is unknown. |
| **Where to look** | How the UE6 stream builds and links native modules called from Verse; any effect or specifier for non-transactional native calls. |
| **Status** | open |
| **Last checked** | 2026-09-23 |

### UE6-03 — Does UHT stay, and in what form?

| | |
|---|---|
| **Why it matters** | The exporter is a UHT plugin; if reflection moves elsewhere, the exporter moves with it. |
| **Where to look** | `Engine/Source/Programs/Shared/EpicGames.UHT` in the UE6 stream. |
| **Status** | open |
| **Last checked** | 2026-09-23 |

### UE6-04 — When do Actors and Blueprints go, and what replaces runtime class creation?

| | |
|---|---|
| **Why it matters** | Reification creates classes at runtime through Blueprint machinery. While Actors and Blueprints are in UE6 it keeps working; once they are deprecated it needs another way to register Rust-defined types, if Scene Graph has one. |
| **Where to look** | UE6 release notes and deprecations; how Scene Graph components are defined and registered. |
| **Status** | open |
| **Last checked** | 2026-09-23 |

### UE6-05 — Is there a UE 5.9?

| | |
|---|---|
| **Why it matters** | A 5.9 would bring the removals the 5.8 deprecations announce ([`ue-deprecations.md`](ue-deprecations.md)); without it, 5.8 stays the target until UE6. |
| **Where to look** | Epic's release announcements. |
| **Status** | open: "not currently planning", with the option kept |
| **Last checked** | 2026-09-23 |

## Timeline

| When | What |
|---|---|
| 2026 | UE 5.8, the last planned UE5 release; the UE6 stream public on GitHub, not an alpha. |
| End of 2027 | UE6 Early Access. |
| 2029, roughly | UE6 full release (12–18 months after Early Access). |
| Later | Actors and Blueprints deprecated, with conversion tools to Scene Graph. |

## Reviewing this file

When Epic publishes something new about Verse, Scene Graph or native interop,
or at each UE release: add the source, answer or update the open questions, and
bump **Last checked**. An answered question keeps its number, with the answer in
**Status**.
