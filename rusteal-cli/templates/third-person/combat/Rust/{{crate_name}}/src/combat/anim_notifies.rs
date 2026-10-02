// The Combat variant's animation notifies in Rust: the attack montages
// (`AM_ComboAttack`, `AM_ChargedAttack`) fire them to trace an attack's hits
// and to decide whether the combo string or the charge goes on. C++'s
// `Notify` virtual is the `Received_Notify` event.

use bindings::engine::{ActorComponentExt, AnimNotify, AnimSequenceBase, FAnimNotifyEventReference, SkeletalMeshComponent};
use rusteal_runtime::runtime::{FName, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::interfaces::{self, CombatAttacker};

/// The attacker owning the mesh the notify fired on.
fn attacker(mesh_comp: UObjectRef<SkeletalMeshComponent>) -> Option<Box<dyn CombatAttacker>> {
    interfaces::attacker(mesh_comp.checked().ok()?.get_owner())
}

/// AnimNotify to tell the actor to perform an attack trace check to look for targets to damage.
#[uclass(parent = AnimNotify)]
pub struct AnimNotifyDoAttackTrace {
    /// Source bone for the attack trace
    #[uproperty(EditAnywhere)]
    attack_bone_name: FName,
}

#[uclass_impl]
impl AnimNotifyDoAttackTrace {
    /// Perform the Anim Notify
    #[ufunction(Override, name = "Received_Notify")]
    fn notify(
        &self,
        mesh_comp: UObjectRef<SkeletalMeshComponent>,
        _animation: UObjectRef<AnimSequenceBase>,
        _event_reference: UStructRef<FAnimNotifyEventReference>,
    ) -> bool {
        // cast the owner to the attacker interface
        if let Some(mut attacker_interface) = attacker(mesh_comp) {
            attacker_interface.do_attack_trace(&self.attack_bone_name().to_string_lossy());
        }
        true
    }
}

/// AnimNotify to tell the actor to check for combo string inputs.
#[uclass(parent = AnimNotify)]
pub struct AnimNotifyCheckCombo {}

#[uclass_impl]
impl AnimNotifyCheckCombo {
    /// Perform the Anim Notify
    #[ufunction(Override, name = "Received_Notify")]
    fn notify(
        &self,
        mesh_comp: UObjectRef<SkeletalMeshComponent>,
        _animation: UObjectRef<AnimSequenceBase>,
        _event_reference: UStructRef<FAnimNotifyEventReference>,
    ) -> bool {
        // tell the actor to check for combo string
        if let Some(mut attacker_interface) = attacker(mesh_comp) {
            attacker_interface.check_combo();
        }
        true
    }
}

/// AnimNotify to tell the actor to check if the charged attack button is still held.
#[uclass(parent = AnimNotify)]
pub struct AnimNotifyCheckChargedAttack {}

#[uclass_impl]
impl AnimNotifyCheckChargedAttack {
    /// Perform the Anim Notify
    #[ufunction(Override, name = "Received_Notify")]
    fn notify(
        &self,
        mesh_comp: UObjectRef<SkeletalMeshComponent>,
        _animation: UObjectRef<AnimSequenceBase>,
        _event_reference: UStructRef<FAnimNotifyEventReference>,
    ) -> bool {
        // tell the actor to check for a charged attack loop
        if let Some(mut attacker_interface) = attacker(mesh_comp) {
            attacker_interface.check_charged_attack();
        }
        true
    }
}
