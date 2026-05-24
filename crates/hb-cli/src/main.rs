//! # Hyperblaze CLI — the user-facing binary
//!
//! 🔥 Next-generation build system.
//! Faster than Bazel. Smarter than everything else.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;

mod commands;

/// 🔥 Hyperblaze — Next-generation build system
#[derive(Parser)]
#[command(
    name = "hyperblaze",
    version,
    about = "🔥 Hyperblaze — Next-generation build system\nFaster than Bazel. Smarter than everything else.",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Quiet mode — only show errors
    #[arg(short, long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the specified targets
    Build {
        /// Targets to build (e.g., //src:main, //...)
        #[arg(default_value = "//...")]
        targets: Vec<String>,

        /// Number of parallel jobs (0 = auto)
        #[arg(short, long, default_value = "0")]
        jobs: usize,
    },

    /// Run tests
    Test {
        /// Test targets
        #[arg(default_value = "//...")]
        targets: Vec<String>,
    },

    /// Run a binary target
    Run {
        /// Target to run
        target: String,

        /// Arguments to pass to the binary
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Clean build outputs
    Clean {
        /// Also remove the action cache
        #[arg(long)]
        expunge: bool,
    },

    /// Initialize a new Hyperblaze workspace
    Init {
        /// Directory to initialize (default: current)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Project name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Query the dependency graph
    Query {
        /// Query expression
        expression: String,
    },

    /// Format BUILD.hb files
    Fmt {
        /// Files or directories to format
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,
    },

    /// Show build system information
    Info,

    /// Diagnose common issues
    Doctor,

    /// Show the dependency graph
    Graph {
        /// Target to show graph for
        target: String,
    },
}

#[tokio::main]
async fn main() -> miette::Result<()> {
    let start = Instant::now();
    let cli = Cli::parse();

    // Set up tracing based on verbosity
    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .with_target(false)
        .init();

    match cli.command {
        Commands::Build { targets, jobs } => {
            commands::build::run(&targets, jobs, cli.quiet).await?;
        }
        Commands::Test { targets } => {
            commands::test::run(&targets, cli.quiet).await?;
        }
        Commands::Run { target, args } => {
            commands::run::run(&target, &args).await?;
        }
        Commands::Clean { expunge } => {
            commands::clean::run(expunge)?;
        }
        Commands::Init { path, name } => {
            commands::init::run(&path, name.as_deref())?;
        }
        Commands::Info => {
            commands::info::run(start)?;
        }
        Commands::Doctor => {
            commands::doctor::run().await?;
        }
        Commands::Query { expression } => {
            println!("Query: {} (not yet implemented)", expression);
        }
        Commands::Fmt { paths } => {
            println!("Format: {:?} (not yet implemented)", paths);
        }
        Commands::Graph { target } => {
            println!("Graph: {} (not yet implemented)", target);
        }
    }

    Ok(())
}
