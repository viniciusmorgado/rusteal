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

#[uclass(parent = StateTreeConditionBlueprintBase)]
pub struct StateTreeCharacterGroundedCondition {
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<Character>,

    #[uproperty(EditAnywhere, category = "Condition", default = false)]
    b_must_be_on_air: bool,
}

#[uclass_impl]
impl StateTreeCharacterGroundedCondition {
    #[ufunction(Override)]
    fn receive_test_condition(&self) -> bool {
        let Ok(movement) = self
            .character()
            .checked()
            .and_then(|c| c.get_character_movement().checked())
        else {
            return false;
        };

        let grounded = movement.is_moving_on_ground();

        if self.b_must_be_on_air() {
            !grounded
        } else {
            grounded
        }
    }
}

#[uclass(parent = StateTreeConditionBlueprintBase)]
pub struct StateTreeIsInDangerCondition {
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<CombatEnemy>,

    #[uproperty(EditAnywhere, category = "Parameters", default = model::MIN_REACTION_TIME)]
    min_reaction_time: f32,

    #[uproperty(EditAnywhere, category = "Parameters", default = model::MAX_REACTION_TIME)]
    max_reaction_time: f32,

    #[uproperty(EditAnywhere, category = "Parameters", default = model::DANGER_SIGHT_CONE_ANGLE)]
    danger_sight_cone_angle: f32,
}

#[uclass_impl]
impl StateTreeIsInDangerCondition {
    #[ufunction(Override)]
    fn receive_test_condition(&self) -> bool {
        let Ok(enemy) = CombatEnemy::from_obj(self.character()) else {
            return false;
        };

        let Ok(character) = enemy.as_ref().checked() else {
            return false;
        };

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

fn task_defaults(task: UObjectRef<StateTreeTaskBlueprintBase>) -> RustealResult<()> {
    task.checked()?.set_should_state_change_on_reselect(false);
    Ok(())
}

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeComboAttackTask {
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<CombatEnemy>,
}

#[uclass_impl]
impl StateTreeComboAttackTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            enemy.set_attack_completed_listener(self.as_ref());

            enemy.do_ai_combo_attack();
        }
    }

    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            enemy.set_attack_completed_listener(UObjectRef::null());
        }
    }
}

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeChargedAttackTask {
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<CombatEnemy>,
}

#[uclass_impl]
impl StateTreeChargedAttackTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            enemy.set_attack_completed_listener(self.as_ref());

            enemy.do_ai_charged_attack();
        }
    }

    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            enemy.set_attack_completed_listener(UObjectRef::null());
        }
    }
}

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeWaitForLandingTask {
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<CombatEnemy>,
}

#[uclass_impl]
impl StateTreeWaitForLandingTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            enemy.set_landed_listener(self.as_ref());
        }
    }

    #[ufunction(Override)]
    fn receive_exit_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(mut enemy) = CombatEnemy::from_obj(self.character()) {
            enemy.set_landed_listener(UObjectRef::null());
        }
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
pub struct StateTreeSetCharacterSpeedTask {
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<Character>,

    #[uproperty(EditAnywhere, category = "Parameter", default = model::CHARACTER_SPEED)]
    speed: f32,
}

#[uclass_impl]
impl StateTreeSetCharacterSpeedTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        if let Ok(movement) = self
            .character()
            .checked()
            .and_then(|c| c.get_character_movement().checked())
        {
            movement.set_max_walk_speed(self.speed());
        }
    }
}

#[uclass(parent = StateTreeTaskBlueprintBase)]
pub struct StateTreeGetPlayerInfoTask {
    #[uproperty(EditAnywhere, category = "Context")]
    character: UObjectRef<Character>,

    #[uproperty(VisibleAnywhere, category = "Output")]
    target_player_character: UObjectRef<Character>,

    #[uproperty(VisibleAnywhere, category = "Output")]
    target_player_location: OwnedStruct<FVector>,

    #[uproperty(VisibleAnywhere, category = "Output", default = 0.0)]
    distance_to_target: f32,

    #[uproperty(VisibleAnywhere, category = "Parameter", default = model::PLAYER_INFO_MAX_RANGE)]
    max_range: f32,
}

#[uclass_impl]
impl StateTreeGetPlayerInfoTask {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        task_defaults(self.as_ref())
    }

    #[ufunction(Override)]
    fn receive_latent_enter_state(&mut self, _transition: UStructRef<FStateTreeTransitionResult>) {
        let found = self.select_target();

        if let Ok(me) = self.as_ref().checked() {
            me.finish_task(Some(found));
        }
    }
}

impl StateTreeGetPlayerInfoTask {
    fn select_target(&mut self) -> bool {
        let Ok(character) = self.character().checked() else {
            return false;
        };

        let world = character.as_ref().upcast_to();
        let location = character.k2_get_actor_location().to_dvec3();

        let mut selected: Option<(UObjectRef<Character>, glam::DVec3)> = None;

        let num_players = GameplayStatics::get_num_local_player_controllers(world);

        for i in 0..num_players {
            let Ok(current) = GameplayStatics::get_player_pawn(world, i).cast::<Character>() else {
                continue;
            };

            let Ok(current_location) = current
                .checked()
                .map(|c| c.k2_get_actor_location().to_dvec3())
            else {
                continue;
            };

            if (current_location - location).length() >= f64::from(self.max_range()) {
                continue;
            }

            if selected.is_none() || KismetMathLibrary::random_bool() {
                selected = Some((current, current_location));
            }
        }

        let Some((target, target_location)) = selected else {
            self.set_target_player_character(UObjectRef::null());
            return false;
        };

        self.set_target_player_character(target);

        self.set_target_player_location(&FVector::from_dvec3(target_location));
        self.set_distance_to_target((target_location - location).length() as f32);

        true
    }
}
