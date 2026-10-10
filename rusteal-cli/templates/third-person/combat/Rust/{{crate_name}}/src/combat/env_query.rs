use bindings::ai::EnvQueryContext_BlueprintBase;
use bindings::core_ue::Object;
use bindings::engine::{Actor, GameplayStatics};
use bindings::prelude::*;
use rusteal_runtime::runtime::{OutRef, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::enemy::CombatEnemy;

#[uclass(parent = EnvQueryContext_BlueprintBase)]
pub struct EnvQueryContextDanger {}

#[uclass_impl]
impl EnvQueryContextDanger {
    #[ufunction(Override)]
    fn provide_single_location(
        &self,
        _querier_object: UObjectRef<Object>,
        querier_actor: UObjectRef<Actor>,
        resulting_location: UStructRef<FVector>,
    ) {
        if let Ok(querier) = CombatEnemy::from_obj(querier_actor) {
            let location = querier.last_danger_location();
            resulting_location.set_x(location.x);
            resulting_location.set_y(location.y);
            resulting_location.set_z(location.z);
        }
    }
}

#[uclass(parent = EnvQueryContext_BlueprintBase)]
pub struct EnvQueryContextPlayer {}

#[uclass_impl]
impl EnvQueryContextPlayer {
    #[ufunction(Override)]
    fn provide_single_actor(
        &self,
        querier_object: UObjectRef<Object>,
        _querier_actor: UObjectRef<Actor>,
        resulting_actor: OutRef<UObjectRef<Actor>>,
    ) {
        let player_pawn = GameplayStatics::get_player_pawn(querier_object, 0);

        resulting_actor.set(player_pawn.upcast_to());
    }
}
