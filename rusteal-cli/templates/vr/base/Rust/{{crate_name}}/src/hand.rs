use bindings::engine::{AnimInstance, AnimInstanceExt, SkeletalMeshComponent};
use rusteal_runtime::{uclass, uclass_impl};

use crate::model;

#[uclass(parent = SkeletalMeshComponent)]
pub struct MannequinsXR {
    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Hand", default = false)]
    b_mirror: bool,
}

#[uclass(parent = AnimInstance)]
pub struct HandAnimInstance {
    #[uproperty(BlueprintReadWrite, category = "Pose", default = 0.0)]
    pose_alpha_grasp: f64,

    #[uproperty(BlueprintReadWrite, category = "Pose", default = 0.0)]
    pose_alpha_index_curl: f64,

    #[uproperty(BlueprintReadWrite, category = "Pose", default = 0.0)]
    pose_alpha_point: f32,

    #[uproperty(BlueprintReadWrite, category = "Pose", default = 0.0)]
    pose_alpha_thumb_up: f32,

    #[uproperty(BlueprintReadOnly, category = "Pose", default = 0.0)]
    current_pose_alpha_point: f64,

    #[uproperty(BlueprintReadOnly, category = "Pose", default = 0.0)]
    current_pose_alpha_thumb_up: f64,

    #[uproperty(BlueprintReadOnly, category = "Pose", default = false)]
    b_mirror: bool,
}

#[uclass_impl]
impl HandAnimInstance {
    #[ufunction(Override)]
    fn blueprint_initialize_animation(&mut self) {
        let mirror = self
            .as_ref()
            .checked()
            .ok()
            .and_then(|anim| MannequinsXR::from_obj(anim.get_owning_component()).ok())
            .is_some_and(|hand| hand.b_mirror());

        self.set_b_mirror(mirror);
    }

    #[ufunction(Override)]
    fn blueprint_update_animation(&mut self, delta_time_x: f32) {
        let delta = f64::from(delta_time_x);
        let speed = model::HAND_POSE_INTERP_SPEED;

        self.set_current_pose_alpha_thumb_up(model::f_interp_to(
            self.current_pose_alpha_thumb_up(),
            f64::from(self.pose_alpha_thumb_up()),
            delta,
            speed,
        ));

        self.set_current_pose_alpha_point(model::f_interp_to(
            self.current_pose_alpha_point(),
            f64::from(self.pose_alpha_point()),
            delta,
            speed,
        ));
    }
}
