// The Third Person template's SideScrolling variant in Rust: a side view
// character with double jump, wall jump, soft platforms to drop through and
// physics objects to push; a camera that scrolls along the level; pickups,
// jump pads, moving platforms and NPCs run by a StateTree. Its Blueprint
// children are in `Content/Variant_SideScrolling/`.

mod ai_controller;
mod camera_manager;
mod character;
mod game_mode;
mod interactable;
mod jump_pad;
mod model;
mod moving_platform;
mod npc;
mod pickup;
mod player_controller;
mod soft_platform;
mod state_tree;
mod ui;
