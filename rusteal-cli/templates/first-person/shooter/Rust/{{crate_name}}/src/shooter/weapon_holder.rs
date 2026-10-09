use bindings::engine::{Actor, AnimMontage};
use glam::DVec3;
use rusteal_runtime::runtime::{SubclassOf, UObjectRef};

use super::character::ShooterCharacter;
use super::npc::ShooterNPC;
use super::weapon::ShooterWeapon;

pub trait ShooterWeaponHolder {
    fn attach_weapon_meshes(&mut self, weapon: &ShooterWeapon);
    fn play_firing_montage(&mut self, montage: UObjectRef<AnimMontage>);
    fn add_weapon_recoil(&mut self, recoil: f32);
    fn update_weapon_hud(&mut self, current_ammo: i32, magazine_size: i32);
    fn get_weapon_target_location(&mut self) -> DVec3;
    fn add_weapon_class(&mut self, weapon_class: SubclassOf<ShooterWeapon>);
    fn on_weapon_activated(&mut self, weapon: &ShooterWeapon);
    fn on_weapon_deactivated(&mut self, weapon: &ShooterWeapon);
    fn on_semi_weapon_refire(&mut self);
}

pub fn weapon_holder(actor: UObjectRef<Actor>) -> Option<Box<dyn ShooterWeaponHolder>> {
    if let Ok(character) = ShooterCharacter::from_obj(actor) {
        Some(Box::new(character))
    } else if let Ok(npc) = ShooterNPC::from_obj(actor) {
        Some(Box::new(npc))
    } else {
        None
    }
}
