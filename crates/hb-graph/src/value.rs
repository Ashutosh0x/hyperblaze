//! Node values — the result of computing a node in the graph.
//!
//! Equivalent to Bazel's SkyValue. Must be cheaply cloneable (Arc-wrapped).

use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt;
use std::sync::Arc;

/// The result of computing a node. Type-erased to support different
/// function types in the same graph.
#[derive(Clone)]
pub struct NodeValue {
    /// The actual value, type-erased
    inner: Arc<dyn Any + Send + Sync>,
    /// Human-readable type name for debugging
    type_name: &'static str,
}

impl NodeValue {
    /// Create a new NodeValue wrapping a typed value.
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self {
            inner: Arc::new(value),
            type_name: std::any::type_name::<T>(),
        }
    }

    /// Downcast to a specific type.
    pub fn downcast_ref<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.inner.downcast_ref::<T>()
    }

    /// Downcast to a specific type, panicking if wrong type.
    pub fn expect<T: Any + Send + Sync>(&self, msg: &str) -> &T {
        self.downcast_ref::<T>().expect(msg)
    }

    /// Get the type name for debugging.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}

impl fmt::Debug for NodeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeValue({})", self.type_name)
    }
}

// Common value types used by the build system:

/// Result of reading file state (mtime + digest).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStateValue {
    pub path: String,
    pub digest: hb_core::digest::ContentDigest,
    pub size: u64,
    pub mtime: u64,
    pub exists: bool,
}

/// Result of loading a package (BUILD.hb file).
#[derive(Debug, Clone)]
pub struct PackageValue {
    pub name: String,
    pub targets: Vec<TargetDef>,
}

/// A target definition from a BUILD.hb file.
#[derive(Debug, Clone)]
pub struct TargetDef {
    pub name: String,
    pub rule_type: String,
    pub srcs: Vec<String>,
    pub deps: Vec<String>,
    pub visibility: Vec<String>,
}

/// Result of action execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub output_files: Vec<OutputFile>,
    pub execution_time_ms: u64,
    pub cache_hit: bool,
}

/// An output file produced by an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFile {
    pub path: String,
    pub digest: hb_core::digest::ContentDigest,
    pub size: u64,
}
