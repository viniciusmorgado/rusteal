// ShooterPickup: the Shooter variant's `AShooterPickup` in Rust, a weapon
// pickup that grants a weapon to whoever holds weapons and respawns after a
// while. Which weapon, and the mesh it shows, is a row of the weapon data
// table (`DT_WeaponData`), whose rows are `WeaponTableRow`s.
//
// `OnConstruction` sets the mesh in C++; the Blueprint child's construction
// script would hide a Rust one, so the mesh is set when play begins (a placed
// pickup keeps the one the editor saved with it).

use bindings::engine::{
    Actor, ActorExt, ECollisionChannel, ECollisionEnabled, ECollisionResponse,
    FDataTableRowHandle, KismetSystemLibrary, PrimitiveComponentExt, SceneComponent,
    SceneComponentExt, SphereComponent, StaticMesh, StaticMeshComponent,
    StaticMeshComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, OwnedStruct, RustealResult, SoftObjectRef, SubclassOf, UObjectRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl, ustruct};

use super::weapon::ShooterWeapon;
use super::weapon_holder::weapon_holder;

/// `SphereCollision->SetRelativeLocation(FVector(0.0f, 0.0f, 84.0f))`.
const SPHERE_COLLISION_HEIGHT: f64 = 84.0;

/// Holds information about a type of weapon pickup
#[ustruct]
pub struct WeaponTableRow {
    /// Mesh to display on the pickup
    #[uproperty(EditAnywhere)]
    static_mesh: SoftObjectRef<StaticMesh>,

    /// Weapon class to grant on pickup
    #[uproperty(EditAnywhere)]
    weapon_to_spawn: SubclassOf<ShooterWeapon>,
}

#[uclass(parent = Actor)]
pub struct ShooterPickup {
    #[component(root, name = "Root")]
    root: SceneComponent,

    /// Collision sphere
    #[component(attach = "root", name = "Sphere Collision")]
    sphere_collision: SphereComponent,

    /// Weapon pickup mesh. Its mesh asset is set from the weapon data table
    #[component(attach = "sphere_collision", name = "Mesh")]
    mesh: StaticMeshComponent,

    /// Data on the type of picked weapon and visuals of this pickup
    #[uproperty(EditAnywhere, category = "Pickup")]
    weapon_type: OwnedStruct<FDataTableRowHandle>,

    /// Type to weapon to grant on pickup. Set from the weapon data table.
    #[uproperty]
    weapon_class: SubclassOf<ShooterWeapon>,

    /// Time to wait before respawning this pickup
    #[uproperty(EditAnywhere, category = "Pickup", default = 4.0)]
    respawn_time: f32,
}

#[uclass_impl]
impl ShooterPickup {
    /// Everything `AShooterPickup::AShooterPickup()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // create the collision sphere
        let sphere = self.sphere_collision()?.checked()?;
        sphere.k2_set_relative_location(
            &FVector::from_dvec3(glam::DVec3::new(0.0, 0.0, SPHERE_COLLISION_HEIGHT)),
            false,
            false,
        );
        sphere.set_collision_enabled(ECollisionEnabled::QueryOnly);
        sphere.set_collision_object_type(ECollisionChannel::ECC_WorldStatic);
        sphere.set_collision_response_to_all_channels(ECollisionResponse::ECR_Ignore);
        sphere.set_collision_response_to_channel(ECollisionChannel::ECC_Pawn, ECollisionResponse::ECR_Overlap);
        sphere.set_fill_collision_underneath_for_navmesh(true);

        // create the mesh
        self.mesh()?
            .checked()?
            .set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));
        Ok(())
    }

    /// Gameplay Initialization
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let weapon_type = self.weapon_type();
        let Some(weapon_data) = weapon_type.get_row::<WeaponTableRow>() else {
            ulog!(LOG_WARNING, "[Shooter] pickup without a weapon type");
            return;
        };

        // set the mesh
        match weapon_data.static_mesh().load_synchronous() {
            Ok(static_mesh) => {
                if let Ok(mesh) = self.mesh().and_then(|m| m.checked()) {
                    mesh.set_static_mesh(static_mesh);
                }
            }
            Err(e) => ulog!(LOG_WARNING, "[Shooter] pickup mesh: {e}"),
        }

        // copy the weapon class
        self.set_weapon_class(weapon_data.weapon_to_spawn());
    }

    /// Gameplay cleanup
    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        // clear the respawn timer
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "RespawnPickup");
    }

    /// Handles collision overlap
    #[ufunction(Override)]
    fn receive_actor_begin_overlap(&mut self, other_actor: UObjectRef<Actor>) {
        // have we collided against a weapon holder?
        let Some(mut weapon_holder) = weapon_holder(other_actor) else {
            return;
        };
        weapon_holder.add_weapon_class(self.weapon_class());

        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        // hide this mesh
        me.set_actor_hidden_in_game(true);

        // disable collision
        me.set_actor_enable_collision(false);

        // disable ticking
        me.set_actor_tick_enabled(false);

        // schedule the respawn
        KismetSystemLibrary::k2_set_timer(
            me.as_ref().upcast_to(),
            "RespawnPickup",
            self.respawn_time(),
            false,
            None,
            None,
            None,
        );
    }

    /// Called when it's time to respawn this pickup
    #[ufunction]
    fn respawn_pickup(&mut self) {
        // unhide this pickup
        if let Ok(me) = self.as_ref().checked() {
            me.set_actor_hidden_in_game(false);
        }

        // call the BP handler
        self.bp_on_respawn();
    }

    /// Passes control to Blueprint to animate the pickup respawn. Should end by calling FinishRespawn
    #[ufunction(BlueprintImplementableEvent, name = "BP_OnRespawn")]
    fn bp_on_respawn(&self) {}

    /// Enables this pickup after respawning
    #[ufunction(BlueprintCallable)]
    fn finish_respawn(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            // enable collision
            me.set_actor_enable_collision(true);

            // enable tick
            me.set_actor_tick_enabled(true);
        }
    }
}
