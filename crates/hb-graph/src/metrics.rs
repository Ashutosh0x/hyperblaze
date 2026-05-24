//! Performance metrics for the HyperGraph.

use std::sync::atomic::{AtomicU64, Ordering};

/// Thread-safe performance counters for the graph.
pub struct GraphMetrics {
    pub nodes_created: AtomicU64,
    pub node_hits: AtomicU64,
    pub edges_added: AtomicU64,
    pub invalidations: AtomicU64,
    pub evaluations: AtomicU64,
    pub cache_hits: AtomicU64,
}

impl GraphMetrics {
    pub fn new() -> Self {
        Self {
            nodes_created: AtomicU64::new(0),
            node_hits: AtomicU64::new(0),
            edges_added: AtomicU64::new(0),
            invalidations: AtomicU64::new(0),
            evaluations: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
        }
    }

    pub fn record_node_create(&self) {
        self.nodes_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_node_hit(&self) {
        self.node_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_edge_add(&self) {
        self.edges_added.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_invalidation(&self) {
        self.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_evaluation(&self) {
        self.evaluations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn reset(&self) {
        self.nodes_created.store(0, Ordering::Relaxed);
        self.node_hits.store(0, Ordering::Relaxed);
        self.edges_added.store(0, Ordering::Relaxed);
        self.invalidations.store(0, Ordering::Relaxed);
        self.evaluations.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
    }

    /// Summary string for display.
    pub fn summary(&self) -> String {
        format!(
            "nodes={}, hits={}, edges={}, invalidations={}, evals={}, cache_hits={}",
            self.nodes_created.load(Ordering::Relaxed),
            self.node_hits.load(Ordering::Relaxed),
            self.edges_added.load(Ordering::Relaxed),
            self.invalidations.load(Ordering::Relaxed),
            self.evaluations.load(Ordering::Relaxed),
            self.cache_hits.load(Ordering::Relaxed),
        )
    }
}

impl Default for GraphMetrics {
    fn default() -> Self {
        Self::new()
    }
}
