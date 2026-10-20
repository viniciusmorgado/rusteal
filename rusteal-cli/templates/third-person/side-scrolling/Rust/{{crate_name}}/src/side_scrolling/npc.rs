use std::time::Duration;

use bindings::engine::{
    Actor, ActorExt, Character, CharacterExt, CharacterMovementComponentExt, MovementComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::timer::{self, TimerHandle};
use rusteal_runtime::runtime::{RustealResult, UObjectRef, method};
use rusteal_runtime::{uclass, uclass_impl};

use super::interactable::Interactable;
use super::model;

#[uclass(parent = Character)]
pub struct SideScrollingNPC {
    #[uproperty(EditAnywhere, default = model::NPC_LAUNCH_IMPULSE)]
    launch_impulse: f32,

    #[uproperty(EditAnywhere, default = model::NPC_LAUNCH_VERTICAL_IMPULSE)]
    launch_vertical_impulse: f32,

    #[uproperty(EditAnywhere, default = model::NPC_DEACTIVATION_TIME)]
    deactivation_time: f32,

    #[uproperty(VisibleAnywhere, BlueprintReadOnly, default = false)]
    b_deactivated: bool,

    deactivation_timer: Option<TimerHandle>,
}

#[uclass_impl]
impl SideScrollingNPC {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        me.get_character_movement()
            .checked()?
            .set_max_walk_speed(model::NPC_MAX_WALK_SPEED);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        if let Some(deactivation_timer) = self.deactivation_timer() {
            deactivation_timer.clear();
        }
    }

    #[ufunction(BlueprintCallable)]
    fn reset_deactivation(&mut self) {
        self.set_b_deactivated(false);
    }
}

impl SideScrollingNPC {
    fn launch(&mut self, interactor: UObjectRef<Actor>) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        me.get_character_movement()
            .checked()?
            .stop_movement_immediately();

        let forward = interactor.checked()?.get_actor_forward_vector().to_dvec3();

        let launch = model::npc_launch(
            forward,
            self.launch_impulse(),
            self.launch_vertical_impulse(),
        );

        me.launch_character(&FVector::from_dvec3(launch), true, true);

        if let Some(deactivation_timer) = self.deactivation_timer() {
            deactivation_timer.clear();
        }

        let deactivation_timer = timer::set_timer(
            &self.as_ref(),
            Duration::from_secs_f32(self.deactivation_time()),
            false,
            method(self, Self::reset_deactivation),
        )?;

        self.set_deactivation_timer(Some(deactivation_timer));

        Ok(())
    }
}

impl Interactable for SideScrollingNPC {
    fn interaction(&mut self, interactor: UObjectRef<Actor>) {
        if self.b_deactivated() {
            return;
        }

        self.set_b_deactivated(true);
        let _ = self.launch(interactor);
    }
}
