// The Top Down template's gameplay classes in Rust.
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
// Hold the mouse button (or a finger) to walk towards the cursor; a short
// click (or tap) walks there along the navigation mesh, with a cursor effect.
// The template's variants (Strategy, TwinStick) are their own projects:
// `rusteal new --variant`.

rusteal_runtime::entry!();

mod character;
mod game_mode;
mod player_controller;
