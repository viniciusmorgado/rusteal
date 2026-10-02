// The Shooter variant's StateTree nodes in Rust: the C++ template's are
// structs (`FStateTreeSenseEnemiesTask`, ...); here they are Blueprint-style
// task and condition classes, which the NPC's StateTrees (`ST_Shooter`,
// `ST_Shooter_ShootAtTarget`) run the same way. Their properties keep the C++
// instance data's names and categories, so the trees' bindings hold:
// Context ones the tree fills in, Input ones it binds, Output ones it reads
// back, and the Sense Enemies task's delegate dispatchers, which its
// transitions listen to.
//
// The C++ Sense Enemies task binds lambdas to its controller's perception
// delegates; this one registers with the controller, which calls it.

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

/// What the tasks' C++ constructors set: `bShouldCallTick = false` (they
/// have no tick event here) and `bShouldStateChangeOnReselect = false`.
fn task_defaults(task: UObjectRef<StateTreeTaskBlueprintBase>) -> RustealResult<()> {
    task.checked()?.set_should_state_change_on_reselect(false);
    Ok(())
}

/// Whether a visibility trace from `start` to `end`, ignoring `ignored`, is
/// blocked.
fn visibility_blocked(world: UObjectRef<Actor>, start: DVec3, end: DVec3, ignored: &[UObjectRef<Actor>]) -> bool {
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

/// StateTree condition to check if the character has line of sight to its target
#[uclass(parent = StateTreeConditionBlueprintBase)]
pub struct StateTreeLineOfSightToTargetCondition {
    /// Targeting character
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<ShooterNPC>,

    /// Target to check line of sight for
    #[uproperty(EditAnywhere, category = "Condition")]
    target: UObjectRef<Actor>,

    /// Max allowed line of sight cone angle, in degrees
    #[uproperty(EditAnywhere, category = "Condition", default = 35.0)]
    line_of_sight_cone_angle: f32,

    /// Number of vertical line of sight checks to run to try and get around low obstacles
    #[uproperty(EditAnywhere, category = "Condition", default = 5)]
    number_of_vertical_line_of_sight_checks: i32,

    /// If true, the condition passes if the character has line of sight
    #[uproperty(EditAnywhere, category = "Condition", default = true)]
    b_must_have_line_of_sight: bool,
}

#[uclass_impl]
impl StateTreeLineOfSightToTargetCondition {
    /// Tests the StateTree condition
    #[ufunction(Override)]
    fn receive_test_condition(&self) -> bool {
        let must_see = self.b_must_have_line_of_sight();
        if self.has_line_of_sight() { must_see } else { !must_see }
    }
}

impl StateTreeLineOfSightToTargetCondition {
    fn has_line_of_sight(&self) -> bool {
        // ensure the target is valid
        let (Ok(target), Ok(character)) = (self.target().checked(), self.character().checked()) else {
            return false;
        };

        // check if the character is facing towards the target
        let character_location = character.k2_get_actor_location().to_dvec3();
        let target_dir = (target.k2_get_actor_location().to_dvec3() - character_location).normalize_or_zero();
        let facing_dot = target_dir.dot(character.get_actor_forward_vector().to_dvec3());

        // is the facing outside of our cone half angle?
        if facing_dot <= model::cone_cosine(self.line_of_sight_cone_angle()) {
            return false;
        }

        // get the target's bounding box
        let (center_of_mass, extent) = target.get_actor_bounds(true, Some(false));
        let center_of_mass = center_of_mass.to_dvec3();

        // get the character's camera location as the source for the line checks
        let Ok(camera) = FirstPersonCharacter::from_obj(self.character())
            .and_then(|base| base.first_person_camera_component()?.checked())
        else {
            return false;
        };
        let start = camera.k2_get_component_location().to_dvec3();

        // ignore the character and target. We want to ensure there's an unobstructed trace not counting them
        let ignored = [character.as_ref().upcast_to(), target.as_ref().upcast_to()];

        // run a number of vertically offset line traces to the target location;
        // we only need one unobstructed trace
        model::line_of_sight_heights(extent.to_dvec3().z, self.number_of_vertical_line_of_sight_checks())
            .into_iter()
            .any(|height| {
                let end = center_of_mass + DVec3::new(0.0, 0.0, height);
                !visibility_blocked(character.as_ref().upcast_to(), start, end, &ignored)
            })
    }
}

/// StateTree task to face an AI-Controlled Pawn towards an Actor
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeFaceActorTask {
    /// AI Controller that will determine the focused actor
    #[uproperty(EditAnywhere, category = "Context")]
    controller: UObjectRef<AIController>,

    /// Actor that will be faced towards
    #[uproperty(EditAnywhere, category = "Input")]
    actor_to_face_towards: UObjectRef<Actor>,
}

#[uclass_impl]
impl StateTreeFaceActorTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // set the AI Controller's focus
        if let Ok(controller) = self.controller().checked() {
            controller.k2_set_focus(self.actor_to_face_towards());
        }
    }

    /// Runs when the owning state is ended
    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // clear the AI Controller's focus
        if let Ok(controller) = self.controller().checked() {
            controller.k2_clear_focus();
        }
    }
}

/// StateTree task to face an AI-Controlled Pawn towards a world location
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeFaceLocationTask {
    /// AI Controller that will determine the focused location
    #[uproperty(EditAnywhere, category = "Context")]
    controller: UObjectRef<AIController>,

    /// Location that will be faced towards
    #[uproperty(EditAnywhere, category = "Parameter")]
    face_location: OwnedStruct<FVector>,
}

#[uclass_impl]
impl StateTreeFaceLocationTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // set the AI Controller's focus
        if let Ok(controller) = self.controller().checked() {
            controller.k2_set_focal_point(&self.face_location());
        }
    }

    /// Runs when the owning state is ended
    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // clear the AI Controller's focus
        if let Ok(controller) = self.controller().checked() {
            controller.k2_clear_focus();
        }
    }
}

/// StateTree task to calculate a random float value within the specified range
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeSetRandomFloatTask {
    /// Minimum random value
    #[uproperty(EditAnywhere, category = "Parameter", default = 0.0)]
    min_value: f32,

    /// Maximum random value
    #[uproperty(EditAnywhere, category = "Parameter", default = 0.0)]
    max_value: f32,

    /// Output calculated value
    #[uproperty(EditAnywhere, category = "Output", default = 0.0)]
    out_value: f32,
}

#[uclass_impl]
impl StateTreeSetRandomFloatTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // calculate the output value
        let value = KismetMathLibrary::random_float_in_range(f64::from(self.min_value()), f64::from(self.max_value()));
        self.set_out_value(value as f32);
    }
}

/// StateTree task to have an NPC shoot at an actor
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeShootAtTargetTask {
    /// NPC that will do the shooting
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<ShooterNPC>,

    /// Target to shoot at
    #[uproperty(EditAnywhere, category = "Input")]
    target: UObjectRef<Actor>,
}

#[uclass_impl]
impl StateTreeShootAtTargetTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // tell the character to shoot the target
        if let Ok(mut character) = ShooterNPC::from_obj(self.character()) {
            character.start_shooting(self.target());
        }
    }

    /// Runs when the owning state is ended
    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // tell the character to stop shooting
        if let Ok(mut character) = ShooterNPC::from_obj(self.character()) {
            character.stop_shooting();
        }
    }
}

/// StateTree task to have an NPC process AI Perceptions and sense nearby enemies
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeSenseEnemiesTask {
    /// Sensing AI Controller
    #[uproperty(EditAnywhere, category = "Context")]
    controller: UObjectRef<ShooterAIController>,

    /// Sensing NPC
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<ShooterNPC>,

    /// Sensed actor to target
    #[uproperty(EditAnywhere, category = "Output")]
    target_actor: UObjectRef<Actor>,

    /// Sensed location to investigate
    #[uproperty(EditAnywhere, category = "Output")]
    investigate_location: OwnedStruct<FVector>,

    /// Tag required on sensed actors
    #[uproperty(EditAnywhere, category = "Parameter")]
    sense_tag: FName,

    /// StateTree delegate to broadcast when the AI should move to investigate something suspicious
    #[uproperty(EditAnywhere)]
    on_investigate_location_delegate: OwnedStruct<FStateTreeDelegateDispatcher>,

    /// StateTree delegate to broadcast when the AI has detected an enemy and should attack it
    #[uproperty(EditAnywhere)]
    on_see_enemy_delegate: OwnedStruct<FStateTreeDelegateDispatcher>,

    /// StateTree delegate to broadcast when the AI has lost track of its target
    #[uproperty(EditAnywhere)]
    on_forget_enemy_delegate: OwnedStruct<FStateTreeDelegateDispatcher>,

    /// Line of sight cone half angle to consider a full sense
    #[uproperty(EditAnywhere, category = "Parameter", default = 85.0)]
    direct_line_of_sight_cone: f32,

    /// Strength of the last processed stimulus
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

    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // bind the perception delegates on the controller
        if let Ok(controller) = ShooterAIController::from_obj(self.controller()) {
            controller.set_perception_listener(self.as_ref().cast().unwrap_or_default());
        }
    }

    /// Runs when the owning state is ended
    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // unbind the perception delegates
        if let Ok(controller) = ShooterAIController::from_obj(self.controller()) {
            controller.set_perception_listener(UObjectRef::null());
        }
    }
}

impl StateTreeSenseEnemiesTask {
    /// The perception updated delegate: a perception of `sensed_actor`.
    pub fn on_perception_updated(
        &mut self,
        controller: &ShooterAIController,
        sensed_actor: UObjectRef<Actor>,
        stimulus: &OwnedStruct<FAIStimulus>,
    ) {
        let (Ok(sensed), Ok(character)) = (sensed_actor.checked(), self.character().checked()) else {
            return;
        };
        let sense_tag = self.sense_tag();

        // have we sensed the enemy directly?
        if sensed.actor_has_tag(sense_tag.handle()) {
            self.see_enemy(controller, sensed_actor);
            return;
        }

        // have we sensed something owned by the enemy? firing noise, bullet impacts, etc.
        if FName(stimulus.as_ref().get_tag()) != sense_tag {
            return;
        }

        // calculate the direction of the stimulus
        let character_location = character.k2_get_actor_location().to_dvec3();
        let stimulus_location = stimulus.as_ref().get_stimulus_location().to_dvec3();
        let stimulus_dir = (stimulus_location - character_location).normalize_or_zero();

        // is the direction within our perception cone? we have direct line
        // of sight if a trace between the character and the sensed actor is
        // unobstructed
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

        // no direct line of sight to target: if we already have a target,
        // ignore the partial sense and keep on them
        if self.target_actor().is_valid() {
            return;
        }

        // scale the last stimulus by time elapsed so we phase it out over time
        let now = GameplayStatics::get_time_seconds(character.as_ref().upcast_to()) as f32;
        let scaled = model::scaled_stimulus(self.last_stimulus_strength(), now - self.last_stimulus_time());

        // is this stimulus stronger?
        let strength = stimulus.as_ref().get_strength();
        if strength > scaled {
            // update the stimulus strength and time
            self.set_last_stimulus_strength(strength);
            self.set_last_stimulus_time(now);

            // set the investigate location
            self.set_investigate_location(&FVector::from_dvec3(stimulus_location));

            // broadcast the investigate delegate
            self.broadcast(&self.on_investigate_location_delegate());
        }
    }

    /// The perception forgotten delegate: `sensed_actor` is forgotten.
    pub fn on_perception_forgotten(&mut self, controller: &ShooterAIController, sensed_actor: UObjectRef<Actor>) {
        // reset the stimulus strength
        self.set_last_stimulus_strength(0.0);

        // are we forgetting the current target, or a partial sense while we have no target?
        let target = self.target_actor();
        if sensed_actor != target && target.is_valid() {
            return;
        }

        // clear the target
        self.set_target_actor(UObjectRef::null());

        // clear the target on the controller
        controller.clear_current_target();
        controller.clear_gameplay_focus();

        // broadcast the forget delegate
        self.broadcast(&self.on_forget_enemy_delegate());
    }

    /// Target `sensed_actor` and tell the tree an enemy was seen.
    fn see_enemy(&mut self, controller: &ShooterAIController, sensed_actor: UObjectRef<Actor>) {
        // set the controller's target
        controller.set_current_target(sensed_actor);

        // set the task output
        self.set_target_actor(sensed_actor);

        // broadcast the see enemy delegate
        self.broadcast(&self.on_see_enemy_delegate());
    }

    fn broadcast(&self, dispatcher: &OwnedStruct<FStateTreeDelegateDispatcher>) {
        if let Ok(task) = self.as_ref().checked() {
            task.broadcast_delegate(dispatcher);
        }
    }
}
