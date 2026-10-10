use bindings::engine::{GameModeBase, PlayerController};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = GameModeBase)]
pub struct ARGameMode {}

#[uclass_impl]
impl ARGameMode {}

#[uclass(parent = PlayerController)]
pub struct ARPlayerController {}

#[uclass_impl]
impl ARPlayerController {}
