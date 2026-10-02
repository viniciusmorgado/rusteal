// Pure logic for the side scrolling variant: no engine calls, fully unit-tested.
//
// The numbers are the ones the C++ template's constructors and property
// initializers set, so the Rust classes behave exactly like the C++ ones.

use glam::DVec3;

// --- Character: template constructor values ---------------------------------

/// `Camera->SetRelativeLocationAndRotation(FVector(0, 300, 0), FRotator(0, -90, 0))`.
pub const CAMERA_LOCATION: DVec3 = DVec3::new(0.0, 300.0, 0.0);
pub const CAMERA_YAW: f64 = -90.0;

/// `GetCapsuleComponent()->SetCapsuleSize(35.0f, 90.0f)`.
pub const CAPSULE_RADIUS: f32 = 35.0;
pub const CAPSULE_HALF_HEIGHT: f32 = 90.0;

pub const GRAVITY_SCALE: f32 = 1.75;
pub const MAX_ACCELERATION: f32 = 1500.0;
pub const BRAKING_FRICTION_FACTOR: f32 = 1.0;
pub const MASS: f32 = 500.0;
pub const WALKABLE_FLOOR_ANGLE: f32 = 75.0;
pub const MAX_WALK_SPEED: f32 = 500.0;
pub const MIN_ANALOG_WALK_SPEED: f32 = 20.0;
pub const BRAKING_DECELERATION_WALKING: f32 = 2000.0;
pub const PERCH_RADIUS_THRESHOLD: f32 = 15.0;
pub const LEDGE_CHECK_THRESHOLD: f32 = 6.0;
pub const JUMP_Z_VELOCITY: f32 = 750.0;
pub const AIR_CONTROL: f32 = 1.0;
/// `RotationRate = FRotator(0.0f, 750.0f, 0.0f)`.
pub const ROTATION_RATE_YAW: f64 = 750.0;
/// `SetPlaneConstraintNormal(FVector(0.0f, 1.0f, 0.0f))`: movement on the XZ plane.
pub const PLANE_CONSTRAINT_NORMAL: DVec3 = DVec3::Y;
/// `JumpMaxCount = 3`: double jump and coyote time.
pub const JUMP_MAX_COUNT: i32 = 3;

// --- Character: property initializers ---------------------------------------

pub const JUMP_PUSH_IMPULSE: f32 = 600.0;
pub const INTERACTION_RADIUS: f32 = 200.0;
pub const DELAY_BETWEEN_WALL_JUMPS: f32 = 0.3;
pub const WALL_JUMP_TRACE_DISTANCE: f32 = 50.0;
pub const WALL_JUMP_HORIZONTAL_IMPULSE: f32 = 500.0;
pub const WALL_JUMP_VERTICAL_MULTIPLIER: f32 = 1.4;
pub const SOFT_COLLISION_TRACE_DISTANCE: f32 = 1000.0;
pub const MAX_COYOTE_TIME: f32 = 0.16;

/// How far ahead `DoInteract` sweeps for something to interact with.
pub const INTERACTION_REACH: f64 = 100.0;

// --- Character: movement ----------------------------------------------------

/// `UE_SMALL_NUMBER`, what `FMath::IsNearlyZero` compares to.
const SMALL_NUMBER: f64 = 1.0e-8;

/// The character's movement state: the C++ class's flags and last inputs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveState {
    /// Last time the character started falling.
    pub last_fall_time: f64,
    /// Last captured horizontal movement input value.
    pub action_value_y: f32,
    /// Last captured platform drop axis value.
    pub drop_value: f32,
    pub has_wall_jumped: bool,
    pub has_double_jumped: bool,
}

/// `DoMove`'s direction: along X, nudged towards the camera axis so the
/// character turns to face where it walks.
pub fn move_direction(forward: f32) -> DVec3 {
    DVec3::new(1.0, if forward > 0.0 { 0.1 } else { -0.1 }, 0.0)
}

/// The X direction of the last horizontal input: where wall jumps trace and
/// physics objects get pushed.
pub fn facing_x(action_value_y: f32) -> f64 {
    if action_value_y > 0.0 { 1.0 } else { -1.0 }
}

/// What a jump press does, before any trace: `MultiJump`'s decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpPress {
    /// Holding down: drop through a soft platform.
    Drop,
    /// On the ground: a regular jump.
    Ground,
    /// In the air with a horizontal input: try a wall jump first.
    TryWallJump,
    /// In the air: coyote time or double jump.
    Air,
}

/// `MultiJump`, up to the wall trace.
pub fn jump_press(state: &MoveState, falling: bool) -> JumpPress {
    // does the user want to drop to a lower platform?
    if state.drop_value > 0.0 {
        return JumpPress::Drop;
    }
    // if we're grounded, disregard advanced jump logic
    if !falling {
        return JumpPress::Ground;
    }
    // if we have a horizontal input, try for wall jump first
    if !state.has_wall_jumped && f64::from(state.action_value_y).abs() > SMALL_NUMBER {
        JumpPress::TryWallJump
    } else {
        JumpPress::Air
    }
}

/// A jump in the air without a wall to jump from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirJump {
    Coyote,
    Double,
    None,
}

/// `MultiJump` once the wall trace found nothing (or was not tried).
pub fn air_jump(state: &MoveState, now: f64, max_coyote_time: f32) -> AirJump {
    // test for double jump only if we haven't already tested for wall jump
    if state.has_wall_jumped {
        return AirJump::None;
    }
    // are we still within coyote time frames?
    if now - state.last_fall_time < f64::from(max_coyote_time) {
        AirJump::Coyote
    } else if !state.has_double_jumped {
        AirJump::Double
    } else {
        AirJump::None
    }
}

/// The launch velocity of a wall jump: away from the wall horizontally, and
/// up at the jump velocity times the multiplier.
pub fn wall_jump_impulse(normal: DVec3, horizontal: f32, jump_z_velocity: f32, multiplier: f32) -> DVec3 {
    let mut impulse = normal * f64::from(horizontal);
    impulse.z = f64::from(jump_z_velocity) * f64::from(multiplier);
    impulse
}

/// `UKismetMathLibrary::MakeRotFromX(Normal).Yaw`: the yaw facing along it.
pub fn yaw_of(direction: DVec3) -> f64 {
    direction.y.atan2(direction.x).to_degrees()
}

// --- Camera manager ---------------------------------------------------------

/// The camera's view: it looks along -Y from the side, with this FOV.
pub const CAMERA_VIEW_YAW: f64 = -90.0;
pub const CAMERA_FOV: f32 = 65.0;

/// `ASideScrollingCameraManager`'s property initializers.
pub const CURRENT_ZOOM: f32 = 1000.0;
pub const CAMERA_Z_OFFSET: f32 = 100.0;
pub const CAMERA_X_MIN_BOUNDS: f32 = -400.0;
pub const CAMERA_X_MAX_BOUNDS: f32 = 10000.0;

/// The camera manager's settings (its editable properties).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraSettings {
    pub zoom: f32,
    pub z_offset: f32,
    pub x_min: f32,
    pub x_max: f32,
}

/// The camera manager's state between updates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraState {
    /// Last cached camera height goal.
    pub current_z: f64,
    /// The first update placed the camera.
    pub set_up: bool,
}

/// What `UpdateViewTarget` reads from the world each frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraFrame {
    pub target_location: DVec3,
    pub camera_location: DVec3,
    pub target_moving_vertically: bool,
    /// Whether ground is right below the target (only traced while it moves
    /// vertically).
    pub ground_below: bool,
    pub delta_time: f64,
}

/// `ASideScrollingCameraManager::UpdateViewTarget`: the camera location for
/// this frame, and the state for the next one.
pub fn camera_location(settings: &CameraSettings, state: &CameraState, frame: &CameraFrame) -> (DVec3, CameraState) {
    let target = frame.target_location;
    // calculate the "zoom distance" - in reality the distance we want to keep to the target
    let current_y = f64::from(settings.zoom) + target.y;

    // do first-time setup
    if !state.set_up {
        let location = DVec3::new(target.x, current_y, target.z + f64::from(settings.z_offset));
        return (location, CameraState { current_z: location.z, set_up: true });
    }

    // check if the camera needs to update its height
    let z_update = if !frame.target_moving_vertically {
        is_nearly_equal(state.current_z, frame.camera_location.z, 25.0)
    } else {
        // only update height if we're not about to hit ground
        !frame.ground_below
    };

    let current_z = if z_update || is_nearly_equal(state.current_z, target.z, 100.0) {
        // set the height goal from the actor location
        target.z
    } else {
        // blend the height towards the actor location
        f_interp_to(state.current_z, target.z, frame.delta_time, 2.0)
    };

    // clamp the X axis to the min and max camera bounds
    let current_x = target.x.clamp(f64::from(settings.x_min), f64::from(settings.x_max));

    // blend towards the new camera location
    let goal = DVec3::new(current_x, current_y, current_z);
    let location = v_interp_to(frame.camera_location, goal, frame.delta_time, 2.0);
    (location, CameraState { current_z, set_up: true })
}

/// `FMath::IsNearlyEqual(A, B, Tolerance)`.
pub fn is_nearly_equal(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() <= tolerance
}

/// `FMath::FInterpTo`.
pub fn f_interp_to(current: f64, target: f64, delta_time: f64, speed: f64) -> f64 {
    if speed <= 0.0 {
        return target;
    }
    let dist = target - current;
    if dist * dist < SMALL_NUMBER {
        return target;
    }
    current + dist * (delta_time * speed).clamp(0.0, 1.0)
}

/// `FMath::VInterpTo`.
pub fn v_interp_to(current: DVec3, target: DVec3, delta_time: f64, speed: f64) -> DVec3 {
    if speed <= 0.0 {
        return target;
    }
    let dist = target - current;
    if dist.length_squared() < 1.0e-4 {
        return target;
    }
    current + dist * (delta_time * speed).clamp(0.0, 1.0)
}

// --- Gameplay ---------------------------------------------------------------

/// `ASideScrollingJumpPad`: its box and its launch strength.
pub const JUMP_PAD_BOX_EXTENT: DVec3 = DVec3::new(115.0, 90.0, 20.0);
pub const JUMP_PAD_BOX_LOCATION: DVec3 = DVec3::new(0.0, 0.0, 16.0);
pub const JUMP_PAD_Z_STRENGTH: f32 = 1000.0;

/// `ASideScrollingPickup`'s sphere.
pub const PICKUP_SPHERE_RADIUS: f32 = 100.0;

/// `ASideScrollingSoftPlatform`'s collision check box, below the mesh.
pub const SOFT_PLATFORM_CHECK_BOX_LOCATION: DVec3 = DVec3::new(0.0, 0.0, -40.0);

/// `ASideScrollingMovingPlatform::MoveDuration`.
pub const MOVE_DURATION: f32 = 5.0;

/// `ASideScrollingNPC`: walk speed and how it gets launched when interacted with.
pub const NPC_MAX_WALK_SPEED: f32 = 150.0;
pub const NPC_LAUNCH_IMPULSE: f32 = 500.0;
pub const NPC_LAUNCH_VERTICAL_IMPULSE: f32 = 500.0;
pub const NPC_DEACTIVATION_TIME: f32 = 3.0;

/// `ASideScrollingNPC::Interaction`: away from the interactor along X, up.
pub fn npc_launch(interactor_forward: DVec3, impulse: f32, vertical: f32) -> DVec3 {
    let mut launch = interactor_forward * f64::from(impulse);
    launch.y = 0.0;
    launch.z = f64::from(vertical);
    launch
}

/// `FStateTreeGetPlayerTask::RangeMax`.
pub const GET_PLAYER_RANGE_MAX: f32 = 1000.0;

/// `FStateTreeGetPlayerTask`: the closest of the players' pawns, and whether
/// it is within range.
pub fn closest_player<T: Copy>(npc: DVec3, players: &[(T, DVec3)], range_max: f32) -> Option<(T, bool)> {
    let mut selected: Option<(T, f64)> = None;
    for &(player, location) in players {
        let distance = (location - npc).length();
        if selected.is_none_or(|(_, closest)| distance < closest) {
            selected = Some((player, distance));
        }
    }
    selected.map(|(player, distance)| (player, distance < f64::from(range_max)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jump_presses() {
        let state = MoveState::default();
        assert_eq!(jump_press(&state, false), JumpPress::Ground);
        assert_eq!(jump_press(&state, true), JumpPress::Air);
        let moving = MoveState { action_value_y: 1.0, ..state };
        assert_eq!(jump_press(&moving, true), JumpPress::TryWallJump);
        let walled = MoveState { has_wall_jumped: true, ..moving };
        assert_eq!(jump_press(&walled, true), JumpPress::Air);
        let dropping = MoveState { drop_value: 1.0, ..state };
        assert_eq!(jump_press(&dropping, false), JumpPress::Drop);
    }

    #[test]
    fn air_jumps() {
        let state = MoveState { last_fall_time: 1.0, ..Default::default() };
        assert_eq!(air_jump(&state, 1.1, MAX_COYOTE_TIME), AirJump::Coyote);
        assert_eq!(air_jump(&state, 2.0, MAX_COYOTE_TIME), AirJump::Double);
        let doubled = MoveState { has_double_jumped: true, ..state };
        assert_eq!(air_jump(&doubled, 2.0, MAX_COYOTE_TIME), AirJump::None);
        let walled = MoveState { has_wall_jumped: true, ..state };
        assert_eq!(air_jump(&walled, 1.1, MAX_COYOTE_TIME), AirJump::None);
    }

    #[test]
    fn wall_jump_goes_up_by_the_multiplier() {
        let impulse = wall_jump_impulse(DVec3::new(-1.0, 0.0, 0.0), 500.0, 750.0, 1.4);
        assert_eq!(impulse.x, -500.0);
        assert!((impulse.z - 1050.0).abs() < 1e-3);
        assert!((yaw_of(DVec3::new(-1.0, 0.0, 0.0)) - 180.0).abs() < 1e-9);
    }

    #[test]
    fn camera_sets_up_then_follows_within_bounds() {
        let settings = CameraSettings {
            zoom: CURRENT_ZOOM,
            z_offset: CAMERA_Z_OFFSET,
            x_min: CAMERA_X_MIN_BOUNDS,
            x_max: CAMERA_X_MAX_BOUNDS,
        };
        let frame = CameraFrame {
            target_location: DVec3::new(-1000.0, 0.0, 100.0),
            camera_location: DVec3::ZERO,
            target_moving_vertically: false,
            ground_below: true,
            delta_time: 1.0,
        };
        let (first, state) = camera_location(&settings, &CameraState::default(), &frame);
        assert_eq!(first, DVec3::new(-1000.0, 1000.0, 200.0));
        assert!(state.set_up);
        let frame = CameraFrame { camera_location: first, ..frame };
        let (next, _) = camera_location(&settings, &state, &frame);
        // clamped to the minimum X bound, reached in one second at speed 2
        assert_eq!(next.x, -400.0);
    }

    #[test]
    fn interp_reaches_the_target() {
        assert_eq!(f_interp_to(0.0, 10.0, 0.25, 2.0), 5.0);
        assert_eq!(f_interp_to(0.0, 10.0, 0.25, 0.0), 10.0);
        assert_eq!(v_interp_to(DVec3::ZERO, DVec3::X, 1.0, 2.0), DVec3::X);
    }

    #[test]
    fn npc_launches_away_and_up() {
        assert_eq!(npc_launch(DVec3::new(1.0, 1.0, 0.0), 500.0, 500.0), DVec3::new(500.0, 0.0, 500.0));
    }

    #[test]
    fn closest_player_in_range() {
        let players = [(1, DVec3::new(2000.0, 0.0, 0.0)), (2, DVec3::new(500.0, 0.0, 0.0))];
        assert_eq!(closest_player(DVec3::ZERO, &players, 1000.0), Some((2, true)));
        assert_eq!(closest_player(DVec3::ZERO, &players[..1], 1000.0), Some((1, false)));
        assert_eq!(closest_player::<i32>(DVec3::ZERO, &[], 1000.0), None);
    }
}
