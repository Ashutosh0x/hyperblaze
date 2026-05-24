//! Local process runner — executes build actions as OS processes.

use crate::action::{Action, ActionOutput};
use hb_core::digest::ContentDigest;
use hb_core::error::HbResult;
use std::time::Instant;
use tracing::{debug, info};

/// Executes actions as local OS processes.
pub struct LocalRunner;

impl LocalRunner {
    pub fn new() -> Self {
        Self
    }

    /// Execute a build action locally.
    pub async fn execute(&self, action: &Action) -> HbResult<ActionOutput> {
        let start = Instant::now();

        debug!(
            "Executing action: {} [{}]",
            action.description, action.executable
        );

        let mut cmd = tokio::process::Command::new(&action.executable);
        cmd.args(&action.args)
            .envs(&action.env)
            .current_dir(&action.working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let output = cmd.spawn()?.wait_with_output().await?;

        let elapsed = start.elapsed();

        // Compute output digests
        let mut output_digests = Vec::new();
        for output_path in &action.outputs {
            if output_path.exists() {
                let digest = ContentDigest::of_file(output_path)?;
                output_digests.push((output_path.clone(), digest));
            }
        }

        let result = ActionOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            output_digests,
            execution_time_ms: elapsed.as_millis() as u64,
            cache_hit: false,
        };

        if result.exit_code != 0 {
            info!(
                "Action failed (exit {}): {}",
                result.exit_code, action.description
            );
        } else {
            debug!(
                "Action completed in {}ms: {}",
                result.execution_time_ms, action.description
            );
        }

        Ok(result)
    }
}

impl Default for LocalRunner {
    fn default() -> Self {
        Self::new()
    }
}
