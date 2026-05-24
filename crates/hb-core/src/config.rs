//! Configuration system for Hyperblaze.
//!
//! Replaces Bazel's WORKSPACE + .bazelrc + MODULE.bazel with a single
//! `HYPERBLAZE.toml` file.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Main configuration file name
pub const CONFIG_FILE_NAME: &str = "HYPERBLAZE.toml";

/// Build file name
pub const BUILD_FILE_NAME: &str = "BUILD.hb";

/// Root configuration for a Hyperblaze workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperblazeConfig {
    /// Project metadata
    #[serde(default)]
    pub project: ProjectConfig,

    /// Build settings
    #[serde(default)]
    pub build: BuildConfig,

    /// Remote execution and caching
    #[serde(default)]
    pub remote: RemoteConfig,

    /// Language-specific settings
    #[serde(default)]
    pub languages: LanguagesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    #[serde(default = "default_project_name")]
    pub name: String,

    /// Project version
    #[serde(default = "default_version")]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Maximum number of parallel jobs (0 = auto-detect)
    #[serde(default)]
    pub jobs: usize,

    /// Default target platforms
    #[serde(default)]
    pub default_platforms: Vec<String>,

    /// Output base directory
    #[serde(default = "default_output_base")]
    pub output_base: String,

    /// Enable local disk cache
    #[serde(default = "default_true")]
    pub disk_cache: bool,

    /// Disk cache directory
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    /// Remote cache URL (e.g., "grpc://cache.company.com:8080")
    #[serde(default)]
    pub cache_url: Option<String>,

    /// Remote execution URL
    #[serde(default)]
    pub exec_url: Option<String>,

    /// Instance name for remote execution
    #[serde(default)]
    pub instance_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguagesConfig {
    /// Rust language settings
    #[serde(default)]
    pub rust: Option<RustConfig>,

    /// Go language settings
    #[serde(default)]
    pub go: Option<GoConfig>,

    /// Python language settings
    #[serde(default)]
    pub python: Option<PythonConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustConfig {
    /// Rust edition (e.g., "2024")
    #[serde(default = "default_rust_edition")]
    pub edition: String,

    /// Toolchain channel
    #[serde(default = "default_rust_toolchain")]
    pub toolchain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoConfig {
    /// Go version
    #[serde(default = "default_go_version")]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonConfig {
    /// Python version
    #[serde(default = "default_python_version")]
    pub version: String,
}

// Default value functions
fn default_project_name() -> String { "hyperblaze-project".to_string() }
fn default_version() -> String { "0.1.0".to_string() }
fn default_output_base() -> String { ".hb-out".to_string() }
fn default_true() -> bool { true }
fn default_cache_dir() -> String { ".hb-cache".to_string() }
fn default_rust_edition() -> String { "2024".to_string() }
fn default_rust_toolchain() -> String { "stable".to_string() }
fn default_go_version() -> String { "1.23".to_string() }
fn default_python_version() -> String { "3.12".to_string() }

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: default_project_name(),
            version: default_version(),
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            jobs: 0,
            default_platforms: vec![],
            output_base: default_output_base(),
            disk_cache: true,
            cache_dir: default_cache_dir(),
        }
    }
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            cache_url: None,
            exec_url: None,
            instance_name: None,
        }
    }
}

impl Default for HyperblazeConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig::default(),
            build: BuildConfig::default(),
            remote: RemoteConfig::default(),
            languages: LanguagesConfig::default(),
        }
    }
}

impl HyperblazeConfig {
    /// Load configuration from a HYPERBLAZE.toml file.
    pub fn load(workspace_root: &Path) -> crate::error::HbResult<Self> {
        let config_path = workspace_root.join(CONFIG_FILE_NAME);
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).map_err(|e| {
                crate::error::HbError::Config(format!(
                    "Failed to read {}: {}",
                    config_path.display(),
                    e
                ))
            })?;
            let config: Self = toml::from_str(&content).map_err(|e| {
                crate::error::HbError::Config(format!(
                    "Failed to parse {}: {}",
                    config_path.display(),
                    e
                ))
            })?;
            Ok(config)
        } else {
            // No config file — use defaults (zero-config mode)
            Ok(Self::default())
        }
    }

    /// Find the workspace root by walking up from the given directory.
    pub fn find_workspace_root(start: &Path) -> Option<PathBuf> {
        let mut current = start.to_path_buf();
        loop {
            if current.join(CONFIG_FILE_NAME).exists() {
                return Some(current);
            }
            // Also check for BUILD.hb (implicit workspace)
            if current.join(BUILD_FILE_NAME).exists() {
                return Some(current);
            }
            if !current.pop() {
                return None;
            }
        }
    }

    /// Get effective number of parallel jobs.
    pub fn effective_jobs(&self) -> usize {
        if self.build.jobs == 0 {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        } else {
            self.build.jobs
        }
    }

    /// Generate a default HYPERBLAZE.toml content.
    pub fn generate_default_toml(project_name: &str) -> String {
        format!(
            r#"[project]
name = "{project_name}"
version = "0.1.0"

[build]
# Maximum parallel jobs (0 = auto-detect CPU count)
jobs = 0
# Enable local disk cache for faster rebuilds
disk_cache = true

# [remote]
# cache_url = "grpc://cache.example.com:8080"
# exec_url = "grpc://exec.example.com:9090"
"#
        )
    }
}
