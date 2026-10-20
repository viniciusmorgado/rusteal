use std::time::Duration;

use bindings::engine::{
    Actor, ActorComponentExt, ActorExt, ECollisionChannel, PrimitiveComponentExt,
    StaticMeshComponent,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::timer::{self, TimerHandle};
use rusteal_runtime::runtime::{FName, OwnedStruct, RustealResult, UObjectRef, method};
use rusteal_runtime::{uclass, uclass_impl};

use super::interfaces::CombatDamageable;
use super::model;

#[uclass(parent = Actor)]
pub struct CombatDamageableBox {
    #[component(root)]
    mesh: StaticMeshComponent,

    #[uproperty(EditAnywhere, default = model::BOX_HP, name = "CurrentHP")]
    current_hp: f32,

    #[uproperty(EditAnywhere, default = model::BOX_DEATH_DELAY_TIME)]
    death_delay_time: f32,

    removal_timer: Option<TimerHandle>,
}

#[uclass_impl]
impl CombatDamageableBox {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let mesh = self.mesh()?.checked()?;

        mesh.set_collision_profile_name(FName::new("BlockAllDynamic").handle(), Some(true));

        mesh.set_simulate_physics(true);

        mesh.set_can_ever_affect_navigation(false);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        if let Some(removal_timer) = self.removal_timer() {
            removal_timer.clear();
        }
    }

    #[ufunction(BlueprintCallable)]
    fn remove_from_level(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }

    #[ufunction(BlueprintImplementableEvent)]
    fn on_box_damaged(
        &self,
        damage_location: &OwnedStruct<FVector>,
        damage_impulse: &OwnedStruct<FVector>,
    ) {
    }

    #[ufunction(BlueprintImplementableEvent)]
    fn on_box_destroyed(&self) {}
}

impl CombatDamageable for CombatDamageableBox {
    fn apply_damage(
        &mut self,
        damage: f32,
        _damage_causer: UObjectRef<Actor>,
        damage_location: DVec3,
        damage_impulse: DVec3,
    ) {
        if self.current_hp() <= 0.0 {
            return;
        }

        self.set_current_hp(self.current_hp() - damage);

        if self.current_hp() <= 0.0 {
            self.handle_death();
        }

        if let Ok(mesh) = self.mesh().and_then(|m| m.checked()) {
            let impulse = damage_impulse * f64::from(mesh.get_mass());

            mesh.add_impulse_at_location(
                &FVector::from_dvec3(impulse),
                &FVector::from_dvec3(damage_location),
                None,
            );
        }

        self.on_box_damaged(
            &FVector::from_dvec3(damage_location),
            &FVector::from_dvec3(damage_impulse),
        );
    }

    fn handle_death(&mut self) {
        if let Ok(mesh) = self.mesh().and_then(|m| m.checked()) {
            mesh.set_collision_object_type(ECollisionChannel::ECC_Visibility);
        }

        self.on_box_destroyed();

        if let Some(removal_timer) = self.removal_timer() {
            removal_timer.clear();
        }

        let removal_timer = timer::set_timer(
            &self.as_ref(),
            Duration::from_secs_f32(self.death_delay_time()),
            false,
            method(self, Self::remove_from_level),
        )
        .inspect_err(|e| log::warn!("[Combat] box removal timer: {e}"))
        .ok();

        self.set_removal_timer(removal_timer);
    }

    fn apply_healing(&mut self, _healing: f32, _healer: UObjectRef<Actor>) {}

    fn notify_danger(&mut self, _danger_location: DVec3, _danger_source: UObjectRef<Actor>) {}
}
