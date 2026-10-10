use bindings::engine::{
    Actor, PhysicsConstraintComponent, PhysicsConstraintComponentExt, PrimitiveComponentExt,
    SceneComponent, StaticMeshComponent,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{FName, OwnedStruct, RustealResult, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::interfaces::CombatDamageable;
use super::model;

#[uclass(parent = Actor)]
pub struct CombatDummy {
    #[component(root)]
    root: SceneComponent,

    #[component(attach = "root", name = "Base Plate")]
    base_plate: StaticMeshComponent,

    #[component(attach = "root")]
    dummy: StaticMeshComponent,

    #[component(attach = "root", name = "Physics Constraint")]
    physics_constraint: PhysicsConstraintComponent,
}

#[uclass_impl]
impl CombatDummy {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.dummy()?.checked()?.set_simulate_physics(true);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let (Ok(constraint), Ok(base_plate), Ok(dummy)) = (
            self.physics_constraint().and_then(|c| c.checked()),
            self.base_plate(),
            self.dummy(),
        ) {
            constraint.set_constrained_components(
                base_plate.upcast_to(),
                FName::NONE.handle(),
                dummy.upcast_to(),
                FName::NONE.handle(),
            );
        }
    }

    #[ufunction(BlueprintImplementableEvent, name = "BP_OnDummyDamaged")]
    fn bp_on_dummy_damaged(
        &self,
        location: &OwnedStruct<FVector>,
        direction: &OwnedStruct<FVector>,
    ) {
    }
}

impl CombatDamageable for CombatDummy {
    fn apply_damage(
        &mut self,
        _damage: f32,
        _damage_causer: UObjectRef<Actor>,
        damage_location: DVec3,
        damage_impulse: DVec3,
    ) {
        if let Ok(dummy) = self.dummy().and_then(|d| d.checked()) {
            dummy.add_impulse_at_location(
                &FVector::from_dvec3(damage_impulse),
                &FVector::from_dvec3(damage_location),
                None,
            );
        }

        self.bp_on_dummy_damaged(
            &FVector::from_dvec3(damage_location),
            &FVector::from_dvec3(model::safe_normal(damage_impulse)),
        );
    }

    fn handle_death(&mut self) {}

    fn apply_healing(&mut self, _healing: f32, _healer: UObjectRef<Actor>) {}

    fn notify_danger(&mut self, _danger_location: DVec3, _danger_source: UObjectRef<Actor>) {}
}
