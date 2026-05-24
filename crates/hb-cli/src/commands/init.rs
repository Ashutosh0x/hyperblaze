//! `hyperblaze init` command

use hb_core::config::HyperblazeConfig;
use hb_core::error::HbResult;
use std::path::Path;

pub fn run(path: &Path, name: Option<&str>) -> HbResult<()> {
    let target = if path == Path::new(".") {
        std::env::current_dir()?
    } else {
        path.to_path_buf()
    };

    let project_name = name.unwrap_or_else(|| {
        target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-project")
    });

    println!();
    println!("  🔥 \x1b[1;38;5;208mHyperblaze\x1b[0m — Initializing workspace");
    println!();

    // Create HYPERBLAZE.toml
    let config_path = target.join("HYPERBLAZE.toml");
    if config_path.exists() {
        println!("  ⚠️  HYPERBLAZE.toml already exists, skipping");
    } else {
        let content = HyperblazeConfig::generate_default_toml(project_name);
        std::fs::write(&config_path, &content)?;
        println!("  ✅ Created HYPERBLAZE.toml");
    }

    // Create output directory
    let out_dir = target.join(".hb-out");
    std::fs::create_dir_all(&out_dir)?;

    // Create .gitignore entry
    let gitignore_path = target.join(".gitignore");
    let gitignore_entry = "\n# Hyperblaze\n.hb-out/\n.hb-cache/\n";
    if gitignore_path.exists() {
        let content = std::fs::read_to_string(&gitignore_path)?;
        if !content.contains(".hb-out") {
            std::fs::write(&gitignore_path, format!("{}{}", content, gitignore_entry))?;
            println!("  ✅ Updated .gitignore");
        }
    } else {
        std::fs::write(&gitignore_path, gitignore_entry)?;
        println!("  ✅ Created .gitignore");
    }

    // Auto-detect languages
    let mut detected = Vec::new();
    if target.join("Cargo.toml").exists() {
        detected.push("Rust (Cargo.toml found)");
    }
    if target.join("go.mod").exists() {
        detected.push("Go (go.mod found)");
    }
    if target.join("package.json").exists() {
        detected.push("TypeScript/JavaScript (package.json found)");
    }
    if target.join("pyproject.toml").exists() || target.join("setup.py").exists() {
        detected.push("Python (pyproject.toml found)");
    }

    if !detected.is_empty() {
        println!();
        println!("  📦 Detected languages:");
        for lang in &detected {
            println!("     • {}", lang);
        }
    }

    println!();
    println!("  🎉 Workspace initialized! Next steps:");
    println!("     • Run \x1b[1mhyperblaze build\x1b[0m to build your project");
    println!("     • Run \x1b[1mhyperblaze info\x1b[0m to see system information");
    println!();

    Ok(())
}
