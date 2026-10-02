// Pure logic for the TwinStick variant: no engine calls, fully unit-tested.

use glam::{DVec2, DVec3};

/// The aim yaw, in degrees, of a stick deflection, as the C++ character
/// computes it: `Atan2(AxisY, -AxisX)`.
pub fn aim_angle(axis_x: f32, axis_y: f32) -> f32 {
    axis_y.atan2(-axis_x).to_degrees()
}

/// The direction a dash launches the character in: the last move input,
/// each axis clamped to [-1, 1], on the ground plane.
pub fn dash_direction(last_move_input: DVec2) -> DVec3 {
    DVec3::new(last_move_input.x.clamp(-1.0, 1.0), last_move_input.y.clamp(-1.0, 1.0), 0.0)
}

/// The score and its combo multiplier: kills in quick succession raise the
/// multiplier, a pause lowers it again.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Combo {
    /// Current game score
    pub score: i32,
    /// Current combo multiplier
    pub combo: i32,
    /// Current combo increment value
    pub combo_increment: i32,
}

impl Default for Combo {
    fn default() -> Self {
        Combo { score: 0, combo: 1, combo_increment: 0 }
    }
}

/// What a combo change asks of the game mode.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComboUpdate {
    /// The multiplier changed: show it.
    pub combo_changed: bool,
    /// Restart the cooldown after which the multiplier drops.
    pub restart_cooldown: bool,
}

impl Combo {
    /// `ScoreUpdate`: add `value` times the multiplier, then count the kill
    /// towards the next multiplier (`ComboUpdate`).
    pub fn add_score(&mut self, value: i32, increment_max: i32, cap: i32) -> ComboUpdate {
        // multiply the base score by the combo multiplier and add it to the score
        self.score += value * self.combo;

        // return
        if self.combo > cap {
            return ComboUpdate::default();
        }

        // update the combo increment
        self.combo_increment += 1;

        // is it time to increase the multiplier?
        let mut update = ComboUpdate { combo_changed: false, restart_cooldown: true };
        if self.combo_increment > increment_max {
            // reset the combo increment and increase the combo multiplier
            self.combo_increment = 0;
            self.combo += 1;
            update.combo_changed = true;
        }
        update
    }

    /// `ResetCombo`: the cooldown expired, lower the multiplier a step.
    pub fn cool_down(&mut self) -> ComboUpdate {
        // is the combo multiplier above min?
        if self.combo <= 1 {
            return ComboUpdate::default();
        }
        // reset the combo increment and tick down the multiplier
        self.combo_increment = 0;
        self.combo -= 1;
        ComboUpdate { combo_changed: true, restart_cooldown: true }
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
        assert_eq!(dash_direction(DVec2::new(2.0, -0.5)), DVec3::new(1.0, -0.5, 0.0));
    }

    #[test]
    fn combo_rises_after_enough_kills_and_falls_back() {
        let mut combo = Combo::default();
        for _ in 0..5 {
            assert_eq!(combo.add_score(1, 5, 4), ComboUpdate { combo_changed: false, restart_cooldown: true });
        }
        assert_eq!(combo.score, 5);
        let update = combo.add_score(1, 5, 4);
        assert!(update.combo_changed);
        assert_eq!(combo.combo, 2);
        combo.add_score(1, 5, 4);
        assert_eq!(combo.score, 8);

        assert_eq!(combo.cool_down(), ComboUpdate { combo_changed: true, restart_cooldown: true });
        assert_eq!(combo.combo, 1);
        assert_eq!(combo.cool_down(), ComboUpdate::default());
    }

    #[test]
    fn combo_stops_counting_past_the_cap() {
        let mut combo = Combo { score: 0, combo: 5, combo_increment: 0 };
        assert_eq!(combo.add_score(2, 5, 4), ComboUpdate::default());
        assert_eq!(combo.score, 10);
        assert_eq!(combo.combo_increment, 0);
    }
}
