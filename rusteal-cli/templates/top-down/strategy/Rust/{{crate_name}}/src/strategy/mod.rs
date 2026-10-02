// The Strategy variant's gameplay classes in Rust: an isometric camera pawn
// and a player controller that selects units with clicks, boxes and touches
// and sends them to move along the navigation mesh, where they interact
// with the units they reach.

pub mod env_query;
pub mod game_mode;
pub mod hud;
mod model;
pub mod pawn;
pub mod player_controller;
pub mod touch_controls;
pub mod ui;
pub mod unit;
