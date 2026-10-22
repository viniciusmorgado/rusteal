#![deny(unsafe_code)]

rusteal_runtime::entry!();

mod checks;
mod subsystem;
mod utility;

rusteal_runtime::on_load!(checks::register);
