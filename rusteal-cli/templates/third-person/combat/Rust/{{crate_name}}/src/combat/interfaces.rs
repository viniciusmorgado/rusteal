use bindings::engine::Actor;
use glam::DVec3;
use rusteal_runtime::runtime::{RustRef, UObjectRef};

use super::character::CombatCharacter;
use super::damageable_box::CombatDamageableBox;
use super::dummy::CombatDummy;
use super::enemy::CombatEnemy;
use super::enemy_spawner::CombatEnemySpawner;

pub trait CombatAttacker {
    fn do_attack_trace(&mut self, damage_source_bone: &str);
    fn check_combo(&mut self);
    fn check_charged_attack(&mut self);
}

pub trait CombatDamageable {
    fn apply_damage(
        &mut self,
        damage: f32,
        damage_causer: UObjectRef<Actor>,
        damage_location: DVec3,
        damage_impulse: DVec3,
    );
    fn handle_death(&mut self);
    #[allow(dead_code)]
    fn apply_healing(&mut self, healing: f32, healer: UObjectRef<Actor>);
    fn notify_danger(&mut self, danger_location: DVec3, danger_source: UObjectRef<Actor>);
}

pub trait CombatActivatable {
    #[allow(dead_code)]
    fn toggle_interaction(&mut self, activation_instigator: UObjectRef<Actor>);
    fn activate_interaction(&mut self, activation_instigator: UObjectRef<Actor>);
    #[allow(dead_code)]
    fn deactivate_interaction(&mut self, activation_instigator: UObjectRef<Actor>);
}

/// The Rust class behind `actor` as a `CombatAttacker`, for this call.
pub fn attacker(actor: UObjectRef<'_, Actor>) -> Option<RustRef<'_, Box<dyn CombatAttacker>>> {
    if let Ok(character) = CombatCharacter::from_obj(actor) {
        Some(character.map(|c| Box::new(c) as Box<_>))
    } else if let Ok(enemy) = CombatEnemy::from_obj(actor) {
        Some(enemy.map(|c| Box::new(c) as Box<_>))
    } else {
        None
    }
}

/// The Rust class behind `actor` as a `CombatDamageable`, for this call.
pub fn damageable(actor: UObjectRef<'_, Actor>) -> Option<RustRef<'_, Box<dyn CombatDamageable>>> {
    if let Ok(character) = CombatCharacter::from_obj(actor) {
        Some(character.map(|c| Box::new(c) as Box<_>))
    } else if let Ok(enemy) = CombatEnemy::from_obj(actor) {
        Some(enemy.map(|c| Box::new(c) as Box<_>))
    } else if let Ok(damageable_box) = CombatDamageableBox::from_obj(actor) {
        Some(damageable_box.map(|c| Box::new(c) as Box<_>))
    } else if let Ok(dummy) = CombatDummy::from_obj(actor) {
        Some(dummy.map(|c| Box::new(c) as Box<_>))
    } else {
        None
    }
}

/// The Rust class behind `actor` as a `CombatActivatable`, for this call.
pub fn activatable(
    actor: UObjectRef<'_, Actor>,
) -> Option<RustRef<'_, Box<dyn CombatActivatable>>> {
    CombatEnemySpawner::from_obj(actor)
        .ok()
        .map(|spawner| spawner.map(|s| Box::new(s) as Box<dyn CombatActivatable>))
}
