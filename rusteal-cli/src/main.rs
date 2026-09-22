// rusteal: CLI entry point (setup, build, generate, sync-plugin).
//
// Commands act on a Rusteal project: the directory holding the .uproject and
// `rusteal.toml`. It is given as an argument or found by walking up from the
// current directory. The engine location comes from the per-machine config.

mod build_cmd;
mod global_config;
mod new_cmd;
mod setup;
mod sync_plugin;
mod templates;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use rusteal_codegen::config::find_project_root;

#[derive(Parser)]
#[command(name = "rusteal", about = "Rusteal CLI — Rust for Unreal Engine")]
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
        /// Depend on a local Rusteal checkout instead of the published crates.
        #[arg(long)]
        runtime_path: Option<PathBuf>,
        /// Create the project without running the build pipeline.
        #[arg(long)]
        no_build: bool,
    },
    /// Install the Rusteal plugins and a starter rusteal.toml into a UE project.
    Setup {
        /// The UE project directory (the one holding the .uproject).
        project: PathBuf,
    },
    /// Run the build pipeline: UE build, codegen, UE rebuild, cargo build, deploy.
    Build {
        /// Project directory (default: found from the current directory).
        project: Option<PathBuf>,
        /// Run only step N (1-5).
        #[arg(long)]
        step: Option<u8>,
        /// Start from step N (1-5, default: 1).
        #[arg(long, default_value_t = 1)]
        from: u8,
    },
    /// Generate the bindings crate and the C++ wrappers from the reflection JSON.
    Generate {
        /// Project directory (default: found from the current directory).
        project: Option<PathBuf>,
    },
    /// Sync hand-written plugin files into ue_plugin_embed/ for crates.io packaging.
    SyncPlugin,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, dir, runtime_path, no_build } => {
            let engine = global_config::engine_path();
            new_cmd::run_new(&new_cmd::NewOptions {
                name: &name,
                parent: &dir,
                engine: &engine,
                runtime_path: runtime_path.as_deref(),
                build: !no_build,
            });
        }
        Commands::Setup { project } => {
            let engine = global_config::engine_path();
            setup::run_setup(&project, &engine);
        }
        Commands::Build { project, step, from } => {
            let root = project_root(project.as_deref());
            let engine = global_config::engine_path();
            build_cmd::run_build(&root, &engine, step, from);
        }
        Commands::Generate { project } => {
            let root = project_root(project.as_deref());
            rusteal_codegen::run_generate(&root);
        }
        Commands::SyncPlugin => {
            sync_plugin::run_sync();
        }
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
