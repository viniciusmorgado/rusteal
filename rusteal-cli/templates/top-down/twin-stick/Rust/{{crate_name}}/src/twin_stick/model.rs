use glam::{DVec2, DVec3};

pub fn aim_angle(axis_x: f32, axis_y: f32) -> f32 {
    axis_y.atan2(-axis_x).to_degrees()
}

pub fn dash_direction(last_move_input: DVec2) -> DVec3 {
    DVec3::new(
        last_move_input.x.clamp(-1.0, 1.0),
        last_move_input.y.clamp(-1.0, 1.0),
        0.0,
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Combo {
    pub score: i32,
    pub combo: i32,
    pub combo_increment: i32,
}

impl Default for Combo {
    fn default() -> Self {
        Combo {
            score: 0,
            combo: 1,
            combo_increment: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComboUpdate {
    pub combo_changed: bool,
    pub restart_cooldown: bool,
}

impl Combo {
    pub fn add_score(&mut self, value: i32, increment_max: i32, cap: i32) -> ComboUpdate {
        self.score += value * self.combo;

        if self.combo > cap {
            return ComboUpdate::default();
        }

        self.combo_increment += 1;

        let mut update = ComboUpdate {
            combo_changed: false,
            restart_cooldown: true,
        };

        if self.combo_increment > increment_max {
            self.combo_increment = 0;
            self.combo += 1;
            update.combo_changed = true;
        }

        update
    }

    pub fn cool_down(&mut self) -> ComboUpdate {
        if self.combo <= 1 {
            return ComboUpdate::default();
        }

        self.combo_increment = 0;
        self.combo -= 1;

        ComboUpdate {
            combo_changed: true,
            restart_cooldown: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_aims_like_the_template() {
        assert!((aim_angle(-1.0, 0.0) - 0.0).abs() < 1e-5);
        assert!((aim_angle(0.0, 1.0) - 90.0).abs() < 1e-5);
        assert!((aim_angle(1.0, 0.0).abs() - 180.0).abs() < 1e-5);
    }

    #[test]
    fn dashes_follow_the_clamped_move_input() {
        assert_eq!(
            dash_direction(DVec2::new(2.0, -0.5)),
            DVec3::new(1.0, -0.5, 0.0)
        );
    }

    #[test]
    fn combo_rises_after_enough_kills_and_falls_back() {
        let mut combo = Combo::default();

        for _ in 0..5 {
            assert_eq!(
                combo.add_score(1, 5, 4),
                ComboUpdate {
                    combo_changed: false,
                    restart_cooldown: true
                }
            );
        }

        assert_eq!(combo.score, 5);
        let update = combo.add_score(1, 5, 4);
        assert!(update.combo_changed);
        assert_eq!(combo.combo, 2);
        combo.add_score(1, 5, 4);
        assert_eq!(combo.score, 8);

        assert_eq!(
            combo.cool_down(),
            ComboUpdate {
                combo_changed: true,
                restart_cooldown: true
            }
        );

        assert_eq!(combo.combo, 1);
        assert_eq!(combo.cool_down(), ComboUpdate::default());
    }

    #[test]
    fn combo_stops_counting_past_the_cap() {
        let mut combo = Combo {
            score: 0,
            combo: 5,
            combo_increment: 0,
        };

        assert_eq!(combo.add_score(2, 5, 4), ComboUpdate::default());
        assert_eq!(combo.score, 10);
        assert_eq!(combo.combo_increment, 0);
    }
}
