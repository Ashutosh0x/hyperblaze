//! `hyperblaze build` command

use hb_core::config::HyperblazeConfig;
use hb_core::error::HbResult;
use hb_graph::{Evaluator, HyperGraph, NodeKey, NodeValue};
use hb_graph::function::compute_fn;
use hb_graph::key::FunctionType;
use std::sync::Arc;
use std::time::Instant;

pub async fn run(targets: &[String], jobs: usize, quiet: bool) -> HbResult<()> {
    let start = Instant::now();
    let cwd = std::env::current_dir()?;

    // Find workspace root
    let workspace_root = HyperblazeConfig::find_workspace_root(&cwd)
        .unwrap_or_else(|| cwd.clone());

    let config = HyperblazeConfig::load(&workspace_root)?;
    let effective_jobs = if jobs > 0 { jobs } else { config.effective_jobs() };

    if !quiet {
        println!();
        println!(
            "  🔥 \x1b[1;38;5;208mHyperblaze\x1b[0m v{} — Building {} target{}",
            hb_core::VERSION,
            targets.len(),
            if targets.len() == 1 { "" } else { "s" }
        );
        println!("     Workspace: {}", workspace_root.display());
        println!("     Jobs: {} | Cache: {}", effective_jobs, if config.build.disk_cache { "on" } else { "off" });
        println!();
    }

    // Create graph and evaluator
    let graph = Arc::new(HyperGraph::new());
    let evaluator = Evaluator::new(graph.clone());

    // Register built-in compute functions
    register_builtins(&evaluator);

    // For MVP: demonstrate the engine working
    // In full implementation, we'd parse BUILD.hb files and resolve targets
    let keys: Vec<NodeKey> = targets
        .iter()
        .map(|t| NodeKey::configured_target(t))
        .collect();

    let result = evaluator.evaluate_many(&keys).await?;

    let elapsed = start.elapsed();

    if !quiet {
        println!(
            "  \x1b[1;32m✅ Build successful\x1b[0m in {:.1}s",
            elapsed.as_secs_f64()
        );
        println!(
            "     {} targets built, {} cached, {} evaluated, {} failed",
            result.values.len(),
            result.cached,
            result.evaluated,
            result.errors
        );
        println!(
            "     Graph: {} nodes | {}",
            graph.node_count(),
            graph.metrics().summary()
        );
        println!();
    }

    Ok(())
}

/// Register built-in compute functions for the evaluator.
fn register_builtins(evaluator: &Evaluator) {
    // File state function
    evaluator.register(
        FunctionType::FileState,
        compute_fn(|key, _ctx| async move {
            let path = key.argument();
            let path = std::path::Path::new(path);
            if path.exists() {
                let digest = hb_core::digest::ContentDigest::of_file(path)
                    .map_err(|e| hb_core::error::HbError::Io(e))?;
                Ok(NodeValue::new(hb_graph::value::FileStateValue {
                    path: key.argument().to_string(),
                    digest: digest.clone(),
                    size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
                    mtime: 0,
                    exists: true,
                }))
            } else {
                Ok(NodeValue::new(hb_graph::value::FileStateValue {
                    path: key.argument().to_string(),
                    digest: hb_core::digest::ContentDigest::empty(),
                    size: 0,
                    mtime: 0,
                    exists: false,
                }))
            }
        }),
    );

    // Configured target function (placeholder for MVP)
    evaluator.register(
        FunctionType::ConfiguredTarget,
        compute_fn(|key, _ctx| async move {
            Ok(NodeValue::new(format!("configured:{}", key.argument())))
        }),
    );

    // Package loading function (placeholder for MVP)
    evaluator.register(
        FunctionType::PackageLoad,
        compute_fn(|key, _ctx| async move {
            Ok(NodeValue::new(format!("package:{}", key.argument())))
        }),
    );
}
