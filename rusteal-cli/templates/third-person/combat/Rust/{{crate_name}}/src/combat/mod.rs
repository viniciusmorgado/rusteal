// The Third Person template's Combat variant in Rust: a melee combat character
// (combo string, charged attack, damage, death and respawn), enemies run by a
// StateTree and spawned by spawners, activation and checkpoint volumes,
// damageable boxes, a training dummy and lava. Its Blueprint children are in
// `Content/Variant_Combat/`.

mod ai_controller;
mod anim_notifies;
mod character;
mod damageable_box;
mod dummy;
mod enemy;
mod enemy_spawner;
mod env_query;
mod game_mode;
mod interfaces;
mod lava_floor;
mod life_bar;
mod model;
mod player_controller;
mod state_tree;
mod volumes;
