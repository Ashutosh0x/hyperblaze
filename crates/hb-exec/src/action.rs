//! Action model — represents a build action to execute.

use hb_core::digest::ContentDigest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A build action — a command to execute with inputs and outputs.
///
/// Equivalent to Bazel's Spawn, but simpler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Unique identifier for this action
    pub id: String,
    /// Human-readable description (e.g., "Compiling main.rs")
    pub description: String,
    /// The executable to run
    pub executable: String,
    /// Command-line arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: BTreeMap<String, String>,
    /// Working directory
    pub working_dir: PathBuf,
    /// Input files (paths + digests)
    pub inputs: Vec<ActionInput>,
    /// Expected output files
    pub outputs: Vec<PathBuf>,
    /// Resource requirements
    pub resources: ResourceReq,
}

/// An input to a build action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionInput {
    pub path: PathBuf,
    pub digest: ContentDigest,
}

/// Resource requirements for an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReq {
    /// CPU cores needed (fractional OK, e.g., 0.5)
    pub cpu: f64,
    /// Memory in bytes
    pub memory: u64,
}

impl Default for ResourceReq {
    fn default() -> Self {
        Self {
            cpu: 1.0,
            memory: 256 * 1024 * 1024, // 256 MB
        }
    }
}

/// The result of executing an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub output_digests: Vec<(PathBuf, ContentDigest)>,
    pub execution_time_ms: u64,
    pub cache_hit: bool,
}

impl Action {
    /// Compute the cache key for this action.
    pub fn cache_key(&self) -> ContentDigest {
        let input_digests: Vec<ContentDigest> =
            self.inputs.iter().map(|i| i.digest.clone()).collect();
        let env_pairs: Vec<(String, String)> =
            self.env.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        hb_core::digest::action_cache_key(&input_digests, &self.args, &env_pairs)
    }
}
