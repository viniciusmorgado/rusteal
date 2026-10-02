// FirstPersonGameMode: the First Person template's `ATP_FirstPersonGameMode` in Rust.
//
// Like the C++ class, it is a stub: its Blueprint child `BP_FirstPersonGameMode`
// sets the classes (Default Pawn Class = `BP_FirstPersonCharacter`, Player
// Controller Class = `BP_FirstPersonPlayerController`).

use bindings::engine::GameModeBase;
use rusteal_runtime::uclass;

#[uclass(parent = GameModeBase)]
pub struct FirstPersonGameMode {}
