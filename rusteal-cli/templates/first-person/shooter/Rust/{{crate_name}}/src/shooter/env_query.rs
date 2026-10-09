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
    fn provide_single_actor(
        &self,
        querier_object: UObjectRef<Object>,
        _querier_actor: UObjectRef<Actor>,
        resulting_actor: OutRef<UObjectRef<Actor>>,
    ) {
        let Ok(controller) = ShooterAIController::from_obj(querier_object) else {
            return;
        };

        let target = controller.get_current_target();

        if target.is_valid() {
            resulting_actor.set(target);
        } else {
            resulting_actor.set(controller.as_ref().upcast_to());
        }
    }
}
