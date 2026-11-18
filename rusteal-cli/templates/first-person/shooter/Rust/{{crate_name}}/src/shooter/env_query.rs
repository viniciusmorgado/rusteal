use bindings::ai::EnvQueryContext_BlueprintBase;
use bindings::core_ue::Object;
use bindings::engine::Actor;
use rusteal_runtime::runtime::{OutRef, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::ai_controller::ShooterAIController;

#[uclass(parent = EnvQueryContext_BlueprintBase)]
pub struct EnvQueryContextTarget {}

#[uclass_impl]
impl EnvQueryContextTarget {
    #[ufunction(Override)]
    fn provide_single_actor<'a>(
        &self,
        querier_object: UObjectRef<'a, Object>,
        _querier_actor: UObjectRef<'a, Actor>,
        resulting_actor: OutRef<UObjectRef<'a, Actor>>,
    ) {
        let Ok(controller) = ShooterAIController::from_obj(querier_object) else {
            return;
        };

        // What the controller gives lives as long as `controller`; the result must live as long as
        // the query's objects.
        let target = controller
            .get_current_target()
            .with_scope_of(&querier_object);

        if target.is_valid() {
            resulting_actor.set(target);
        } else {
            resulting_actor.set(
                controller
                    .as_ref()
                    .with_scope_of(&querier_object)
                    .upcast_to(),
            );
        }
    }
}
