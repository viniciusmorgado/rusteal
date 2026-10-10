use bindings::ai::EnvQueryContext_BlueprintBase;
use bindings::core_ue::{FVector, FVectorExt, Object};
use bindings::engine::Actor;
use rusteal_runtime::runtime::{UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::unit::StrategyUnit;

#[uclass(parent = EnvQueryContext_BlueprintBase)]
pub struct EnvQueryContextMoveGoal {}

#[uclass_impl]
impl EnvQueryContextMoveGoal {
    #[ufunction(Override)]
    fn provide_single_location(
        &self,
        querier_object: UObjectRef<Object>,
        _querier_actor: UObjectRef<Actor>,
        resulting_location: UStructRef<FVector>,
    ) {
        if let Ok(querier) = StrategyUnit::from_obj(querier_object) {
            let goal = querier.get_movement_goal();
            resulting_location.set_x(goal.x);
            resulting_location.set_y(goal.y);
            resulting_location.set_z(goal.z);
        }
    }
}
