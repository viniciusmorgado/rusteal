#![deny(unsafe_code)]

rusteal_runtime::entry!();

mod checks;
mod fixtures;
mod gas;
mod interfaces;
mod libraries;
mod movement_commandlet;
mod net;
mod reload;
mod runner;
mod tags;
mod virtuals;
mod widgets_anim;

rusteal_runtime::on_load!(runner::register);
rusteal_runtime::on_load!(reload::register);
rusteal_runtime::on_load!(tags::register);
rusteal_runtime::on_load!(net::register);
