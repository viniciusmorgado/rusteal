// The Strategy variant's EnvQuery context in Rust, which the units' movement
// queries (`EnvQuery_MoveUnitClosest`, `EnvQuery_MoveUnitAdditional`) run
// around: the C++ `UEnvQueryContext_MoveGoal` overrides the `ProvideContext`
// virtual; this is a Blueprint-style context, which provides a single
// location through its event.

use bindings::ai::EnvQueryContext_BlueprintBase;
use bindings::core_ue::{FVector, FVectorExt, Object};
use bindings::engine::Actor;
use rusteal_runtime::runtime::{UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::unit::StrategyUnit;

/// Simple EnvQueryContext that returns a Unit's current movement goal location
#[uclass(parent = EnvQueryContext_BlueprintBase)]
pub struct EnvQueryContextMoveGoal {}

#[uclass_impl]
impl EnvQueryContextMoveGoal {
    /// Provides the context location
    #[ufunction(Override)]
    fn provide_single_location(
        &self,
        querier_object: UObjectRef<Object>,
        _querier_actor: UObjectRef<Actor>,
        resulting_location: UStructRef<FVector>,
    ) {
        // get the querying unit
        if let Ok(querier) = StrategyUnit::from_obj(querier_object) {
            // add the unit's movement goal to the context
            let goal = querier.get_movement_goal();
            resulting_location.set_x(goal.x);
            resulting_location.set_y(goal.y);
            resulting_location.set_z(goal.z);
        }
    }
}
