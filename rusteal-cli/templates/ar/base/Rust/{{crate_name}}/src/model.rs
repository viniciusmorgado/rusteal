use glam::{DVec2, DVec3};

use crate::enums::EGestureType;

pub const GESTURE_VALIDATION_SECONDS: f64 = 0.2;
pub const GESTURE_RESET_SECONDS: f64 = 0.1;
pub const PLANE_UPDATE_SECONDS: f64 = 1.0;
pub const SCAN_SECONDS: f64 = 2.0;
pub const HIT_TEST_DISTANCE: f64 = 1000.0;
pub const PLANE_EXTENT_SCALE: f64 = 50.0;
pub const LUMENS_TO_INTENSITY: f64 = 1000.0;
pub const PLANE_COLORS: i32 = 10;
pub const SCALE_STEP: f64 = 0.01;
pub const MIN_SCALE: f64 = 0.5;
pub const MAX_SCALE: f64 = 1.5;
pub const ROTATION_SNAP_TRAVEL: f64 = 3.0;
pub const CIRCULAR_MENU_BUTTONS: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Touch {
    pub pressed: bool,
    pub location: DVec2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gestures {
    pub one_finger_pressed: bool,
    pub two_finger_pressed: bool,
    pub recognised: bool,
    pub current: EGestureType,
    pub one_finger_timer: f64,
    pub two_fingers_timer: f64,
    pub reset_timer: f64,
    pub previous_fingers_distance: f64,
}

impl Default for Gestures {
    fn default() -> Self {
        Self {
            one_finger_pressed: false,
            two_finger_pressed: false,
            recognised: false,
            current: EGestureType::Translation,
            one_finger_timer: 0.0,
            two_fingers_timer: 0.0,
            reset_timer: 0.0,
            previous_fingers_distance: 0.0,
        }
    }
}

impl Gestures {
    pub fn reset(&mut self, delta: f64, any_touch: bool) -> bool {
        if !self.recognised {
            return false;
        }

        if any_touch {
            self.reset_timer = 0.0;
            return false;
        }

        self.reset_timer += delta;

        if self.reset_timer > GESTURE_RESET_SECONDS {
            self.recognised = false;
            self.reset_timer = 0.0;
            return true;
        }

        false
    }

    pub fn one_finger(&mut self, delta: f64, object_placed: bool) {
        if object_placed && self.one_finger_pressed && !self.recognised && !self.two_finger_pressed
        {
            self.one_finger_timer += delta;

            if self.one_finger_timer > GESTURE_VALIDATION_SECONDS {
                self.recognised = true;
                self.current = EGestureType::Translation;
            }
        } else {
            self.one_finger_timer = 0.0;
        }
    }

    pub fn two_fingers(
        &mut self,
        delta: f64,
        object_placed: bool,
        fingers_distance: f64,
        fingers_direction: DVec2,
    ) {
        if !(object_placed && self.two_finger_pressed && !self.recognised) {
            self.two_fingers_timer = 0.0;
            return;
        }

        self.two_fingers_timer += delta;

        if self.two_fingers_timer <= GESTURE_VALIDATION_SECONDS {
            return;
        }

        if (self.previous_fingers_distance - fingers_distance).abs() <= 0.001 {
            self.two_fingers_timer = 0.0;
            return;
        }

        self.recognised = true;
        self.current = if fingers_direction.y.abs() < 0.5 {
            EGestureType::Rotation
        } else {
            EGestureType::Scale
        };
    }
}

pub fn normal_2d(v: DVec2) -> DVec2 {
    let length = v.length();

    if length > 1.0e-8 {
        v / length
    } else {
        DVec2::ZERO
    }
}

pub fn grid_snap(value: f64, grid: f64) -> f64 {
    if grid == 0.0 {
        value
    } else {
        ((value + 0.5 * grid) / grid).floor() * grid
    }
}

pub fn snap(v: DVec3, grid: f64, snapping: bool) -> DVec3 {
    if snapping {
        DVec3::new(
            grid_snap(v.x, grid),
            grid_snap(v.y, grid),
            grid_snap(v.z, grid),
        )
    } else {
        v
    }
}

pub fn sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

pub fn next_scale(previous_scale: f64, previous_mapping: f64, mapping: f64) -> f64 {
    (previous_scale + sign(mapping - previous_mapping) * SCALE_STEP).clamp(MIN_SCALE, MAX_SCALE)
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rotation {
    pub distance_travelled: f64,
    pub previous_location: DVec2,
}

impl Rotation {
    pub fn step(
        &mut self,
        direction: DVec2,
        location: DVec2,
        snapping: bool,
        grid: f64,
    ) -> Option<f64> {
        self.distance_travelled += location.distance(self.previous_location);
        self.previous_location = location;

        if !snapping {
            return Some(-direction.x);
        }

        if self.distance_travelled > grid * ROTATION_SNAP_TRAVEL {
            self.distance_travelled = 0.0;
            Some(-direction.x * grid)
        } else {
            None
        }
    }
}

pub fn plane_color_index(geometry_index: Option<usize>) -> Option<usize> {
    geometry_index.map(|index| index % PLANE_COLORS as usize)
}

pub fn light_intensity(lumens: f64) -> f64 {
    lumens / LUMENS_TO_INTENSITY
}

pub fn circular_menu_rotation(button_id: i32) -> f64 {
    f64::from(button_id) / CIRCULAR_MENU_BUTTONS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_finger_held_on_a_placed_object_becomes_a_translation() {
        let mut gestures = Gestures {
            one_finger_pressed: true,
            ..Default::default()
        };

        gestures.one_finger(0.1, true);
        assert!(!gestures.recognised);
        gestures.one_finger(0.15, true);
        assert!(gestures.recognised);
        assert_eq!(gestures.current, EGestureType::Translation);
    }

    #[test]
    fn one_finger_waits_for_a_placed_object() {
        let mut gestures = Gestures {
            one_finger_pressed: true,
            ..Default::default()
        };

        gestures.one_finger(1.0, false);
        assert!(!gestures.recognised);
        assert_eq!(gestures.one_finger_timer, 0.0);
    }

    #[test]
    fn two_fingers_rotate_sideways_and_scale_vertically() {
        let mut rotate = Gestures {
            two_finger_pressed: true,
            previous_fingers_distance: 100.0,
            ..Default::default()
        };

        rotate.two_fingers(0.3, true, 120.0, DVec2::new(1.0, 0.2));
        assert_eq!(rotate.current, EGestureType::Rotation);

        let mut scale = Gestures {
            two_finger_pressed: true,
            previous_fingers_distance: 100.0,
            ..Default::default()
        };

        scale.two_fingers(0.3, true, 140.0, DVec2::new(0.1, 0.9));
        assert!(scale.recognised);
        assert_eq!(scale.current, EGestureType::Scale);
    }

    #[test]
    fn two_still_fingers_are_not_a_gesture() {
        let mut gestures = Gestures {
            two_finger_pressed: true,
            previous_fingers_distance: 100.0,
            ..Default::default()
        };

        gestures.two_fingers(0.3, true, 100.0005, DVec2::X);
        assert!(!gestures.recognised);
        assert_eq!(gestures.two_fingers_timer, 0.0);
    }

    #[test]
    fn a_gesture_ends_shortly_after_the_last_touch() {
        let mut gestures = Gestures {
            recognised: true,
            ..Default::default()
        };

        assert!(!gestures.reset(0.05, false));
        assert!(!gestures.reset(0.01, true));
        assert!(!gestures.reset(0.05, false));
        assert!(gestures.reset(0.06, false));
        assert!(!gestures.recognised);
    }

    #[test]
    fn snapping_rounds_to_the_grid() {
        assert_eq!(
            snap(DVec3::new(7.4, -2.6, 12.5), 5.0, true),
            DVec3::new(5.0, -5.0, 15.0)
        );
        assert_eq!(
            snap(DVec3::new(7.4, 0.0, 0.0), 5.0, false),
            DVec3::new(7.4, 0.0, 0.0)
        );
        assert_eq!(grid_snap(3.0, 0.0), 3.0);
    }

    #[test]
    fn pinching_steps_the_scale_within_bounds() {
        assert!((next_scale(1.0, 0.2, 0.5) - 1.01).abs() < 1.0e-12);
        assert!((next_scale(1.0, 0.5, 0.2) - 0.99).abs() < 1.0e-12);
        assert_eq!(next_scale(1.0, 0.5, 0.5), 1.0);
        assert_eq!(next_scale(1.5, 0.0, 1.0), 1.5);
        assert_eq!(next_scale(0.5, 1.0, 0.0), 0.5);
    }

    #[test]
    fn snapped_rotation_turns_in_steps() {
        let mut rotation = Rotation::default();
        assert_eq!(
            rotation.step(DVec2::X, DVec2::new(30.0, 0.0), true, 20.0),
            None
        );
        assert_eq!(
            rotation.step(DVec2::X, DVec2::new(70.0, 0.0), true, 20.0),
            Some(-20.0)
        );
        assert_eq!(rotation.distance_travelled, 0.0);
        assert_eq!(
            rotation.step(DVec2::new(-0.5, 0.0), DVec2::new(71.0, 0.0), false, 20.0),
            Some(0.5)
        );
    }

    #[test]
    fn planes_cycle_through_ten_colors() {
        assert_eq!(plane_color_index(Some(13)), Some(3));
        assert_eq!(plane_color_index(None), None);
        assert_eq!(light_intensity(1500.0), 1.5);
        assert_eq!(circular_menu_rotation(2), 0.5);
    }

    #[test]
    fn directions_are_normalized() {
        assert_eq!(normal_2d(DVec2::new(3.0, 4.0)), DVec2::new(0.6, 0.8));
        assert_eq!(normal_2d(DVec2::ZERO), DVec2::ZERO);
    }
}
