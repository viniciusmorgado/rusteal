use bindings::ai::{AIController, AIControllerExt, FAIStimulus, FAIStimulusExt};
use bindings::engine::{
    Actor, ActorExt, EDrawDebugTrace, ETraceTypeQuery, GameplayStatics, KismetMathLibrary,
    KismetSystemLibrary, SceneComponentExt,
};
use bindings::prelude::*;
use bindings::state_tree::{
    FStateTreeDelegateDispatcher, FStateTreeTransitionResult, StateTreeConditionBlueprintBase,
    StateTreeTaskBlueprintBase, StateTreeTaskBlueprintBaseExt,
};
use glam::DVec3;
use rusteal_runtime::runtime::{FName, OwnedStruct, RustealResult, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::ai_controller::ShooterAIController;
use super::model;
use super::npc::ShooterNPC;
use crate::character::FirstPersonCharacter;

fn task_defaults(task: UObjectRef<StateTreeTaskBlueprintBase>) -> RustealResult<()> {
    task.checked()?.set_should_state_change_on_reselect(false);
    Ok(())
}

fn visibility_blocked(
    world: UObjectRef<Actor>,
    start: DVec3,
    end: DVec3,
    ignored: &[UObjectRef<Actor>],
) -> bool {
    let (hit, _) = KismetSystemLibrary::line_trace_single(
        world.upcast_to(),
        &FVector::from_dvec3(start),
        &FVector::from_dvec3(end),
        ETraceTypeQuery::TraceTypeQuery1,
        false,
        ignored,
        EDrawDebugTrace::None,
        false,
        &Default::default(),
        &Default::default(),
        None,
    );

    hit
}

#[uclass(parent = StateTreeConditionBlueprintBase)]
pub struct StateTreeLineOfSightToTargetCondition {
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<ShooterNPC>,

    #[uproperty(EditAnywhere, category = "Condition")]
    target: UObjectRef<Actor>,

    #[uproperty(EditAnywhere, category = "Condition", default = 35.0)]
    line_of_sight_cone_angle: f32,

    #[uproperty(EditAnywhere, category = "Condition", default = 5)]
    number_of_vertical_line_of_sight_checks: i32,

    #[uproperty(EditAnywhere, category = "Condition", default = true)]
    b_must_have_line_of_sight: bool,
}

#[uclass_impl]
impl StateTreeLineOfSightToTargetCondition {
    #[ufunction(Override)]
    fn receive_test_condition(&self) -> bool {
        let must_see = self.b_must_have_line_of_sight();

        if self.has_line_of_sight() {
            must_see
        } else {
            !must_see
        }
    }
}

impl StateTreeLineOfSightToTargetCondition {
    fn has_line_of_sight(&self) -> bool {
        let (Ok(target), Ok(character)) = (self.target().checked(), self.character().checked())
        else {
            return false;
        };

        let character_location = character.k2_get_actor_location().to_dvec3();

        let target_dir =
            (target.k2_get_actor_location().to_dvec3() - character_location).normalize_or_zero();

        let facing_dot = target_dir.dot(character.get_actor_forward_vector().to_dvec3());

        if facing_dot <= model::cone_cosine(self.line_of_sight_cone_angle()) {
            return false;
        }

        let (center_of_mass, extent) = target.get_actor_bounds(true, Some(false));
        let center_of_mass = center_of_mass.to_dvec3();

        let Ok(camera) = FirstPersonCharacter::from_obj(self.character())
            .and_then(|base| base.first_person_camera_component()?.checked())
        else {
            return false;
        };

        let start = camera.k2_get_component_location().to_dvec3();

        let ignored = [character.as_ref().upcast_to(), target.as_ref().upcast_to()];

        model::line_of_sight_heights(
            extent.to_dvec3().z,
            self.number_of_vertical_line_of_sight_checks(),
        )
        .into_iter()
        .any(|height| {
            let end = center_of_mass + DVec3::new(0.0, 0.0, height);

            !visibility_blocked(character.as_ref().upcast_to(), start, end, &ignored)
        })
    }
}

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeFaceActorTask {
    #[uproperty(EditAnywhere, category = "Context")]
    controller: UObjectRef<AIController>,

    #[uproperty(EditAnywhere, category = "Input")]
    actor_to_face_towards: UObjectRef<Actor>,
}

#[uclass_impl]
impl StateTreeFaceActorTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(controller) = self.controller().checked() {
            controller.k2_set_focus(self.actor_to_face_towards());
        }
    }

    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(controller) = self.controller().checked() {
            controller.k2_clear_focus();
        }
    }
}

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeFaceLocationTask {
    #[uproperty(EditAnywhere, category = "Context")]
    controller: UObjectRef<AIController>,

    #[uproperty(EditAnywhere, category = "Parameter")]
    face_location: OwnedStruct<FVector>,
}

#[uclass_impl]
impl StateTreeFaceLocationTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(controller) = self.controller().checked() {
            controller.k2_set_focal_point(&self.face_location());
        }
    }

    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(controller) = self.controller().checked() {
            controller.k2_clear_focus();
        }
    }
}

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeSetRandomFloatTask {
    #[uproperty(EditAnywhere, category = "Parameter", default = 0.0)]
    min_value: f32,

    #[uproperty(EditAnywhere, category = "Parameter", default = 0.0)]
    max_value: f32,

    #[uproperty(EditAnywhere, category = "Output", default = 0.0)]
    out_value: f32,
}

#[uclass_impl]
impl StateTreeSetRandomFloatTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        let value = KismetMathLibrary::random_float_in_range(
            f64::from(self.min_value()),
            f64::from(self.max_value()),
        );

        self.set_out_value(value as f32);
    }
}

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeShootAtTargetTask {
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<ShooterNPC>,

    #[uproperty(EditAnywhere, category = "Input")]
    target: UObjectRef<Actor>,
}

#[uclass_impl]
impl StateTreeShootAtTargetTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(mut character) = ShooterNPC::from_obj(self.character()) {
            character.start_shooting(self.target());
        }
    }

    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(mut character) = ShooterNPC::from_obj(self.character()) {
            character.stop_shooting();
        }
    }
}

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeSenseEnemiesTask {
    #[uproperty(EditAnywhere, category = "Context")]
    controller: UObjectRef<ShooterAIController>,

    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<ShooterNPC>,

    #[uproperty(EditAnywhere, category = "Output")]
    target_actor: UObjectRef<Actor>,

    #[uproperty(EditAnywhere, category = "Output")]
    investigate_location: OwnedStruct<FVector>,

    #[uproperty(EditAnywhere, category = "Parameter")]
    sense_tag: FName,

    #[uproperty(EditAnywhere)]
    on_investigate_location_delegate: OwnedStruct<FStateTreeDelegateDispatcher>,

    #[uproperty(EditAnywhere)]
    on_see_enemy_delegate: OwnedStruct<FStateTreeDelegateDispatcher>,

    #[uproperty(EditAnywhere)]
    on_forget_enemy_delegate: OwnedStruct<FStateTreeDelegateDispatcher>,

    #[uproperty(EditAnywhere, category = "Parameter", default = 85.0)]
    direct_line_of_sight_cone: f32,

    #[uproperty(EditAnywhere, default = 0.0)]
    last_stimulus_strength: f32,

    #[uproperty(EditAnywhere, default = 0.0)]
    last_stimulus_time: f32,
}

#[uclass_impl]
impl StateTreeSenseEnemiesTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.set_sense_tag(FName::new("Player"));

        task_defaults(self.as_ref())
    }

    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(controller) = ShooterAIController::from_obj(self.controller()) {
            controller.set_perception_listener(self.as_ref().cast().unwrap_or_default());
        }
    }

    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(controller) = ShooterAIController::from_obj(self.controller()) {
            controller.set_perception_listener(UObjectRef::null());
        }
    }
}

impl StateTreeSenseEnemiesTask {
    pub fn on_perception_updated(
        &mut self,
        controller: &ShooterAIController,
        sensed_actor: UObjectRef<Actor>,
        stimulus: &OwnedStruct<FAIStimulus>,
    ) {
        let (Ok(sensed), Ok(character)) = (sensed_actor.checked(), self.character().checked())
        else {
            return;
        };

        let sense_tag = self.sense_tag();

        if sensed.actor_has_tag(sense_tag.handle()) {
            self.see_enemy(controller, sensed_actor);
            return;
        }

        if FName(stimulus.as_ref().get_tag()) != sense_tag {
            return;
        }

        let character_location = character.k2_get_actor_location().to_dvec3();
        let stimulus_location = stimulus.as_ref().get_stimulus_location().to_dvec3();
        let stimulus_dir = (stimulus_location - character_location).normalize_or_zero();

        let direct_line_of_sight = model::within_cone(
            stimulus_dir,
            character.get_actor_forward_vector().to_dvec3(),
            self.direct_line_of_sight_cone(),
        ) && !visibility_blocked(
            character.as_ref().upcast_to(),
            character_location,
            sensed.k2_get_actor_location().to_dvec3(),
            &[character.as_ref().upcast_to(), sensed_actor],
        );

        if direct_line_of_sight {
            self.see_enemy(controller, sensed_actor);
            return;
        }

        if self.target_actor().is_valid() {
            return;
        }

        let now = GameplayStatics::get_time_seconds(character.as_ref().upcast_to()) as f32;

        let scaled = model::scaled_stimulus(
            self.last_stimulus_strength(),
            now - self.last_stimulus_time(),
        );

        let strength = stimulus.as_ref().get_strength();

        if strength > scaled {
            self.set_last_stimulus_strength(strength);
            self.set_last_stimulus_time(now);

            self.set_investigate_location(&FVector::from_dvec3(stimulus_location));

            self.broadcast(&self.on_investigate_location_delegate());
        }
    }

    pub fn on_perception_forgotten(
        &mut self,
        controller: &ShooterAIController,
        sensed_actor: UObjectRef<Actor>,
    ) {
        self.set_last_stimulus_strength(0.0);

        let target = self.target_actor();

        if sensed_actor != target && target.is_valid() {
            return;
        }

        self.set_target_actor(UObjectRef::null());

        controller.clear_current_target();
        controller.clear_gameplay_focus();

        self.broadcast(&self.on_forget_enemy_delegate());
    }

    fn see_enemy(&mut self, controller: &ShooterAIController, sensed_actor: UObjectRef<Actor>) {
        controller.set_current_target(sensed_actor);

        self.set_target_actor(sensed_actor);

        self.broadcast(&self.on_see_enemy_delegate());
    }

    fn broadcast(&self, dispatcher: &OwnedStruct<FStateTreeDelegateDispatcher>) {
        if let Ok(task) = self.as_ref().checked() {
            task.broadcast_delegate(dispatcher);
        }
    }
}
