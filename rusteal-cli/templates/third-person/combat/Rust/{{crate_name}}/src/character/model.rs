// Pure logic for the third-person character: no engine calls, fully unit-tested.
//
// The numbers are the ones `ATP_ThirdPersonCharacter::ATP_ThirdPersonCharacter()` sets in
// the C++ template, so the Rust character moves exactly like the C++ one.

use glam::DVec3;

// --- Template constructor values --------------------------------------------

/// `GetCapsuleComponent()->InitCapsuleSize(42.f, 96.0f)`.
pub const CAPSULE_RADIUS: f32 = 42.0;
pub const CAPSULE_HALF_HEIGHT: f32 = 96.0;

/// `GetCharacterMovement()->RotationRate = FRotator(0.0f, 500.0f, 0.0f)`.
pub const ROTATION_RATE_YAW: f64 = 500.0;
pub const JUMP_Z_VELOCITY: f32 = 500.0;
pub const AIR_CONTROL: f32 = 0.35;
pub const MAX_WALK_SPEED: f32 = 500.0;
pub const MIN_ANALOG_WALK_SPEED: f32 = 20.0;
pub const BRAKING_DECELERATION_WALKING: f32 = 2000.0;
pub const BRAKING_DECELERATION_FALLING: f32 = 1500.0;

/// `CameraBoom->TargetArmLength = 400.0f`.
pub const CAMERA_BOOM_LENGTH: f32 = 400.0;

// --- Movement ---------------------------------------------------------------

/// Movement request in the template's `DoMove(Right, Forward)` convention.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveInput {
    pub right: f32,
    pub forward: f32,
}

/// Forward and right unit vectors on the ground plane for a control yaw, the
/// same as `FRotationMatrix(FRotator(0, Yaw, 0)).GetUnitAxis(EAxis::X / Y)`.
///
/// UE is Z-up with yaw rotating +X towards +Y.
pub fn ground_axes(yaw_degrees: f64) -> (DVec3, DVec3) {
    let (sin, cos) = yaw_degrees.to_radians().sin_cos();
    (DVec3::new(cos, sin, 0.0), DVec3::new(-sin, cos, 0.0))
}

// --- Look -------------------------------------------------------------------

/// Look request in the template's `DoLook(Yaw, Pitch)` convention: values go
/// straight to `AddControllerYawInput` / `AddControllerPitchInput`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LookInput {
    pub yaw: f32,
    pub pitch: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: DVec3, b: DVec3) -> bool {
        (a - b).length() < 1e-9
    }

    #[test]
    fn ground_axes_follow_control_yaw() {
        let (f, r) = ground_axes(0.0);
        assert!(close(f, DVec3::X));
        assert!(close(r, DVec3::Y));

        let (f, r) = ground_axes(90.0);
        assert!(close(f, DVec3::Y));
        assert!(close(r, -DVec3::X));

        let (f, r) = ground_axes(180.0);
        assert!(close(f, -DVec3::X));
        assert!(close(r, -DVec3::Y));
    }

    #[test]
    fn ground_axes_ignore_pitch_by_construction() {
        for yaw in [-135.0, 33.0, 720.0] {
            let (f, r) = ground_axes(yaw);
            assert_eq!(f.z, 0.0);
            assert_eq!(r.z, 0.0);
            assert!((f.length() - 1.0).abs() < 1e-12);
            assert!((r.length() - 1.0).abs() < 1e-12);
            assert!(f.dot(r).abs() < 1e-12);
        }
    }
}
