//! Node keys — unique identifiers for computations in the graph.
//!
//! Equivalent to Bazel's SkyKey, but simpler. A NodeKey is a
//! (function_type, argument) pair that uniquely identifies a value.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

/// The type of function that computes a node's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionType {
    /// Load and parse a BUILD.hb file
    PackageLoad,
    /// Resolve a target label to its definition
    TargetResolve,
    /// Analyze a configured target (rule analysis)
    ConfiguredTarget,
    /// Execute a build action
    ActionExecution,
    /// Read file metadata (mtime, digest)
    FileState,
    /// List directory contents
    DirectoryListing,
    /// Resolve an external dependency
    ExternalDep,
    /// Compute a glob pattern
    Glob,
    /// Toolchain resolution
    Toolchain,
    /// A user-defined custom function
    Custom,
}

impl fmt::Display for FunctionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionType::PackageLoad => write!(f, "PackageLoad"),
            FunctionType::TargetResolve => write!(f, "TargetResolve"),
            FunctionType::ConfiguredTarget => write!(f, "ConfiguredTarget"),
            FunctionType::ActionExecution => write!(f, "ActionExecution"),
            FunctionType::FileState => write!(f, "FileState"),
            FunctionType::DirectoryListing => write!(f, "DirectoryListing"),
            FunctionType::ExternalDep => write!(f, "ExternalDep"),
            FunctionType::Glob => write!(f, "Glob"),
            FunctionType::Toolchain => write!(f, "Toolchain"),
            FunctionType::Custom => write!(f, "Custom"),
        }
    }
}

/// A unique key identifying a node in the HyperGraph.
///
/// Designed to be cheap to clone (Arc-backed) and fast to hash.
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeKey {
    /// The type of function
    pub function_type: FunctionType,
    /// The argument (label, path, etc.)
    argument: String,
    /// Pre-computed hash for fast HashMap lookups
    #[serde(skip, default)]
    hash: u64,
}

impl NodeKey {
    /// Create a new NodeKey.
    pub fn new(function_type: FunctionType, argument: impl Into<String>) -> Self {
        let argument: String = argument.into();
        let hash = compute_hash(function_type, &argument);
        Self {
            function_type,
            argument,
            hash,
        }
    }

    /// Get the argument string.
    pub fn argument(&self) -> &str {
        &self.argument
    }

    /// Create a FileState key.
    pub fn file_state(path: &str) -> Self {
        Self::new(FunctionType::FileState, path)
    }

    /// Create a PackageLoad key.
    pub fn package_load(label: &str) -> Self {
        Self::new(FunctionType::PackageLoad, label)
    }

    /// Create an ActionExecution key.
    pub fn action_execution(action_id: &str) -> Self {
        Self::new(FunctionType::ActionExecution, action_id)
    }

    /// Create a ConfiguredTarget key.
    pub fn configured_target(label: &str) -> Self {
        Self::new(FunctionType::ConfiguredTarget, label)
    }

    /// Get a canonical string representation: "FunctionType:argument"
    pub fn canonical(&self) -> String {
        format!("{}:{}", self.function_type, self.argument)
    }
}

impl PartialEq for NodeKey {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
            && self.function_type == other.function_type
            && self.argument == other.argument
    }
}

impl Eq for NodeKey {}

impl Hash for NodeKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Use pre-computed hash for speed
        state.write_u64(self.hash);
    }
}

impl fmt::Debug for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NodeKey({}:{})", self.function_type, self.argument)
    }
}

impl fmt::Display for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.function_type, self.argument)
    }
}

/// Compute a stable hash for a (function_type, argument) pair.
fn compute_hash(ft: FunctionType, arg: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    ft.hash(&mut hasher);
    arg.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_key_equality() {
        let k1 = NodeKey::file_state("/src/main.rs");
        let k2 = NodeKey::file_state("/src/main.rs");
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_key_inequality() {
        let k1 = NodeKey::file_state("/src/main.rs");
        let k2 = NodeKey::file_state("/src/lib.rs");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_key_in_hashset() {
        let mut set = HashSet::new();
        let k1 = NodeKey::file_state("/src/main.rs");
        let k2 = NodeKey::file_state("/src/main.rs");
        set.insert(k1);
        assert!(set.contains(&k2));
    }

    #[test]
    fn test_different_types_different_keys() {
        let k1 = NodeKey::new(FunctionType::FileState, "foo");
        let k2 = NodeKey::new(FunctionType::PackageLoad, "foo");
        assert_ne!(k1, k2);
    }
}
