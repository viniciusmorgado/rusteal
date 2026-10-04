// rusteal: CLI entry point (new, plugin, setup, build, generate, package,
// upgrade, sync-plugin).
//
// Commands act on a Rusteal project: the directory holding the .uproject and
// `rusteal.toml`. It is given as an argument or found by walking up from the
// current directory. The engine location comes from the per-machine config.

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

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use rusteal_codegen::config::{find_plugins, find_project_root};

use project_version::Scope;

#[derive(Parser)]
#[command(name = "rusteal", version, about = "Rusteal CLI — Rust for Unreal Engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a UE project with Rust inside and build it.
    New {
        /// Project name: letters and digits, 20 characters at most.
        name: String,
        /// Where to create it (default: the current directory).
        #[arg(long, default_value = ".")]
        dir: PathBuf,
        /// What the project starts as: `blank` (an actor in Rust) or one of
        /// the engine's game templates in Rust (`third-person`,
        /// `first-person`, `top-down`); an unknown name lists them all.
        #[arg(long, default_value = "blank")]
        template: String,
        /// The template's variant (default: `base`, the template itself).
        #[arg(long)]
        variant: Option<String>,
        /// Depend on a local Rusteal checkout instead of the published crates.
        #[arg(long)]
        runtime_path: Option<PathBuf>,
        /// Create the project without running the build pipeline.
        #[arg(long)]
        no_build: bool,
    },
    /// Make and manage Rusteal plugins: UE plugins written in Rust, with their
    /// own library next to the game's.
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },
    /// Install the Rusteal plugins and a starter rusteal.toml into a UE project.
    Setup {
        /// The UE project directory (the one holding the .uproject).
        project: PathBuf,
    },
    /// Build the Rust libraries and deploy them (steps 4-5 of the pipeline);
    /// `--all` runs the whole pipeline: UE build, codegen, UE rebuild, cargo
    /// build, deploy.
    Build {
        /// Project directory (default: found from the current directory).
        project: Option<PathBuf>,
        /// Run every step (1-5): needed after changing rusteal.toml or the
        /// engine, which regenerates the bindings and the C++ wrappers.
        #[arg(long, conflicts_with_all = ["step", "from"])]
        all: bool,
        /// Run only step N (1-5).
        #[arg(long, conflicts_with = "from")]
        step: Option<u8>,
        /// Start from step N (1-5, default: 4, the Rust libraries).
        #[arg(long)]
        from: Option<u8>,
        /// Build the libraries with the release profile, to ship the game
        /// (default: the dev profile, to iterate).
        #[arg(long)]
        release: bool,
        /// Generate, build and deploy only this Rusteal plugin's library
        /// (the UE steps still build the whole project).
        #[arg(long)]
        plugin: Option<String>,
    },
    /// Generate the bindings crates and the C++ wrappers of the game's and the
    /// Rusteal plugins' libraries from the reflection JSON.
    Generate {
        /// Project directory (default: found from the current directory).
        project: Option<PathBuf>,
    },
    /// Build every library with the release profile, then cook and package
    /// the game for this platform (UAT BuildCookRun).
    Package {
        /// Project directory (default: found from the current directory).
        project: Option<PathBuf>,
        /// Where the packaged game goes (default: <project>/Packaged).
        #[arg(long)]
        output: Option<PathBuf>,
        /// Package the Shipping configuration (default: Development).
        #[arg(long)]
        shipping: bool,
    },
    /// Move a project to this CLI's Rusteal version: the pins in Rust/Cargo.toml,
    /// the plugins (Plugins/Rusteal and Plugins/RustealGenerator are replaced
    /// wholesale) and a full build.
    Upgrade {
        /// Project directory (default: found from the current directory).
        project: Option<PathBuf>,
    },
    /// Sync hand-written plugin files into ue_plugin_embed/ for crates.io packaging.
    SyncPlugin,
}

#[derive(Subcommand)]
enum PluginCommands {
    /// Create a Rusteal plugin in a project (Plugins/<Name>/) and build it.
    New {
        /// Plugin name, also its C++ module's: letters and digits.
        name: String,
        /// What the plugin starts as (default: `blank`); an unknown name
        /// lists them all.
        #[arg(long, default_value = templates::DEFAULT_PLUGIN_TEMPLATE)]
        template: String,
        /// The project (default: found from the current directory).
        #[arg(long)]
        project: Option<PathBuf>,
        /// Create the plugin without running the build pipeline.
        #[arg(long)]
        no_build: bool,
    },
    /// Build a Rusteal plugin's library with the release profile and copy
    /// the plugin, without build output, ready for another project.
    Package {
        /// The plugin's name.
        name: String,
        /// The project (default: found from the current directory).
        #[arg(long)]
        project: Option<PathBuf>,
        /// Where the plugin goes (default: <project>/Packaged/Plugins).
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, dir, template, variant, runtime_path, no_build } => {
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
        Commands::Plugin { command: PluginCommands::New { name, template, project, no_build } } => {
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
        Commands::Plugin { command: PluginCommands::Package { name, project, output } } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            let engine = global_config::engine_path();
            let output = output.unwrap_or_else(|| root.join("Packaged/Plugins"));
            package_cmd::run_plugin_package(&root, &engine, &name, &output);
        }
        Commands::Package { project, output, shipping } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            let engine = global_config::engine_path();
            let output = output.unwrap_or_else(|| root.join("Packaged"));
            let configuration = if shipping { "Shipping" } else { "Development" };
            package_cmd::run_package(&root, &engine, &output, configuration);
        }
        Commands::Setup { project } => {
            // A project that already has its Rust workspace keeps the version
            // it pins; the plugins this installs must be that version.
            if project.join("Rust/Cargo.toml").exists() {
                check_version(&project, Scope::Pins);
            }
            let engine = global_config::engine_path();
            setup::run_setup(&project, &engine);
        }
        Commands::Build { project, all, step, from, release, plugin } => {
            let root = project_root(project.as_deref());
            check_version(&root, Scope::PinsAndPlugins);
            let engine = global_config::engine_path();
            let from = first_build_step(all, step, from);
            build_cmd::run_build(&root, &engine, step, from, release, plugin.as_deref());
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

/// The step `rusteal build` starts from: the Rust libraries by default, every
/// step with `--all` (or with `--step`, which ignores it).
fn first_build_step(all: bool, step: Option<u8>, from: Option<u8>) -> u8 {
    from.unwrap_or(if all || step.is_some() { 1 } else { build_cmd::RUST_STEP })
}

/// Stop unless the project is at this CLI's Rusteal version.
fn check_version(root: &Path, scope: Scope) {
    if let Err(message) = project_version::check(root, scope) {
        eprintln!("Error: {message}");
        std::process::exit(1);
    }
}

/// The project root: the directory given, or the nearest ancestor of the
/// current directory holding a .uproject.
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
        assert_eq!(Cli::command().get_version(), Some(env!("CARGO_PKG_VERSION")));
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
            assert!(Cli::try_parse_from(args).is_err(), "{args:?} should be refused");
        }
        assert!(Cli::try_parse_from(["rusteal", "build", "--all", "--release"]).is_ok());
    }
}
