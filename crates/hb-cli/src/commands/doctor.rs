//! `hyperblaze doctor` command — diagnose environment issues

use hb_core::error::HbResult;
use hb_core::platform::PlatformInfo;

pub async fn run() -> HbResult<()> {
    println!();
    println!("  🩺 \x1b[1;38;5;208mHyperblaze Doctor\x1b[0m — Checking environment");
    println!();

    let platform = PlatformInfo::detect();
    let mut issues = 0;

    // Check platform
    print!("  Checking platform... ");
    println!("\x1b[32m✓\x1b[0m {:?}-{:?}", platform.os, platform.arch);

    // Check CPU
    print!("  Checking CPU cores... ");
    if platform.cpu_count >= 2 {
        println!("\x1b[32m✓\x1b[0m {} cores", platform.cpu_count);
    } else {
        println!(
            "\x1b[33m⚠\x1b[0m Only {} core — builds will be slow",
            platform.cpu_count
        );
        issues += 1;
    }

    // Check memory
    print!("  Checking memory... ");
    let gb = platform.total_memory / (1024 * 1024 * 1024);
    if gb >= 4 {
        println!("\x1b[32m✓\x1b[0m {} GB", gb);
    } else {
        println!(
            "\x1b[33m⚠\x1b[0m Only {} GB — may be insufficient for large builds",
            gb
        );
        issues += 1;
    }

    // Check for common tools
    for (tool, purpose) in &[
        ("rustc", "Rust compiler"),
        ("go", "Go compiler"),
        ("git", "Version control"),
    ] {
        print!("  Checking {}... ", purpose);
        match which(tool) {
            Some(path) => println!("\x1b[32m✓\x1b[0m {}", path),
            None => {
                println!("\x1b[33m⚠\x1b[0m not found");
                issues += 1;
            }
        }
    }

    // Check workspace
    let cwd = std::env::current_dir()?;
    print!("  Checking workspace... ");
    match hb_core::config::HyperblazeConfig::find_workspace_root(&cwd) {
        Some(root) => println!("\x1b[32m✓\x1b[0m {}", root.display()),
        None => {
            println!("\x1b[33m⚠\x1b[0m No workspace found (run `hyperblaze init`)");
            issues += 1;
        }
    }

    println!();
    if issues == 0 {
        println!("  \x1b[1;32m✅ Everything looks good!\x1b[0m");
    } else {
        println!(
            "  \x1b[1;33m⚠ {} issue{} found\x1b[0m",
            issues,
            if issues == 1 { "" } else { "s" }
        );
    }
    println!();

    Ok(())
}

/// Simple which implementation
fn which(cmd: &str) -> Option<String> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let separator = if cfg!(windows) { ';' } else { ':' };
    let extensions: Vec<&str> = if cfg!(windows) {
        vec![".exe", ".cmd", ".bat", ""]
    } else {
        vec![""]
    };

    for dir in path_var.split(separator) {
        for ext in &extensions {
            let candidate = std::path::Path::new(dir).join(format!("{}{}", cmd, ext));
            if candidate.exists() {
                return Some(candidate.display().to_string());
            }
        }
    }
    None
}
