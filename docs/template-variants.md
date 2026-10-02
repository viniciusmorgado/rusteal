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

- **Delegates a gameplay class declares.** A Rust class declares no
  delegates, so the class calls its one listener instead of broadcasting: the
  Horror character calls the UI that registered itself
  (`set_sprint_listener`), and the Shooter AI controller calls the StateTree
  task that registered itself (`set_perception_listener`). A delegate nothing
  binds, such as the Strategy unit's `OnMoveCompleted`, is left out.
- **Engine delegates outside reflection.** The Strategy unit listens to its
  path following component's `OnRequestFinished`, a C++ delegate; the Rust
  unit listens to its AI controller's `ReceiveMoveCompleted`, which the
  controller broadcasts for the same requests.
- **The input action instance.** A Rust input handler receives the action's
  value, not its `FInputActionInstance`. The Strategy player controller's
  touch hold reads how long the finger has been down from the instance; the
  Rust controller times the hold from the action's `Started` event.
- **C++ quirks are kept.** The Strategy unit's constructor calls
  `SetFixedBrakingDistance(200.0f)` and then `SetFixedBrakingDistance(true)`,
  which sets the distance to 1; the Rust unit sets 1.

## Gaps

| Gap | Rusteal needs | Workaround today |
|---|---|---|
| Delegates declared in Rust | `macros` and `plugin`: a `#[udelegate]` that adds a multicast delegate property to a Rust class, broadcast from Rust and bindable from Blueprint. | The class keeps its listener and calls it. |
| The input action instance in handlers | `plugin`: pass the elapsed and triggered times to handlers that take them, as the engine's dynamic binding signature does. | Time the action from its `Started` event. |
| C++ delegates of engine components | `codegen` or `manual`: hand-written bindings for delegates outside reflection (`UPathFollowingComponent::OnRequestFinished`). | A reflected delegate raised for the same thing. |

## Engine issues seen in the variants

These show in the engine's own C++ projects too; they are not the ports'.

- The Combat and Horror levels place a door frame whose mesh,
  `/Game/LevelPrototyping/Interactable/Door/Assets/Meshes/SM_DoorFrame_Edge`,
  the engine does not ship: loading the level logs a missing package.
- The TwinStick NPC's StateTree component starts its tree before the AI
  controller possesses the NPC, so each NPC logs that the StateTree could not
  find its context actor, then runs once possessed.
- Compiling the TwinStick NPC's StateTree warns that the `Active` state's
  transition targets `Dead` in a different subtree, as the engine's asset is
  made.
