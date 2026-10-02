// The Combat variant's trigger volumes in Rust: `ACombatActivationVolume`,
// which activates actors (an enemy spawner) when the player enters it, and
// `ACombatCheckpointVolume`, which makes the player respawn there.

use bindings::engine::{
    Actor, ActorExt, BoxComponent, BoxComponentExt, Character, FHitResult, PawnExt,
    PrimitiveComponent, PrimitiveComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{FName, LOG_WARNING, RustealResult, UObjectRef, UStructRef, UeArray, ulog};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::CombatCharacter;
use super::interfaces;
use super::model;
use super::player_controller::CombatPlayerController;

/// `Box->SetBoxExtent(500)`, `OverlapAllDynamic`: what both volumes' constructors set.
fn configure_box(volume_box: UObjectRef<BoxComponent>) -> RustealResult<()> {
    let volume_box = volume_box.checked()?;
    volume_box.set_box_extent(&FVector::from_dvec3(model::VOLUME_EXTENT), Some(true));
    volume_box.set_collision_profile_name(FName::new("OverlapAllDynamic").handle(), Some(true));
    Ok(())
}

/// Bind the box's begin overlap to the volume's `OnOverlap`, as the C++
/// constructors do.
fn bind_overlap(volume: UObjectRef<Actor>, volume_box: RustealResult<UObjectRef<BoxComponent>>) {
    let bound = volume_box.and_then(|b| b.checked()?.on_component_begin_overlap().add_ufunction(&volume, "OnOverlap"));
    if let Err(e) = bound {
        ulog!(LOG_WARNING, "[Combat] volume overlap: {e}");
    }
}

#[uclass(parent = Actor)]
pub struct CombatActivationVolume {
    /// Collision box volume
    #[component(root)]
    r#box: BoxComponent,

    /// List of actors to activate when this volume is entered
    #[uproperty(EditAnywhere)]
    actors_to_activate: UeArray<UObjectRef<Actor>>,
}

#[uclass_impl]
impl CombatActivationVolume {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        configure_box(self.r#box()?)
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        bind_overlap(self.as_ref().upcast_to(), self.r#box());
    }

    /// Handles overlaps with the box volume
    #[ufunction(BlueprintCallable)]
    #[allow(clippy::too_many_arguments)]
    fn on_overlap(
        &mut self,
        _overlapped_component: UObjectRef<PrimitiveComponent>,
        other_actor: UObjectRef<Actor>,
        _other_comp: UObjectRef<PrimitiveComponent>,
        _other_body_index: i32,
        _b_from_sweep: bool,
        _sweep_result: UStructRef<FHitResult>,
    ) {
        // has a Character entered the volume? is the Character controlled by a player
        let Ok(player_character) = other_actor.cast::<Character>() else {
            return;
        };
        if !player_character.checked().is_ok_and(|c| c.is_player_controlled()) {
            return;
        }
        // process the actors to activate list
        let Ok(actors) = self.actors_to_activate().to_vec() else {
            return;
        };
        for current_actor in actors {
            // is the referenced actor activatable?
            if let Some(mut activatable) = interfaces::activatable(current_actor) {
                activatable.activate_interaction(player_character.upcast_to());
            }
        }
    }
}

#[uclass(parent = Actor)]
pub struct CombatCheckpointVolume {
    /// Collision box volume
    #[component(root)]
    r#box: BoxComponent,
}

#[uclass_impl]
impl CombatCheckpointVolume {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        configure_box(self.r#box()?)
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        bind_overlap(self.as_ref().upcast_to(), self.r#box());
    }

    /// Handles overlaps with the box volume
    #[ufunction(BlueprintCallable)]
    #[allow(clippy::too_many_arguments)]
    fn on_overlap(
        &mut self,
        _overlapped_component: UObjectRef<PrimitiveComponent>,
        other_actor: UObjectRef<Actor>,
        _other_comp: UObjectRef<PrimitiveComponent>,
        _other_body_index: i32,
        _b_from_sweep: bool,
        _sweep_result: UStructRef<FHitResult>,
    ) {
        // has the player entered this volume?
        let Ok(player_character) = CombatCharacter::from_obj(other_actor) else {
            return;
        };
        let Ok(character) = player_character.as_ref().checked() else {
            return;
        };
        if let Ok(controller) = CombatPlayerController::from_obj(character.get_controller()) {
            // update the player's respawn checkpoint
            controller.set_respawn_transform(&character.get_transform());
        }
    }
}
