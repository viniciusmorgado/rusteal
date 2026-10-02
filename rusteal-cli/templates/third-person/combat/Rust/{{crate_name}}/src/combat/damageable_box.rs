// CombatDamageableBox: the Combat variant's `ACombatDamageableBox` in Rust. A
// physics box that takes hits and breaks; its Blueprint child
// `BP_CombatDamageableBox` plays the effects (`OnBoxDamaged`, `OnBoxDestroyed`).

use bindings::engine::{
    Actor, ActorComponentExt, ActorExt, ECollisionChannel, KismetSystemLibrary, PrimitiveComponentExt, StaticMeshComponent,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{FName, OwnedStruct, RustealResult, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::interfaces::CombatDamageable;
use super::model;

#[uclass(parent = Actor)]
pub struct CombatDamageableBox {
    /// Damageable box mesh
    #[component(root)]
    mesh: StaticMeshComponent,

    /// Amount of HP this box starts with.
    #[uproperty(EditAnywhere, default = model::BOX_HP, name = "CurrentHP")]
    current_hp: f32,

    /// Time to wait before we remove this box from the level
    #[uproperty(EditAnywhere, default = model::BOX_DEATH_DELAY_TIME)]
    death_delay_time: f32,
}

#[uclass_impl]
impl CombatDamageableBox {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let mesh = self.mesh()?.checked()?;
        // set the collision properties
        mesh.set_collision_profile_name(FName::new("BlockAllDynamic").handle(), Some(true));
        // enable physics
        mesh.set_simulate_physics(true);
        // disable navigation relevance so boxes don't affect NavMesh generation
        mesh.set_can_ever_affect_navigation(false);
        Ok(())
    }

    /// Cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the death timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "RemoveFromLevel");
    }

    /// Timer callback to remove the box from the level after it dies
    #[ufunction(BlueprintCallable)]
    fn remove_from_level(&mut self) {
        // destroy this actor
        if let Ok(me) = self.as_ref().checked() {
            me.k2_destroy_actor();
        }
    }

    /// Blueprint handler to play damage effects
    #[ufunction(BlueprintImplementableEvent)]
    fn on_box_damaged(&self, damage_location: &OwnedStruct<FVector>, damage_impulse: &OwnedStruct<FVector>) {}

    /// Blueprint handler to play destruction effects
    #[ufunction(BlueprintImplementableEvent)]
    fn on_box_destroyed(&self) {}
}

impl CombatDamageable for CombatDamageableBox {
    /// Handles damage and knockback events
    fn apply_damage(&mut self, damage: f32, _damage_causer: UObjectRef<Actor>, damage_location: DVec3, damage_impulse: DVec3) {
        // only process damage if we still have HP
        if self.current_hp() <= 0.0 {
            return;
        }
        // apply the damage
        self.set_current_hp(self.current_hp() - damage);
        // are we dead?
        if self.current_hp() <= 0.0 {
            self.handle_death();
        }
        // apply a physics impulse to the box, ignoring its mass
        if let Ok(mesh) = self.mesh().and_then(|m| m.checked()) {
            let impulse = damage_impulse * f64::from(mesh.get_mass());
            mesh.add_impulse_at_location(&FVector::from_dvec3(impulse), &FVector::from_dvec3(damage_location), None);
        }
        // call the BP handler to play effects, etc.
        self.on_box_damaged(&FVector::from_dvec3(damage_location), &FVector::from_dvec3(damage_impulse));
    }

    /// Handles death events
    fn handle_death(&mut self) {
        // change the collision object type to Visibility so we ignore most interactions but still retain physics collisions
        if let Ok(mesh) = self.mesh().and_then(|m| m.checked()) {
            mesh.set_collision_object_type(ECollisionChannel::ECC_Visibility);
        }
        // call the BP handler to play effects, etc.
        self.on_box_destroyed();
        // set up the death cleanup timer
        KismetSystemLibrary::k2_set_timer(
            self.as_ref().upcast_to(),
            "RemoveFromLevel",
            self.death_delay_time(),
            false,
            None,
            None,
            None,
        );
    }

    fn apply_healing(&mut self, _healing: f32, _healer: UObjectRef<Actor>) {
        // stub
    }

    fn notify_danger(&mut self, _danger_location: DVec3, _danger_source: UObjectRef<Actor>) {
        // stub
    }
}
