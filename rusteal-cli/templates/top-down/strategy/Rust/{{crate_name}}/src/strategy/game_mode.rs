use bindings::engine::GameModeBase;
use rusteal_runtime::uclass;

#[uclass(parent = GameModeBase)]
pub struct StrategyGameMode {}
