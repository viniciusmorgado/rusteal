use bindings::engine::{
    Actor, BoxComponent, ECollisionChannel, ECollisionEnabled, ECollisionResponse, FHitResult,
    PrimitiveComponent, PrimitiveComponentExt, SceneComponent, SceneComponentExt,
    StaticMeshComponent,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{LOG_WARNING, RustealResult, UObjectRef, UStructRef, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::SideScrollingCharacter;
use super::model;

#[uclass(parent = Actor)]
pub struct SideScrollingSoftPlatform {
    #[component(root)]
    root: SceneComponent,

    #[component(attach = "root")]
    mesh: StaticMeshComponent,

    #[component(attach = "mesh", name = "Collision Check Box")]
    collision_check_box: BoxComponent,
}

#[uclass_impl]
impl SideScrollingSoftPlatform {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let mesh = self.mesh()?.checked()?;
        mesh.set_collision_enabled(ECollisionEnabled::QueryAndPhysics);
        mesh.set_collision_object_type(ECollisionChannel::ECC_WorldStatic);
        mesh.set_collision_response_to_all_channels(ECollisionResponse::ECR_Block);

        let check_box = self.collision_check_box()?.checked()?;

        check_box.k2_set_relative_location(
            &FVector::from_dvec3(model::SOFT_PLATFORM_CHECK_BOX_LOCATION),
            false,
            false,
        );

        check_box.set_collision_enabled(ECollisionEnabled::QueryOnly);
        check_box.set_collision_object_type(ECollisionChannel::ECC_WorldDynamic);
        check_box.set_collision_response_to_all_channels(ECollisionResponse::ECR_Ignore);

        check_box.set_collision_response_to_channel(
            ECollisionChannel::ECC_Pawn,
            ECollisionResponse::ECR_Overlap,
        );

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let me = self.as_ref();

        let bound = self.collision_check_box().and_then(|check_box| {
            check_box
                .checked()?
                .on_component_begin_overlap()
                .add_ufunction(&me, "OnSoftCollisionOverlap")
        });

        if let Err(e) = bound {
            ulog!(LOG_WARNING, "[SideScrolling] soft platform overlap: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    #[allow(clippy::too_many_arguments)]
    fn on_soft_collision_overlap(
        &mut self,
        _overlapped_component: UObjectRef<PrimitiveComponent>,
        other_actor: UObjectRef<Actor>,
        _other_comp: UObjectRef<PrimitiveComponent>,
        _other_body_index: i32,
        _b_from_sweep: bool,
        _sweep_result: UStructRef<FHitResult>,
    ) {
        if let Ok(character) = SideScrollingCharacter::from_obj(other_actor) {
            let _ = character.set_soft_collision(true);
        }
    }

    #[ufunction(Override)]
    fn receive_actor_end_overlap(&mut self, other_actor: UObjectRef<Actor>) {
        if let Ok(character) = SideScrollingCharacter::from_obj(other_actor) {
            let _ = character.set_soft_collision(false);
        }
    }
}
