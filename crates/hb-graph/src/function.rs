//! Compute functions — the async equivalent of Bazel's SkyFunction.
//!
//! The critical difference: compute functions use async/await to suspend
//! when dependencies aren't ready, instead of returning null and being
//! restarted from scratch.

use crate::graph::HyperGraph;
use crate::key::NodeKey;
use crate::value::NodeValue;
use hb_core::error::{HbError, HbResult};
use std::sync::Arc;

/// Context passed to compute functions, providing access to the graph.
///
/// This replaces Bazel's SkyFunction.Environment. The key method is
/// `require()` which is async — it suspends the function until the
/// dependency is ready, instead of returning null.
pub struct ComputeContext {
    /// Reference to the graph
    graph: Arc<HyperGraph>,
    /// The key being computed
    self_key: NodeKey,
    /// Dependencies discovered during this evaluation
    discovered_deps: Vec<NodeKey>,
    /// Current graph version
    version: u64,
}

impl ComputeContext {
    /// Create a new compute context.
    pub fn new(graph: Arc<HyperGraph>, self_key: NodeKey, version: u64) -> Self {
        Self {
            graph,
            self_key,
            discovered_deps: Vec::new(),
            version,
        }
    }

    /// Request a dependency value. This is the core API.
    ///
    /// **This is async** — if the dependency isn't ready, this function
    /// suspends (yields the thread) until it is. No restarts, no wasted work.
    ///
    /// Equivalent to Bazel's `env.getValue(depKey)` but without returning null.
    pub async fn require(&mut self, dep_key: NodeKey) -> HbResult<NodeValue> {
        // Track this dependency
        self.discovered_deps.push(dep_key.clone());

        // Register the edge in the graph
        self.graph.add_edge(&self.self_key, &dep_key).await;

        // Get or create the dependency node
        let entry = self.graph.get_or_create(&dep_key);

        // If it's already done, return immediately (hot path)
        {
            let state = entry.state().await;
            if let Some(value) = state.value() {
                if !entry.is_dirty() {
                    return Ok(value.clone());
                }
            }
        }

        // Otherwise, wait for it to complete
        // THIS IS THE KEY DIFFERENCE FROM SKYFRAME:
        // We suspend here instead of returning null and restarting!
        entry
            .wait_for_completion()
            .await
            .map_err(|e| HbError::GraphEvaluation(format!("{}", e)))
    }

    /// Request multiple dependencies in parallel.
    ///
    /// Equivalent to Bazel's `env.getValuesAndExceptions()` but async.
    pub async fn require_all(
        &mut self,
        dep_keys: Vec<NodeKey>,
    ) -> HbResult<Vec<NodeValue>> {
        use tokio::task::JoinSet;

        let mut results = Vec::with_capacity(dep_keys.len());
        let mut join_set: JoinSet<HbResult<(usize, NodeValue)>> = JoinSet::new();

        for (idx, dep_key) in dep_keys.iter().enumerate() {
            self.discovered_deps.push(dep_key.clone());
            self.graph.add_edge(&self.self_key, dep_key).await;

            let graph = self.graph.clone();
            let key = dep_key.clone();

            join_set.spawn(async move {
                let entry = graph.get_or_create(&key);

                // Check if already done
                {
                    let state = entry.state().await;
                    if let Some(value) = state.value() {
                        if !entry.is_dirty() {
                            return Ok((idx, value.clone()));
                        }
                    }
                }

                // Wait for completion
                let value = entry
                    .wait_for_completion()
                    .await
                    .map_err(|e| HbError::GraphEvaluation(format!("{}", e)))?;
                Ok((idx, value))
            });
        }

        // Collect results in order
        let mut indexed_results: Vec<(usize, NodeValue)> = Vec::new();
        while let Some(result) = join_set.join_next().await {
            let (idx, value) = result
                .map_err(|e| HbError::Internal(format!("Join error: {}", e)))??;
            indexed_results.push((idx, value));
        }
        indexed_results.sort_by_key(|(idx, _)| *idx);
        results.extend(indexed_results.into_iter().map(|(_, v)| v));

        Ok(results)
    }

    /// Get the key being computed.
    pub fn self_key(&self) -> &NodeKey {
        &self.self_key
    }

    /// Get the current graph version.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Get the dependencies discovered during this evaluation.
    pub fn discovered_deps(&self) -> &[NodeKey] {
        &self.discovered_deps
    }

    /// Access the underlying graph (for advanced use).
    pub fn graph(&self) -> &Arc<HyperGraph> {
        &self.graph
    }
}

// We use ComputeFnBox (below) as the primary compute function type.
// A trait-based approach can be added later when trait async fn stabilizes further.

// We can't use trait_variant in practice yet, so let's use a simpler approach:
// A boxed async function type.

/// Type alias for a compute function — a boxed async closure.
pub type ComputeFnBox = Arc<
    dyn Fn(NodeKey, ComputeContext) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = HbResult<NodeValue>> + Send>,
    > + Send
    + Sync,
>;

/// Register a compute function from a closure.
pub fn compute_fn<F, Fut>(f: F) -> ComputeFnBox
where
    F: Fn(NodeKey, ComputeContext) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = HbResult<NodeValue>> + Send + 'static,
{
    Arc::new(move |key, ctx| Box::pin(f(key, ctx)))
}
