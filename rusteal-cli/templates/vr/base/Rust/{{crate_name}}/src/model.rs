use glam::{DVec2, DVec3};

pub const TELEPORT_LAUNCH_SPEED: f64 = 650.0;
pub const TELEPORT_PROJECTILE_RADIUS: f32 = 3.6;
pub const TELEPORT_SIM_FREQUENCY: f32 = 15.0;
pub const TELEPORT_MAX_SIM_TIME: f32 = 2.0;
pub const NAV_MESH_CELL_HEIGHT: f64 = 8.0;
pub const HAND_POSE_INTERP_SPEED: f64 = 13.0;
pub const MENU_OPEN_SECONDS: f64 = 0.2;
pub const SPECTATOR_FADE_SECONDS: f64 = 2.0;
pub const SPECTATOR_MAX_PITCH: f64 = 89.9;
pub const SPECTATOR_FOV_MIN: f64 = 1.0;
pub const SPECTATOR_FOV_MAX: f64 = 150.0;
pub const CURSOR_WIDGET_OFFSET: f64 = -10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hand {
    Left,
    Right,
}

pub fn hand_of_motion_source(motion_source: &str) -> Hand {
    if motion_source.get(..4) == Some("Left") {
        Hand::Left
    } else {
        Hand::Right
    }
}

pub fn rotate_yaw(v: DVec3, yaw_degrees: f64) -> DVec3 {
    let (sin, cos) = yaw_degrees.to_radians().sin_cos();
    DVec3::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos, v.z)
}

pub fn projected_teleport_location(nav_point: DVec3) -> DVec3 {
    nav_point - DVec3::Z * NAV_MESH_CELL_HEIGHT
}

pub fn teleport_destination(projected: DVec3, camera_relative: DVec3, actor_yaw: f64) -> DVec3 {
    projected
        - rotate_yaw(
            DVec3::new(camera_relative.x, camera_relative.y, 0.0),
            actor_yaw,
        )
}

pub fn snap_turn_delta(snap_turn_degrees: f64, right_turn: bool) -> f64 {
    if right_turn {
        snap_turn_degrees.abs()
    } else {
        snap_turn_degrees
    }
}

pub fn snap_turn_location(camera_world: DVec3, camera_relative: DVec3, new_yaw: f64) -> DVec3 {
    camera_world - rotate_yaw(camera_relative, new_yaw)
}

pub fn nearest<T: Copy>(grip: DVec3, candidates: &[(T, DVec3)]) -> Option<T> {
    let mut best = None;
    let mut best_distance = 9_999_999.0;

    for (candidate, location) in candidates {
        let distance = (*location - grip).length_squared();

        if distance <= best_distance {
            best_distance = distance;
            best = Some(*candidate);
        }
    }

    best
}

pub fn f_interp_to(current: f64, target: f64, delta_seconds: f64, speed: f64) -> f64 {
    if speed <= 0.0 {
        return target;
    }

    let distance = target - current;

    if distance * distance < 1.0e-8 {
        return target;
    }

    current + distance * (delta_seconds * speed).clamp(0.0, 1.0)
}

pub fn v_interp_to(current: DVec3, target: DVec3, delta_seconds: f64, speed: f64) -> DVec3 {
    if speed <= 0.0 {
        return target;
    }

    let distance = target - current;

    if distance.length_squared() < 1.0e-8 {
        return target;
    }

    current + distance * (delta_seconds * speed).clamp(0.0, 1.0)
}

pub fn x_axis_positive_only(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

pub fn cursor_step(current: f64, input: f64, speed: f64, limit: f64) -> f64 {
    (current + speed * input).clamp(-limit, limit)
}

pub fn cursor_limits(widget_size: DVec2) -> DVec2 {
    widget_size * 0.5
}

pub fn spectator_pitch_allowed(pitch: f64, delta: f64) -> bool {
    (pitch + delta).abs() <= SPECTATOR_MAX_PITCH
}

pub fn spectator_fov(fov: f64, delta: f64) -> f64 {
    (fov + delta).clamp(SPECTATOR_FOV_MIN, SPECTATOR_FOV_MAX)
}

pub fn circular_in(alpha: f64) -> f64 {
    let alpha = alpha.clamp(0.0, 1.0);
    -((1.0 - alpha * alpha).sqrt() - 1.0)
}

pub fn spectator_fade_gain(alpha: f64) -> f64 {
    1.0 + (0.0 - 1.0) * circular_in(alpha)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timeline {
    pub length: f64,
    pub position: f64,
    pub forward: bool,
    pub playing: bool,
}

impl Timeline {
    pub fn new(length: f64) -> Self {
        Self {
            length,
            position: 0.0,
            forward: true,
            playing: false,
        }
    }

    pub fn play(&mut self) {
        self.forward = true;
        self.playing = true;
    }

    pub fn reverse(&mut self) {
        self.forward = false;
        self.playing = true;
    }

    pub fn alpha(&self) -> f64 {
        if self.length <= 0.0 {
            1.0
        } else {
            (self.position / self.length).clamp(0.0, 1.0)
        }
    }

    pub fn advance(&mut self, delta_seconds: f64) -> bool {
        if !self.playing {
            return false;
        }

        let step = if self.forward {
            delta_seconds
        } else {
            -delta_seconds
        };
        self.position = (self.position + step).clamp(0.0, self.length);

        let finished = if self.forward {
            self.position >= self.length
        } else {
            self.position <= 0.0
        };

        if finished {
            self.playing = false;
        }

        finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hands_come_from_the_motion_source_prefix() {
        assert_eq!(hand_of_motion_source("LeftGrip"), Hand::Left);
        assert_eq!(hand_of_motion_source("LeftAim"), Hand::Left);
        assert_eq!(hand_of_motion_source("RightGrip"), Hand::Right);
        assert_eq!(hand_of_motion_source("Lef"), Hand::Right);
    }

    #[test]
    fn teleports_keep_the_head_over_the_target() {
        let destination = teleport_destination(
            DVec3::new(100.0, 0.0, 0.0),
            DVec3::new(10.0, 0.0, 170.0),
            90.0,
        );
        assert!((destination - DVec3::new(100.0, -10.0, 0.0)).length() < 1.0e-9);
    }

    #[test]
    fn the_teleport_target_sits_a_cell_below_the_navigation_mesh() {
        assert_eq!(
            projected_teleport_location(DVec3::new(0.0, 0.0, 10.0)),
            DVec3::new(0.0, 0.0, 2.0)
        );
    }

    #[test]
    fn snap_turns_use_the_sign_of_the_input() {
        assert_eq!(snap_turn_delta(-45.0, true), 45.0);
        assert_eq!(snap_turn_delta(-45.0, false), -45.0);
    }

    #[test]
    fn snap_turns_rotate_around_the_head() {
        let camera = DVec3::new(10.0, 0.0, 170.0);
        let location = snap_turn_location(DVec3::new(110.0, 0.0, 170.0), camera, 90.0);
        assert!(
            (location + rotate_yaw(camera, 90.0) - DVec3::new(110.0, 0.0, 170.0)).length() < 1.0e-9
        );
    }

    #[test]
    fn the_nearest_grab_component_wins_and_ties_go_to_the_last() {
        let grip = DVec3::ZERO;
        let picked = nearest(
            grip,
            &[
                (1, DVec3::new(5.0, 0.0, 0.0)),
                (2, DVec3::new(2.0, 0.0, 0.0)),
                (3, DVec3::new(0.0, 2.0, 0.0)),
            ],
        );
        assert_eq!(picked, Some(3));
        assert_eq!(nearest::<i32>(grip, &[]), None);
    }

    #[test]
    fn only_forward_thumbstick_input_aims_a_teleport() {
        assert_eq!(x_axis_positive_only(-0.6), 0.0);
        assert_eq!(x_axis_positive_only(0.8), 0.8);
        assert_eq!(x_axis_positive_only(1.4), 1.0);
    }

    #[test]
    fn the_cursor_stays_on_the_widget() {
        assert_eq!(cursor_step(0.0, 1.0, -8.0, 100.0), -8.0);
        assert_eq!(cursor_step(95.0, -1.0, -8.0, 100.0), 100.0);
        assert_eq!(
            cursor_limits(DVec2::new(300.0, 200.0)),
            DVec2::new(150.0, 100.0)
        );
    }

    #[test]
    fn the_spectator_never_turns_upside_down() {
        assert!(spectator_pitch_allowed(80.0, 9.0));
        assert!(!spectator_pitch_allowed(85.0, 5.0));
        assert_eq!(spectator_fov(149.0, 5.0), 150.0);
        assert_eq!(spectator_fov(2.0, -5.0), 1.0);
    }

    #[test]
    fn the_fade_goes_from_full_to_black() {
        assert_eq!(spectator_fade_gain(0.0), 1.0);
        assert!((spectator_fade_gain(1.0)).abs() < 1.0e-12);
        assert!(spectator_fade_gain(0.5) > 0.8);
    }

    #[test]
    fn timelines_play_and_reverse() {
        let mut timeline = Timeline::new(0.2);
        timeline.play();
        assert!(!timeline.advance(0.1));
        assert!((timeline.alpha() - 0.5).abs() < 1.0e-9);
        assert!(timeline.advance(0.2));
        assert_eq!(timeline.alpha(), 1.0);
        timeline.reverse();
        assert!(timeline.advance(0.5));
        assert_eq!(timeline.alpha(), 0.0);
    }

    #[test]
    fn interpolation_reaches_and_stops() {
        assert_eq!(f_interp_to(0.0, 1.0, 0.1, 13.0), 1.0);
        assert!((f_interp_to(0.0, 1.0, 0.01, 13.0) - 0.13).abs() < 1.0e-12);
        assert_eq!(v_interp_to(DVec3::ZERO, DVec3::X, 1.0, 0.0), DVec3::X);
    }
}
