use bindings::engine::GameModeBase;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = GameModeBase)]
pub struct XRGameMode {}

#[uclass_impl]
impl XRGameMode {}
