// CombatAIController: the Combat variant's `ACombatAIController` in Rust. An
// AI controller running a StateTree, started on possession; its Blueprint
// child `BP_CombatAIController` gives it the enemy's tree (`ST_CombatEnemy`).

use bindings::ai::{AIController, AIControllerExt, BrainComponentExt};
use bindings::engine::{ControllerExt, Pawn};
use bindings::gameplay_state_tree::StateTreeAIComponent;
use bindings::gameplay_state_tree::StateTreeComponentExt;
use rusteal_runtime::runtime::{RustealResult, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = AIController)]
pub struct CombatAIController {
    /// StateTree Component
    #[component(name = "StateTreeAI")]
    state_tree_ai: StateTreeAIComponent,
}

#[uclass_impl]
impl CombatAIController {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        // ensure we start the StateTree when we possess the pawn
        me.set_start_ai_logic_on_possess(false);
        self.state_tree_ai()?.checked()?.set_start_logic_automatically(false);
        // ensure we're attached to the possessed character.
        // this is necessary for EnvQueries to work correctly
        me.set_attach_to_pawn(true);
        Ok(())
    }

    /// Pawn initialization
    #[ufunction(Override)]
    fn receive_possess(&mut self, _possessed_pawn: UObjectRef<Pawn>) {
        // start the StateTree
        if let Ok(state_tree) = self.state_tree_ai().and_then(|s| s.checked()) {
            state_tree.start_logic();
        }
    }
}
