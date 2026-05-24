//! `hyperblaze build` command
//!
//! Parses BUILD.hb, resolves targets, fingerprints inputs via BLAKE3,
//! invokes rustc for real compilation, and caches results.

use hb_core::build_file::BuildFile;
use hb_core::config::HyperblazeConfig;
use hb_core::error::{HbError, HbResult};
use hb_core::rules;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

pub async fn run(targets: &[String], jobs: usize, quiet: bool) -> HbResult<()> {
    let start = Instant::now();
    let cwd = std::env::current_dir()?;

    // Find workspace root
    let workspace_root = HyperblazeConfig::find_workspace_root(&cwd).unwrap_or_else(|| cwd.clone());

    let config = HyperblazeConfig::load(&workspace_root)?;
    let effective_jobs = if jobs > 0 {
        jobs
    } else {
        config.effective_jobs()
    };

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
        println!(
            "  \x1b[1;32mBuild successful\x1b[0m in {:.1}s",
            start.elapsed().as_secs_f64()
        );
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
            if build_file.target.len() == 1 {
                ""
            } else {
                "s"
            }
        );
    }

    let target_map: HashMap<String, _> = build_file
        .target
        .iter()
        .map(|target| (target.name.clone(), target.clone()))
        .collect();

    // Resolve which targets to build, including transitive dependencies.
    let requested_names: Vec<String> = if targets.is_empty() || targets.iter().any(|t| t == "//...")
    {
        build_file
            .target
            .iter()
            .map(|target| target.name.clone())
            .collect()
    } else {
        targets
            .iter()
            .map(|pattern| target_name_from_label(pattern))
            .collect()
    };

    let mut targets_to_build = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for name in &requested_names {
        if target_map.contains_key(name) {
            collect_target_with_deps(
                name,
                &target_map,
                &mut visiting,
                &mut visited,
                &mut targets_to_build,
            )?;
        }
    }

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

    // Build targets in dependency order.
    for target in &targets_to_build {
        let mut target_deps = HashMap::new();
        for dep_label in &target.deps {
            let dep_name = target_name_from_label(dep_label);
            let dep_path = dep_outputs.get(&dep_name).ok_or_else(|| {
                HbError::Internal(format!(
                    "Target '{}' depends on '{}' but it was not built",
                    target.name, dep_name
                ))
            })?;
            target_deps.insert(dep_name, dep_path.clone());
        }

        if !quiet {
            print!("     Compiling {} ({})...", target.name, target.rule);
        }

        let result = match target.rule.as_str() {
            "rust_library" => {
                rules::execute_rust_library(target, &workspace_root, &output_dir, &target_deps)
            }
            "rust_binary" => {
                rules::execute_rust_binary(target, &workspace_root, &output_dir, &target_deps)
            }
            other => Err(HbError::Internal(format!(
                "Target '{}' has unknown rule '{}'",
                target.name, other
            ))),
        };

        match result {
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

fn target_name_from_label(label: &str) -> String {
    label
        .rsplit(':')
        .next()
        .unwrap_or(label)
        .trim_start_matches('/')
        .to_string()
}

fn collect_target_with_deps(
    name: &str,
    target_map: &HashMap<String, hb_core::build_file::TargetDef>,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    ordered: &mut Vec<hb_core::build_file::TargetDef>,
) -> HbResult<()> {
    if visited.contains(name) {
        return Ok(());
    }
    if !visiting.insert(name.to_string()) {
        return Err(HbError::Internal(format!(
            "Dependency cycle detected at target '{}'",
            name
        )));
    }

    let target = target_map
        .get(name)
        .ok_or_else(|| HbError::Internal(format!("Unknown target dependency '{}'", name)))?;
    for dep_label in &target.deps {
        let dep_name = target_name_from_label(dep_label);
        collect_target_with_deps(&dep_name, target_map, visiting, visited, ordered)?;
    }

    visiting.remove(name);
    visited.insert(name.to_string());
    ordered.push(target.clone());
    Ok(())
}
