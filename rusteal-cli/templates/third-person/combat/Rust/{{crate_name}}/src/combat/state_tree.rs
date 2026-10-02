// The Combat variant's StateTree nodes in Rust: the C++ template's are structs
// (`FStateTreeComboAttackTask`, ...); here they are Blueprint-style task and
// condition classes, which the enemy's StateTree (`ST_CombatEnemy`) runs the
// same way. Their properties keep the C++ instance data's names and
// categories, so the tree's bindings hold: Context ones the tree fills in,
// Input ones it binds, Output ones it reads back.
//
// The attack and landing tasks finish when the enemy says so: the C++ tasks
// bind a lambda to the enemy's delegates, these register with the enemy,
// which finishes them.

use bindings::ai::{AIController, AIControllerExt};
use bindings::engine::{
    Actor, ActorExt, Character, CharacterExt, CharacterMovementComponentExt, GameplayStatics,
    KismetMathLibrary, NavMovementComponentExt,
};
use bindings::prelude::*;
use bindings::state_tree::{
    FStateTreeTransitionResult, StateTreeConditionBlueprintBase, StateTreeTaskBlueprintBase,
    StateTreeTaskBlueprintBaseExt,
};
use rusteal_runtime::runtime::{OwnedStruct, RustealResult, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::enemy::CombatEnemy;
use super::model;

/// StateTree condition to check if the character is grounded
#[uclass(parent = StateTreeConditionBlueprintBase)]
pub struct StateTreeCharacterGroundedCondition {
    /// Character to check grounded status on
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<Character>,

    /// If true, the condition passes if the character is not grounded instead
    #[uproperty(EditAnywhere, category = "Condition", default = false)]
    b_must_be_on_air: bool,
}

#[uclass_impl]
impl StateTreeCharacterGroundedCondition {
    /// Tests the StateTree condition
    #[ufunction(Override)]
    fn receive_test_condition(&self) -> bool {
        // is the character currently grounded?
        let Ok(movement) = self.character().checked().and_then(|c| c.get_character_movement().checked()) else {
            return false;
        };
        let grounded = movement.is_moving_on_ground();
        if self.b_must_be_on_air() { !grounded } else { grounded }
    }
}

/// StateTree condition to check if the character is about to be hit by an attack
#[uclass(parent = StateTreeConditionBlueprintBase)]
pub struct StateTreeIsInDangerCondition {
    /// Character to check danger status on
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<CombatEnemy>,

    /// Minimum time to wait before reacting to the attack
    #[uproperty(EditAnywhere, category = "Parameters", default = model::MIN_REACTION_TIME)]
    min_reaction_time: f32,

    /// Maximum time to wait before ignoring the attack
    #[uproperty(EditAnywhere, category = "Parameters", default = model::MAX_REACTION_TIME)]
    max_reaction_time: f32,

    /// Line of sight half angle for detecting incoming attacks, in degrees
    #[uproperty(EditAnywhere, category = "Parameters", default = model::DANGER_SIGHT_CONE_ANGLE)]
    danger_sight_cone_angle: f32,
}

#[uclass_impl]
impl StateTreeIsInDangerCondition {
    /// Tests the StateTree condition
    #[ufunction(Override)]
    fn receive_test_condition(&self) -> bool {
        // ensure we have a valid enemy character
        let Ok(enemy) = CombatEnemy::from_obj(self.character()) else {
            return false;
        };
        let Ok(character) = enemy.as_ref().checked() else {
            return false;
        };
        // is the last detected danger event within the reaction threshold?
        let now = GameplayStatics::get_time_seconds(character.as_ref().upcast_to());
        model::is_in_danger(
            now - enemy.last_danger_time(),
            self.min_reaction_time(),
            self.max_reaction_time(),
            character.k2_get_actor_location().to_dvec3(),
            character.get_actor_forward_vector().to_dvec3(),
            enemy.last_danger_location(),
            self.danger_sight_cone_angle(),
        )
    }
}

/// What the attack and landing tasks set on their task class defaults:
/// `bShouldStateChangeOnReselect = false` (they do not tick).
fn task_defaults(task: UObjectRef<StateTreeTaskBlueprintBase>) -> RustealResult<()> {
    task.checked()?.set_should_state_change_on_reselect(false);
    Ok(())
}

/// StateTree task to perform a combo attack
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeComboAttackTask {
    /// Character that will perform the attack
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<CombatEnemy>,
}

#[uclass_impl]
impl StateTreeComboAttackTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            // bind to the on attack completed delegate
            enemy.set_attack_completed_listener(self.as_ref());
            // tell the character to do a combo attack
            enemy.do_ai_combo_attack();
        }
    }

    /// Runs when the owning state is ended
    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // unbind the on attack completed delegate
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            enemy.set_attack_completed_listener(UObjectRef::null());
        }
    }
}

/// StateTree task to perform a charged attack
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeChargedAttackTask {
    /// Character that will perform the attack
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<CombatEnemy>,
}

#[uclass_impl]
impl StateTreeChargedAttackTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            // bind to the on attack completed delegate
            enemy.set_attack_completed_listener(self.as_ref());
            // tell the character to do a charged attack
            enemy.do_ai_charged_attack();
        }
    }

    /// Runs when the owning state is ended
    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // unbind the on attack completed delegate
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            enemy.set_attack_completed_listener(UObjectRef::null());
        }
    }
}

/// StateTree task to wait for the character to land
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeWaitForLandingTask {
    /// Character that will be waiting to land
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<CombatEnemy>,
}

#[uclass_impl]
impl StateTreeWaitForLandingTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // bind to the on enemy landed delegate
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            enemy.set_landed_listener(self.as_ref());
        }
    }

    /// Runs when the owning state is ended
    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // unbind the on enemy landed delegate
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            enemy.set_landed_listener(UObjectRef::null());
        }
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

/// StateTree task to change a Character's ground speed
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeSetCharacterSpeedTask {
    /// Character that will be affected
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<Character>,

    /// Max ground speed to set for the character
    #[uproperty(EditAnywhere, category = "Parameter", default = model::CHARACTER_SPEED)]
    speed: f32,
}

#[uclass_impl]
impl StateTreeSetCharacterSpeedTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        // set the character's max ground speed
        if let Ok(movement) = self.character().checked().and_then(|c| c.get_character_movement().checked()) {
            movement.set_max_walk_speed(self.speed());
        }
    }
}

/// StateTree task to get information about the player character
#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeGetPlayerInfoTask {
    /// Character that owns this task
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<Character>,

    /// Character that owns this task
    #[uproperty(VisibleAnywhere, category = "Output")]
    target_player_character: UObjectRef<Character>,

    /// Last known location for the target
    #[uproperty(VisibleAnywhere, category = "Output")]
    target_player_location: OwnedStruct<FVector>,

    /// Distance to the target
    #[uproperty(VisibleAnywhere, category = "Output", default = 0.0)]
    distance_to_target: f32,

    /// Max range to consider targets
    #[uproperty(VisibleAnywhere, category = "Parameter", default = model::PLAYER_INFO_MAX_RANGE)]
    max_range: f32,
}

#[uclass_impl]
impl StateTreeGetPlayerInfoTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    /// Runs when the owning state is entered
    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        let found = self.select_target();
        // if the target is not valid, fail the task
        if let Ok(me) = self.as_ref().checked() {
            me.finish_task(Some(found));
        }
    }
}

impl StateTreeGetPlayerInfoTask {
    /// A local player's character within range, picked at random among
    /// several; `false` when there is none.
    fn select_target(&mut self) -> bool {
        let Ok(character) = self.character().checked() else {
            return false;
        };
        let world = character.as_ref().upcast_to();
        let location = character.k2_get_actor_location().to_dvec3();
        // reset the selected target
        let mut selected: Option<(UObjectRef<Character>, glam::DVec3)> = None;
        // iterate through each local player
        let num_players = GameplayStatics::get_num_local_player_controllers(world);
        for i in 0..num_players {
            let Ok(current) = GameplayStatics::get_player_pawn(world, i).cast::<Character>() else {
                continue;
            };
            let Ok(current_location) = current.checked().map(|c| c.k2_get_actor_location().to_dvec3()) else {
                continue;
            };
            // is this target within range?
            if (current_location - location).length() >= f64::from(self.max_range()) {
                continue;
            }
            // no valid target yet, so choose this one; else randomly switch to the new target
            if selected.is_none() || KismetMathLibrary::random_bool() {
                selected = Some((current, current_location));
            }
        }

        // set the new target
        let Some((target, target_location)) = selected else {
            self.set_target_player_character(UObjectRef::null());
            return false;
        };
        self.set_target_player_character(target);
        // set the target location and distance
        self.set_target_player_location(&FVector::from_dvec3(target_location));
        self.set_distance_to_target((target_location - location).length() as f32);
        true
    }
}
