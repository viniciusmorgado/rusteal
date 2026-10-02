// TwinStickAIController: the TwinStick variant's `ATwinStickAIController` in
// Rust, an AI controller that runs the NPC's StateTree (`ST_TwinStickNPC`,
// from its Blueprint child `BP_TwinStickAIController`) once it possesses it.

use bindings::ai::{AIController, AIControllerExt};
use bindings::engine::ControllerExt;
use bindings::gameplay_state_tree::StateTreeAIComponent;
use rusteal_runtime::runtime::RustealResult;
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = AIController)]
pub struct TwinStickAIController {
    /// StateTree Component
    #[component(name = "StateTreeAI")]
    state_tree_ai: StateTreeAIComponent,
}

#[uclass_impl]
impl TwinStickAIController {
    /// Everything `ATwinStickAIController::ATwinStickAIController()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        // ensure we start the StateTree when we possess the pawn
        me.set_start_ai_logic_on_possess(true);
        // ensure we're attached to the possessed character.
        // this is necessary for EnvQueries to work correctly
        me.set_attach_to_pawn(true);
        Ok(())
    }
}
