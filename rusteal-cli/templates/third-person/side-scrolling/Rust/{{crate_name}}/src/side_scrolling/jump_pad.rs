// SideScrollingJumpPad: the SideScrolling variant's `ASideScrollingJumpPad` in
// Rust. Launches characters that step on it upwards.

use bindings::engine::{
    Actor, ActorExt, BoxComponent, BoxComponentExt, Character, CharacterExt, ECollisionChannel,
    ECollisionEnabled, ECollisionResponse, PrimitiveComponentExt, SceneComponent,
    SceneComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{LOG_WARNING, RustealResult, UObjectRef, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::model;

#[uclass(parent = Actor)]
pub struct SideScrollingJumpPad {
    #[component(root)]
    root: SceneComponent,

    /// Jump pad bounding box
    #[component(attach = "root")]
    r#box: BoxComponent,

    /// Vertical velocity to set the character to when they use the jump pad
    #[uproperty(EditAnywhere, default = model::JUMP_PAD_Z_STRENGTH)]
    z_strength: f32,
}

#[uclass_impl]
impl SideScrollingJumpPad {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        // configure the bounding box
        let r#box = self.r#box()?.checked()?;
        r#box.set_box_extent(&FVector::from_dvec3(model::JUMP_PAD_BOX_EXTENT), Some(false));
        r#box.k2_set_relative_location(&FVector::from_dvec3(model::JUMP_PAD_BOX_LOCATION), false, false);
        r#box.set_collision_object_type(ECollisionChannel::ECC_WorldDynamic);
        r#box.set_collision_enabled(ECollisionEnabled::QueryOnly);
        r#box.set_collision_response_to_all_channels(ECollisionResponse::ECR_Ignore);
        r#box.set_collision_response_to_channel(ECollisionChannel::ECC_Pawn, ECollisionResponse::ECR_Overlap);
        Ok(())
    }

    /// Add the overlap handler, as the C++ constructor does.
    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        let me = self.as_ref();
        if let Err(e) = me.checked().and_then(|actor| actor.on_actor_begin_overlap().add_ufunction(&me, "BeginOverlap")) {
            ulog!(LOG_WARNING, "[SideScrolling] jump pad overlap: {e}");
        }
    }

    /// Handles the jump pad overlap
    #[ufunction(BlueprintCallable)]
    fn begin_overlap(&mut self, _overlapped_actor: UObjectRef<Actor>, other_actor: UObjectRef<Actor>) {
        // were we overlapped by a character?
        let Ok(overlapping_character) = other_actor.cast::<Character>().and_then(|c| c.checked()) else {
            return;
        };
        // force the character to jump
        overlapping_character.jump();
        // launch the character to override its vertical velocity
        let launch_velocity = glam::DVec3::Z * f64::from(self.z_strength());
        overlapping_character.launch_character(&FVector::from_dvec3(launch_velocity), false, true);
    }
}
