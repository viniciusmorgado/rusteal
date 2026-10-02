// The Top Down template's and its TwinStick variant's gameplay classes in Rust.
//
// UE's Top Down C++ template ships `ATP_TopDownCharacter`,
// `ATP_TopDownGameMode` and `ATP_TopDownPlayerController`; here they are
// `TopDownCharacter`, `TopDownGameMode` and `TopDownPlayerController`. The
// engine template's Blueprints in `Content/TopDown/Blueprints/` implement the
// same controls in their own graphs and leave the C++ classes unused; here
// they are children of the Rust classes and keep only their assets and
// values:
// - `BP_TopDownCharacter`: the mannequin (skeletal mesh, anim class) and the
//   camera framing (boom length 1400, pitch -50, yaw 45, lag, field of view 55);
// - `BP_TopDownController`: the mapping context (IMC_Default), the click and
//   touch actions, the short press threshold and the cursor effect;
// - `BP_TopDownGameMode`: Default Pawn Class = BP_TopDownCharacter, Player
//   Controller Class = BP_TopDownController. It is the project's default game
//   mode (Config/DefaultEngine.ini).
//
// The TwinStick variant (`twin_stick/`) adds `TwinStickCharacter`, which
// moves with one stick, aims with the other (or the mouse), shoots
// `TwinStickProjectile`s, dashes and spends items on `TwinStickAoEAttack`s;
// `TwinStickNPC`s, run by `TwinStickAIController`'s StateTree with a Rust
// task, spawned in waves by `TwinStickSpawner`s and replaced by a
// `TwinStickNPCDestruction` when destroyed, dropping `TwinStickPickup`s; and
// `TwinStickGameMode`, which keeps the score and its combo multiplier on
// `TwinStickUI`, with `TwinStickPlayerController`, which respawns the
// player. Their Blueprint children are in `Content/Variant_TwinStick/`, and
// the project opens and plays `LVL_TwinStick`.
//
// WASD / left stick move, mouse / right stick aim, left click / RT shoot,
// Left Shift / LB dash, right click / LT area attack; touch controls on touch
// devices.

rusteal_runtime::entry!();

mod character;
mod game_mode;
mod player_controller;
mod twin_stick;
