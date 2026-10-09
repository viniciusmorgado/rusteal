use glam::DVec3;

pub const CAPSULE_RADIUS: f32 = 35.0;
pub const CAPSULE_HALF_HEIGHT: f32 = 90.0;
pub const MAX_WALK_SPEED: f32 = 400.0;
pub const PLAYER_TAG: &str = "Player";

pub const MAX_HP: f32 = 5.0;
pub const ATTACK_INPUT_CACHE_TIME_TOLERANCE: f32 = 1.0;
pub const MELEE_TRACE_DISTANCE: f32 = 75.0;
pub const MELEE_TRACE_RADIUS: f32 = 75.0;
pub const DANGER_TRACE_DISTANCE: f32 = 300.0;
pub const DANGER_TRACE_RADIUS: f32 = 100.0;
pub const MELEE_DAMAGE: f32 = 1.0;
pub const MELEE_KNOCKBACK_IMPULSE: f32 = 250.0;
pub const MELEE_LAUNCH_IMPULSE: f32 = 300.0;
pub const COMBO_INPUT_CACHE_TIME_TOLERANCE: f32 = 0.45;
pub const DEATH_CAMERA_DISTANCE: f32 = 400.0;
pub const DEFAULT_CAMERA_DISTANCE: f32 = 100.0;
pub const RESPAWN_TIME: f32 = 3.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AttackState {
    pub cached_attack_input_time: f64,
    pub is_attacking: bool,
    pub combo_count: i32,
    pub is_charging_attack: bool,
    pub has_looped_charged_attack: bool,
    pub has_released_charged_attack: bool,
}

pub fn input_is_fresh(now: f64, cached: f64, tolerance: f32) -> bool {
    now - cached <= f64::from(tolerance)
}

pub fn knockback(impact_normal: DVec3, knockback: f32, launch: f32) -> DVec3 {
    impact_normal * -f64::from(knockback) + DVec3::Z * f64::from(launch)
}

pub fn safe_normal(v: DVec3) -> DVec3 {
    if v.length_squared() < 1.0e-8 {
        DVec3::ZERO
    } else {
        v.normalize()
    }
}

pub fn take_damage(current_hp: f32, damage: f32) -> Option<(f32, bool)> {
    if current_hp <= 0.0 {
        return None;
    }

    let hp = current_hp - damage;
    Some((hp, hp <= 0.0))
}

pub const ENEMY_MAX_HP: f32 = 3.0;
pub const ENEMY_MELEE_TRACE_DISTANCE: f32 = 75.0;
pub const ENEMY_MELEE_TRACE_RADIUS: f32 = 50.0;
pub const ENEMY_MELEE_DAMAGE: f32 = 1.0;
pub const ENEMY_MELEE_KNOCKBACK_IMPULSE: f32 = 150.0;
pub const ENEMY_MELEE_LAUNCH_IMPULSE: f32 = 350.0;
pub const MIN_CHARGE_LOOPS: i32 = 2;
pub const MAX_CHARGE_LOOPS: i32 = 5;
pub const DEATH_REMOVAL_TIME: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyState {
    pub is_attacking: bool,
    pub target_combo_count: i32,
    pub current_combo_attack: i32,
    pub target_charge_loops: i32,
    pub current_charge_loop: i32,
    pub last_danger_location: DVec3,
    pub last_danger_time: f64,
}

impl Default for EnemyState {
    fn default() -> Self {
        Self {
            is_attacking: false,
            target_combo_count: 0,
            current_combo_attack: 0,
            target_charge_loops: 0,
            current_charge_loop: 0,
            last_danger_location: DVec3::ZERO,
            last_danger_time: -1000.0,
        }
    }
}

pub fn is_in_danger(
    reaction: f64,
    min_reaction: f32,
    max_reaction: f32,
    location: DVec3,
    forward: DVec3,
    danger: DVec3,
    cone_angle_degrees: f32,
) -> bool {
    if !(reaction < f64::from(max_reaction) && reaction > f64::from(min_reaction)) {
        return false;
    }

    let to_danger = safe_normal(DVec3::new(
        danger.x - location.x,
        danger.y - location.y,
        0.0,
    ));

    to_danger.dot(forward) > f64::from(cone_angle_degrees).to_radians().cos()
}

pub const SPAWN_CAPSULE_LOCATION: DVec3 = DVec3::new(0.0, 0.0, 90.0);
pub const INITIAL_SPAWN_DELAY: f32 = 5.0;
pub const SPAWN_COUNT: i32 = 1;
pub const RESPAWN_DELAY: f32 = 5.0;
pub const ACTIVATION_DELAY: f32 = 1.0;

pub const VOLUME_EXTENT: DVec3 = DVec3::new(500.0, 500.0, 500.0);

pub const BOX_HP: f32 = 3.0;
pub const BOX_DEATH_DELAY_TIME: f32 = 6.0;

pub const LAVA_DAMAGE: f32 = 10000.0;

pub const MIN_REACTION_TIME: f32 = 0.35;
pub const MAX_REACTION_TIME: f32 = 0.75;
pub const DANGER_SIGHT_CONE_ANGLE: f32 = 120.0;
pub const CHARACTER_SPEED: f32 = 600.0;
pub const PLAYER_INFO_MAX_RANGE: f32 = 2500.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_until_death() {
        assert_eq!(take_damage(3.0, 1.0), Some((2.0, false)));
        assert_eq!(take_damage(1.0, 1.0), Some((0.0, true)));
        assert_eq!(take_damage(0.0, 1.0), None);
    }

    #[test]
    fn cached_input_expires() {
        assert!(input_is_fresh(10.4, 10.0, COMBO_INPUT_CACHE_TIME_TOLERANCE));

        assert!(!input_is_fresh(
            10.5,
            10.0,
            COMBO_INPUT_CACHE_TIME_TOLERANCE
        ));
    }

    #[test]
    fn knockback_pushes_away_and_up() {
        let impulse = knockback(DVec3::new(-1.0, 0.0, 0.0), 250.0, 300.0);
        assert_eq!(impulse, DVec3::new(250.0, 0.0, 300.0));
    }

    #[test]
    fn danger_needs_the_window_and_the_cone() {
        let ahead = DVec3::new(100.0, 0.0, 50.0);
        let behind = DVec3::new(-100.0, 0.0, 0.0);

        let check = |reaction, danger| {
            is_in_danger(reaction, 0.35, 0.75, DVec3::ZERO, DVec3::X, danger, 120.0)
        };

        assert!(check(0.5, ahead));
        assert!(!check(0.5, behind));
        assert!(!check(0.1, ahead));
        assert!(!check(1.0, ahead));
    }

    #[test]
    fn enemy_starts_without_danger() {
        assert_eq!(EnemyState::default().last_danger_time, -1000.0);
    }
}
