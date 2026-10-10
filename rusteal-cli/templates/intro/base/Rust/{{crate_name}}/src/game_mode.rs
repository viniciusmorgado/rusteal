use bindings::engine::GameModeBase;
use rusteal_runtime::uclass;

#[uclass(parent = GameModeBase)]
pub struct FirstPersonGameMode {}

#[uclass(parent = GameModeBase)]
pub struct IntroGameMode {}
