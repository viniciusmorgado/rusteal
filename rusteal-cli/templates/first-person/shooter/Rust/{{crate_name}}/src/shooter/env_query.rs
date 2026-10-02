// The Shooter variant's EnvQuery context in Rust, which the NPC's sniping
// location query (`EQS_FindSnipingLocation`) runs from: the C++
// `UEnvQueryContext_Target` overrides the `ProvideContext` virtual; this is a
// Blueprint-style context, which provides a single actor through its event.

use bindings::ai::EnvQueryContext_BlueprintBase;
use bindings::core_ue::Object;
use bindings::engine::Actor;
use rusteal_runtime::runtime::{OutRef, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::ai_controller::ShooterAIController;

/// Custom EnvQuery Context that returns the actor currently targeted by an NPC
#[uclass(parent = EnvQueryContext_BlueprintBase)]
pub struct EnvQueryContextTarget {}

#[uclass_impl]
impl EnvQueryContextTarget {
    /// Provides the context actor
    #[ufunction(Override)]
    fn provide_single_actor(
        &self,
        querier_object: UObjectRef<Object>,
        _querier_actor: UObjectRef<Actor>,
        resulting_actor: OutRef<UObjectRef<Actor>>,
    ) {
        // get the controller from the query instance
        let Ok(controller) = ShooterAIController::from_obj(querier_object) else {
            return;
        };

        // ensure the target is valid; if for any reason there's no target, default to the controller
        let target = controller.get_current_target();
        if target.is_valid() {
            // add the controller's target actor to the context
            resulting_actor.set(target);
        } else {
            resulting_actor.set(controller.as_ref().upcast_to());
        }
    }
}
