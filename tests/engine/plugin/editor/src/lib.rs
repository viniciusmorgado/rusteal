#![deny(unsafe_code)]

rusteal_runtime::entry!();

mod checks;
mod subsystem;

rusteal_runtime::on_load!(checks::register);
