use bindings::ai::{AIController, AIControllerExt};
use bindings::engine::ControllerExt;
use bindings::gameplay_state_tree::StateTreeAIComponent;
use rusteal_runtime::runtime::RustealResult;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = AIController)]
pub struct TwinStickAIController {
    #[component(name = "StateTreeAI")]
    state_tree_ai: StateTreeAIComponent,
}

#[uclass_impl]
impl TwinStickAIController {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        me.set_start_ai_logic_on_possess(true);

        me.set_attach_to_pawn(true);

        Ok(())
    }
}
