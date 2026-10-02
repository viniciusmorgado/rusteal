// Pure logic for the Shooter variant: no engine calls, fully unit-tested.

use std::collections::HashMap;

use glam::DVec3;

// --- Weapons ------------------------------------------------------------------

/// What pulling the trigger does, `AShooterWeapon::StartFiring`: fire now, or
/// for a full auto weapon still cooling down, fire after a delay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TriggerPull {
    FireNow,
    FireIn(f32),
    Wait,
}

/// `StartFiring` with the time since the last shot.
pub fn pull_trigger(time_since_last_shot: f32, refire_rate: f32, full_auto: bool) -> TriggerPull {
    // check how much time has passed since we last shot
    // this may be under the refire rate if the weapon shoots slow enough and the player is spamming the trigger
    if time_since_last_shot > refire_rate {
        // fire the weapon right away
        TriggerPull::FireNow
    } else if full_auto {
        // if we're full auto, schedule the next shot
        TriggerPull::FireIn(time_since_last_shot)
    } else {
        TriggerPull::Wait
    }
}

/// The bullets left after a shot: one less, a full magazine once it is empty.
pub fn bullets_after_shot(bullets: i32, magazine_size: i32) -> i32 {
    // consume bullets
    let left = bullets - 1;
    // if the clip is depleted, reload it
    if left <= 0 { magazine_size } else { left }
}

/// The weapon after `current` in a list of `count`, wrapping around.
pub fn next_weapon_index(current: usize, count: usize) -> usize {
    // is this the last weapon?
    if current + 1 >= count {
        // loop back to the beginning of the array
        0
    } else {
        // select the next weapon index
        current + 1
    }
}

// --- Senses -------------------------------------------------------------------

/// The cosine a direction's dot product with the forward vector is compared
/// to, for a cone of `half_angle_degrees`.
pub fn cone_cosine(half_angle_degrees: f32) -> f64 {
    f64::from(half_angle_degrees.to_radians().cos())
}

/// Whether `direction` (normalized) is within `half_angle_degrees` of
/// `forward`: the dot product against the cone's cosine.
pub fn within_cone(direction: DVec3, forward: DVec3, half_angle_degrees: f32) -> bool {
    direction.dot(forward) >= cone_cosine(half_angle_degrees)
}

/// The heights above the target's center the line of sight condition traces
/// to: from the top of its bounds down, `checks - 1` of them.
pub fn line_of_sight_heights(extent_z: f64, checks: i32) -> Vec<f64> {
    // divide the vertical extent by the number of line of sight checks we'll do
    let step = extent_z * 2.0 / f64::from(checks.max(1));
    (0..checks - 1).map(|i| extent_z - step * f64::from(i)).collect()
}

/// A remembered stimulus's strength, phased out over the time since it was
/// sensed.
pub fn scaled_stimulus(last_strength: f32, time_since_last: f32) -> f32 {
    last_strength / time_since_last.max(1.0)
}

// --- Teams --------------------------------------------------------------------

/// The score of each team, by team byte.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TeamScores(HashMap<u8, i32>);

impl TeamScores {
    /// Increases the score for the given team and returns it.
    pub fn increment(&mut self, team: u8) -> i32 {
        let score = self.0.entry(team).or_insert(0);
        *score += 1;
        *score
    }
}

/// The team of the `index`th local player (from 2), as the game mode assigns
/// it: alternating, the second on team 0.
pub fn local_player_team(index: i32) -> u8 {
    (1 - index % 2) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_fires_once_cooled_down() {
        assert_eq!(pull_trigger(0.6, 0.5, false), TriggerPull::FireNow);
        assert_eq!(pull_trigger(0.2, 0.5, false), TriggerPull::Wait);
        assert_eq!(pull_trigger(0.2, 0.5, true), TriggerPull::FireIn(0.2));
    }

    #[test]
    fn magazine_reloads_when_empty() {
        assert_eq!(bullets_after_shot(10, 10), 9);
        assert_eq!(bullets_after_shot(1, 10), 10);
    }

    #[test]
    fn weapons_cycle() {
        assert_eq!(next_weapon_index(0, 3), 1);
        assert_eq!(next_weapon_index(2, 3), 0);
    }

    #[test]
    fn cones() {
        assert!(within_cone(DVec3::X, DVec3::X, 10.0));
        assert!(!within_cone(DVec3::Y, DVec3::X, 85.0));
        assert!(within_cone(DVec3::new(1.0, 1.0, 0.0).normalize(), DVec3::X, 46.0));
    }

    #[test]
    fn line_of_sight_traces_from_the_top_down() {
        assert_eq!(line_of_sight_heights(100.0, 5), vec![100.0, 60.0, 20.0, -20.0]);
        assert!(line_of_sight_heights(100.0, 1).is_empty());
    }

    #[test]
    fn stimuli_fade() {
        assert_eq!(scaled_stimulus(4.0, 0.5), 4.0);
        assert_eq!(scaled_stimulus(4.0, 2.0), 2.0);
    }

    #[test]
    fn team_scores_count_up() {
        let mut scores = TeamScores::default();
        assert_eq!(scores.increment(1), 1);
        assert_eq!(scores.increment(1), 2);
        assert_eq!(scores.increment(0), 1);
        assert_eq!(local_player_team(2), 1);
        assert_eq!(local_player_team(3), 0);
    }
}
