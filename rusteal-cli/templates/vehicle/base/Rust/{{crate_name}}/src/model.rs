pub const MPH_PER_CM_PER_SECOND: f32 = 0.022;
pub const KMH_PER_CM_PER_SECOND: f32 = 0.036;

pub const AIRBORNE_ANGULAR_DAMPING: f32 = 3.0;
pub const CAMERA_RECENTER_SPEED: f64 = 1.0;
pub const RESET_HEIGHT: f64 = 50.0;

const SMALL_NUMBER: f64 = 1.0e-8;

pub fn display_speed(forward_speed: f32, is_mph: bool) -> f32 {
    let factor = if is_mph {
        MPH_PER_CM_PER_SECOND
    } else {
        KMH_PER_CM_PER_SECOND
    };

    forward_speed.abs() * factor
}

pub fn f_interp_to(current: f64, target: f64, delta_time: f64, speed: f64) -> f64 {
    if speed <= 0.0 {
        return target;
    }

    let distance = target - current;

    if distance * distance < SMALL_NUMBER {
        return target;
    }

    current + distance * (delta_time * speed).clamp(0.0, 1.0)
}

pub fn angular_damping(moving_on_ground: bool) -> f32 {
    if moving_on_ground {
        0.0
    } else {
        AIRBORNE_ANGULAR_DAMPING
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlipCheck {
    pub reset: bool,
    pub flipped: bool,
}

pub fn flip_check(previously_flipped: bool, up_dot: f32, min_dot: f32) -> FlipCheck {
    let flipped = up_dot < min_dot;

    FlipCheck {
        reset: flipped && previously_flipped,
        flipped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_is_shown_in_the_chosen_unit_whatever_the_direction() {
        assert!((display_speed(1000.0, true) - 22.0).abs() < 1e-4);
        assert!((display_speed(-1000.0, false) - 36.0).abs() < 1e-4);
    }

    #[test]
    fn interpolation_moves_toward_the_target_and_snaps_when_close() {
        assert_eq!(f_interp_to(90.0, 0.0, 0.5, 1.0), 45.0);
        assert_eq!(f_interp_to(90.0, 0.0, 2.0, 1.0), 0.0);
        assert_eq!(f_interp_to(1.0e-5, 0.0, 0.1, 1.0), 0.0);
        assert_eq!(f_interp_to(30.0, 0.0, 0.1, 0.0), 0.0);
    }

    #[test]
    fn a_car_is_reset_on_the_second_flipped_check_in_a_row() {
        let first = flip_check(false, -0.9, -0.2);
        assert_eq!(first, FlipCheck { reset: false, flipped: true });

        let second = flip_check(first.flipped, -0.9, -0.2);
        assert_eq!(second, FlipCheck { reset: true, flipped: true });

        let upright = flip_check(second.flipped, 1.0, -0.2);
        assert_eq!(upright, FlipCheck { reset: false, flipped: false });
    }

    #[test]
    fn the_car_is_damped_only_in_the_air() {
        assert_eq!(angular_damping(true), 0.0);
        assert_eq!(angular_damping(false), AIRBORNE_ANGULAR_DAMPING);
    }
}
