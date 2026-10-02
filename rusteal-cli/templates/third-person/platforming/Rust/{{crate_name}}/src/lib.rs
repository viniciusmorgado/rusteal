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
// The Platforming variant (`platforming/`) adds, as the C++ template's
// `Variant_Platforming`: `PlatformingCharacter` (press and hold jump, double
// jump, wall jump, coyote time, dash), `PlatformingGameMode` (local
// multiplayer, a PlayerStart per player), `PlatformingPlayerController`
// (respawns the character) and the dash montage's `AnimNotifyEndDash`. Their
// Blueprint children are in `Content/Variant_Platforming/`, and the project
// opens `Lvl_Platforming`.
//
// Each glue file only talks to the engine; the logic that can be reasoned
// about without one lives in `*/model.rs` and is unit-tested with
// `cargo test` (run inside `Rust/`).

rusteal_runtime::entry!();

mod character;
mod game_mode;
mod platforming;
mod player_controller;
