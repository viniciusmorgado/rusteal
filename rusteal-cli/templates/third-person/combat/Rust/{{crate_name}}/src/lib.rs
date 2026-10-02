// The Third Person template's gameplay classes in Rust.
//
// UE's Third Person C++ template ships `ATP_ThirdPersonCharacter`,
// `ATP_ThirdPersonGameMode` and `ATP_ThirdPersonPlayerController`; here they
// are `ThirdPersonCharacter`, `ThirdPersonGameMode` and
// `ThirdPersonPlayerController`. As in the C++ template, a Blueprint child of
// each holds the assets and the classes, in `Content/ThirdPerson/Blueprints/`:
// - `BP_ThirdPersonCharacter`: the mannequin (skeletal mesh, anim class, mesh
//   offset in the capsule) and the input actions (IA_Jump, IA_Move, IA_Look,
//   IA_MouseLook);
// - `BP_ThirdPersonPlayerController`: the mapping contexts (Default =
//   IMC_Default, Mobile Excluded = IMC_MouseLook);
// - `BP_ThirdPersonGameMode`: Default Pawn Class = BP_ThirdPersonCharacter,
//   Player Controller Class = BP_ThirdPersonPlayerController. It is the
//   project's default game mode (Config/DefaultEngine.ini).
//
// WASD / left stick move, mouse / right stick look, Space / A jump; touch
// controls on touch devices.
//
// The Combat variant (`combat/`) adds, as the C++ template's
// `Variant_Combat`: `CombatCharacter` (combo string, charged attack, damage,
// death and respawn at the last checkpoint), its game mode and player
// controller, enemies run by a StateTree (`CombatEnemy`, its AI controller,
// the StateTree's tasks and conditions and the EnvQuery contexts), enemy
// spawners, activation and checkpoint volumes, damageable boxes, a training
// dummy, lava, the life bar UI and the attack montages' notifies. Their
// Blueprint children are in `Content/Variant_Combat/`, and the project opens
// `Lvl_Combat`.
//
// Each glue file only talks to the engine; the logic that can be reasoned
// about without one lives in `*/model.rs` and is unit-tested with
// `cargo test` (run inside `Rust/`).

rusteal_runtime::entry!();

mod character;
mod combat;
mod game_mode;
mod player_controller;
