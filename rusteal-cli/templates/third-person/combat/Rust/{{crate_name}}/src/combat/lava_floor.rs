// CombatLavaFloor: the Combat variant's `ACombatLavaFloor` in Rust. A floor that
// kills whatever damageable lands on it.

use bindings::engine::{Actor, FHitResult, FHitResultExt, PrimitiveComponent, PrimitiveComponentExt, StaticMeshComponent};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{LOG_WARNING, UObjectRef, UStructRef, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::interfaces;
use super::model;

#[uclass(parent = Actor)]
pub struct CombatLavaFloor {
    /// Floor mesh
    #[component(root)]
    mesh: StaticMeshComponent,

    /// Amount of damage to deal on contact
    #[uproperty(EditAnywhere, default = model::LAVA_DAMAGE)]
    damage: f32,
}

#[uclass_impl]
impl CombatLavaFloor {
    /// Bind the hit handler, as the C++ constructor does.
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let me = self.as_ref();
        let bound = self.mesh().and_then(|mesh| mesh.checked()?.on_component_hit().add_ufunction(&me, "OnFloorHit"));
        if let Err(e) = bound {
            ulog!(LOG_WARNING, "[Combat] lava floor hit: {e}");
        }
    }

    /// Blocking hit handler
    #[ufunction(BlueprintCallable)]
    fn on_floor_hit(
        &mut self,
        _hit_component: UObjectRef<PrimitiveComponent>,
        other_actor: UObjectRef<Actor>,
        _other_comp: UObjectRef<PrimitiveComponent>,
        _normal_impulse: UStructRef<FVector>,
        hit: UStructRef<FHitResult>,
    ) {
        // check if the hit actor is damageable by casting to the interface
        if let Some(mut damageable) = interfaces::damageable(other_actor) {
            // damage the actor
            let impact_point = hit.get_impact_point().to_dvec3();
            damageable.apply_damage(self.damage(), self.as_ref().upcast_to(), impact_point, DVec3::ZERO);
        }
    }
}
