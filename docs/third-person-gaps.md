# Third Person gaps

What Rusteal is missing to write the core of Unreal Engine's Third Person C++
template entirely in Rust: `AMyProjectCharacter`, `AMyProjectGameMode` and
`AMyProjectPlayerController` (the template's variants are out of scope). The
template is the guide: a feature earns a place here because the template needs
it, not because it exists in the engine.

The reference port is a Third Person project whose Rust crate provides
`ThirdPersonCharacter`, `ThirdPersonGameMode` and `ThirdPersonPlayerController`.
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

- **A Blueprint can derive from a Rust class** (TP-GAP-04 fixed the loss of its Rust components in the editor).
  `BP_RustCharacter` (parent `ThirdPersonCharacter`) and `BP_RustGameMode`
  (parent `ThirdPersonGameMode`) are created, compile and play. The Rust `#[uproperty]` fields show up as
  editable class defaults, and so do inherited ones: `Player Controller Class`,
  `Default Pawn Class`, the skeletal mesh and the anim class. This is the
  template's own pattern (a Blueprint child holds the assets and the classes),
  so several limits have a Blueprint route today; see TP-GAP-01.
- **Enhanced Input bindings are generated per project** (TP-GAP-05 a, b fixed).
  `rusteal new` turns the `enhanced-input` feature on; the module generates 43
  classes, 13 structs and 17 enums, `AddMappingContext` included, and builds
  without a blocklist.
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
| **Done when** | Boom and camera are attached at construction, visible in the Blueprint child's component tree. Today the tree lists native (C++) components and a Blueprint parent's construction-script components, and Rust components are neither, so they exist on the pawn but do not show up there. |
| **Status** | open |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-03 — `#[uproperty]` of object, class and array types

| | |
|---|---|
| **Template** | `UInputAction*` ×4 (`MyProjectCharacter.h:38-50`), `TArray<UInputMappingContext*>` ×2 and `TSubclassOf<UUserWidget>` (`MyProjectPlayerController.h:25-33`). |
| **Today** | The character declares `jump_action`, `move_action`, `look_action` and `mouse_look_action` (`UObjectRef<InputAction>`), assigned in `BP_RustCharacter` and logged at BeginPlay; they are bound to handlers with TP-GAP-05 (c, d). |
| **Rusteal needs** | `macros` + `plugin` (reify): object references (`UObjectRef<T>`), class references (`TSubclassOf`) and `TArray` of those as properties, editable in a Blueprint child. Done: `#[uproperty]` takes `UObjectRef<T>`, `SubclassOf<T>` (new in `core`) and `UeArray<E>` of those or of the scalars; the plugin creates the `TArray` property (`RustealReifyPropType::Array`). `default = ...` stays scalar-only: the others are set in the Blueprint child. |
| **Evidence** | `rusteal-codegen/tests/manual_compiles.rs` (`uproperty_types_compile`); in the port, the four properties on the class and, in a temporary run, a `SubclassOf` and arrays of classes, objects and floats written and read back from Rust. |
| **Depends on** | — |
| **Done when** | The input actions, mapping contexts and widget class are assigned in the Blueprint children, as in the template. |
| **Status** | fixed, not released |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-04 — Blueprint child of a Rust class

| | |
|---|---|
| **Template** | `BP_ThirdPersonCharacter` and `BP_ThirdPersonGameMode` derive from the C++ classes and hold the assets. |
| **Today** | works. A Blueprint child of a Rust class keeps its Rust components across editor sessions: checked in the reference port with two sessions (create and play, then reopen and play) and headless. |
| **Rusteal needs** | Done in the plugin. The cause: Rust components were created in the constructor but no property referenced them. A C++ component is also a `UPROPERTY` (`UPROPERTY(VisibleAnywhere) USpringArmComponent* CameraBoom`), and the editor saves and reinstances a class's components through those references. When the editor regenerates a Blueprint on load (`FLinkerLoad` → `RegenerateBlueprintClass`), the child's class default object came back without the Rust components, so each spawned pawn built its own, detached from the Blueprint's, and the lookup by name rejected them. Each `#[component]` now gets an object property of the same name (`VisibleAnywhere`/`BlueprintReadOnly` flags) that the constructor points at the component. |
| **Evidence** | A headless editor (`-nullrhi -unattended`, no `-game`) loading `BP_RustCharacter` listed the class default object without `CameraBoom`/`FollowCamera` before the change and with them, correctly archetyped, after it. `AddDefaultSubobjectImpl` (`RustealReifyApiImpl.cpp`) and `RustealClassConstructor` (`URustealReifiedClass.cpp`). |
| **Depends on** | — |
| **Done when** | A Blueprint child of a Rust class keeps its Rust components across editor sessions. |
| **Status** | fixed, not released |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-05 — Enhanced Input

| | |
|---|---|
| **Template** | The player controller adds the mapping contexts (`MyProjectPlayerController.cpp:46-56`); the character binds Jump, Move, Look and MouseLook in `SetupPlayerInputComponent` (`MyProjectCharacter.cpp:53-67`). |
| **Today** | As in the template: `ThirdPersonPlayerController` adds the mapping contexts its Blueprint child lists (in `ReceiveBeginPlay`), and the character binds Jump, Move, Look and MouseLook to `Jump`/`StopJumping` and its `Move`/`Look` handlers in `ReceiveRestarted`. The polling is gone. |
| **Rusteal needs** | Four pieces, all done: **(a)** `codegen`: generate compiling wrappers for the module — it wrapped protected `_Implementation` functions (`UInputModifier::GetVisualizationColor`, `ModifyRaw`; `UInputTrigger::GetTriggerType`, `UpdateState`), declared the `TMap<TObjectPtr<UInputMappingContext>, int32>` return of `UPlayerMappableInputConfig::GetMappingContexts` as `TMap<UInputMappingContext*, int32>` and missed the include for `FEnhancedActionKeyMapping` in `UPlayerMappableInputConfig`; the plugin did not declare the EnhancedInput plugin dependency. Now a local type re-exports `_Implementation` with a using-declaration, a returned container takes the function's own return type, the exporter records each struct's header, and `Rusteal.uplugin` lists EnhancedInput. **(b)** `codegen`: emit the `UFUNCTION`s declared in interfaces for the classes implementing them — `AddMappingContext` lives in `IEnhancedInputSubsystemInterface`. Now each class gets the callable functions of the interfaces it implements, called through the interface. **(c)** `plugin` + `core` + `macros`: bind an input action to a Rust handler; `BindAction`/`BindActionValue` are C++ templates, not `UFUNCTION`s. Now `bind_action(actor, action, trigger_event, "Function")` (bindings' `manual/`, over a plugin helper) binds a `UFUNCTION` taking nothing or an `FInputActionValue`, idempotently per input component; `enhanced_input_subsystem(controller)` and `InputActionValueExt::axis2d()` stand for the C++ templates `ULocalPlayer::GetSubsystem<>` and `FInputActionValue::Get<>`. **(d)** a hook to bind from, since `SetupPlayerInputComponent` is a C++ virtual and not a Blueprint event. `ReceiveRestarted` is it: `APawn::DispatchRestart` runs `PawnClientRestart`, which creates the input component and calls `SetupPlayerInputComponent`, before `NotifyRestarted`. |
| **Evidence** | `EnhancedInputComponent.h` (`BindActionValue` is not a `UFUNCTION`); `Pawn.cpp` (`DispatchRestart`); `rusteal-codegen/tests/manual_compiles.rs` (`enhanced_input_bindings`, `uclass_types_compile`); in the port, input injected with `InjectInputForAction` reached the Rust `Look` and `Move` handlers. |
| **Depends on** | TP-GAP-03 (the actions and contexts are asset references), TP-GAP-08 for handlers that take an `FInputActionValue`. |
| **Done when** | The character reacts to the template's `IA_*` actions through its `IMC_*` contexts, and the polling is gone. |
| **Status** | fixed, not released |
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
| **Today** | `Move`/`Look` take a `UStructRef<FInputActionValue>`, as the template's take the value. |
| **Rusteal needs** | `macros`: struct and object parameters and returns in `#[ufunction]`. Done: parameters may be `UStructRef<T>` (a reference into the call's parameters; `to_owned()` copies it), `UObjectRef<T>` and `SubclassOf<T>`; returns may be `UObjectRef<T>` and `SubclassOf<T>`. A struct cannot be returned yet. |
| **Evidence** | `rusteal-codegen/tests/manual_compiles.rs` (`uclass_types_compile`); the port's handlers. |
| **Depends on** | — |
| **Done when** | Input handlers can take the action value as the template's do. |
| **Status** | fixed, not released |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-09 — Disabled modules leave their bindings behind

| | |
|---|---|
| **Template** | — (found while enabling and disabling Enhanced Input) |
| **Today** | Turning a module off removes its directory from `Rust/bindings/src/` along with its C++ wrappers; turning it back on regenerates the same files. |
| **Rusteal needs** | `codegen`: remove module directories that are no longer generated. Done: directories whose `mod.rs` carries the codegen header and whose module is off are deleted. |
| **Evidence** | Enhanced Input toggled off and on in the port; `rusteal-codegen/tests/manual_compiles.rs` (`enhanced_input_bindings`). |
| **Depends on** | — |
| **Done when** | Toggling a module leaves `bindings/src/` matching `lib.rs`. |
| **Status** | fixed, not released |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-10 — Rust classes are listed under a `_BP` name

| | |
|---|---|
| **Template** | — (found while creating the Blueprint children) |
| **Today** | The class picker shows a Rust class under its stub Blueprint's name, `ThirdPersonCharacter_BP`, easy to confuse with the template's `BP_ThirdPersonCharacter`. Its search box matches the class name (`ThirdPersonCharacter`), not the name it shows, so searching the shown name finds nothing. The class itself keeps its name (`/Script/Rusteal.ThirdPersonCharacter`). |
| **Rusteal needs** | `plugin`: name the stub Blueprint `RS_<Class>`, so Rust classes are recognisable in the editor. Blueprint children reference the class, not the stub, so they keep working. Done; it reaches projects with the next release and `rusteal upgrade`. |
| **Evidence** | `RustealReifyApiImpl.cpp`: the stub Blueprint is created as `ClassName + "_BP"`. |
| **Depends on** | — |
| **Done when** | The picker shows `RS_ThirdPersonCharacter` and `RS_ThirdPersonGameMode`; they are found by searching the class name (`ThirdPerson`), not `RS_`. |
| **Status** | fixed, not released |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-11 — Enum values UHT does not parse

| | |
|---|---|
| **Template** | `ETriggerEvent` (`Started`, `Completed`, `Triggered`), which the input bindings take. |
| **Today** | The exporter computes the values UHT leaves as -1; `ETriggerEvent` is `None = 0, Triggered = 1, Started = 2, ...`. |
| **Rusteal needs** | `exporter`: UHT records a value only when it is a plain literal and stores -1 otherwise (`(1 << 0)`, `X \| Y`, `BLEND_Translucent`), and so does every implicit value after it; codegen then kept one variant per value, so `ETriggerEvent` had only `None = -1`. Done: the exporter evaluates the enum's initializers from its header (literals, parentheses, arithmetic and bitwise operators, earlier values). 175 of 3186 enums of a UE 5.8.2 project had unparsed values; 22 remain, whose initializers name macros or other enums' values. |
| **Evidence** | `UhtEnumParser.cs` ("-1 if not parsed"); `ue_plugin/RustealGenerator/Source/RustealExporter/RustealEnumValues.cs`. |
| **Depends on** | — |
| **Done when** | The enums the template uses have their engine values. |
| **Status** | fixed, not released |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

### TP-GAP-12 — Generated class references are typed as objects

| | |
|---|---|
| **Template** | `ULocalPlayer::GetSubsystem<UEnhancedInputLocalPlayerSubsystem>(...)`; `TSubclassOf` properties such as `DefaultPawnClass`. |
| **Today** | workaround: codegen types every `TSubclassOf<T>` (property, parameter, return, container element) as `UObjectRef<T>`, an object of class `T`, while it holds a class. The bindings' `enhanced_input_subsystem` and the port's `helpers::class_of` convert a class to that type by hand. |
| **Rusteal needs** | `codegen`: map class properties to `SubclassOf<T>` (added to `core` for `#[uproperty]`), which the generated conversions (`ConversionKind::ObjectRef`, about 30 places) do not know yet. |
| **Evidence** | `rusteal-codegen/src/type_map.rs` (`ClassProperty` → `rusteal_core::UObjectRef<{cls}>`). |
| **Depends on** | — |
| **Done when** | Generated class references are `SubclassOf<T>` and the hand conversions are gone. |
| **Status** | workaround |
| **Last checked** | Rusteal 0.3.0, UE 5.8.2 |

## Order

By what each one unblocks for the template:

1. **TP-GAP-05 (a, b)**, fixed — Enhanced Input bindings that compile and include
   `AddMappingContext`: input is the core of the template.
2. **TP-GAP-03**, fixed — asset references as properties, so the actions and
   contexts are assigned in the Blueprint children.
3. **TP-GAP-08** and **TP-GAP-05 (c, d)**, fixed — binding actions to Rust
   handlers; the polling goes.
4. **TP-GAP-02** — attach at construction.
5. **TP-GAP-07** — the character's private components.
6. **TP-GAP-01** — defaults written from Rust; until then the Blueprint children
   carry them, as the template's do.
7. **TP-GAP-06**, **TP-GAP-09**, **TP-GAP-10**, **TP-GAP-11** and **TP-GAP-12**.

The Blueprint route needs none of these and is applied in the port: the
`GetDefaultPawnClassForController` override and the mannequin loaded by path are
gone. It also lets the Rust player controller be selected: `BP_RustGameMode`
sets `BP_RustPlayerController` as its Player Controller Class.
