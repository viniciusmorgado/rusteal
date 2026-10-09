# Template variants

How the engine's C++ templates and their variants became Rusteal templates,
where the Rust ports differ from the C++ and why, and what Rusteal still lacks
for them. The templates themselves are listed in the README
([Templates](../README.md#templates)); the gaps of the Third Person base,
closed since, are in [`third-person-gaps.md`](third-person-gaps.md).

Every variant was ported in a project the editor created from the UE 5.8.2 C++
template, played headless after each step, and copied into
`rusteal-cli/templates/<template>/<variant>/` once it had no gameplay C++
left.

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
- The TwinStick NPC's StateTree component starts its tree before the AI
  controller possesses the NPC, so each NPC logs that the StateTree could not
  find its context actor, then runs once possessed.
- Compiling the TwinStick NPC's StateTree warns that the `Active` state's
  transition targets `Dead` in a different subtree, as the engine's asset is
  made.
