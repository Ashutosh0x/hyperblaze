//! The HyperGraph — lock-free concurrent dependency graph.
//!
//! This is the central data structure. All build computations are nodes
//! in this graph, connected by dependency edges.

use crate::key::NodeKey;
use crate::node::NodeEntry;
use crate::value::NodeValue;
use crate::metrics::GraphMetrics;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// The HyperGraph — a lock-free concurrent dependency graph.
///
/// Equivalent to Bazel's InMemoryGraphImpl, but using DashMap for
/// lock-free concurrent access instead of a ConcurrentHashMap behind
/// synchronized methods.
pub struct HyperGraph {
    /// All nodes indexed by key. DashMap provides lock-free concurrent access.
    nodes: DashMap<NodeKey, Arc<NodeEntry>>,

    /// Global version counter. Incremented on each evaluation cycle.
    version: AtomicU64,

    /// Performance metrics
    metrics: GraphMetrics,
}

impl HyperGraph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            version: AtomicU64::new(1),
            metrics: GraphMetrics::new(),
        }
    }

    /// Get or create a node for the given key.
    pub fn get_or_create(&self, key: &NodeKey) -> Arc<NodeEntry> {
        if let Some(entry) = self.nodes.get(key) {
            self.metrics.record_node_hit();
            return entry.value().clone();
        }

        // Node doesn't exist — create it
        self.metrics.record_node_create();
        let entry = Arc::new(NodeEntry::new(key.clone()));
        self.nodes.entry(key.clone()).or_insert(entry.clone());
        entry
    }

    /// Get a node if it exists.
    pub fn get(&self, key: &NodeKey) -> Option<Arc<NodeEntry>> {
        self.nodes.get(key).map(|e| e.value().clone())
    }

    /// Check if a node exists and is done.
    pub async fn is_done(&self, key: &NodeKey) -> bool {
        if let Some(entry) = self.get(key) {
            entry.state().await.is_done()
        } else {
            false
        }
    }

    /// Get the current value of a node (if computed).
    pub async fn get_value(&self, key: &NodeKey) -> Option<NodeValue> {
        if let Some(entry) = self.get(key) {
            let state = entry.state().await;
            state.value().cloned()
        } else {
            None
        }
    }

    /// Invalidate a node and all its transitive reverse dependencies.
    /// Returns the number of nodes invalidated.
    pub async fn invalidate(&self, key: &NodeKey) -> usize {
        let dirty_version = self.bump_version();
        let mut invalidated = 0;
        let mut queue = vec![key.clone()];
        let mut visited = std::collections::HashSet::new();

        while let Some(current) = queue.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if let Some(entry) = self.get(&current) {
                entry.mark_dirty(dirty_version).await;
                invalidated += 1;
                self.metrics.record_invalidation();

                // Propagate to reverse dependencies
                let rdeps = entry.rdeps().await;
                queue.extend(rdeps);
            }
        }

        invalidated
    }

    /// Invalidate multiple keys efficiently.
    pub async fn invalidate_many(&self, keys: &[NodeKey]) -> usize {
        let mut total = 0;
        for key in keys {
            total += self.invalidate(key).await;
        }
        total
    }

    /// Register a dependency edge: `from` depends on `to`.
    pub async fn add_edge(&self, from: &NodeKey, to: &NodeKey) {
        let from_entry = self.get_or_create(from);
        let to_entry = self.get_or_create(to);

        // Add forward dep
        from_entry.add_dep(to.clone()).await;

        // Add reverse dep
        to_entry.add_rdep(from.clone()).await;

        self.metrics.record_edge_add();
    }

    /// Get the current graph version.
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Bump the version counter and return the new version.
    pub fn bump_version(&self) -> u64 {
        self.version.fetch_add(1, Ordering::AcqRel) + 1
    }

    /// Get the total number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get graph metrics.
    pub fn metrics(&self) -> &GraphMetrics {
        &self.metrics
    }

    /// Clear all nodes from the graph.
    pub fn clear(&self) {
        self.nodes.clear();
        self.metrics.reset();
    }

    /// Get all node keys (for debugging/query).
    pub fn all_keys(&self) -> Vec<NodeKey> {
        self.nodes.iter().map(|e| e.key().clone()).collect()
    }
}

impl Default for HyperGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for HyperGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HyperGraph(nodes={}, version={})",
            self.node_count(),
            self.version()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_or_create() {
        let graph = HyperGraph::new();
        let key = NodeKey::file_state("/src/main.rs");
        let entry = graph.get_or_create(&key);
        assert_eq!(entry.key, key);
        assert_eq!(graph.node_count(), 1);

        // Getting again should return the same node
        let entry2 = graph.get_or_create(&key);
        assert_eq!(entry.key, entry2.key);
        assert_eq!(graph.node_count(), 1);
    }

    #[tokio::test]
    async fn test_invalidation_propagates() {
        let graph = HyperGraph::new();
        let file_key = NodeKey::file_state("/src/main.rs");
        let pkg_key = NodeKey::package_load("//src");
        let target_key = NodeKey::configured_target("//src:main");

        // Set up edges: target → pkg → file
        graph.add_edge(&target_key, &pkg_key).await;
        graph.add_edge(&pkg_key, &file_key).await;

        // Mark all as done
        let v = graph.version();
        graph.get_or_create(&file_key).complete(
            NodeValue::new("file_content".to_string()), v, None
        ).await;
        graph.get_or_create(&pkg_key).complete(
            NodeValue::new("package".to_string()), v, None
        ).await;
        graph.get_or_create(&target_key).complete(
            NodeValue::new("target".to_string()), v, None
        ).await;

        // Invalidate the file — should propagate to pkg and target
        let count = graph.invalidate(&file_key).await;
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_version_bumps() {
        let graph = HyperGraph::new();
        assert_eq!(graph.version(), 1);
        graph.bump_version();
        assert_eq!(graph.version(), 2);
    }
}
