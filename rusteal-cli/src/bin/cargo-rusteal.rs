use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn forwarded_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut args: Vec<OsString> = args.into_iter().skip(1).collect();

    if args.first().is_some_and(|first| first == "rusteal") {
        args.remove(0);
    }

    args
}

fn rusteal_binary() -> PathBuf {
    let name = format!("rusteal{}", std::env::consts::EXE_SUFFIX);

    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(&name)))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

#[cfg(unix)]
fn run(mut command: Command) -> ExitCode {
    use std::os::unix::process::CommandExt;

    let error = command.exec();
    eprintln!("cargo rusteal: cannot run rusteal: {error}");
    ExitCode::FAILURE
}

#[cfg(not(unix))]
fn run(mut command: Command) -> ExitCode {
    match command.status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8),
        Err(error) => {
            eprintln!("cargo rusteal: cannot run rusteal: {error}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let mut command = Command::new(rusteal_binary());
    command.args(forwarded_args(std::env::args_os()));
    run(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn cargo_passes_the_subcommand_name_first() {
        assert_eq!(
            forwarded_args(args(&[
                "cargo-rusteal",
                "rusteal",
                "watch",
                "--plugin",
                "Gear"
            ])),
            args(&["watch", "--plugin", "Gear"])
        );
    }

    #[test]
    fn a_direct_call_keeps_its_arguments() {
        assert_eq!(
            forwarded_args(args(&["cargo-rusteal", "build", "--all"])),
            args(&["build", "--all"])
        );
    }

    #[test]
    fn no_command_asks_rusteal_for_its_help() {
        assert!(forwarded_args(args(&["cargo-rusteal", "rusteal"])).is_empty());
    }
}
