use bindings::engine::{
    Actor, ActorExt, Character, CharacterExt, CharacterMovementComponentExt, KismetSystemLibrary,
    MovementComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{RustealResult, UObjectRef};
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
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "ResetDeactivation");
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

        KismetSystemLibrary::k2_set_timer(
            self.as_ref().upcast_to(),
            "ResetDeactivation",
            self.deactivation_time(),
            false,
            None,
            None,
            None,
        );

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
