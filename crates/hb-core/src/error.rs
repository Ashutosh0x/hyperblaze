//! Error types and diagnostics for Hyperblaze.
//!
//! Uses `miette` for beautiful, actionable error messages —
//! a direct improvement over Bazel's often cryptic error output.

use miette::Diagnostic;
use thiserror::Error;

/// The main error type for Hyperblaze.
#[derive(Error, Debug, Diagnostic)]
pub enum HbError {
    #[error("Configuration error: {0}")]
    #[diagnostic(code(hyperblaze::config))]
    Config(String),

    #[error("Workspace not found. Run `hyperblaze init` to create one, or navigate to a directory containing HYPERBLAZE.toml")]
    #[diagnostic(
        code(hyperblaze::workspace::not_found),
        help("Create a workspace with: hyperblaze init")
    )]
    WorkspaceNotFound,

    #[error("Target not found: {target}")]
    #[diagnostic(code(hyperblaze::target::not_found))]
    TargetNotFound {
        target: String,
        #[help]
        suggestion: Option<String>,
    },

    #[error("Build file not found: {path}")]
    #[diagnostic(code(hyperblaze::build_file::not_found))]
    BuildFileNotFound { path: String },

    #[error("Action execution failed: {message}")]
    #[diagnostic(code(hyperblaze::exec::failed))]
    ActionFailed {
        message: String,
        #[help]
        stderr: Option<String>,
    },

    #[error("Dependency cycle detected: {cycle}")]
    #[diagnostic(
        code(hyperblaze::graph::cycle),
        help("Remove the circular dependency to fix this error")
    )]
    DependencyCycle { cycle: String },

    #[error("Graph evaluation error: {0}")]
    #[diagnostic(code(hyperblaze::graph::eval))]
    GraphEvaluation(String),

    #[error("Cache error: {0}")]
    #[diagnostic(code(hyperblaze::cache))]
    Cache(String),

    #[error("Remote error: {0}")]
    #[diagnostic(code(hyperblaze::remote))]
    Remote(String),

    #[error("I/O error: {0}")]
    #[diagnostic(code(hyperblaze::io))]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    #[diagnostic(
        code(hyperblaze::internal),
        help("This is a bug in Hyperblaze. Please report it at https://github.com/hyperblaze-build/hyperblaze/issues")
    )]
    Internal(String),
}

/// Convenience Result type alias.
pub type HbResult<T> = Result<T, HbError>;
