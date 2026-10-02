// The Shooter variant's gameplay classes in Rust: a first person arena
// shooter with weapon pickups (a weapon data table of `WeaponTableRow`s),
// projectiles, teams and scores, and NPCs run by a StateTree with AI
// perception and an EnvQuery.

pub mod ai_controller;
pub mod character;
pub mod env_query;
pub mod game_mode;
mod model;
pub mod npc;
pub mod npc_spawner;
pub mod pickup;
pub mod player_controller;
pub mod projectile;
pub mod state_tree;
pub mod ui;
pub mod weapon;
mod weapon_holder;
