use bindings::ai::{
    AIController, AIControllerExt, AIPerceptionComponent, AIPerceptionComponentExt,
    BrainComponentExt, FAIStimulus,
};
use bindings::engine::{Actor, ActorExt, ControllerExt, Pawn};
use bindings::gameplay_state_tree::{StateTreeAIComponent, StateTreeComponentExt};
use rusteal_runtime::runtime::{FName, LOG_WARNING, OwnedStruct, RustealResult, UObjectRef, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::npc::ShooterNPC;
use super::state_tree::StateTreeSenseEnemiesTask;

#[uclass(parent = AIController)]
pub struct ShooterAIController {
    #[component(name = "StateTreeAI")]
    state_tree_ai: StateTreeAIComponent,

    #[component(name = "AIPerception")]
    ai_perception: AIPerceptionComponent,

    #[uproperty(EditAnywhere, category = "Shooter")]
    team_tag: FName,

    #[uproperty]
    target_enemy: UObjectRef<Actor>,

    #[uproperty]
    perception_listener: UObjectRef<StateTreeSenseEnemiesTask>,
}

#[uclass_impl]
impl ShooterAIController {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.set_team_tag(FName::new("Enemy"));

        self.state_tree_ai()?
            .checked()?
            .set_start_logic_automatically(false);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_possess(&mut self, possessed_pawn: UObjectRef<Pawn>) {
        if let Err(e) = self.bind_perception() {
            ulog!(LOG_WARNING, "[Shooter] AI perception: {e}");
        }

        if ShooterNPC::from_obj(possessed_pawn).is_err() {
            return;
        }

        if let Ok(pawn) = possessed_pawn.checked() {
            let _ = pawn.tags().push(&self.team_tag().handle());
        }

        if let Ok(state_tree) = self.state_tree_ai().and_then(|s| s.checked()) {
            state_tree.start_logic();
        }
    }
}

impl ShooterAIController {
    pub fn on_pawn_death(&mut self) {
        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        me.stop_movement();

        if let Ok(state_tree) = self.state_tree_ai().and_then(|s| s.checked()) {
            state_tree.stop_logic("");
        }

        me.un_possess();

        me.k2_destroy_actor();
    }

    pub fn set_current_target(&self, target: UObjectRef<Actor>) {
        self.set_target_enemy(target);
    }

    pub fn clear_current_target(&self) {
        self.set_target_enemy(UObjectRef::null());
    }

    pub fn get_current_target(&self) -> UObjectRef<Actor> {
        self.target_enemy()
    }

    pub fn clear_gameplay_focus(&self) {
        if let Ok(me) = self.as_ref().checked() {
            me.k2_clear_focus();
        }
    }

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

    fn on_perception_updated(&self, actor: UObjectRef<Actor>, stimulus: OwnedStruct<FAIStimulus>) {
        if let Ok(mut task) = StateTreeSenseEnemiesTask::from_obj(self.perception_listener()) {
            task.on_perception_updated(self, actor, &stimulus);
        }
    }

    fn on_perception_forgotten(&self, actor: UObjectRef<Actor>) {
        if let Ok(mut task) = StateTreeSenseEnemiesTask::from_obj(self.perception_listener()) {
            task.on_perception_forgotten(self, actor);
        }
    }
}
