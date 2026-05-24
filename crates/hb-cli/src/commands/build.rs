//! `hyperblaze build` command
//!
//! Parses BUILD.hb, resolves targets, fingerprints inputs via BLAKE3,
//! invokes rustc for real compilation, and caches results.

use hb_core::build_file::BuildFile;
use hb_core::config::HyperblazeConfig;
use hb_core::error::HbResult;
use hb_core::rules;
use std::collections::HashMap;
use std::path::PathBuf;
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
            "  \x1b[1;38;5;208mHyperblaze\x1b[0m v{} -- Building",
            hb_core::VERSION,
        );
        println!("     Workspace: {}", workspace_root.display());
        println!(
            "     Jobs: {} | Cache: {}",
            effective_jobs,
            if config.build.disk_cache { "on" } else { "off" }
        );
        println!();
    }

    // Look for BUILD.hb in workspace root
    let build_file_path = workspace_root.join("BUILD.hb");
    if !build_file_path.exists() {
        // Fallback: no BUILD.hb, try auto-detect
        if !quiet {
            println!(
                "  \x1b[33mNo BUILD.hb found.\x1b[0m Run `hyperblaze init` or create BUILD.hb"
            );
            println!();
        }
        println!("  \x1b[1;32mBuild successful\x1b[0m in {:.1}s", start.elapsed().as_secs_f64());
        println!("     0 targets built");
        println!();
        return Ok(());
    }

    // Parse BUILD.hb
    let build_file = BuildFile::parse(&build_file_path)?;

    if !quiet {
        println!(
            "     Found {} target{} in BUILD.hb",
            build_file.target.len(),
            if build_file.target.len() == 1 { "" } else { "s" }
        );
    }

    // Resolve which targets to build
    let targets_to_build = if targets.is_empty() || targets.iter().any(|t| t == "//...") {
        // Build all targets
        build_file.target.clone()
    } else {
        // Filter to requested targets
        let mut matched = Vec::new();
        for pattern in targets {
            // Strip label prefix (e.g. "//src:main" -> "main", "//..." -> all)
            let name = pattern
                .rsplit(':')
                .next()
                .unwrap_or(pattern)
                .trim_start_matches('/');

            for t in &build_file.target {
                if t.name == name || pattern == "//..." {
                    matched.push(t.clone());
                }
            }
        }
        matched
    };

    if targets_to_build.is_empty() {
        if !quiet {
            println!("  \x1b[33mNo matching targets found.\x1b[0m");
            println!();
        }
        return Ok(());
    }

    let output_dir = workspace_root.join(&config.build.output_base);
    let mut dep_outputs: HashMap<String, PathBuf> = HashMap::new();
    let mut total_cached = 0usize;
    let mut total_built = 0usize;
    let mut total_failed = 0usize;

    // Topological build order: libraries first, then binaries
    let libs: Vec<_> = targets_to_build
        .iter()
        .filter(|t| t.rule == "rust_library")
        .collect();
    let bins: Vec<_> = targets_to_build
        .iter()
        .filter(|t| t.rule == "rust_binary")
        .collect();

    // Build libraries first
    for target in &libs {
        if !quiet {
            print!("     Compiling {} (rust_library)...", target.name);
        }
        match rules::execute_rust_library(target, &workspace_root, &output_dir, &dep_outputs) {
            Ok(result) => {
                dep_outputs.insert(target.name.clone(), result.output.clone());
                if result.cached {
                    total_cached += 1;
                    if !quiet {
                        println!(" \x1b[36mcached\x1b[0m ({}ms)", result.elapsed_ms);
                    }
                } else {
                    total_built += 1;
                    if !quiet {
                        println!(" \x1b[32mok\x1b[0m ({}ms)", result.elapsed_ms);
                    }
                }
            }
            Err(e) => {
                total_failed += 1;
                if !quiet {
                    println!(" \x1b[31mFAILED\x1b[0m");
                    eprintln!("     {}", e);
                }
            }
        }
    }

    // Build binaries (with library deps available)
    for target in &bins {
        // Collect deps for this target
        let mut target_deps = HashMap::new();
        for dep_name in &target.deps {
            if let Some(dep_path) = dep_outputs.get(dep_name) {
                target_deps.insert(dep_name.clone(), dep_path.clone());
            }
        }

        if !quiet {
            print!("     Compiling {} (rust_binary)...", target.name);
        }
        match rules::execute_rust_binary(target, &workspace_root, &output_dir, &target_deps) {
            Ok(result) => {
                dep_outputs.insert(target.name.clone(), result.output.clone());
                if result.cached {
                    total_cached += 1;
                    if !quiet {
                        println!(" \x1b[36mcached\x1b[0m ({}ms)", result.elapsed_ms);
                    }
                } else {
                    total_built += 1;
                    if !quiet {
                        println!(" \x1b[32mok\x1b[0m ({}ms)", result.elapsed_ms);
                    }
                }
            }
            Err(e) => {
                total_failed += 1;
                if !quiet {
                    println!(" \x1b[31mFAILED\x1b[0m");
                    eprintln!("     {}", e);
                }
            }
        }
    }

    let elapsed = start.elapsed();
    println!();

    if total_failed > 0 {
        println!(
            "  \x1b[1;31mBuild failed\x1b[0m in {:.1}s",
            elapsed.as_secs_f64()
        );
    } else {
        println!(
            "  \x1b[1;32mBuild successful\x1b[0m in {:.1}s",
            elapsed.as_secs_f64()
        );
    }
    println!(
        "     {} compiled, {} cached, {} failed",
        total_built, total_cached, total_failed
    );
    println!();

    if total_failed > 0 {
        Err(hb_core::error::HbError::Internal(format!(
            "{} target(s) failed",
            total_failed
        )))
    } else {
        Ok(())
    }
}
