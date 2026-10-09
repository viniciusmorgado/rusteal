use bindings::engine::{Character, GameplayStatics};
use bindings::state_tree::StateTreeTaskBlueprintBase;
use rusteal_runtime::runtime::UObjectRef;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeGetPlayerTask {
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<Character>,

    #[uproperty(VisibleAnywhere, category = "Output")]
    target_player_character: UObjectRef<Character>,
}

#[uclass_impl]
impl StateTreeGetPlayerTask {
    #[ufunction(Override)]
    fn receive_tick(&mut self, _delta_time: f32) {
        let player = GameplayStatics::get_player_pawn(self.character().upcast_to(), 0);
        self.set_target_player_character(player.cast().unwrap_or_default());
    }
}
