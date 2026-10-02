// AnimNotifyEndDash: the Platforming variant's `UAnimNotify_EndDash` in Rust.
// The dash montage (`AM_Dash`) fires it to finish the dash and give control
// back to the player. C++'s `Notify` virtual is the `Received_Notify` event.

use bindings::engine::{AnimNotify, AnimSequenceBase, FAnimNotifyEventReference, SkeletalMeshComponent};
use bindings::engine::ActorComponentExt;
use rusteal_runtime::runtime::{UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::PlatformingCharacter;

#[uclass(parent = AnimNotify)]
pub struct AnimNotifyEndDash {}

#[uclass_impl]
impl AnimNotifyEndDash {
    /// Perform the Anim Notify
    #[ufunction(Override, name = "Received_Notify")]
    fn notify(
        &self,
        mesh_comp: UObjectRef<SkeletalMeshComponent>,
        _animation: UObjectRef<AnimSequenceBase>,
        _event_reference: UStructRef<FAnimNotifyEventReference>,
    ) -> bool {
        // cast the owner to the platforming character
        let Ok(owner) = mesh_comp.checked().map(|mesh| mesh.get_owner()) else {
            return false;
        };
        if let Ok(mut platforming_character) = PlatformingCharacter::from_obj(owner) {
            // tell the actor to end the dash
            platforming_character.end_dash();
        }
        true
    }
}
