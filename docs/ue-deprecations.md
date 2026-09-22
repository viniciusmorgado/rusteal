# Unreal Engine deprecations

Engine APIs that Rusteal uses and Unreal Engine has deprecated. A deprecated API
still compiles, with a warning, until the engine removes it; from then on the
code that uses it no longer builds. This file tracks each one until it is gone
from Rusteal.

## How to read an entry

Every entry has the same fields:

| Field | Meaning |
|---|---|
| **API** | The deprecated function, constructor, class or feature. |
| **Kind** | `function`, `constructor`, `overload`, `class` or `feature`. |
| **Deprecated in** | The UE version that deprecated it: the first argument of `UE_DEPRECATED(version, ...)` in the engine header. |
| **Removal** | When the engine may drop it. UE's warning says to update "before upgrading to the next release"; removals are not guaranteed to happen then, so this is the earliest version the code can break in. |
| **Engine message** | The text of the deprecation, as the compiler prints it. |
| **Replacement** | What to use instead. |
| **Where** | Files and lines in this repository, as of **Last checked**. |
| **Status** | `open`, or `fixed in <rusteal version>`. |
| **Last checked** | The Rusteal and UE versions the entry was verified against. |

Entries are numbered `UE-DEP-NNN` and never renumbered. A fixed entry keeps its
number and moves to [Fixed](#fixed).

## Adding an entry

1. Build a project with the engine version in question and collect the
   warnings: `rusteal build` prints them as
   `warning: '<API>' is deprecated: <message> [-Wdeprecated-declarations]`.
2. For each API, find the `UE_DEPRECATED(<version>, ...)` line in the engine
   headers under `Engine/Source/` for **Deprecated in**.
3. Copy the template below into the right section, with the next free number.

```markdown
### UE-DEP-NNN — `<API>`

| | |
|---|---|
| **Kind** | |
| **Deprecated in** | UE x.y |
| **Removal** | UE x.y at the earliest |
| **Engine message** | |
| **Replacement** | |
| **Where** | `path/to/file.cpp:line` |
| **Status** | open |
| **Last checked** | Rusteal x.y.z, UE x.y.z |
```

## Hand-written code

Code in `ue_plugin/` that calls a deprecated API. These have to be fixed in
Rusteal before the engine removes them.

### UE-DEP-001 — `F*Property` constructors taking `InObjectFlags`

| | |
|---|---|
| **Kind** | constructor |
| **Deprecated in** | UE 5.8 |
| **Removal** | UE 5.9 at the earliest |
| **Engine message** | `F<Type>Property constructor with InObjectFlags is deprecated, remove that parameter.` |
| **Replacement** | The same constructors without the flags argument: `new FIntProperty(Owner, PropName)` instead of `new FIntProperty(Owner, PropName, RF_Public)`. |
| **Where** | `ue_plugin/Rusteal/Source/Rusteal/Private/RustealReifyApiImpl.cpp`: `FBoolProperty` :43, `FInt8Property` :49, `FInt16Property` :54, `FIntProperty` :59, `FInt64Property` :64, `FByteProperty` :69 and :174, `FUInt16Property` :74, `FUInt32Property` :79, `FUInt64Property` :84, `FFloatProperty` :89, `FDoubleProperty` :94, `FStrProperty` :99, `FNameProperty` :104, `FTextProperty` :109, `FObjectProperty` :114, `FClassProperty` :128, `FStructProperty` :156, `FEnumProperty` :171 |
| **Status** | open |
| **Last checked** | Rusteal 0.2.1, UE 5.8.2 |

The reified classes build their properties with these constructors, so this is
the one that breaks `#[uproperty]` when it goes. In 5.8 the deprecated
constructors already ignore the flags and forward to the ones without them
(`UnrealType.h`), so dropping the argument changes no behaviour.

### UE-DEP-002 — `GetObjectsWithOuter` with a `bool bIncludeNestedObjects`

| | |
|---|---|
| **Kind** | overload |
| **Deprecated in** | UE 5.8 |
| **Removal** | UE 5.9 at the earliest |
| **Engine message** | `GetObjectsWithOuter with a boolean bIncludeNestedObjects has been deprecated - please use the EGetObjectsFlags enum instead` |
| **Replacement** | The overload taking `EGetObjectsFlags`: `EGetObjectsFlags::None` for `false`, which is what the deprecated one forwards to (`UObjectHash.h`). |
| **Where** | `ue_plugin/Rusteal/Source/Rusteal/Private/RustealDelegateApiImpl.cpp:143` |
| **Status** | open |
| **Last checked** | Rusteal 0.2.1, UE 5.8.2 |

## Generated bindings

Deprecated engine functions and classes that are still reflected, so codegen
wraps them in `Plugins/Rusteal/Source/Rusteal/Generated/` and exposes them in the
`bindings` crate. Nothing in Rusteal has to change: when the engine removes one,
UHT stops exporting it and the next `rusteal build` stops generating it. Game
code that calls it from Rust breaks at that point, so these are listed for
whoever uses them.

The files are generated per project; **Where** names the generated wrapper.

| ID | API | Kind | Deprecated in | Replacement | Where (`Generated/`) |
|---|---|---|---|---|---|
| UE-DEP-003 | `UAtmosphericFogComponent` | class | UE 4.26 | the SkyAtmosphere component | `RustealFunc_engine_AtmosphericFogComponent.cpp` |
| UE-DEP-004 | `AActor::DetachRootComponentFromParent` | function | UE 4.17 | `DetachFromActor()` | `RustealFunc_engine_Actor.cpp` |
| UE-DEP-005 | `AActor::K2_AttachRootComponentTo` | function | UE 4.17 | `AttachToComponent()` | `RustealFunc_engine_Actor.cpp` |
| UE-DEP-006 | `AActor::K2_AttachRootComponentToActor` | function | UE 4.17 | `AttachToActor()` | `RustealFunc_engine_Actor.cpp` |
| UE-DEP-007 | `USceneComponent::K2_AttachTo` | function | UE 4.17 | `AttachToComponent()` | `RustealFunc_engine_SceneComponent.cpp` |
| UE-DEP-008 | `USceneComponent::DetachFromParent` | function | UE 4.12 | `DetachFromComponent()` | `RustealFunc_engine_SceneComponent.cpp` |
| UE-DEP-009 | `UAnimInstance::CalculateDirection` | function | UE 5.0 | `UKismetAnimationLibrary::CalculateDirection` | `RustealFunc_engine_AnimInstance.cpp` |
| UE-DEP-010 | `ACharacter::ClientAckGoodMove`, `ClientAdjustPosition`, `ClientAdjustRootMotionPosition`, `ClientVeryShortAdjustPosition` | function | UE 4.26 | `ClientMoveResponsePacked()` | `RustealFunc_engine_Character.cpp` |
| UE-DEP-011 | `APlayerController::Get/SetDeprecatedInput{Pitch,Yaw,Roll}Scale` | function | UE 5.0 | the Enhanced Input Scalar modifier | `RustealFunc_engine_PlayerController.cpp` |
| UE-DEP-012 | `USkeletalMeshComponent::GetDisableAnimCurves`, `SetDisableAnimCurves` | function | UE 4.18 | `GetAllowedAnimCurveEvaluate`, `SetAllowAnimCurveEvaluation` (meaning reversed) | `RustealFunc_engine_SkeletalMeshComponent.cpp` |
| UE-DEP-013 | `UGameUserSettings::GetSyncInterval` | function | UE 4.25 | `GetFramePace` | `RustealFunc_engine_GameUserSettings.cpp` |
| UE-DEP-014 | `APawn::IsControlled` | function | UE 4.24 | `IsPawnControlled`, `IsPlayerControlled` | `RustealFunc_engine_Pawn.cpp` |
| UE-DEP-015 | `UKismetInputLibrary::PointerEvent_GetTouchpadIndex` | function | UE 5.8 | none: retired | `RustealFunc_engine_KismetInputLibrary.cpp` |
| UE-DEP-016 | `UDataLayerSubsystem::SetDataLayerRuntimeState` (label/instance overload), `SetDataLayerRuntimeStateByLabel` | function | UE 5.1 | the `UDataLayerAsset*` overload, `SetDataLayerInstanceRuntimeState` | `RustealFunc_engine_DataLayerSubsystem.cpp` |

Last checked: Rusteal 0.2.1, UE 5.8.2, with the modules `rusteal new` enables
(`core`, `engine`, `input`, `slate`, `umg`). Status of all: open, followed by the
engine.

## Fixed

None yet.
