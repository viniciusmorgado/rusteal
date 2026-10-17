mod build_cmd;
mod console_check;
mod global_config;
mod migrate;
mod new_cmd;
mod optimizations;
mod package_cmd;
mod platform;
mod plugin_cmd;
mod project_version;
mod setup;
mod static_link;
mod sync_plugin;
mod templates;
mod upgrade_cmd;
mod watch_cmd;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod xcode_shim;

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
        #[arg(long = "feature")]
        features: Vec<String>,
        #[arg(long)]
        no_build: bool,
        #[arg(long, value_parser = optimizations::Switch::parse, default_value = "on")]
        optimizations: optimizations::Switch,
        #[arg(long)]
        engine: Option<PathBuf>,
        #[arg(long)]
        in_place: bool,
    },
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
    Platform {
        #[command(subcommand)]
        command: PlatformCommands,
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
        #[arg(long, value_parser = optimizations::Switch::parse, default_value = "on")]
        optimizations: optimizations::Switch,
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
        #[arg(long = "static")]
        link_statically: bool,
        #[arg(long)]
        platform: Option<String>,
        #[arg(long, value_parser = optimizations::Switch::parse, default_value = "on")]
        optimizations: optimizations::Switch,
    },
    Upgrade {
        project: Option<PathBuf>,
    },
    SyncPlugin,
}

#[derive(Subcommand)]
enum PlatformCommands {
    List,
    Show { name: String },
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
            features,
            no_build,
            optimizations,
            engine,
            in_place,
        } => {
            let engine = global_config::engine_path_or(engine.as_deref());

            new_cmd::run_new(&new_cmd::NewOptions {
                name: &name,
                parent: &dir,
                template: &template,
                variant: variant.as_deref(),
                engine: &engine,
                runtime_path: runtime_path.as_deref(),
                features: &features,
                build: !no_build,
                optimizations,
                in_place,
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
            link_statically,
            platform,
            optimizations,
        } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            let engine = global_config::engine_path();
            let output = output.unwrap_or_else(|| root.join("Packaged"));
            let configuration = if shipping { "Shipping" } else { "Development" };
            let target = package_cmd::target_platform(platform.as_deref());

            package_cmd::run_package(
                &root,
                &engine,
                &output,
                configuration,
                optimizations,
                link_statically,
                &target,
            );
        }
        Commands::Platform {
            command: PlatformCommands::List,
        } => {
            for platform in platform::builtins() {
                eprintln!("{} (built in)", platform.name);
            }

            for dir in platform::search_dirs() {
                for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                    if entry.path().extension().is_some_and(|ext| ext == "toml") {
                        eprintln!("{}", entry.path().display());
                    }
                }
            }
        }
        Commands::Platform {
            command: PlatformCommands::Show { name },
        } => {
            let platform = platform::find(&name, &platform::search_dirs()).unwrap_or_else(|e| {
                eprintln!("Error: {e}");
                std::process::exit(1);
            });

            print!("{}", platform.to_toml());
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
        Commands::Watch {
            project,
            plugin,
            optimizations,
        } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            optimizations::report("watch", &root, optimizations);
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

    use super::{Cli, Commands, first_build_step};
    use crate::optimizations::Switch;

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

    #[test]
    fn new_takes_repeated_features() {
        let cli = Cli::try_parse_from([
            "rusteal",
            "new",
            "Game",
            "--template",
            "third-person",
            "--feature",
            "mannequin",
            "--feature",
            "checkpoints",
        ])
        .unwrap();

        let Commands::New { features, .. } = cli.command else {
            panic!("not the new command");
        };

        assert_eq!(features, ["mannequin", "checkpoints"]);
    }

    #[test]
    fn optimizations_are_on_unless_turned_off() {
        let watch = |args: &[&str]| match Cli::try_parse_from(args).unwrap().command {
            Commands::Watch { optimizations, .. } => optimizations,
            _ => panic!("not the watch command"),
        };

        assert_eq!(watch(&["rusteal", "watch"]), Switch::On);
        assert_eq!(
            watch(&["rusteal", "watch", "--optimizations", "off"]),
            Switch::Off
        );
        assert_eq!(
            watch(&["rusteal", "watch", "--optimizations", "0"]),
            Switch::Off
        );
        assert!(Cli::try_parse_from(["rusteal", "package", "--optimizations", "no"]).is_err());
        assert!(Cli::try_parse_from(["rusteal", "new", "G", "--optimizations", "off"]).is_ok());
    }

    #[test]
    fn package_takes_static_and_a_platform() {
        let parsed = Cli::try_parse_from([
            "rusteal",
            "package",
            "--static",
            "--platform",
            "packs/console.toml",
        ])
        .unwrap();

        let Commands::Package {
            link_statically,
            platform,
            ..
        } = parsed.command
        else {
            panic!("not package");
        };

        assert!(link_statically);
        assert_eq!(platform.as_deref(), Some("packs/console.toml"));
        assert!(Cli::try_parse_from(["rusteal", "platform", "show", "Linux"]).is_ok());
        assert!(Cli::try_parse_from(["rusteal", "platform", "list"]).is_ok());
    }
}
