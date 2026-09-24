# Third Person gaps

What Rusteal is missing to write the core of Unreal Engine's Third Person C++
template entirely in Rust: `AMyProjectCharacter`, `AMyProjectGameMode` and
`AMyProjectPlayerController` (the template's variants are out of scope). The
template is the guide: a feature earns a place here because the template needs
it, not because it exists in the engine.

The reference port is a Third Person project whose Rust crate provides
`ThirdPersonCharacter` and `ThirdPersonGameMode`, built with the published CLI.
Each entry records how that port copes today and what Rusteal would need so the
workaround, and then the C++ class, can go.

## How to read an entry

| Field | Meaning |
|---|---|
| **Template** | What the C++ template does, with file and line (`MyProject*` files as the template generates them). |
| **Today** | The workaround in the Rust port, or `blocked`. |
| **Rusteal needs** | The change, by layer: `macros`, `core`, `codegen`, `plugin` (C++), `exporter` (UHT plugin). |
| **Evidence** | Where the limit is, in Rusteal or in the engine. |
| **Depends on** | Other entries that must land first. |
| **Done when** | The workaround is gone from the port, and headless and in-editor play still work. |
| **Status** | `open`, `workaround`, or `fixed in <rusteal version>`. |
| **Last checked** | Rusteal and UE versions. |

Entries are numbered `TP-GAP-NN` and never renumbered.

## Findings that change the picture

Three experiments on the reference port (Rusteal 0.3.0, UE 5.8.2), all reverted:

- **A Blueprint can derive from a Rust class.** `BP_RustCharacter` (parent
  `ThirdPersonCharacter`) and `BP_RustGameMode` (parent `ThirdPersonGameMode`)
  are created, compile and play. The Rust `#[uproperty]` fields show up as
  editable class defaults, and so do inherited ones: `Player Controller Class`,
  `Default Pawn Class`, the skeletal mesh and the anim class. This is the
  template's own pattern (a Blueprint child holds the assets and the classes),
  so several limits have a Blueprint route today; see TP-GAP-01.
- **Enhanced Input bindings can be generated per project.** Adding
  `EnhancedInput = { module = "enhanced_input", feature = "enhanced-input" }` to
  `rusteal.toml` generates 43 classes, 13 structs and 17 enums, and the project
  builds once five functions and one class are blocklisted (TP-GAP-05).
- **Every value the template's constructor sets has a reflected setter**
  (`set_player_controller_class`, `set_orient_rotation_to_movement`,
  `set_target_arm_length`, ...). What is missing is reaching the private
  components of `ACharacter` (TP-GAP-07).

## Gaps

### TP-GAP-01 — Inherited class defaults from Rust

| | |
|---|---|
| **Template** | The constructors set inherited defaults: capsule size, `bUseControllerRotation*`, the `CharacterMovement` tuning (`MyProjectCharacter.cpp:18-35`); the game mode's Blueprint child sets `DefaultPawnClass` and `PlayerControllerClass`. |
| **Today** | workaround. The game mode's classes come from its Blueprint child, as in the template: `BP_RustGameMode` sets `Default Pawn Class = BP_RustCharacter`, and the Rust `ThirdPersonGameMode` is a stub like `AMyProjectGameMode`. The character still applies its constructor values in `ReceiveBeginPlay`. |
| **Rusteal needs** | `macros`: a class-defaults hook run at registration, where Rust sets inherited properties on the class default object, next to the `#[uproperty]` defaults the finalize step already writes. |
| **Evidence** | `rusteal-macros/src/uclass.rs` (finalize: `reify_get_cdo`, `finalize_cdo_stmts`); `RustealReifyApiImpl.cpp` `GetCdoImpl`. The player controller is spawned by `SpawnPlayActor` before the world's `BeginPlay` (`Engine/Private/UnrealEngine.cpp` `LoadMap`, `GameInstance.cpp` PIE), so it can only be chosen by a default, not at runtime. |
| **Depends on** | TP-GAP-07 for the `CharacterMovement` and capsule values. |
| **Done when** | The Rust classes carry the template's defaults with no Blueprint child and no `BeginPlay` configuration. |
| **Status** | workaround (Blueprint child) |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-02 — `#[component(attach)]` to inherited components and sockets

| | |
|---|---|
| **Template** | `CameraBoom->SetupAttachment(RootComponent)`, `FollowCamera->SetupAttachment(CameraBoom, USpringArmComponent::SocketName)` (`MyProjectCharacter.cpp:40, 46`). |
| **Today** | workaround: both components are attached with `k2_attach_to_component` in `ReceiveBeginPlay`. |
| **Rusteal needs** | `plugin`: resolve the attach parent among inherited components (the actor's root) and accept a socket name; `macros`: `#[component(attach = "root", socket = "SpringEndpoint")]` or similar. |
| **Evidence** | `URustealReifiedClass.cpp` `RustealClassConstructor`: the parent is looked up only in `CreatedComponents`, the components the Rust class itself declares. |
| **Depends on** | — |
| **Done when** | Boom and camera are attached at construction, visible in the Blueprint child's component tree. |
| **Status** | open |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-03 — `#[uproperty]` of object, class and array types

| | |
|---|---|
| **Template** | `UInputAction*` ×4 (`MyProjectCharacter.h:38-50`), `TArray<UInputMappingContext*>` ×2 and `TSubclassOf<UUserWidget>` (`MyProjectPlayerController.h:25-33`). |
| **Today** | blocked: input keys are hard-coded. |
| **Rusteal needs** | `macros` + `plugin` (reify): object references (`UObjectRef<T>`), class references (`TSubclassOf`) and `TArray` of those as properties, editable in a Blueprint child. |
| **Evidence** | `rusteal-macros/src/uclass.rs`: "unsupported uproperty type: only bool/i32/i64/u8/f32/f64". |
| **Depends on** | — |
| **Done when** | The input actions, mapping contexts and widget class are assigned in the Blueprint children, as in the template. |
| **Status** | open |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-04 — Blueprint child of a Rust class

| | |
|---|---|
| **Template** | `BP_ThirdPersonCharacter` and `BP_ThirdPersonGameMode` derive from the C++ classes and hold the assets. |
| **Today** | used. `BP_RustCharacter` holds the mannequin (skeletal mesh, anim class, offset in the capsule) and `BP_RustGameMode` the pawn class, as `BP_ThirdPersonCharacter` and `BP_ThirdPersonGameMode` do for the C++ classes. In the class picker the Rust classes appear under their stub Blueprint's name (TP-GAP-10). |
| **Rusteal needs** | nothing. |
| **Evidence** | Experiment above; the reified class constructor already handles Blueprint children (`URustealReifiedClass.cpp`). |
| **Depends on** | — |
| **Done when** | — |
| **Status** | not a gap |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-05 — Enhanced Input

| | |
|---|---|
| **Template** | The player controller adds the mapping contexts (`MyProjectPlayerController.cpp:46-56`); the character binds Jump, Move, Look and MouseLook in `SetupPlayerInputComponent` (`MyProjectCharacter.cpp:53-67`). |
| **Today** | workaround: the character polls fixed keys and sticks in `ReceiveTick`; the input assets are unused. |
| **Rusteal needs** | Four pieces: **(a)** `codegen`: generate compiling wrappers for the module — today it wraps protected `_Implementation` functions (`UInputModifier::GetVisualizationColor`, `ModifyRaw`; `UInputTrigger::GetTriggerType`, `UpdateState`), declares a `TMap<const UInputMappingContext*, int32>` return without `const` (`UPlayerMappableInputConfig::GetMappingContexts`) and misses the include for `FEnhancedActionKeyMapping` in `UPlayerMappableInputConfig`; the plugin should also declare the EnhancedInput plugin dependency (UBT warns). **(b)** `codegen`: emit the `UFUNCTION`s declared in interfaces for the classes implementing them — `AddMappingContext` lives in `IEnhancedInputSubsystemInterface` and is not generated. **(c)** `plugin` + `core` + `macros`: bind an input action to a Rust handler; `BindAction`/`BindActionValue` are C++ templates, not `UFUNCTION`s. **(d)** a hook to bind from, since `SetupPlayerInputComponent` is a C++ virtual and not a Blueprint event (`ReceiveRestarted` fires after the input component exists). |
| **Evidence** | Experiment above; `EnhancedInputSubsystemInterface.h` (`AddMappingContext` is a `UFUNCTION`); `EnhancedInputComponent.h` (`BindActionValue` is not); `GetBoundActionValue` and `FInputActionValue` are generated. |
| **Depends on** | TP-GAP-03 (the actions and contexts are asset references), TP-GAP-08 for handlers that take an `FInputActionValue`. |
| **Done when** | The character reacts to the template's `IA_*` actions through its `IMC_*` contexts, and the polling is gone. |
| **Status** | open |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-06 — Touch interface check

| | |
|---|---|
| **Template** | `ShouldUseTouchControls()` is `SVirtualJoystick::ShouldDisplayTouchInterface() \|\| bForceTouchControls` (`MyProjectPlayerController.cpp:66`). |
| **Today** | blocked (mobile only). |
| **Rusteal needs** | `plugin`: expose the Slate check, which has no reflection. |
| **Evidence** | `SVirtualJoystick` is a Slate widget class, not a `UObject`. |
| **Depends on** | TP-GAP-03 (the widget class property). |
| **Done when** | The Rust player controller spawns the touch controls where the C++ one would. |
| **Status** | open, lowest priority |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-07 — Private components of `ACharacter`

| | |
|---|---|
| **Template** | `GetCapsuleComponent()`, `GetCharacterMovement()`, `GetMesh()`. |
| **Today** | workaround: `GetComponentByClass` through a helper in the port. |
| **Rusteal needs** | `exporter`: export private properties marked `AllowPrivateAccess` / `BlueprintReadOnly` (`CapsuleComponent`, `CharacterMovement`, `Mesh`), which the exporter skips today. |
| **Evidence** | The generated `Character` has no `get_character_movement`, `get_mesh` or `get_capsule_component`. |
| **Depends on** | — |
| **Done when** | The port reaches the three components through the generated accessors. |
| **Status** | workaround |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-08 — `#[ufunction]` parameter and return types

| | |
|---|---|
| **Template** | `Move`/`Look` take a `const FInputActionValue&`; `DoMove`/`DoLook` take floats. |
| **Today** | the float functions work; nothing takes a struct or an object. |
| **Rusteal needs** | `macros`: struct and object parameters and returns in `#[ufunction]`. |
| **Evidence** | `rusteal-macros/src/uclass_impl.rs`: "only bool/i32/i64/u8/f32/f64 are supported". |
| **Depends on** | — |
| **Done when** | Input handlers can take the action value as the template's do. |
| **Status** | open |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-09 — Disabled modules leave their bindings behind

| | |
|---|---|
| **Template** | — (found while enabling and disabling Enhanced Input) |
| **Today** | After a module is removed from `rusteal.toml`, its directory stays in `Rust/bindings/src/` (not referenced, so harmless); its C++ wrappers are removed. |
| **Rusteal needs** | `codegen`: remove module directories that are no longer generated. |
| **Evidence** | Experiment above. |
| **Depends on** | — |
| **Done when** | Toggling a module leaves `bindings/src/` matching `lib.rs`. |
| **Status** | open, cosmetic |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-10 — Rust classes are listed under a `_BP` name

| | |
|---|---|
| **Template** | — (found while creating the Blueprint children) |
| **Today** | The class picker lists `ThirdPersonCharacter` as `ThirdPersonCharacter_BP`, easy to confuse with the template's `BP_ThirdPersonCharacter`. The class itself keeps its name (`/Script/Rusteal.ThirdPersonCharacter`). |
| **Rusteal needs** | `plugin`: name the stub Blueprint `<Class>_RS`, so Rust classes are recognisable in the editor. Blueprint children reference the class, not the stub, so they keep working. |
| **Evidence** | `RustealReifyApiImpl.cpp`: the stub Blueprint is created as `ClassName + "_BP"`. |
| **Depends on** | — |
| **Done when** | The picker lists `ThirdPersonCharacter_RS` and `ThirdPersonGameMode_RS`. |
| **Status** | open |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

## Order

By what each one unblocks for the template:

1. **TP-GAP-05 (a, b)** — Enhanced Input bindings that compile and include
   `AddMappingContext`: input is the core of the template.
2. **TP-GAP-03** — asset references as properties, so the actions and contexts
   are assigned in the Blueprint children.
3. **TP-GAP-08** and **TP-GAP-05 (c, d)** — binding actions to Rust handlers;
   the polling goes.
4. **TP-GAP-02** — attach at construction.
5. **TP-GAP-07** — the character's private components.
6. **TP-GAP-01** — defaults written from Rust; until then the Blueprint children
   carry them, as the template's do.
7. **TP-GAP-06**, **TP-GAP-09** and **TP-GAP-10**.

The Blueprint route needs none of these and is applied in the port: the
`GetDefaultPawnClassForController` override and the mannequin loaded by path are
gone. It also lets a Rust player controller be selected, which waits for
TP-GAP-05: without it, the controller would only cover the touch controls.
