use bindings::ai::{AIController, AIControllerExt, BrainComponentExt};
use bindings::engine::{ControllerExt, Pawn};
use bindings::gameplay_state_tree::StateTreeAIComponent;
use bindings::gameplay_state_tree::StateTreeComponentExt;
use rusteal_runtime::runtime::{RustealResult, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = AIController)]
pub struct CombatAIController {
    #[component(name = "StateTreeAI")]
    state_tree_ai: StateTreeAIComponent,
}

#[uclass_impl]
impl CombatAIController {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        me.set_start_ai_logic_on_possess(false);

        self.state_tree_ai()?
            .checked()?
            .set_start_logic_automatically(false);

        me.set_attach_to_pawn(true);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_possess(&mut self, _possessed_pawn: UObjectRef<Pawn>) {
        if let Ok(state_tree) = self.state_tree_ai().and_then(|s| s.checked()) {
            state_tree.start_logic();
        }
    }
}
