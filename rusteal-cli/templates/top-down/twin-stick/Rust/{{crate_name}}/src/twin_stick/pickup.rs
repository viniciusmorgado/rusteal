// TwinStickPickup: the TwinStick variant's `ATwinStickPickup` in Rust, an
// item an NPC drops: the player character picks it up by touching it, for
// one more area attack.

use bindings::engine::{
    Actor, ActorExt, ECollisionChannel, ECollisionEnabled, ECollisionResponse,
    PrimitiveComponentExt, SceneComponent, SceneComponentExt, SphereComponent, SphereComponentExt,
    StaticMeshComponent,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{FName, RustealResult, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::TwinStickCharacter;

/// `CollisionSphere->SetSphereRadius(100.0f)`, 125 above the root.
const COLLISION_RADIUS: f32 = 100.0;
const COLLISION_HEIGHT: f64 = 125.0;

#[uclass(parent = Actor)]
pub struct TwinStickPickup {
    #[component(root, name = "Root")]
    root: SceneComponent,

    /// Pickup collision sphere
    #[component(attach = "root", name = "Collision Sphere")]
    collision_sphere: SphereComponent,

    /// Provides visual representation for the pickup
    #[component(attach = "collision_sphere")]
    mesh: StaticMeshComponent,
}

#[uclass_impl]
impl TwinStickPickup {
    /// Everything `ATwinStickPickup::ATwinStickPickup()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // create the collision sphere
        let sphere = self.collision_sphere()?.checked()?;
        sphere.set_sphere_radius(COLLISION_RADIUS, Some(true));
        sphere.k2_set_relative_location(
            &FVector::from_dvec3(glam::DVec3::new(0.0, 0.0, COLLISION_HEIGHT)),
            false,
            false,
        );
        sphere.set_collision_enabled(ECollisionEnabled::QueryOnly);
        sphere.set_collision_object_type(ECollisionChannel::ECC_WorldDynamic);
        sphere.set_collision_response_to_all_channels(ECollisionResponse::ECR_Ignore);
        sphere.set_collision_response_to_channel(ECollisionChannel::ECC_Pawn, ECollisionResponse::ECR_Overlap);

        // create the mesh
        self.mesh()?
            .checked()?
            .set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));
        Ok(())
    }

    /// Collision handling
    #[ufunction(Override)]
    fn receive_actor_begin_overlap(&mut self, other_actor: UObjectRef<Actor>) {
        // have we overlapped the player character?
        if let Ok(player_character) = TwinStickCharacter::from_obj(other_actor) {
            // give the pickup to the player
            player_character.add_pickup();

            // destroy this pickup
            if let Ok(me) = self.as_ref().checked() {
                me.k2_destroy_actor();
            }
        }
    }
}
