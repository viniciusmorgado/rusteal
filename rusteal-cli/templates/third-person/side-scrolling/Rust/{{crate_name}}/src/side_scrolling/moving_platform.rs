use bindings::engine::{Actor, SceneComponent};
use bindings::prelude::*;
use rusteal_runtime::runtime::{OwnedStruct, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::interactable::Interactable;
use super::model;

#[uclass(parent = Actor)]
pub struct SideScrollingMovingPlatform {
    #[component(root)]
    root: SceneComponent,

    #[uproperty(EditAnywhere, BlueprintReadOnly)]
    platform_target: OwnedStruct<FVector>,

    #[uproperty(EditAnywhere, BlueprintReadOnly, default = model::MOVE_DURATION)]
    move_duration: f32,

    #[uproperty(EditAnywhere, default = false)]
    b_one_shot: bool,

    moving: bool,
}

#[uclass_impl]
impl SideScrollingMovingPlatform {
    #[ufunction(BlueprintCallable)]
    fn reset_interaction(&mut self) {
        if self.b_one_shot() {
            return;
        }

        self.set_moving(false);
    }

    #[ufunction(BlueprintImplementableEvent, name = "BP_MoveToTarget")]
    fn bp_move_to_target(&self) {}
}

impl Interactable for SideScrollingMovingPlatform {
    fn interaction(&mut self, _interactor: UObjectRef<Actor>) {
        if self.moving() {
            return;
        }

        self.set_moving(true);

        self.bp_move_to_target();
    }
}
