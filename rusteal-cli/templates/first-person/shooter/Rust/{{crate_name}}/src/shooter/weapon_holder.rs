// The Shooter variant's C++ interface `IShooterWeaponHolder`, what a weapon
// asks of whoever holds it: only Rust classes implement it and no asset
// reaches it, so it is a Rust trait, and `weapon_holder` finds which class an
// actor is, as C++'s `Cast<IShooterWeaponHolder>(Actor)` does.

use bindings::engine::{Actor, AnimMontage};
use glam::DVec3;
use rusteal_runtime::runtime::{SubclassOf, UObjectRef};

use super::character::ShooterCharacter;
use super::npc::ShooterNPC;
use super::weapon::ShooterWeapon;

/// Common interface for Shooter Game weapon holder classes
pub trait ShooterWeaponHolder {
    /// Attaches a weapon's meshes to the owner
    fn attach_weapon_meshes(&mut self, weapon: &ShooterWeapon);
    /// Plays the firing montage for the weapon
    fn play_firing_montage(&mut self, montage: UObjectRef<AnimMontage>);
    /// Applies weapon recoil to the owner
    fn add_weapon_recoil(&mut self, recoil: f32);
    /// Updates the weapon's HUD with the current ammo count
    fn update_weapon_hud(&mut self, current_ammo: i32, magazine_size: i32);
    /// Calculates and returns the aim location for the weapon
    fn get_weapon_target_location(&mut self) -> DVec3;
    /// Gives a weapon of this class to the owner
    fn add_weapon_class(&mut self, weapon_class: SubclassOf<ShooterWeapon>);
    /// Activates the passed weapon
    fn on_weapon_activated(&mut self, weapon: &ShooterWeapon);
    /// Deactivates the passed weapon
    fn on_weapon_deactivated(&mut self, weapon: &ShooterWeapon);
    /// Notifies the owner that the weapon cooldown has expired and it's ready to shoot again
    fn on_semi_weapon_refire(&mut self);
}

/// `Cast<IShooterWeaponHolder>(Actor)`.
pub fn weapon_holder(actor: UObjectRef<Actor>) -> Option<Box<dyn ShooterWeaponHolder>> {
    if let Ok(character) = ShooterCharacter::from_obj(actor) {
        Some(Box::new(character))
    } else if let Ok(npc) = ShooterNPC::from_obj(actor) {
        Some(Box::new(npc))
    } else {
        None
    }
}
