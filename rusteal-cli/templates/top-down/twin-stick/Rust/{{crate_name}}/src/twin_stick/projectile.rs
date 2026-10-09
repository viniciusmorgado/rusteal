use bindings::engine::{
    Actor, ActorExt, ECollisionChannel, ECollisionEnabled, ECollisionResponse, FHitResult,
    PrimitiveComponent, PrimitiveComponentExt, ProjectileMovementComponent,
    ProjectileMovementComponentExt, SphereComponent, SphereComponentExt, StaticMeshComponent,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{FName, LOG_WARNING, RustealResult, UObjectRef, UStructRef, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::npc::TwinStickNPC;

const LIFE_SPAN: f32 = 2.0;
const COLLISION_RADIUS: f32 = 35.0;
const INITIAL_SPEED: f32 = 2000.0;
const MAX_SPEED: f32 = 15000.0;

#[uclass(parent = Actor)]
pub struct TwinStickProjectile {
    #[component(root, name = "Collision Sphere")]
    collision_sphere: SphereComponent,

    #[component(attach = "collision_sphere")]
    mesh: StaticMeshComponent,

    #[component(name = "Projectile Movement")]
    projectile_movement: ProjectileMovementComponent,
}

#[uclass_impl]
impl TwinStickProjectile {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.as_ref().checked()?.set_initial_life_span(LIFE_SPAN);

        let sphere = self.collision_sphere()?.checked()?;
        sphere.set_sphere_radius(COLLISION_RADIUS, Some(true));
        sphere.set_notify_rigid_body_collision(true);
        sphere.set_collision_enabled(ECollisionEnabled::QueryOnly);
        sphere.set_collision_object_type(ECollisionChannel::ECC_WorldDynamic);
        sphere.set_collision_response_to_all_channels(ECollisionResponse::ECR_Block);

        self.mesh()?
            .checked()?
            .set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));

        let movement = self.projectile_movement()?.checked()?;
        movement.set_initial_speed(INITIAL_SPEED);
        movement.set_max_speed(MAX_SPEED);
        movement.set_rotation_follows_velocity(true);
        movement.set_rotation_remains_vertical(true);
        movement.set_projectile_gravity_scale(0.0);
        movement.set_should_bounce(true);
        movement.set_force_sub_stepping(true);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.bind_stop() {
            ulog!(LOG_WARNING, "[TwinStick] projectile stop: {e}");
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[ufunction(Override)]
    fn receive_hit(
        &mut self,
        _my_comp: UObjectRef<PrimitiveComponent>,
        other: UObjectRef<Actor>,
        _other_comp: UObjectRef<PrimitiveComponent>,
        _b_self_moved: bool,
        _hit_location: UStructRef<FVector>,
        _hit_normal: UStructRef<FVector>,
        _normal_impulse: UStructRef<FVector>,
        _hit: UStructRef<FHitResult>,
    ) {
        if let Ok(npc) = TwinStickNPC::from_obj(other) {
            npc.projectile_impact();

            if let Ok(me) = self.as_ref().checked() {
                me.k2_destroy_actor();
            }
        }
    }
}

impl TwinStickProjectile {
    fn bind_stop(&self) -> RustealResult<()> {
        let me: UObjectRef<Actor> = self.as_ref();

        self.projectile_movement()?
            .checked()?
            .on_projectile_stop()
            .add(move |_impact_result| {
                if let Ok(projectile) = me.checked() {
                    projectile.k2_destroy_actor();
                }
            })?
            .detach();

        Ok(())
    }
}
