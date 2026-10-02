// StrategyGameMode: the Strategy variant's `AStrategyGameMode` in Rust.
//
// Like the C++ class, it is a stub: its Blueprint child `BP_StrategyGameMode`
// sets the classes (the camera pawn, the player controller and the HUD).

use bindings::engine::GameModeBase;
use rusteal_runtime::uclass;

/// Simple GameMode for a top down strategy game.
#[uclass(parent = GameModeBase)]
pub struct StrategyGameMode {}
