// TopDownGameMode: the Top Down template's `ATP_TopDownGameMode` in Rust.
//
// Like the C++ class, it is a stub: its Blueprint child `BP_TopDownGameMode`
// sets the classes (Default Pawn Class = `BP_TopDownCharacter`, Player
// Controller Class = `BP_TopDownController`).

use bindings::engine::GameModeBase;
use rusteal_runtime::uclass;

#[uclass(parent = GameModeBase)]
pub struct TopDownGameMode {}
