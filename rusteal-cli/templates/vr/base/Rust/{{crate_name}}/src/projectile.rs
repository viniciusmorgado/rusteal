use bindings::engine::{
    Actor, ActorExt, EAttachmentRule, ECollisionChannel, ECollisionEnabled, ECollisionResponse,
    FHitResult, FHitResultExt, MovementComponentExt, PrimitiveComponent, PrimitiveComponentExt,
    ProjectileMovementComponent, ProjectileMovementComponentExt, SceneComponentExt,
    SphereComponent, SphereComponentExt, StaticMeshComponent,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, OwnedStruct, Rotator, RustealResult, UObjectRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = Actor)]
pub struct Projectile {
    #[component(root)]
    sphere_collision: SphereComponent,

    #[component(attach = "sphere_collision")]
    static_mesh: StaticMeshComponent,

    #[component]
    projectile_movement: ProjectileMovementComponent,
}

#[uclass_impl]
impl Projectile {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        self.as_ref().checked()?.set_initial_life_span(10.0);

        let sphere = self.sphere_collision()?.checked()?;
        sphere.set_sphere_radius(1.1, Some(true));
        sphere.set_collision_profile_name(FName::new("BlockAllDynamic").handle(), Some(true));

        for channel in [
            ECollisionChannel::ECC_WorldDynamic,
            ECollisionChannel::ECC_Pawn,
            ECollisionChannel::ECC_Visibility,
            ECollisionChannel::ECC_Camera,
            ECollisionChannel::ECC_PhysicsBody,
            ECollisionChannel::ECC_Vehicle,
            ECollisionChannel::ECC_Destructible,
        ] {
            sphere.set_collision_response_to_channel(channel, ECollisionResponse::ECR_Overlap);
        }

        let mesh = self.static_mesh()?.checked()?;
        mesh.set_generate_overlap_events(false);
        mesh.set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));
        mesh.k2_set_relative_location_and_rotation(
            &FVector::from_dvec3(DVec3::new(-1.471127, 0.0, 0.0)),
            &FRotator::from_rotator(Rotator::new(-90.0, 0.0, 0.0)),
            false,
            false,
        );
        mesh.set_relative_scale3_d(&FVector::from_dvec3(DVec3::new(0.022125, 0.022125, 0.0708)));

        let movement = self.projectile_movement()?.checked()?;
        movement.set_rotation_follows_velocity(true);
        movement.set_velocity(&FVector::from_dvec3(DVec3::new(1600.0, 0.0, 0.0)));

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let me = self.as_ref();

        let bound = self
            .sphere_collision()
            .and_then(|sphere| sphere.checked())
            .and_then(|sphere| {
                sphere.on_component_begin_overlap().add(
                    move |_overlapped,
                          _other_actor,
                          other_comp,
                          _body,
                          _from_sweep,
                          sweep_result| {
                        if let Ok(projectile) = Projectile::from_obj(me)
                            && let Err(e) = projectile.hit(other_comp, &sweep_result)
                        {
                            ulog!(LOG_WARNING, "[VR] projectile hit failed: {e}");
                        }
                    },
                )
            });

        if let Ok(binding) = bound {
            binding.detach();
        }

        let stopped = self
            .projectile_movement()
            .and_then(|movement| movement.checked())
            .and_then(|movement| {
                movement.on_projectile_stop().add(move |_impact| {
                    if let Ok(projectile) = Projectile::from_obj(me)
                        && let Ok(sphere) = projectile
                            .sphere_collision()
                            .and_then(|sphere| sphere.checked())
                    {
                        sphere.set_collision_enabled(ECollisionEnabled::NoCollision);
                    }
                })
            });

        if let Ok(binding) = stopped {
            binding.detach();
        }
    }
}

impl Projectile {
    fn hit(
        &self,
        other: UObjectRef<PrimitiveComponent>,
        sweep: &OwnedStruct<FHitResult>,
    ) -> RustealResult<()> {
        let other_component = other.checked()?;
        let movement = self.projectile_movement()?.checked()?;

        if other_component.is_simulating_physics(None) {
            other_component.add_impulse(&movement.get_velocity(), None, Some(false));
        }

        movement.stop_simulating(sweep);

        let me = self.as_ref().checked()?;
        me.k2_attach_to_component(
            other.upcast_to(),
            FName::NONE.handle(),
            EAttachmentRule::KeepWorld,
            EAttachmentRule::KeepWorld,
            EAttachmentRule::KeepWorld,
            true,
        );

        me.k2_set_actor_location(
            &FVector::from_dvec3(sweep.as_ref().get_impact_point().to_dvec3()),
            false,
            false,
        );
        Ok(())
    }
}
