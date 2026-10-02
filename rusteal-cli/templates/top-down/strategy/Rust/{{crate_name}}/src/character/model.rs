// The numbers `ATP_TopDownCharacter::ATP_TopDownCharacter()` sets in the C++
// template, so the Rust character moves and is framed exactly like the C++ one.

/// `GetCapsuleComponent()->InitCapsuleSize(42.f, 96.0f)`.
pub const CAPSULE_RADIUS: f32 = 42.0;
pub const CAPSULE_HALF_HEIGHT: f32 = 96.0;

/// `GetCharacterMovement()->RotationRate = FRotator(0.f, 640.f, 0.f)`.
pub const ROTATION_RATE_YAW: f64 = 640.0;

/// `CameraBoom->TargetArmLength = 800.f`.
pub const CAMERA_BOOM_LENGTH: f32 = 800.0;

/// `CameraBoom->SetRelativeRotation(FRotator(-60.f, 0.f, 0.f))`: looking down.
pub const CAMERA_BOOM_PITCH: f64 = -60.0;
