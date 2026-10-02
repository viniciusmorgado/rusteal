// The Top Down template's and its Strategy variant's gameplay classes in Rust.
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
// The Strategy variant (`strategy/`) adds `StrategyPawn`, an orthographic
// camera that pans and zooms; `StrategyPlayerController`, which selects
// `StrategyUnit`s with clicks, double clicks, boxes and touches and sends
// them somewhere; `StrategyUnit`s, which resolve their destinations with
// EnvQueries around `EnvQueryContextMoveGoal`, walk there along the
// navigation mesh and interact with the units they reach; `StrategyHUD`, which
// draws the selection box and marks the selected units, and counts them on
// `StrategyUI`; `StrategyTouchControls`, the touchscreen buttons and zoom
// slider; and `StrategyGameMode`. Their Blueprint children are in
// `Content/Variant_Strategy/`, and the project opens and plays
// `LVL_Strategy`.
//
// Left click selects a unit (Shift adds to the selection), a double click
// selects every unit on screen and dragging selects the units in a box; right
// click sends the selection there, and right drag, WASD, the mouse wheel and
// Q / E move and zoom the camera (middle click resets the zoom). On touch
// devices a tap selects or sends, a held finger pans and a second finger
// draws the selection box.

rusteal_runtime::entry!();

mod character;
mod game_mode;
mod player_controller;
mod strategy;
