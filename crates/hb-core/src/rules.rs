//! Rust build rules -- rust_binary and rust_library.
//!
//! These rules invoke `rustc` to compile real Rust code.
//! Input fingerprinting via BLAKE3 enables cache hits on no-op rebuilds.

use crate::build_file::TargetDef;
use crate::digest;
use crate::error::{HbError, HbResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

/// Result of executing a build rule.
#[derive(Debug, Clone)]
pub struct RuleResult {
    /// Output file path
    pub output: PathBuf,
    /// BLAKE3 digest of all inputs (for cache key)
    pub input_digest: String,
    /// Whether this was a cache hit (skipped compilation)
    pub cached: bool,
    /// Wall-clock time for this action
    pub elapsed_ms: u64,
}

/// Cache entry stored on disk.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    input_digest: String,
    output_digest: String,
}

/// Execute a rust_binary target: compile srcs into an executable.
pub fn execute_rust_binary(
    target: &TargetDef,
    workspace_root: &Path,
    output_dir: &Path,
    dep_outputs: &HashMap<String, PathBuf>,
) -> HbResult<RuleResult> {
    execute_rust_target(target, workspace_root, output_dir, dep_outputs, false)
}

/// Execute a rust_library target: compile srcs into a .rlib.
pub fn execute_rust_library(
    target: &TargetDef,
    workspace_root: &Path,
    output_dir: &Path,
    dep_outputs: &HashMap<String, PathBuf>,
) -> HbResult<RuleResult> {
    execute_rust_target(target, workspace_root, output_dir, dep_outputs, true)
}

fn execute_rust_target(
    target: &TargetDef,
    workspace_root: &Path,
    output_dir: &Path,
    dep_outputs: &HashMap<String, PathBuf>,
    is_library: bool,
) -> HbResult<RuleResult> {
    let start = Instant::now();

    // Resolve source files to absolute paths
    let src_paths: Vec<PathBuf> = target
        .srcs
        .iter()
        .map(|s| workspace_root.join(s))
        .collect();

    // Verify all sources exist
    for src in &src_paths {
        if !src.exists() {
            return Err(HbError::Internal(format!(
                "Source file not found: {}",
                src.display()
            )));
        }
    }

    // Compute BLAKE3 digest of all input files (content-addressed)
    let input_digest = compute_input_digest(&src_paths, dep_outputs)?;

    // Check cache: if input digest matches, skip compilation
    let cache_dir = output_dir.join(".cache");
    std::fs::create_dir_all(&cache_dir).ok();
    let cache_file = cache_dir.join(format!("{}.json", target.name));

    let output_filename = if is_library {
        let crate_name = target
            .crate_name
            .as_deref()
            .unwrap_or(&target.name)
            .replace('-', "_");
        format!("lib{}.rlib", crate_name)
    } else {
        let name = &target.name;
        if cfg!(windows) {
            format!("{}.exe", name)
        } else {
            name.clone()
        }
    };

    let output_path = output_dir.join(&output_filename);

    // Cache check
    if let Ok(cache_content) = std::fs::read_to_string(&cache_file) {
        if let Ok(entry) = serde_json::from_str::<CacheEntry>(&cache_content) {
            if entry.input_digest == input_digest && output_path.exists() {
                return Ok(RuleResult {
                    output: output_path,
                    input_digest,
                    cached: true,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
    }

    // Ensure output directory exists
    std::fs::create_dir_all(output_dir).map_err(|e| {
        HbError::Internal(format!("Failed to create output dir: {}", e))
    })?;

    // Build rustc command
    let mut cmd = Command::new("rustc");

    // Set edition
    cmd.arg("--edition").arg(&target.edition);

    // Set crate type
    if is_library {
        cmd.arg("--crate-type").arg("rlib");
        let crate_name = target
            .crate_name
            .as_deref()
            .unwrap_or(&target.name)
            .replace('-', "_");
        cmd.arg("--crate-name").arg(&crate_name);
    }

    // Output path
    cmd.arg("-o").arg(&output_path);

    // Add dependency extern flags
    for (dep_name, dep_path) in dep_outputs {
        let crate_name = dep_name.replace('-', "_");
        cmd.arg("--extern")
            .arg(format!("{}={}", crate_name, dep_path.display()));
    }

    // Add source files (rustc takes the root source file)
    if let Some(main_src) = src_paths.first() {
        cmd.arg(main_src);
    }

    // Execute rustc
    let output = cmd.output().map_err(|e| {
        HbError::Internal(format!(
            "Failed to execute rustc: {}. Is rustc installed?",
            e
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HbError::Internal(format!(
            "rustc failed for target '{}':\n{}",
            target.name, stderr
        )));
    }

    // Compute output digest and write cache entry
    let output_digest = if output_path.exists() {
        digest::digest_file(&output_path)
            .map(|d| d.hex())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let cache_entry = CacheEntry {
        input_digest: input_digest.clone(),
        output_digest,
    };
    if let Ok(json) = serde_json::to_string(&cache_entry) {
        let _ = std::fs::write(&cache_file, json);
    }

    Ok(RuleResult {
        output: output_path,
        input_digest,
        cached: false,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

/// Compute a combined BLAKE3 digest of all input files + dependency outputs.
fn compute_input_digest(
    src_paths: &[PathBuf],
    dep_outputs: &HashMap<String, PathBuf>,
) -> HbResult<String> {
    let mut digests = Vec::new();

    // Hash each source file
    for src in src_paths {
        let d = digest::digest_file(src).map_err(|e| {
            HbError::Internal(format!("Failed to hash {}: {}", src.display(), e))
        })?;
        digests.push(d);
    }

    // Hash each dependency output
    let mut dep_names: Vec<&String> = dep_outputs.keys().collect();
    dep_names.sort(); // Deterministic ordering
    for dep_name in dep_names {
        if let Some(dep_path) = dep_outputs.get(dep_name) {
            if dep_path.exists() {
                let d = digest::digest_file(dep_path).map_err(|e| {
                    HbError::Internal(format!(
                        "Failed to hash dep {}: {}",
                        dep_path.display(),
                        e
                    ))
                })?;
                digests.push(d);
            }
        }
    }

    Ok(digest::combine_digests(&digests).hex())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_rust_binary_hello_world() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path();

        // Create a hello world source
        let src_dir = workspace.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("main.rs"),
            "fn main() { println!(\"hello from hyperblaze\"); }\n",
        )
        .unwrap();

        let target = TargetDef {
            name: "hello".to_string(),
            rule: "rust_binary".to_string(),
            srcs: vec!["src/main.rs".to_string()],
            deps: vec![],
            edition: "2021".to_string(),
            crate_name: None,
        };

        let output_dir = workspace.join(".hb-out");
        let result =
            execute_rust_binary(&target, workspace, &output_dir, &HashMap::new())
                .unwrap();

        assert!(result.output.exists(), "Binary should exist");
        assert!(!result.cached, "First build should not be cached");
    }

    #[test]
    fn test_cache_hit_on_noop_rebuild() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path();

        let src_dir = workspace.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("main.rs"),
            "fn main() { println!(\"cache test\"); }\n",
        )
        .unwrap();

        let target = TargetDef {
            name: "cache_test".to_string(),
            rule: "rust_binary".to_string(),
            srcs: vec!["src/main.rs".to_string()],
            deps: vec![],
            edition: "2021".to_string(),
            crate_name: None,
        };

        let output_dir = workspace.join(".hb-out");
        let deps = HashMap::new();

        // First build
        let r1 = execute_rust_binary(&target, workspace, &output_dir, &deps).unwrap();
        assert!(!r1.cached, "First build: not cached");

        // Second build (same inputs -- should be cached)
        let r2 = execute_rust_binary(&target, workspace, &output_dir, &deps).unwrap();
        assert!(r2.cached, "Second build: should be cached");
        assert_eq!(r1.input_digest, r2.input_digest);
    }

    #[test]
    fn test_cache_invalidation_on_source_change() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path();

        let src_dir = workspace.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("main.rs"),
            "fn main() { println!(\"v1\"); }\n",
        )
        .unwrap();

        let target = TargetDef {
            name: "invalidation_test".to_string(),
            rule: "rust_binary".to_string(),
            srcs: vec!["src/main.rs".to_string()],
            deps: vec![],
            edition: "2021".to_string(),
            crate_name: None,
        };

        let output_dir = workspace.join(".hb-out");
        let deps = HashMap::new();

        // First build
        let r1 = execute_rust_binary(&target, workspace, &output_dir, &deps).unwrap();
        assert!(!r1.cached);

        // Modify source
        std::fs::write(
            src_dir.join("main.rs"),
            "fn main() { println!(\"v2\"); }\n",
        )
        .unwrap();

        // Rebuild -- should NOT be cached (source changed)
        let r2 = execute_rust_binary(&target, workspace, &output_dir, &deps).unwrap();
        assert!(!r2.cached, "Should recompile after source change");
        assert_ne!(r1.input_digest, r2.input_digest);
    }

    #[test]
    fn test_execute_rust_library() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path();

        let src_dir = workspace.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
        )
        .unwrap();

        let target = TargetDef {
            name: "mylib".to_string(),
            rule: "rust_library".to_string(),
            srcs: vec!["src/lib.rs".to_string()],
            deps: vec![],
            edition: "2021".to_string(),
            crate_name: None,
        };

        let output_dir = workspace.join(".hb-out");
        let result =
            execute_rust_library(&target, workspace, &output_dir, &HashMap::new())
                .unwrap();

        assert!(result.output.exists(), "Library should exist");
        assert!(
            result.output.to_string_lossy().contains("libmylib.rlib"),
            "Should produce .rlib"
        );
    }
}
