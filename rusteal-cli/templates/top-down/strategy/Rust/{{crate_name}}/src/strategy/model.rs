use glam::{DQuat, DVec2, DVec3};

use rusteal_runtime::runtime::Rotator;

const ISO_YAW: f64 = -45.0;

pub fn camera_move_scales(input: DVec2) -> (f64, f64) {
    (input.x + input.y, input.x - input.y)
}

pub fn drag_scroll_offset(start: DVec2, current: DVec2, drag_multiplier: f32) -> DVec3 {
    let move_delta = start - current;

    let iso_rotation = DQuat::from(Rotator::new(0.0, ISO_YAW, 0.0));
    let rotated_delta = iso_rotation * DVec3::new(move_delta.x, move_delta.y, 0.0);

    rotated_delta * f64::from(drag_multiplier)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomRange {
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl ZoomRange {
    pub fn modified(&self, zoom: f32, delta: f32) -> f32 {
        (zoom + delta).clamp(self.min, self.max)
    }

    pub fn at_percentage(&self, percentage: f32) -> f32 {
        self.min + (self.max - self.min) * percentage.clamp(0.0, 1.0)
    }

    pub fn default_percentage(&self) -> f32 {
        ((self.default - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }
}

pub fn closest_to(locations: &[DVec3], target: DVec3) -> Option<usize> {
    let dist_squared_2d = |location: DVec3| (location - target).truncate().length_squared();
    let mut closest: Option<(usize, f64)> = None;

    for (index, location) in locations.iter().enumerate() {
        let dist = dist_squared_2d(*location);

        if closest.is_none_or(|(_, closest_dist)| dist < closest_dist) {
            closest = Some((index, dist));
        }
    }

    closest.map(|(index, _)| index)
}

pub fn line_plane_intersection(
    start: DVec3,
    end: DVec3,
    plane_origin: DVec3,
    plane_normal: DVec3,
) -> DVec3 {
    let direction = end - start;
    start + direction * ((plane_origin - start).dot(plane_normal) / direction.dot(plane_normal))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: DVec3, b: DVec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn camera_moves_along_both_axes_per_input() {
        assert_eq!(camera_move_scales(DVec2::new(1.0, 0.0)), (1.0, 1.0));
        assert_eq!(camera_move_scales(DVec2::new(0.0, 1.0)), (1.0, -1.0));
        assert_eq!(camera_move_scales(DVec2::new(-1.0, 1.0)), (0.0, -2.0));
    }

    #[test]
    fn drag_scroll_is_rotated_to_the_isometric_view_and_scaled() {
        let offset = drag_scroll_offset(DVec2::new(100.0, 0.0), DVec2::ZERO, 0.1);
        let half_sqrt2 = std::f64::consts::FRAC_1_SQRT_2;

        assert!(close(
            offset,
            DVec3::new(10.0 * half_sqrt2, -10.0 * half_sqrt2, 0.0)
        ));

        assert!(close(
            drag_scroll_offset(DVec2::ONE, DVec2::ONE, 0.1),
            DVec3::ZERO
        ));
    }

    #[test]
    fn zoom_stays_in_range() {
        let range = ZoomRange {
            min: 1000.0,
            max: 2500.0,
            default: 1500.0,
        };

        assert_eq!(range.modified(2450.0, 100.0), 2500.0);
        assert_eq!(range.modified(1050.0, -100.0), 1000.0);
        assert_eq!(range.modified(1500.0, 100.0), 1600.0);
        assert_eq!(range.at_percentage(0.5), 1750.0);
        assert_eq!(range.at_percentage(2.0), 2500.0);
        assert_eq!(range.default_percentage(), 1.0 / 3.0);

        assert_eq!(
            ZoomRange {
                default: 500.0,
                ..range
            }
            .default_percentage(),
            0.0
        );
    }

    #[test]
    fn closest_ignores_height_and_keeps_the_first_tie() {
        let locations = [
            DVec3::new(300.0, 0.0, 0.0),
            DVec3::new(100.0, 0.0, 900.0),
            DVec3::new(-100.0, 0.0, 0.0),
        ];

        assert_eq!(closest_to(&locations, DVec3::ZERO), Some(1));
        assert_eq!(closest_to(&[], DVec3::ZERO), None);
    }

    #[test]
    fn line_crosses_the_ground_plane() {
        let hit = line_plane_intersection(
            DVec3::new(0.0, 0.0, 1500.0),
            DVec3::new(100.0, 50.0, 500.0),
            DVec3::ZERO,
            DVec3::Z,
        );

        assert!(close(hit, DVec3::new(150.0, 75.0, 0.0)));
    }
}
