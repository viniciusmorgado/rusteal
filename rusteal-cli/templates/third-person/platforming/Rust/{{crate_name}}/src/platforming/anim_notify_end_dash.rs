use bindings::engine::ActorComponentExt;
use bindings::engine::{
    AnimNotify, AnimSequenceBase, FAnimNotifyEventReference, SkeletalMeshComponent,
};
use rusteal_runtime::runtime::{UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::character::PlatformingCharacter;

#[uclass(parent = AnimNotify)]
pub struct AnimNotifyEndDash {}

#[uclass_impl]
impl AnimNotifyEndDash {
    #[ufunction(Override, name = "Received_Notify")]
    fn notify(
        &self,
        mesh_comp: UObjectRef<SkeletalMeshComponent>,
        _animation: UObjectRef<AnimSequenceBase>,
        _event_reference: UStructRef<FAnimNotifyEventReference>,
    ) -> bool {
        let Ok(owner) = mesh_comp.checked().map(|mesh| mesh.get_owner()) else {
            return false;
        };

        if let Ok(mut platforming_character) = PlatformingCharacter::from_obj(owner) {
            platforming_character.end_dash();
        }

        true
    }
}
