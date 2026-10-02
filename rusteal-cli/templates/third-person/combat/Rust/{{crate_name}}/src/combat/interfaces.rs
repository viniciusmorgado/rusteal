// The Combat variant's C++ interfaces, `ICombatAttacker`, `ICombatDamageable`
// and `ICombatActivatable`: only Rust classes implement them and no asset
// reaches them, so they are Rust traits, and the functions below find which
// class an actor is, as C++'s `Cast<ICombatDamageable>(Actor)` does.

use bindings::engine::Actor;
use glam::DVec3;
use rusteal_runtime::runtime::UObjectRef;

use super::character::CombatCharacter;
use super::damageable_box::CombatDamageableBox;
use super::dummy::CombatDummy;
use super::enemy::CombatEnemy;
use super::enemy_spawner::CombatEnemySpawner;

/// Attack actions an animation notify asks of its character.
pub trait CombatAttacker {
    /// Performs the collision check for an attack
    fn do_attack_trace(&mut self, damage_source_bone: &str);
    /// Performs the combo string check
    fn check_combo(&mut self);
    /// Performs the charged attack hold check
    fn check_charged_attack(&mut self);
}

/// Something that takes damage, dies, heals and senses incoming attacks.
pub trait CombatDamageable {
    /// Handles damage and knockback events
    fn apply_damage(&mut self, damage: f32, damage_causer: UObjectRef<Actor>, damage_location: DVec3, damage_impulse: DVec3);
    /// Handles death events
    fn handle_death(&mut self);
    /// Handles healing events (part of the interface; nothing in the
    /// template heals)
    #[allow(dead_code)]
    fn apply_healing(&mut self, healing: f32, healer: UObjectRef<Actor>);
    /// Notifies the actor of impending danger
    fn notify_danger(&mut self, danger_location: DVec3, danger_source: UObjectRef<Actor>);
}

/// Something a volume or a spawner activates.
pub trait CombatActivatable {
    /// Toggles the interaction state (part of the interface; the template's
    /// volumes and spawners only activate)
    #[allow(dead_code)]
    fn toggle_interaction(&mut self, activation_instigator: UObjectRef<Actor>);
    /// Activates the actor
    fn activate_interaction(&mut self, activation_instigator: UObjectRef<Actor>);
    /// Deactivates the actor (part of the interface, as above)
    #[allow(dead_code)]
    fn deactivate_interaction(&mut self, activation_instigator: UObjectRef<Actor>);
}

/// `Cast<ICombatAttacker>(Actor)`.
pub fn attacker(actor: UObjectRef<Actor>) -> Option<Box<dyn CombatAttacker>> {
    if let Ok(character) = CombatCharacter::from_obj(actor) {
        Some(Box::new(character))
    } else if let Ok(enemy) = CombatEnemy::from_obj(actor) {
        Some(Box::new(enemy))
    } else {
        None
    }
}

/// `Cast<ICombatDamageable>(Actor)`.
pub fn damageable(actor: UObjectRef<Actor>) -> Option<Box<dyn CombatDamageable>> {
    if let Ok(character) = CombatCharacter::from_obj(actor) {
        Some(Box::new(character))
    } else if let Ok(enemy) = CombatEnemy::from_obj(actor) {
        Some(Box::new(enemy))
    } else if let Ok(damageable_box) = CombatDamageableBox::from_obj(actor) {
        Some(Box::new(damageable_box))
    } else if let Ok(dummy) = CombatDummy::from_obj(actor) {
        Some(Box::new(dummy))
    } else {
        None
    }
}

/// `Cast<ICombatActivatable>(Actor)`.
pub fn activatable(actor: UObjectRef<Actor>) -> Option<Box<dyn CombatActivatable>> {
    CombatEnemySpawner::from_obj(actor)
        .ok()
        .map(|spawner| Box::new(spawner) as Box<dyn CombatActivatable>)
}
