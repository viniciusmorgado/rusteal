// The numbers `ATP_FirstPersonCharacter::ATP_FirstPersonCharacter()` sets in the
// C++ template, so the Rust character moves and sees exactly like the C++ one.

use glam::DVec3;

/// `GetCapsuleComponent()->SetCapsuleSize(34.0f, 96.0f)`, the size the
/// constructor ends with (it first sets 55 x 96 with `InitCapsuleSize`).
pub const CAPSULE_RADIUS: f32 = 34.0;
pub const CAPSULE_HALF_HEIGHT: f32 = 96.0;

/// The first person camera's place on the arms' `head` socket.
pub const CAMERA_LOCATION: DVec3 = DVec3::new(-2.8, 5.89, 0.0);
/// The camera's rotation on the socket: `FRotator(0.0f, 90.0f, -90.0f)`.
pub const CAMERA_PITCH: f64 = 0.0;
pub const CAMERA_YAW: f64 = 90.0;
pub const CAMERA_ROLL: f64 = -90.0;

/// The camera's first person field of view and scale, for the arms.
pub const FIRST_PERSON_FIELD_OF_VIEW: f32 = 70.0;
pub const FIRST_PERSON_SCALE: f32 = 0.6;

pub const BRAKING_DECELERATION_FALLING: f32 = 1500.0;
pub const AIR_CONTROL: f32 = 0.5;
