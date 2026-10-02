// ThirdPersonGameMode: the Third Person template's `ATP_ThirdPersonGameMode` in Rust.
//
// Like the C++ class, it is a stub: its Blueprint child `BP_ThirdPersonGameMode`
// sets the classes (Default Pawn Class = `BP_ThirdPersonCharacter`, Player
// Controller Class = `BP_ThirdPersonPlayerController`).

use bindings::engine::GameModeBase;
use rusteal_runtime::uclass;

#[uclass(parent = GameModeBase)]
pub struct ThirdPersonGameMode {}
