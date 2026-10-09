use bindings::engine::{
    Actor, ActorExt, ECollisionChannel, ECollisionEnabled, ECollisionResponse, FDataTableRowHandle,
    KismetSystemLibrary, PrimitiveComponentExt, SceneComponent, SceneComponentExt, SphereComponent,
    StaticMesh, StaticMeshComponent, StaticMeshComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{
    FName, LOG_WARNING, OwnedStruct, RustealResult, SoftObjectRef, SubclassOf, UObjectRef, ulog,
};
use rusteal_runtime::{uclass, uclass_impl, ustruct};

use super::weapon::ShooterWeapon;
use super::weapon_holder::weapon_holder;

const SPHERE_COLLISION_HEIGHT: f64 = 84.0;

#[ustruct]
pub struct WeaponTableRow {
    #[uproperty(EditAnywhere)]
    static_mesh: SoftObjectRef<StaticMesh>,

    #[uproperty(EditAnywhere)]
    weapon_to_spawn: SubclassOf<ShooterWeapon>,
}

#[uclass(parent = Actor)]
pub struct ShooterPickup {
    #[component(root, name = "Root")]
    root: SceneComponent,

    #[component(attach = "root", name = "Sphere Collision")]
    sphere_collision: SphereComponent,

    #[component(attach = "sphere_collision", name = "Mesh")]
    mesh: StaticMeshComponent,

    #[uproperty(EditAnywhere, category = "Pickup")]
    weapon_type: OwnedStruct<FDataTableRowHandle>,

    #[uproperty]
    weapon_class: SubclassOf<ShooterWeapon>,

    #[uproperty(EditAnywhere, category = "Pickup", default = 4.0)]
    respawn_time: f32,
}

#[uclass_impl]
impl ShooterPickup {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let sphere = self.sphere_collision()?.checked()?;

        sphere.k2_set_relative_location(
            &FVector::from_dvec3(glam::DVec3::new(0.0, 0.0, SPHERE_COLLISION_HEIGHT)),
            false,
            false,
        );

        sphere.set_collision_enabled(ECollisionEnabled::QueryOnly);
        sphere.set_collision_object_type(ECollisionChannel::ECC_WorldStatic);
        sphere.set_collision_response_to_all_channels(ECollisionResponse::ECR_Ignore);

        sphere.set_collision_response_to_channel(
            ECollisionChannel::ECC_Pawn,
            ECollisionResponse::ECR_Overlap,
        );

        sphere.set_fill_collision_underneath_for_navmesh(true);

        self.mesh()?
            .checked()?
            .set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let weapon_type = self.weapon_type();

        let Some(weapon_data) = weapon_type.get_row::<WeaponTableRow>() else {
            ulog!(LOG_WARNING, "[Shooter] pickup without a weapon type");
            return;
        };

        match weapon_data.static_mesh().load_synchronous() {
            Ok(static_mesh) => {
                if let Ok(mesh) = self.mesh().and_then(|m| m.checked()) {
                    mesh.set_static_mesh(static_mesh);
                }
            }
            Err(e) => ulog!(LOG_WARNING, "[Shooter] pickup mesh: {e}"),
        }

        self.set_weapon_class(weapon_data.weapon_to_spawn());
    }

    #[ufunction(Override)]
    fn receive_end_play(&mut self, _end_play_reason: u8) {
        KismetSystemLibrary::k2_clear_timer(self.as_ref().upcast_to(), "RespawnPickup");
    }

    #[ufunction(Override)]
    fn receive_actor_begin_overlap(&mut self, other_actor: UObjectRef<Actor>) {
        let Some(mut weapon_holder) = weapon_holder(other_actor) else {
            return;
        };

        weapon_holder.add_weapon_class(self.weapon_class());

        let Ok(me) = self.as_ref().checked() else {
            return;
        };

        me.set_actor_hidden_in_game(true);

        me.set_actor_enable_collision(false);

        me.set_actor_tick_enabled(false);

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

    #[ufunction]
    fn respawn_pickup(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.set_actor_hidden_in_game(false);
        }

        self.bp_on_respawn();
    }

    #[ufunction(BlueprintImplementableEvent, name = "BP_OnRespawn")]
    fn bp_on_respawn(&self) {}

    #[ufunction(BlueprintCallable)]
    fn finish_respawn(&mut self) {
        if let Ok(me) = self.as_ref().checked() {
            me.set_actor_enable_collision(true);

            me.set_actor_tick_enabled(true);
        }
    }
}
