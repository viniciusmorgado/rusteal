// The Combat variant's EnvQuery contexts in Rust, which the enemy's evade
// query (`EnvQuery_Evade`) runs from: the C++ `UEnvQueryContext`s override the
// `ProvideContext` virtual; these are Blueprint-style contexts, which provide
// a single location or actor through their events.

use bindings::ai::EnvQueryContext_BlueprintBase;
use bindings::core_ue::Object;
use bindings::engine::{Actor, GameplayStatics};
use bindings::prelude::*;
use rusteal_runtime::runtime::{OutRef, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::enemy::CombatEnemy;

/// Custom EnvQuery Context that returns the last known danger location for the enemy
#[uclass(parent = EnvQueryContext_BlueprintBase)]
pub struct EnvQueryContextDanger {}

#[uclass_impl]
impl EnvQueryContextDanger {
    /// Provides the context location
    #[ufunction(Override)]
    fn provide_single_location(
        &self,
        _querier_object: UObjectRef<Object>,
        querier_actor: UObjectRef<Actor>,
        resulting_location: UStructRef<FVector>,
    ) {
        // get the querying enemy
        if let Ok(querier) = CombatEnemy::from_obj(querier_actor) {
            // add the last recorded danger location to the context
            let location = querier.last_danger_location();
            resulting_location.set_x(location.x);
            resulting_location.set_y(location.y);
            resulting_location.set_z(location.z);
        }
    }
}

/// Custom EnvQuery Context that returns the actor currently controlled by Player 0
#[uclass(parent = EnvQueryContext_BlueprintBase)]
pub struct EnvQueryContextPlayer {}

#[uclass_impl]
impl EnvQueryContextPlayer {
    /// Provides the context actor
    #[ufunction(Override)]
    fn provide_single_actor(
        &self,
        querier_object: UObjectRef<Object>,
        _querier_actor: UObjectRef<Actor>,
        resulting_actor: OutRef<UObjectRef<Actor>>,
    ) {
        // get the player pawn for the first local player
        let player_pawn = GameplayStatics::get_player_pawn(querier_object, 0);
        // add the actor data to the context
        resulting_actor.set(player_pawn.upcast_to());
    }
}
