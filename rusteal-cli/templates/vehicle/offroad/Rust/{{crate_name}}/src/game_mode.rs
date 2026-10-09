use bindings::engine::{GameModeBase, GameModeBaseExt};
use rusteal_runtime::runtime::{RustealResult, SubclassOf};
use rusteal_runtime::{uclass, uclass_impl};

use crate::player_controller::VehicleAdvPlayerController;

#[uclass(parent = GameModeBase)]
pub struct VehicleAdvGameMode {}

#[uclass_impl]
impl VehicleAdvGameMode {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.as_ref().checked()?.set_player_controller_class(
            SubclassOf::<VehicleAdvPlayerController>::base().upcast_to(),
        );

        Ok(())
    }
}
