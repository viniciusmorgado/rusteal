// SideScrollingMovingPlatform: the SideScrolling variant's
// `ASideScrollingMovingPlatform` in Rust. A platform an interaction sets
// moving; its Blueprint child `BP_SideScrollingMovingPlatform` moves it
// (`BP_MoveToTarget`) and calls `ResetInteraction` when it is back.

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

    /// Destination of the platform in relative coordinates
    #[uproperty(EditAnywhere, BlueprintReadOnly)]
    platform_target: OwnedStruct<FVector>,

    /// Time for the platform to move to the destination
    #[uproperty(EditAnywhere, BlueprintReadOnly, default = model::MOVE_DURATION)]
    move_duration: f32,

    /// If this is true, the platform will only move once.
    #[uproperty(EditAnywhere, default = false)]
    b_one_shot: bool,

    /// If this is true, the platform is moving and will ignore further interactions
    moving: bool,
}

#[uclass_impl]
impl SideScrollingMovingPlatform {
    /// Resets the interaction state. Must be called from BP code to reset the platform
    #[ufunction(BlueprintCallable)]
    fn reset_interaction(&mut self) {
        // ignore if this is a one-shot platform
        if self.b_one_shot() {
            return;
        }
        // reset the movement flag
        self.set_moving(false);
    }

    /// Allows Blueprint code to do the actual platform movement
    #[ufunction(BlueprintImplementableEvent, name = "BP_MoveToTarget")]
    fn bp_move_to_target(&self) {}
}

impl Interactable for SideScrollingMovingPlatform {
    /// Performs an interaction triggered by another actor
    fn interaction(&mut self, _interactor: UObjectRef<Actor>) {
        // ignore interactions if we're already moving
        if self.moving() {
            return;
        }
        // raise the movement flag
        self.set_moving(true);
        // pass control to BP for the actual movement
        self.bp_move_to_target();
    }
}
