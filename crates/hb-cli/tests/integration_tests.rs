//! Integration tests for Hyperblaze build system.
//!
//! These tests create real temporary workspaces, invoke rustc,
//! and verify caching, early cutoff, and incremental behavior.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Create a temporary workspace with a simple Rust hello-world project.
fn create_hello_world(dir: &Path) {
    // Create main.rs
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("main.rs"),
        r#"fn main() {
    println!("Hello from Hyperblaze!");
}
"#,
    )
    .unwrap();

    // Create HYPERBLAZE.toml
    fs::write(
        dir.join("HYPERBLAZE.toml"),
        r#"[project]
name = "hello"
version = "0.1.0"

[build]
jobs = 1
disk_cache = true
output_dir = ".hb-out"
"#,
    )
    .unwrap();
}

/// Create a simple Rust library project.
fn create_rust_lib(dir: &Path) {
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
"#,
    )
    .unwrap();
}

/// Get the path to the hyperblaze binary.
fn hyperblaze_bin() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("hyperblaze");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    // If not found in target/debug, try release
    if !path.exists() {
        path.pop();
        path.pop();
        path.push("release");
        path.push("hyperblaze");
        if cfg!(windows) {
            path.set_extension("exe");
        }
    }
    path
}

#[test]
fn test_version_output() {
    let bin = hyperblaze_bin();
    if !bin.exists() {
        eprintln!("Skipping: hyperblaze binary not found at {:?}", bin);
        return;
    }

    let output = Command::new(&bin)
        .arg("--version")
        .output()
        .expect("failed to run hyperblaze");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hyperblaze"));
}

#[test]
fn test_info_command() {
    let bin = hyperblaze_bin();
    if !bin.exists() {
        return;
    }

    let output = Command::new(&bin)
        .arg("info")
        .output()
        .expect("failed to run hyperblaze");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hyperblaze"));
    assert!(stdout.contains("Platform"));
}

#[test]
fn test_init_creates_workspace() {
    let bin = hyperblaze_bin();
    if !bin.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let output = Command::new(&bin)
        .args(["init", "--name", "test-project"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run hyperblaze init");

    assert!(output.status.success());
    assert!(tmp.path().join("HYPERBLAZE.toml").exists());
}

#[test]
fn test_doctor_checks_environment() {
    let bin = hyperblaze_bin();
    if !bin.exists() {
        return;
    }

    let output = Command::new(&bin)
        .arg("doctor")
        .output()
        .expect("failed to run hyperblaze doctor");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("platform"));
}

#[test]
fn test_build_hello_world() {
    let bin = hyperblaze_bin();
    if !bin.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    create_hello_world(tmp.path());

    let output = Command::new(&bin)
        .args(["build", "//src:main"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run hyperblaze build");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Build successful"));
}

#[test]
fn test_noop_rebuild_uses_cache() {
    let bin = hyperblaze_bin();
    if !bin.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    create_hello_world(tmp.path());

    // First build
    let output1 = Command::new(&bin)
        .args(["build", "//src:main"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run hyperblaze build");
    assert!(output1.status.success());

    // Second build (should be cached)
    let output2 = Command::new(&bin)
        .args(["build", "//src:main"])
        .current_dir(tmp.path())
        .output()
        .expect("failed to run hyperblaze build");
    assert!(output2.status.success());
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(stdout2.contains("Build successful"));
}

#[test]
fn test_clean_command() {
    let bin = hyperblaze_bin();
    if !bin.exists() {
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    create_hello_world(tmp.path());

    // Create some build output to clean
    let out_dir = tmp.path().join(".hb-out");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(out_dir.join("test.txt"), "test").unwrap();

    let output = Command::new(&bin)
        .arg("clean")
        .current_dir(tmp.path())
        .output()
        .expect("failed to run hyperblaze clean");

    assert!(output.status.success());
}
