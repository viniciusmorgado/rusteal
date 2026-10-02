// The SideScrolling variant's StateTree task in Rust: `FStateTreeGetPlayerTask`
// is a C++ StateTree node; here it is a Blueprint-style task class, which the
// NPC's StateTree (`ST_SideScrollingNPC`) runs the same way. Its properties
// keep the C++ instance data's names and categories, so the tree's bindings
// hold: Context ones the tree fills in, Output ones it reads back.

use bindings::ai::AIController;
use bindings::engine::{ActorExt, GameplayStatics, Pawn};
use bindings::prelude::*;
use bindings::state_tree::{FStateTreeTransitionResult, StateTreeTaskBlueprintBase, StateTreeTaskBlueprintBaseExt};
use rusteal_runtime::runtime::{UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::model;

/// StateTree task to get the player-controlled character
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeGetPlayerTask {
    /// NPC owning this task
    #[uproperty(VisibleAnywhere, name = "NPC", category = "Context")]
    npc: UObjectRef<Pawn>,

    /// Holds the found player pawn
    #[uproperty(VisibleAnywhere, category = "Context")]
    controller: UObjectRef<AIController>,

    /// Holds the found player pawn
    #[uproperty(VisibleAnywhere, category = "Output")]
    target_player: UObjectRef<Pawn>,

    /// Is the pawn close enough to be considered a valid target?
    #[uproperty(VisibleAnywhere, category = "Output", default = false)]
    b_valid_target: bool,

    /// Max distance to be considered a valid target
    #[uproperty(EditAnywhere, category = "Parameter", default = model::GET_PLAYER_RANGE_MAX)]
    range_max: f32,
}

#[uclass_impl]
impl StateTreeGetPlayerTask {
    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        let valid = self.select_target();
        // succeed or fail depending on target validity
        if let Ok(me) = self.as_ref().checked() {
            me.finish_task(Some(valid));
        }
    }
}

impl StateTreeGetPlayerTask {
    /// The closest local player's pawn, and whether it is in range.
    fn select_target(&mut self) -> bool {
        let controller = self.controller();
        let Ok(npc) = self.npc().checked() else {
            return false;
        };
        let world = controller.upcast_to();
        // iterate through each local player
        let num_players = GameplayStatics::get_num_local_player_controllers(world);
        let players: Vec<_> = (0..num_players)
            .filter_map(|i| {
                let pawn = GameplayStatics::get_player_pawn(world, i);
                let location = pawn.checked().ok()?.k2_get_actor_location().to_dvec3();
                Some((pawn, location))
            })
            .collect();

        // set the selected target, assume out of range by default
        let selected = model::closest_player(npc.k2_get_actor_location().to_dvec3(), &players, self.range_max());
        let (target, valid) = selected.unwrap_or((UObjectRef::null(), false));
        self.set_target_player(target);
        self.set_b_valid_target(valid);
        valid
    }
}
