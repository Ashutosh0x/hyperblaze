//! `hyperblaze clean` command

use hb_core::error::HbResult;

pub fn run(expunge: bool) -> HbResult<()> {
    println!();
    println!("  🧹 \x1b[1;38;5;208mHyperblaze\x1b[0m — Cleaning build outputs");
    println!();

    let cwd = std::env::current_dir()?;

    // Remove output directory
    let out_dir = cwd.join(".hb-out");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir)?;
        println!("  ✅ Removed .hb-out/");
    }

    if expunge {
        let cache_dir = cwd.join(".hb-cache");
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir)?;
            println!("  ✅ Removed .hb-cache/ (action cache)");
        }
    }

    println!();
    println!("  Done.");
    println!();

    Ok(())
}
