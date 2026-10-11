use bindings::engine::{
    BoxComponent, BoxComponentExt, PrimitiveComponentExt, SceneComponent, SceneComponentExt,
    ShapeComponentExt,
};
use bindings::prelude::*;
use glam::DVec3;
use rusteal_runtime::runtime::{FName, RustealResult, UObjectRef};
use rusteal_runtime::{uclass, uclass_impl};

use super::player_controller::TimeTrialPlayerController;

#[uclass(parent = Actor)]
pub struct TimeTrialTrackGate {
    #[component(root, name = "Root")]
    root: SceneComponent,

    #[component(attach = "root", name = "Collision Box")]
    collision_box: BoxComponent,

    #[uproperty(EditAnywhere, category = "Track Gate", default = false)]
    b_is_finish_line: bool,

    #[uproperty(EditAnywhere, category = "Track Gate")]
    next_marker: UObjectRef<TimeTrialTrackGate>,
}

#[uclass_impl]
impl TimeTrialTrackGate {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let collision_box = self.collision_box()?.checked()?;
        collision_box.set_box_extent(&FVector::from_dvec3(DVec3::splat(1000.0)), Some(false));
        collision_box.set_line_thickness(32.0);
        collision_box.set_hidden_in_game(false, None);
        collision_box
            .set_collision_profile_name(FName::new("OverlapAllDynamic").handle(), Some(true));

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_actor_begin_overlap(&mut self, other_actor: UObjectRef<Actor>) {
        let Ok(other) = other_actor.checked() else {
            return;
        };

        let Ok(mut controller) =
            TimeTrialPlayerController::from_obj(other.get_instigator_controller())
        else {
            return;
        };

        let me: UObjectRef<TimeTrialTrackGate> = self.as_ref().cast().unwrap_or_default();

        if controller.target_gate() != me {
            return;
        }

        controller.set_target_gate(self.next_marker());

        if self.b_is_finish_line() {
            controller.increment_lap_count();
        }
    }
}

impl TimeTrialTrackGate {
    pub fn get_next_marker(&self) -> UObjectRef<TimeTrialTrackGate> {
        self.next_marker()
    }
}
