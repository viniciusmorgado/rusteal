use bindings::engine::{
    ActorComponentExt, AnimNotify, AnimSequenceBase, FAnimNotifyEventReference,
    SkeletalMeshComponent,
};
use rusteal_runtime::runtime::{FName, UObjectRef, UStructRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::interfaces::{self, CombatAttacker};

fn attacker(mesh_comp: UObjectRef<SkeletalMeshComponent>) -> Option<Box<dyn CombatAttacker>> {
    interfaces::attacker(mesh_comp.checked().ok()?.get_owner())
}

#[uclass(parent = AnimNotify)]
pub struct AnimNotifyDoAttackTrace {
    #[uproperty(EditAnywhere)]
    attack_bone_name: FName,
}

#[uclass_impl]
impl AnimNotifyDoAttackTrace {
    #[ufunction(Override, name = "Received_Notify")]
    fn notify(
        &self,
        mesh_comp: UObjectRef<SkeletalMeshComponent>,
        _animation: UObjectRef<AnimSequenceBase>,
        _event_reference: UStructRef<FAnimNotifyEventReference>,
    ) -> bool {
        if let Some(mut attacker_interface) = attacker(mesh_comp) {
            attacker_interface.do_attack_trace(&self.attack_bone_name().to_string_lossy());
        }

        true
    }
}

#[uclass(parent = AnimNotify)]
pub struct AnimNotifyCheckCombo {}

#[uclass_impl]
impl AnimNotifyCheckCombo {
    #[ufunction(Override, name = "Received_Notify")]
    fn notify(
        &self,
        mesh_comp: UObjectRef<SkeletalMeshComponent>,
        _animation: UObjectRef<AnimSequenceBase>,
        _event_reference: UStructRef<FAnimNotifyEventReference>,
    ) -> bool {
        if let Some(mut attacker_interface) = attacker(mesh_comp) {
            attacker_interface.check_combo();
        }

        true
    }
}

#[uclass(parent = AnimNotify)]
pub struct AnimNotifyCheckChargedAttack {}

#[uclass_impl]
impl AnimNotifyCheckChargedAttack {
    #[ufunction(Override, name = "Received_Notify")]
    fn notify(
        &self,
        mesh_comp: UObjectRef<SkeletalMeshComponent>,
        _animation: UObjectRef<AnimSequenceBase>,
        _event_reference: UStructRef<FAnimNotifyEventReference>,
    ) -> bool {
        if let Some(mut attacker_interface) = attacker(mesh_comp) {
            attacker_interface.check_charged_attack();
        }

        true
    }
}
