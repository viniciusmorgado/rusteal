#![deny(unsafe_code)]

rusteal_runtime::entry!();

mod checks;
mod fixtures;
mod runner;

rusteal_runtime::on_load!(runner::register);
