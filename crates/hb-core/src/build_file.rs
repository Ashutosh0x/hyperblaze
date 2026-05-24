//! BUILD.hb parser -- minimal TOML-based build file format.
//!
//! Schema:
//! ```toml
//! [[target]]
//! name = "hello"
//! rule = "rust_binary"
//! srcs = ["src/main.rs"]
//! deps = []
//! edition = "2021"
//! ```

use serde::Deserialize;
use std::path::Path;

use crate::error::{HbError, HbResult};

/// A parsed BUILD.hb file.
#[derive(Debug, Deserialize)]
pub struct BuildFile {
    #[serde(default)]
    pub target: Vec<TargetDef>,
}

/// A single target definition.
#[derive(Debug, Clone, Deserialize)]
pub struct TargetDef {
    /// Target name (e.g. "hello")
    pub name: String,

    /// Rule type (e.g. "rust_binary", "rust_library")
    pub rule: String,

    /// Source files
    #[serde(default)]
    pub srcs: Vec<String>,

    /// Dependencies (other target labels)
    #[serde(default)]
    pub deps: Vec<String>,

    /// Rust edition (default: "2021")
    #[serde(default = "default_edition")]
    pub edition: String,

    /// Crate name override (defaults to target name)
    #[serde(default)]
    pub crate_name: Option<String>,
}

fn default_edition() -> String {
    "2021".to_string()
}

impl BuildFile {
    /// Parse a BUILD.hb file from a path.
    pub fn parse(path: &Path) -> HbResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            HbError::Internal(format!("Failed to read {}: {}", path.display(), e))
        })?;
        Self::parse_str(&content, path)
    }

    /// Parse BUILD.hb content from a string.
    pub fn parse_str(content: &str, path: &Path) -> HbResult<Self> {
        let build_file: BuildFile = toml::from_str(content).map_err(|e| {
            HbError::Internal(format!(
                "Failed to parse {}: {}",
                path.display(),
                e
            ))
        })?;

        // Validate targets
        for target in &build_file.target {
            if target.name.is_empty() {
                return Err(HbError::Internal(format!(
                    "{}: target has empty name",
                    path.display()
                )));
            }
            if target.srcs.is_empty() {
                return Err(HbError::Internal(format!(
                    "{}: target '{}' has no srcs",
                    path.display(),
                    target.name
                )));
            }
            match target.rule.as_str() {
                "rust_binary" | "rust_library" => {}
                other => {
                    return Err(HbError::Internal(format!(
                        "{}: target '{}' has unknown rule '{}'",
                        path.display(),
                        target.name,
                        other
                    )));
                }
            }
        }

        Ok(build_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_rust_binary() {
        let content = r#"
[[target]]
name = "hello"
rule = "rust_binary"
srcs = ["src/main.rs"]
"#;
        let bf = BuildFile::parse_str(content, &PathBuf::from("BUILD.hb")).unwrap();
        assert_eq!(bf.target.len(), 1);
        assert_eq!(bf.target[0].name, "hello");
        assert_eq!(bf.target[0].rule, "rust_binary");
        assert_eq!(bf.target[0].srcs, vec!["src/main.rs"]);
        assert_eq!(bf.target[0].edition, "2021");
    }

    #[test]
    fn test_parse_rust_library() {
        let content = r#"
[[target]]
name = "mylib"
rule = "rust_library"
srcs = ["src/lib.rs"]
edition = "2024"
"#;
        let bf = BuildFile::parse_str(content, &PathBuf::from("BUILD.hb")).unwrap();
        assert_eq!(bf.target[0].rule, "rust_library");
        assert_eq!(bf.target[0].edition, "2024");
    }

    #[test]
    fn test_parse_with_deps() {
        let content = r#"
[[target]]
name = "mylib"
rule = "rust_library"
srcs = ["src/lib.rs"]

[[target]]
name = "main"
rule = "rust_binary"
srcs = ["src/main.rs"]
deps = ["mylib"]
"#;
        let bf = BuildFile::parse_str(content, &PathBuf::from("BUILD.hb")).unwrap();
        assert_eq!(bf.target.len(), 2);
        assert_eq!(bf.target[1].deps, vec!["mylib"]);
    }

    #[test]
    fn test_reject_empty_srcs() {
        let content = r#"
[[target]]
name = "bad"
rule = "rust_binary"
srcs = []
"#;
        let result = BuildFile::parse_str(content, &PathBuf::from("BUILD.hb"));
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_unknown_rule() {
        let content = r#"
[[target]]
name = "bad"
rule = "go_binary"
srcs = ["main.go"]
"#;
        let result = BuildFile::parse_str(content, &PathBuf::from("BUILD.hb"));
        assert!(result.is_err());
    }
}
