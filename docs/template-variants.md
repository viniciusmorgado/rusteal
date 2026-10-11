# Template variants

How the engine's C++ templates and their variants became Rusteal templates,
where the Rust ports differ from the C++ and why, and what Rusteal still lacks
for them. The templates themselves are listed in the game developer's handbook
([Starting from a template](https://viniciusmorgado.github.io/game.rusteal.github.io/getting_started/templates.html)); the gaps of the Third Person base,
closed since, are in [`third-person-gaps.md`](third-person-gaps.md).

Every variant was ported in a project the editor created from the UE 5.8.2 C++
template, played headless after each step, and copied into
`rusteal-cli/templates/<template>/<variant>/` once it had no gameplay C++
left. The engine's Blueprint-only templates (Virtual Reality) were ported from
their Blueprints, as [below](#blueprint-only-templates).

## How a variant is ported

- **Classes keep their C++ names**, without the `A`/`U` prefix:
  `AShooterCharacter` is `ShooterCharacter`, `ATP_TopDownCharacter` is
  `TopDownCharacter`. Their UE names, properties, functions and events keep
  the C++ ones (`name = "..."` where PascalCase does not give them), so the
  variant's Blueprints, StateTrees, EnvQueries and levels keep working on the
  Rust classes.
- **Assets are moved, not rebuilt.** `[CoreRedirects]` send the C++ classes
  to the Rust ones while the assets are resaved; the resaved assets name the
  Rust classes and are what the template ships.
- **Placed actors of a World Partition level** keep their first native class
  in their actor descriptor: a placed `BP_TimeTrialTrackGate` names
  `/Script/TP_VehicleAdv.TimeTrialTrackGate`. A Rust class is not native, so
  a redirect to it leaves the descriptor invalid and the level skips the
  actor. While the level is resaved, the old class is redirected to
  `/Script/Engine.Actor` instead; the actor then loads as its Blueprint, whose
  parent is already the Rust class, and the resaved descriptor names
  `Actor`.
- **C++ virtuals become events** (below). A Blueprint child that implements
  the same event in its graph hides the Rust one, so a port checks that the
  variant's Blueprints only hold data there.
- **StateTree tasks and conditions** written as C++ structs become Rust
  classes on the Blueprint node bases (`StateTreeTaskBlueprintBase`,
  `StateTreeConditionBlueprintBase`), with the StateTree's bindings moved to
  them.
- **EnvQuery contexts** become `EnvQueryContext_BlueprintBase` classes that
  provide a single actor or location.
- **Top Down's base Blueprints** are Blueprint-only in the UE 5.8 template:
  their graphs implement the controls and the C++ classes go unused. The
  template makes them children of the Rust classes and removes their graphs,
  so the game runs what the C++ classes do.

## Blueprint-only templates

Virtual Reality, Handheld AR and Intro To Unreal have no C++: their gameplay
is in Blueprint graphs. `rusteal new`
gives such a project the game module Rusteal builds with (`Source/<Project>`,
its targets, and the `.uproject` entry), and the port works on the Blueprints
themselves:

- **The logic moves to Rust, the data stays.** Each Blueprint's graphs
  (functions, macros, event graphs, timelines, event dispatchers) become a
  Rust class with the same functions, events and properties, and the Blueprint
  is reparented to it with its graphs removed. What it held as data stays in
  it: meshes, materials, Niagara systems, widget classes, haptic effects,
  input actions and mapping contexts, a Widget Blueprint's layout, an Anim
  Blueprint's AnimGraph.
- **Components move to the Rust class.** A Blueprint's components are
  declared by the Rust class under the same names (`#[component(name = ...)]`
  when snake case does not give it), the Blueprint's own are removed, and
  their values are copied onto the inherited ones, so the levels' placed
  actors keep their component overrides.
- **Variables keep their names.** A Rust property takes the Blueprint
  variable's name and type (`real` is `f64`, `float` is `f32`), so the graphs
  left (AnimGraphs, levels) find it. Reparenting renames a Blueprint variable
  that clashes with an inherited property (`PoseAlphaGrasp_0`); its references
  are then moved to the inherited one and the variable deleted.
- **Enums become `#[uenum]`s** (`E_GrabType` is `EGrabType`, the
  spectator's `EVRSpectatorMode` keeps its name), Blueprint Interfaces stay
  assets that the Rust class implements (`BPI_PawnAnim`), and a widget's named
  buttons are `#[uproperty(BindWidget)]` properties.
- **Blueprint structs become `#[ustruct]`s, and their data tables follow.**
  Handheld AR's styles are rows of `Struct_Style` in `DT_Styles`; the Rust
  `StructStyle` takes the struct's field names (`BGColor` with `name = ...`),
  and the table is filled again from its JSON export with the Rust struct as
  its row struct.
- **Values set on nested widgets are set again.** A widget placed in another
  widget's layout keeps the values set on it (each AR toggle button's icon
  type, icons and id, each capsule button's text), stored under the
  Blueprint variable that the reparent removes. The rebase sets them on the
  layout's widgets by name once the reparented class has the Rust property:
  an enum read under the old variable comes back as the enum's last value,
  and the capsule's `FText` is a Rust `String`.
- **Placed Blueprints keep their components.** A level saves each placed
  actor's own component values under the components' names; if the Rust
  parent declares a component of the same name, the level gives it those
  values, marks it as made by the construction script, and the construction
  script then destroys it. The Intro To Unreal level places 102 such actors
  (texts, titles, keys, spline paths, doors), so its Blueprints keep their
  components and the features' Rust classes find them by name; the
  unlockable door gets back, as its own, the `Box` its old Blueprint parent
  gave it. VR and AR, whose levels place few of their Blueprints, moved the
  components to Rust.
- **Anim Blueprints keep their whole AnimGraph.** Only the event graph is
  removed: a state machine's states and transitions are graphs of their own,
  and removing them leaves the AnimGraph pointing at nothing, which crashes
  the editor when it compiles the Animation Blueprint.
- **Timelines become state in Rust.** A timeline's curve is read from the
  asset and played by the class's tick (`model::Timeline`): the menu opens
  over 0.2 s, the spectator fades over 2 s with a circular ease.

The Blueprints are rebased in a headless editor in two passes: the first
removes graphs, dispatchers and components, reparents, compiles and copies the
data back; the second, in a new editor session, moves the references of the
renamed variables and deletes them (the Python API sees the reparented class's
new properties only in a new session).

A VR or AR template cannot be played without its device on Linux: they are
validated in Play In Editor without a headset or an AR session, calling the
pawn's functions the input would call (VR: teleport, snap turn, grab, menu,
spectator; AR: a tap on a plane placed by the test selects it, a second tap
places the object, then moves, twists and pinches with and without
snapping, the options, the styles, the reset). The AR pawn reads the touches
as the template does, polling `GetInputTouchState` for the first two fingers
every tick, and the pinch from the `Gesture_Pinch` key.

## C++ virtuals and their events

| C++ | Event the Rust class overrides |
|---|---|
| `AActor::Tick` | `ReceiveTick` (and ticking is enabled) |
| `AActor::TakeDamage` | `ReceiveAnyDamage` |
| `AActor::NotifyHit` | `ReceiveHit` |
| `AActor::NotifyActorBeginOverlap` | `ReceiveActorBeginOverlap` |
| `AActor::Destroyed` | `ReceiveDestroyed` |
| `APawn::NotifyControllerChanged` | `ReceiveControllerChanged` |
| `APawn::SetupPlayerInputComponent` | `ReceiveRestarted` (the input component exists once a local player possesses the pawn) |
| `APlayerController::SetupInputComponent` | `ReceiveBeginPlay` |
| `AController::OnPossess` | `ReceivePossess` |
| `AHUD::DrawHUD` | `ReceiveDrawHUD` |
| `UEnvQueryContext::ProvideContext` | `ProvideSingleActor`, `ProvideSingleLocation` |
| StateTree task `EnterState`, `ExitState`, `Tick` | `ReceiveLatentEnterState`, `ReceiveExitState`, `ReceiveTick` |
| StateTree condition `TestCondition` | `ReceiveTestCondition` |

## Where the Rust ports differ

- **Delegates a gameplay class declares.** When the ports were written a Rust
  class could declare no delegates (`#[udelegate]` came later), so the class
  calls its one listener instead of broadcasting: the
  Horror character calls the UI that registered itself
  (`set_sprint_listener`), and the Shooter AI controller calls the StateTree
  task that registered itself (`set_perception_listener`). A delegate nothing
  binds, such as the Strategy unit's `OnMoveCompleted`, is left out.
- **Engine delegates outside reflection.** The Strategy unit listens to its
  path following component's `OnRequestFinished`, a C++ delegate; the Rust
  unit listens to its AI controller's `ReceiveMoveCompleted`, which the
  controller broadcasts for the same requests.
- **The input action instance.** When the Strategy port was written a Rust
  input handler received only the action's value (handlers may now take the
  elapsed and triggered seconds too). The Strategy player controller's touch
  hold reads how long the finger has been down; the Rust controller times the
  hold from the action's `Started` event.
- **Delegates the C++ keeps out of reflection.** The TimeTrial widgets
  declare `FStartRaceDelegate` and `FCountdownFinishedDelegate` without a
  `UPROPERTY`, so only C++ binds them; the Rust widgets declare them with
  `#[udelegate]`, which Blueprints can bind as well.
- **Vehicle setups.** The C++ cars fill their movement component's
  `EngineSetup`, `TransmissionSetup`, `SteeringSetup`, `DifferentialSetup`
  and `WheelSetups` in the constructor; the Rust cars read each struct, change
  its fields and write it back in `#[class_defaults]` (`setup.rs`). The
  structs are not Blueprint types, so the exporter binds them because the
  movement component's properties hold them.
- **C++ quirks are kept.** The Strategy unit's constructor calls
  `SetFixedBrakingDistance(200.0f)` and then `SetFixedBrakingDistance(true)`,
  which sets the distance to 1; the Rust unit sets 1.
- **The VR menu's Restart button** opens a level named `VRTemplateMap`, which
  the template does not have; the Rust menu opens the level it is on, or the
  one its `RestartLevelName` names.
- **The Intro's stations are features.** Its Blueprints are children of the
  `mannequin`, `checkpoints`, `interactables` and `tour` plugins' Rust classes,
  which the template adds as plugins. `BP_GM_Template`'s save slot and the
  in game editor panels became world subsystems (`CheckpointSubsystem`,
  `TourSubsystem`), so the game mode keeps only its defaults and
  `BP_SaveData` is left out. Its first person Blueprints are the engine's
  older First Person ones (`FirstPersonMesh` and `FirstPersonCamera`, input
  in the character's graph, `BPI_TouchInterface` for the touch controls):
  the intro's own `FirstPersonCharacter` keeps those names and implements
  the touch interface.
- **The AR capsule button's text** is a `String` property set on each
  capsule in the layouts (the Blueprint's was an `FText`).
- **The VR hand Anim Blueprint** keeps its AnimGraph; its event graph, which
  read the mirroring from the hand component and eased the point and thumb
  poses, is the Rust `HandAnimInstance` it is now a child of.

## Gaps

| Gap | Rusteal needs | Workaround today |
|---|---|---|
| Delegates declared in Rust | Fixed: `#[udelegate]` declares a multicast delegate on a Rust class, broadcast from Rust and bindable from Blueprint and Rust. | The ports still call their listener directly, as written before it existed. |
| The input action instance in handlers | Fixed: handlers may take the elapsed and triggered seconds and the action, as the engine's dynamic binding signature does. | The Strategy port still times the hold from `Started`. |
| Moving a component attached to a skeletal mesh socket in `#[class_defaults]` | `#[class_defaults]` runs on the finished default object, where the mesh has no skeletal mesh yet, so the move asks for a socket that is not there: the First Person camera (`head`) and the OffRoad tires (`VisWheel_FR`, `VisWheel_BR`) log `GetSocketInfoByName` warnings at startup, where the C++ constructors doing the same log none. | None needed: the transform is right once the mesh is assigned. |
| C++ delegates of engine components | `codegen` or `manual`: hand-written bindings for delegates outside reflection (`UPathFollowingComponent::OnRequestFinished`). | A reflected delegate raised for the same thing. |

## Engine issues seen in the variants

These show in the engine's own C++ projects too; they are not the ports'.

- Loading the OffRoad level logs that an external actor
  (`Lvl_Offroad/B/9M/ZNZM1Q9PKXHV07GMT8Q6NN`) depends on a package that is
  not available.
- The TimeTrial countdown is a widget animation, which does not play in an
  editor started with `-nullrhi`: headless tests call the countdown widget's
  `FinishCountdown` themselves.

- The Combat and Horror levels place a door frame whose mesh,
  `/Game/LevelPrototyping/Interactable/Door/Assets/Meshes/SM_DoorFrame_Edge`,
  the engine does not ship: loading the level logs a missing package.
- The VR template's door frame and jump pad (LevelPrototyping) only react to
  a `Character`; the VR pawn is a `Pawn`, so they ignore it.
- The VR template's `IA_Menu_Interact_Right_Released` uses a Pressed trigger,
  and no Blueprint uses it.
- The Intro To Unreal template ships the external actors and objects of
  the First Person template's horror and shooter variants, without their
  levels; `rusteal new` leaves them out.
- The Intro To Unreal confetti and the other GPU Niagara systems do not run
  in an editor started with `-nullrhi`: headless tests count the confetti
  trigger's bursts instead.
- The Handheld AR game mode keeps the engine's `PlayerController`: its
  `BP_ARPlayerController` is left unused, and the Rust port keeps it so.
- The Handheld AR plane picks its scan material by platform name (`Android`,
  `IOS`) and has none on other platforms.
- The TwinStick NPC's StateTree component starts its tree before the AI
  controller possesses the NPC, so each NPC logs that the StateTree could not
  find its context actor, then runs once possessed.
- Compiling the TwinStick NPC's StateTree warns that the `Active` state's
  transition targets `Dead` in a different subtree, as the engine's asset is
  made.
