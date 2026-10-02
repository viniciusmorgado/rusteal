// The TwinStick variant's StateTree task in Rust: the C++ template's is a
// struct (`FStateTreeGetPlayerTask`); here it is a Blueprint-style task
// class, which the NPC's StateTree (`ST_TwinStickNPC`) runs the same way. Its
// properties keep the C++ instance data's names and categories.

use bindings::engine::{Character, GameplayStatics};
use bindings::state_tree::StateTreeTaskBlueprintBase;
use rusteal_runtime::runtime::UObjectRef;
use rusteal_runtime::{uclass, uclass_impl};

/// StateTree task to get the player character
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeGetPlayerTask {
    /// Character that owns this task
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<Character>,

    /// Character that owns this task
    #[uproperty(VisibleAnywhere, category = "Output")]
    target_player_character: UObjectRef<Character>,
}

#[uclass_impl]
impl StateTreeGetPlayerTask {
    /// Runs while the owning state is active
    #[ufunction(Override)]
    fn receive_tick(&mut self, _delta_time: f32) {
        // get the pawn possessed by the first local player
        let player = GameplayStatics::get_player_pawn(self.character().upcast_to(), 0);
        self.set_target_player_character(player.cast().unwrap_or_default());
    }
}
