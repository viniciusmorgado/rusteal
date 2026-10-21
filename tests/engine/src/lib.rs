#![deny(unsafe_code)]

rusteal_runtime::entry!();

mod checks;
mod fixtures;
mod movement_commandlet;
mod runner;
mod virtuals;
mod widgets_anim;

rusteal_runtime::on_load!(runner::register);
