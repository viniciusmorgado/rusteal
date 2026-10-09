use glam::DVec3;

pub const JUMP_MAX_HOLD_TIME: f32 = 0.4;
pub const JUMP_MAX_COUNT: i32 = 3;

pub const CAPSULE_RADIUS: f32 = 35.0;
pub const CAPSULE_HALF_HEIGHT: f32 = 90.0;

pub const GRAVITY_SCALE: f32 = 2.5;
pub const MAX_ACCELERATION: f32 = 1500.0;
pub const BRAKING_FRICTION_FACTOR: f32 = 1.0;
pub const GROUND_FRICTION: f32 = 4.0;
pub const MAX_WALK_SPEED: f32 = 750.0;
pub const MIN_ANALOG_WALK_SPEED: f32 = 20.0;
pub const BRAKING_DECELERATION_WALKING: f32 = 2500.0;
pub const PERCH_RADIUS_THRESHOLD: f32 = 15.0;
pub const JUMP_Z_VELOCITY: f32 = 350.0;
pub const BRAKING_DECELERATION_FALLING: f32 = 750.0;
pub const AIR_CONTROL: f32 = 1.0;
pub const ROTATION_RATE_YAW: f64 = 500.0;
pub const NAV_AGENT_RADIUS: f32 = 42.0;
pub const NAV_AGENT_HEIGHT: f32 = 192.0;

pub const CAMERA_BOOM_LENGTH: f32 = 400.0;
pub const CAMERA_LAG_SPEED: f32 = 8.0;
pub const CAMERA_ROTATION_LAG_SPEED: f32 = 8.0;

pub const WALL_JUMP_TRACE_DISTANCE: f32 = 50.0;
pub const WALL_JUMP_TRACE_RADIUS: f32 = 25.0;
pub const WALL_JUMP_BOUNCE_IMPULSE: f32 = 800.0;
pub const WALL_JUMP_VERTICAL_IMPULSE: f32 = 900.0;
pub const DELAY_BETWEEN_WALL_JUMPS: f32 = 0.1;
pub const MAX_COYOTE_TIME: f32 = 0.16;

pub fn ground_axes(yaw_degrees: f64) -> (DVec3, DVec3) {
    let (sin, cos) = yaw_degrees.to_radians().sin_cos();
    (DVec3::new(cos, sin, 0.0), DVec3::new(-sin, cos, 0.0))
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MoveState {
    pub has_wall_jumped: bool,
    pub has_double_jumped: bool,
    pub has_dashed: bool,
    pub is_dashing: bool,
    pub last_fall_time: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpPress {
    Ignore,
    Ground,
    TryWallJump,
}

pub fn jump_press(state: &MoveState, falling: bool) -> JumpPress {
    if state.is_dashing {
        return JumpPress::Ignore;
    }

    if !falling {
        return JumpPress::Ground;
    }

    if state.has_wall_jumped {
        JumpPress::Ignore
    } else {
        JumpPress::TryWallJump
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirJump {
    Coyote,
    Double,
    None,
}

pub fn air_jump(state: &MoveState, now: f64, max_coyote_time: f32) -> AirJump {
    if now - state.last_fall_time < f64::from(max_coyote_time) {
        AirJump::Coyote
    } else if !state.has_double_jumped {
        AirJump::Double
    } else {
        AirJump::None
    }
}

pub fn yaw_facing(normal: DVec3) -> f64 {
    normal.y.atan2(normal.x).to_degrees()
}

pub fn wall_jump_impulse(normal: DVec3, bounce: f32, vertical: f32) -> DVec3 {
    normal * f64::from(bounce) + DVec3::Z * f64::from(vertical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_axes_follow_the_yaw() {
        let (forward, right) = ground_axes(90.0);
        assert!((forward - DVec3::Y).length() < 1e-9);
        assert!((right + DVec3::X).length() < 1e-9);
    }

    #[test]
    fn grounded_jump_is_regular() {
        assert_eq!(jump_press(&MoveState::default(), false), JumpPress::Ground);
    }

    #[test]
    fn dashing_ignores_jumps() {
        let state = MoveState {
            is_dashing: true,
            ..Default::default()
        };

        assert_eq!(jump_press(&state, false), JumpPress::Ignore);
        assert_eq!(jump_press(&state, true), JumpPress::Ignore);
    }

    #[test]
    fn air_jump_tries_the_wall_once() {
        assert_eq!(
            jump_press(&MoveState::default(), true),
            JumpPress::TryWallJump
        );

        let state = MoveState {
            has_wall_jumped: true,
            ..Default::default()
        };

        assert_eq!(jump_press(&state, true), JumpPress::Ignore);
    }

    #[test]
    fn coyote_then_double_then_nothing() {
        let state = MoveState {
            last_fall_time: 10.0,
            ..Default::default()
        };

        assert_eq!(air_jump(&state, 10.1, MAX_COYOTE_TIME), AirJump::Coyote);
        assert_eq!(air_jump(&state, 10.2, MAX_COYOTE_TIME), AirJump::Double);

        let state = MoveState {
            has_double_jumped: true,
            ..state
        };

        assert_eq!(air_jump(&state, 10.2, MAX_COYOTE_TIME), AirJump::None);
    }

    #[test]
    fn wall_jump_faces_away_and_launches_up() {
        let normal = DVec3::new(0.0, -1.0, 0.0);
        assert!((yaw_facing(normal) + 90.0).abs() < 1e-9);

        assert_eq!(
            wall_jump_impulse(normal, WALL_JUMP_BOUNCE_IMPULSE, WALL_JUMP_VERTICAL_IMPULSE),
            DVec3::new(0.0, -800.0, 900.0)
        );
    }
}
