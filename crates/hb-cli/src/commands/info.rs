//! `hyperblaze info` command

use hb_core::error::HbResult;
use hb_core::platform::PlatformInfo;
use std::time::Instant;

pub fn run(start: Instant) -> HbResult<()> {
    let platform = PlatformInfo::detect();
    let startup_time = start.elapsed();

    println!();
    println!(
        "  🔥 \x1b[1;38;5;208mHyperblaze\x1b[0m v{}",
        hb_core::VERSION
    );
    println!("{}", hb_core::BANNER);
    println!("  Platform:     {}", platform);
    println!(
        "  Startup time: {:.1}ms",
        startup_time.as_secs_f64() * 1000.0
    );
    println!("  CPU cores:    {}", platform.cpu_count);
    println!(
        "  Memory:       {} total, {} available",
        hb_core::platform::format_bytes(platform.total_memory),
        hb_core::platform::format_bytes(platform.available_memory),
    );

    // Find workspace
    let cwd = std::env::current_dir()?;
    match hb_core::config::HyperblazeConfig::find_workspace_root(&cwd) {
        Some(root) => {
            println!("  Workspace:    {}", root.display());
            let config = hb_core::config::HyperblazeConfig::load(&root)?;
            println!(
                "  Project:      {} v{}",
                config.project.name, config.project.version
            );
            println!("  Jobs:         {}", config.effective_jobs());
            println!(
                "  Disk cache:   {}",
                if config.build.disk_cache {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            if let Some(ref url) = config.remote.cache_url {
                println!("  Remote cache: {}", url);
            }
        }
        None => {
            println!("  Workspace:    \x1b[33mnot found\x1b[0m (run `hyperblaze init`)");
        }
    }

    println!();

    Ok(())
}
