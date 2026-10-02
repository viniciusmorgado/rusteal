// TopDownCharacter: the Top Down template's `ATP_TopDownCharacter` in Rust, a
// character seen from above, which its player controller moves to where the
// cursor or the finger points.
//
// What the C++ constructor does happens in two places: the components and
// their attachment are declared on the struct, and the values it sets on
// inherited properties are written by `#[class_defaults]` on the class default
// object. The mannequin and its animation come from its Blueprint child
// `BP_TopDownCharacter`, as they do for the C++ class.

mod model;

use bindings::engine::{
    CameraComponent, CameraComponentExt, CapsuleComponentExt, Character, CharacterExt,
    CharacterMovementComponentExt, MovementComponentExt, PawnExt, SceneComponentExt,
    SpringArmComponent, SpringArmComponentExt,
};
use bindings::prelude::*;
use rusteal_runtime::runtime::{Rotator, RustealResult};
use rusteal_runtime::{uclass, uclass_impl};

#[uclass(parent = Character)]
pub struct TopDownCharacter {
    /// Camera boom positioning the camera above the character
    #[component(attach = "root_component")]
    camera_boom: SpringArmComponent,

    /// Top down camera
    #[component(attach = "camera_boom", socket = "SpringEndpoint", name = "TopDownCamera")]
    top_down_camera_component: CameraComponent,
}

#[uclass_impl]
impl TopDownCharacter {
    /// Everything `ATP_TopDownCharacter::ATP_TopDownCharacter()` sets.
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let me = self.as_ref().checked()?;

        // Set size for player capsule
        me.get_capsule_component().checked()?.set_capsule_size(
            model::CAPSULE_RADIUS,
            model::CAPSULE_HALF_HEIGHT,
            Some(false),
        );

        // Don't rotate character to camera direction
        me.set_use_controller_rotation_pitch(false);
        me.set_use_controller_rotation_yaw(false);
        me.set_use_controller_rotation_roll(false);

        // Configure character movement
        let movement = me.get_character_movement().checked()?;
        movement.set_orient_rotation_to_movement(true);
        movement.set_rotation_rate(&FRotator::from_rotator(Rotator::new(0.0, model::ROTATION_RATE_YAW, 0.0)));
        movement.set_plane_constraint_enabled(true);
        movement.set_snap_to_plane_at_start(true);

        // Create the camera boom component
        let boom = self.camera_boom()?.checked()?;
        boom.set_absolute(None, Some(true), None);
        boom.set_target_arm_length(model::CAMERA_BOOM_LENGTH);
        boom.k2_set_relative_rotation(
            &FRotator::from_rotator(Rotator::new(model::CAMERA_BOOM_PITCH, 0.0, 0.0)),
            false,
            false,
        );
        boom.set_do_collision_test(false);

        // Create the camera component
        self.top_down_camera_component()?.checked()?.set_use_pawn_control_rotation(false);
        Ok(())
    }
}
