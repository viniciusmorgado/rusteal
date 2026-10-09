use bindings::ai::AIController;
use bindings::engine::{ActorExt, GameplayStatics, Pawn};
use bindings::prelude::*;
use bindings::state_tree::{
    FStateTreeTransitionResult, StateTreeTaskBlueprintBase, StateTreeTaskBlueprintBaseExt,
};
use rusteal_runtime::runtime::{UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::model;

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeGetPlayerTask {
    #[uproperty(VisibleAnywhere, name = "NPC", category = "Context")]
    npc: UObjectRef<Pawn>,

    #[uproperty(VisibleAnywhere, category = "Context")]
    controller: UObjectRef<AIController>,

    #[uproperty(VisibleAnywhere, category = "Output")]
    target_player: UObjectRef<Pawn>,

    #[uproperty(VisibleAnywhere, category = "Output", default = false)]
    b_valid_target: bool,

    #[uproperty(EditAnywhere, category = "Parameter", default = model::GET_PLAYER_RANGE_MAX)]
    range_max: f32,
}

#[uclass_impl]
impl StateTreeGetPlayerTask {
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        let valid = self.select_target();

        if let Ok(me) = self.as_ref().checked() {
            me.finish_task(Some(valid));
        }
    }
}

impl StateTreeGetPlayerTask {
    fn select_target(&mut self) -> bool {
        let controller = self.controller();

        let Ok(npc) = self.npc().checked() else {
            return false;
        };

        let world = controller.upcast_to();

        let num_players = GameplayStatics::get_num_local_player_controllers(world);

        let players: Vec<_> = (0..num_players)
            .filter_map(|i| {
                let pawn = GameplayStatics::get_player_pawn(world, i);
                let location = pawn.checked().ok()?.k2_get_actor_location().to_dvec3();

                Some((pawn, location))
            })
            .collect();

        let selected = model::closest_player(
            npc.k2_get_actor_location().to_dvec3(),
            &players,
            self.range_max(),
        );

        let (target, valid) = selected.unwrap_or((UObjectRef::null(), false));
        self.set_target_player(target);
        self.set_b_valid_target(valid);

        valid
    }
}
