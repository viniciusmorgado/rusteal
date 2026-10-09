mod build_cmd;
mod global_config;
mod new_cmd;
mod package_cmd;
mod plugin_cmd;
mod project_version;
mod setup;
mod sync_plugin;
mod templates;
mod upgrade_cmd;
mod watch_cmd;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use rusteal_codegen::config::{find_plugins, find_project_root};

use project_version::Scope;

#[derive(Parser)]
#[command(
    name = "rusteal",
    version,
    about = "Rusteal CLI — Rust for Unreal Engine"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    New {
        name: String,
        #[arg(long, default_value = ".")]
        dir: PathBuf,
        #[arg(long, default_value = "blank")]
        template: String,
        #[arg(long)]
        variant: Option<String>,
        #[arg(long)]
        runtime_path: Option<PathBuf>,
        #[arg(long)]
        no_build: bool,
    },
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
    Setup {
        project: PathBuf,
    },
    Build {
        project: Option<PathBuf>,
        #[arg(long, conflicts_with_all = ["step", "from"])]
        all: bool,
        #[arg(long, conflicts_with = "from")]
        step: Option<u8>,
        #[arg(long)]
        from: Option<u8>,
        #[arg(long)]
        release: bool,
        #[arg(long)]
        plugin: Option<String>,
    },
    Watch {
        project: Option<PathBuf>,
        #[arg(long)]
        plugin: Option<String>,
    },
    Generate {
        project: Option<PathBuf>,
    },
    Package {
        project: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        shipping: bool,
    },
    Upgrade {
        project: Option<PathBuf>,
    },
    SyncPlugin,
}

#[derive(Subcommand)]
enum PluginCommands {
    New {
        name: String,
        #[arg(long, default_value = templates::DEFAULT_PLUGIN_TEMPLATE)]
        template: String,
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        no_build: bool,
    },
    Package {
        name: String,
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            name,
            dir,
            template,
            variant,
            runtime_path,
            no_build,
        } => {
            let engine = global_config::engine_path();

            new_cmd::run_new(&new_cmd::NewOptions {
                name: &name,
                parent: &dir,
                template: &template,
                variant: variant.as_deref(),
                engine: &engine,
                runtime_path: runtime_path.as_deref(),
                build: !no_build,
            });
        }
        Commands::Plugin {
            command:
                PluginCommands::New {
                    name,
                    template,
                    project,
                    no_build,
                },
        } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            let engine = global_config::engine_path();

            plugin_cmd::run_plugin_new(&plugin_cmd::PluginNewOptions {
                name: &name,
                template: &template,
                root: &root,
                engine: &engine,
                build: !no_build,
            });
        }
        Commands::Plugin {
            command:
                PluginCommands::Package {
                    name,
                    project,
                    output,
                },
        } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            let engine = global_config::engine_path();
            let output = output.unwrap_or_else(|| root.join("Packaged/Plugins"));
            package_cmd::run_plugin_package(&root, &engine, &name, &output);
        }
        Commands::Package {
            project,
            output,
            shipping,
        } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            let engine = global_config::engine_path();
            let output = output.unwrap_or_else(|| root.join("Packaged"));
            let configuration = if shipping { "Shipping" } else { "Development" };
            package_cmd::run_package(&root, &engine, &output, configuration);
        }
        Commands::Setup { project } => {
            if project.join("Rust/Cargo.toml").exists() {
                check_version(&project, Scope::Pins);
            }

            let engine = global_config::engine_path();
            setup::run_setup(&project, &engine);
        }
        Commands::Build {
            project,
            all,
            step,
            from,
            release,
            plugin,
        } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            let engine = global_config::engine_path();
            let from = first_build_step(all, step, from);
            build_cmd::run_build(&root, &engine, step, from, release, plugin.as_deref());
        }
        Commands::Watch { project, plugin } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            watch_cmd::run_watch(&root, plugin.as_deref());
        }
        Commands::Generate { project } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            rusteal_codegen::run_generate(&root);

            for plugin in find_plugins(&root) {
                rusteal_codegen::run_generate_plugin(&root, &plugin);
            }
        }
        Commands::Upgrade { project } => {
            let root = project_root(project.as_deref());
            upgrade_cmd::run_upgrade(&root);
        }
        Commands::SyncPlugin => {
            sync_plugin::run_sync();
        }
    }
}

fn first_build_step(all: bool, step: Option<u8>, from: Option<u8>) -> u8 {
    from.unwrap_or(if all || step.is_some() {
        1
    } else {
        build_cmd::RUST_STEP
    })
}

fn check_version(root: &Path, scope: Scope) {
    if let Err(message) = project_version::check(root, scope) {
        eprintln!("Error: {message}");
        std::process::exit(1);
    }
}

fn project_root(given: Option<&Path>) -> PathBuf {
    let start = given
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::env::current_dir().expect("current directory"));

    find_project_root(&start).unwrap_or_else(|| {
        eprintln!(
            "Error: no .uproject in {} or its parents. Pass the project directory.",
            start.display()
        );

        std::process::exit(1);
    })
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::{Cli, first_build_step};

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn version_is_the_crate_version() {
        assert_eq!(
            Cli::command().get_version(),
            Some(env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn build_runs_the_rust_steps_unless_told_otherwise() {
        assert_eq!(first_build_step(false, None, None), 4);
        assert_eq!(first_build_step(true, None, None), 1);
        assert_eq!(first_build_step(false, None, Some(2)), 2);
        assert_eq!(first_build_step(false, Some(3), None), 1);
    }

    #[test]
    fn build_all_excludes_step_and_from() {
        for args in [
            &["rusteal", "build", "--all", "--from", "2"][..],
            &["rusteal", "build", "--all", "--step", "2"],
            &["rusteal", "build", "--step", "2", "--from", "3"],
        ] {
            assert!(
                Cli::try_parse_from(args).is_err(),
                "{args:?} should be refused"
            );
        }

        assert!(Cli::try_parse_from(["rusteal", "build", "--all", "--release"]).is_ok());
    }
}
