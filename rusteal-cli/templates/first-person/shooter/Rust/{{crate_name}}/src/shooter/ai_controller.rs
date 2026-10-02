// ShooterAIController: the Shooter variant's `AShooterAIController` in Rust,
// a simple AI controller for a first person shooter enemy: it runs the NPC's
// StateTree (`ST_Shooter`, from its Blueprint child `BP_ShooterAIController`),
// senses through AI perception (configured in the Blueprint) and keeps the
// enemy it targets.
//
// The C++ class hands its perception updates to the StateTree's Sense Enemies
// task through two delegates the task binds; here the task registers itself
// (`set_perception_listener`) and the controller calls it.

use bindings::ai::{
    AIController, AIControllerExt, AIPerceptionComponent, AIPerceptionComponentExt,
    BrainComponentExt, FAIStimulus,
};
use bindings::engine::{Actor, ActorExt, ControllerExt, Pawn};
use bindings::gameplay_state_tree::{StateTreeAIComponent, StateTreeComponentExt};
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, OwnedStruct, RustealResult, UObjectRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

use super::npc::ShooterNPC;
use super::state_tree::StateTreeSenseEnemiesTask;

#[uclass(parent = AIController)]
pub struct ShooterAIController {
    /// Runs the behavior StateTree for this NPC
    #[component(name = "StateTreeAI")]
    state_tree_ai: StateTreeAIComponent,

    /// Detects other actors through sight, hearing and other senses
    #[component(name = "AIPerception")]
    ai_perception: AIPerceptionComponent,

    /// Team tag for pawn friend or foe identification
    #[uproperty(EditAnywhere, category = "Shooter")]
    team_tag: FName,

    /// Enemy currently being targeted
    #[uproperty]
    target_enemy: UObjectRef<Actor>,

    /// The StateTree task told about perception updates (the C++ delegates' listener)
    #[uproperty]
    perception_listener: UObjectRef<StateTreeSenseEnemiesTask>,
}

#[uclass_impl]
impl ShooterAIController {
    /// Everything `AShooterAIController::AShooterAIController()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.set_team_tag(FName::new("Enemy"));

        // create the StateTree component
        self.state_tree_ai()?.checked()?.set_start_logic_automatically(false);
        Ok(())
    }

    /// Pawn initialization
    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        // subscribe to the AI perception delegates
        if let Err(e) = self.bind_perception() {
            ulog!(LOG_WARNING, "[Shooter] AI perception: {e}");
        }

        // ensure we're possessing an NPC
        if ShooterNPC::from_obj(possessed_pawn).is_err() {
            return;
        }

        // add the team tag to the pawn
        if let Ok(pawn) = possessed_pawn.checked() {
            let _ = pawn.tags().push(&self.team_tag().handle());
        }

        // start AI logic
        if let Ok(state_tree) = self.state_tree_ai().and_then(|s| s.checked()) {
            state_tree.start_logic();
        }
    }
}

impl ShooterAIController {
    /// Called when the possessed pawn dies
    pub fn on_pawn_death(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // stop movement
        me.stop_movement();

        // stop StateTree logic
        if let Ok(state_tree) = self.state_tree_ai().and_then(|s| s.checked()) {
            state_tree.stop_logic("");
        }

        // unpossess the pawn
        me.un_possess();

        // destroy this controller
        me.k2_destroy_actor();
    }

    /// Sets the targeted enemy
    pub fn set_current_target(&self, target: UObjectRef<Actor>) {
        self.set_target_enemy(target);
    }

    /// Clears the targeted enemy
    pub fn clear_current_target(&self) {
        self.set_target_enemy(UObjectRef::null());
    }

    /// Returns the targeted enemy
    pub fn get_current_target(&self) -> UObjectRef<Actor> {
        self.target_enemy()
    }

    /// `ClearFocus(EAIFocusPriority::Gameplay)`, as the tasks clear it.
    pub fn clear_gameplay_focus(&self) {
        if let Ok(me) = self.as_ref().checked() {
            me.k2_clear_focus();
        }
    }

    /// `AIPerception->OnTargetPerceptionUpdated.AddDynamic(...)` and
    /// `OnTargetPerceptionForgotten`.
    fn bind_perception(&self) -> RustealResult<()> {
        let perception = self.ai_perception()?.checked()?;
        let me: UObjectRef<AIController> = self.as_ref();
        perception
            .on_target_perception_updated()
            .add(move |actor, stimulus| {
                if let Ok(controller) = ShooterAIController::from_obj(me) {
                    controller.on_perception_updated(actor, stimulus);
                }
            })?
            .detach();
        perception
            .on_target_perception_forgotten()
            .add(move |actor| {
                if let Ok(controller) = ShooterAIController::from_obj(me) {
                    controller.on_perception_forgotten(actor);
                }
            })?
            .detach();
        Ok(())
    }

    /// Called when the AI perception component updates a perception on a given actor
    fn on_perception_updated(&self, actor: UObjectRef<Actor>, stimulus: OwnedStruct<FAIStimulus>) {
        // pass the data to the StateTree delegate hook
        if let Ok(mut task) = StateTreeSenseEnemiesTask::from_obj(self.perception_listener()) {
            task.on_perception_updated(self, actor, &stimulus);
        }
    }

    /// Called when the AI perception component forgets a given actor
    fn on_perception_forgotten(&self, actor: UObjectRef<Actor>) {
        // pass the data to the StateTree delegate hook
        if let Ok(mut task) = StateTreeSenseEnemiesTask::from_obj(self.perception_listener()) {
            task.on_perception_forgotten(self, actor);
        }
    }
}
