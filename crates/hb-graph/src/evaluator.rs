//! Parallel evaluator — executes compute functions across the graph.
//!
//! Uses tokio's work-stealing runtime for natural parallelism.
//! Unlike Bazel's ParallelEvaluator which uses a fixed thread pool
//! and manually manages node scheduling, we leverage tokio's async
//! runtime to automatically balance work across threads.

use crate::function::ComputeContext;
use crate::function::ComputeFnBox;
use crate::graph::HyperGraph;
use crate::key::{FunctionType, NodeKey};
use crate::value::NodeValue;
use dashmap::DashMap;
use hb_core::error::{HbError, HbResult};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, warn};

/// The parallel evaluator — drives computation across the HyperGraph.
pub struct Evaluator {
    /// The dependency graph
    graph: Arc<HyperGraph>,

    /// Registered compute functions, one per FunctionType
    functions: DashMap<FunctionType, ComputeFnBox>,

    /// Maximum concurrent evaluations (0 = unlimited)
    max_concurrency: usize,
}

/// Result of evaluating a set of root keys.
#[derive(Debug)]
pub struct EvalResult {
    /// The computed values, in the same order as the requested keys
    pub values: Vec<NodeValue>,
    /// Total wall-clock time
    pub elapsed: std::time::Duration,
    /// Number of nodes evaluated (not cached)
    pub evaluated: usize,
    /// Number of cache hits
    pub cached: usize,
    /// Number of errors
    pub errors: usize,
}

impl Evaluator {
    /// Create a new evaluator for the given graph.
    pub fn new(graph: Arc<HyperGraph>) -> Self {
        Self {
            graph,
            functions: DashMap::new(),
            max_concurrency: 0,
        }
    }

    /// Set the maximum number of concurrent evaluations.
    pub fn with_max_concurrency(mut self, max: usize) -> Self {
        self.max_concurrency = max;
        self
    }

    /// Register a compute function for a given FunctionType.
    pub fn register(&self, ft: FunctionType, func: ComputeFnBox) {
        self.functions.insert(ft, func);
    }

    /// Evaluate a single key and return its value.
    pub async fn evaluate(&self, key: &NodeKey) -> HbResult<NodeValue> {
        let start = Instant::now();

        // Check if already computed and clean
        if let Some(entry) = self.graph.get(key) {
            let state = entry.state().await;
            if state.is_done() && !entry.is_dirty() {
                debug!("Cache hit for {}", key);
                return state.value().cloned().ok_or_else(|| {
                    HbError::Internal(format!("Node {} is Done but has no value", key))
                });
            }
        }

        // Need to evaluate — spawn the computation
        let value = self.evaluate_node(key.clone()).await?;

        debug!("Evaluated {} in {:?}", key, start.elapsed());
        Ok(value)
    }

    /// Evaluate multiple root keys in parallel.
    pub async fn evaluate_many(&self, keys: &[NodeKey]) -> HbResult<EvalResult> {
        let start = Instant::now();
        let mut join_set: tokio::task::JoinSet<HbResult<(NodeKey, NodeValue, bool)>> =
            tokio::task::JoinSet::new();
        let mut evaluated = 0usize;
        let mut cached = 0usize;
        let mut errors = 0usize;

        for key in keys {
            let evaluator_graph = self.graph.clone();
            let functions = self.clone_functions();
            let key = key.clone();

            join_set.spawn(async move {
                // Check cache first
                if let Some(entry) = evaluator_graph.get(&key) {
                    let state = entry.state().await;
                    if state.is_done() && !entry.is_dirty() {
                        return Ok((key, state.value().cloned().unwrap(), true));
                    }
                }

                // Evaluate
                let value =
                    Self::evaluate_node_static(evaluator_graph, &functions, key.clone()).await?;
                Ok((key, value, false))
            });
        }

        let mut results: Vec<(usize, NodeValue)> = Vec::new();
        let key_indices: std::collections::HashMap<_, _> = keys
            .iter()
            .enumerate()
            .map(|(i, k)| (k.clone(), i))
            .collect();

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok((key, value, was_cached))) => {
                    if was_cached {
                        cached += 1;
                    } else {
                        evaluated += 1;
                    }
                    if let Some(&idx) = key_indices.get(&key) {
                        results.push((idx, value));
                    }
                }
                Ok(Err(e)) => {
                    errors += 1;
                    warn!("Evaluation error: {}", e);
                }
                Err(e) => {
                    errors += 1;
                    warn!("Join error: {}", e);
                }
            }
        }

        // Sort results by original key order
        results.sort_by_key(|(idx, _)| *idx);

        if errors > 0 && results.len() < keys.len() {
            return Err(HbError::GraphEvaluation(format!(
                "{} out of {} evaluations failed",
                errors,
                keys.len()
            )));
        }

        Ok(EvalResult {
            values: results.into_iter().map(|(_, v)| v).collect(),
            elapsed: start.elapsed(),
            evaluated,
            cached,
            errors,
        })
    }

    /// Evaluate a single node (internal).
    async fn evaluate_node(&self, key: NodeKey) -> HbResult<NodeValue> {
        let functions = self.clone_functions();
        Self::evaluate_node_static(self.graph.clone(), &functions, key).await
    }

    /// Static version of evaluate_node (for use in spawned tasks).
    async fn evaluate_node_static(
        graph: Arc<HyperGraph>,
        functions: &DashMap<FunctionType, ComputeFnBox>,
        key: NodeKey,
    ) -> HbResult<NodeValue> {
        let entry = graph.get_or_create(&key);

        // Try to claim this node for evaluation
        if !entry.start_evaluation().await {
            // Someone else is already evaluating it — just wait
            return entry
                .wait_for_completion()
                .await
                .map_err(|e| HbError::GraphEvaluation(format!("{}", e)));
        }

        // Find the compute function
        let func = functions.get(&key.function_type).ok_or_else(|| {
            HbError::Internal(format!(
                "No compute function registered for {:?}",
                key.function_type
            ))
        })?;
        let func = func.value().clone();
        let _ = functions; // Release the DashMap ref

        // Create the compute context
        let version = graph.version();
        let ctx = ComputeContext::new(graph.clone(), key.clone(), version);

        // Run the compute function
        match func(key.clone(), ctx).await {
            Ok(value) => {
                entry.complete(value.clone(), version, None).await;
                Ok(value)
            }
            Err(err) => {
                entry
                    .fail(HbError::GraphEvaluation(format!("{}", err)))
                    .await;
                Err(err)
            }
        }
    }

    /// Clone the functions map for use in spawned tasks.
    fn clone_functions(&self) -> DashMap<FunctionType, ComputeFnBox> {
        let new_map = DashMap::new();
        for entry in self.functions.iter() {
            new_map.insert(*entry.key(), entry.value().clone());
        }
        new_map
    }

    /// Get a reference to the underlying graph.
    pub fn graph(&self) -> &Arc<HyperGraph> {
        &self.graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::compute_fn;

    #[tokio::test]
    async fn test_simple_evaluation() {
        let graph = Arc::new(HyperGraph::new());
        let evaluator = Evaluator::new(graph.clone());

        // Register a simple file state function
        evaluator.register(
            FunctionType::FileState,
            compute_fn(|key, _ctx| async move {
                Ok(NodeValue::new(format!("content_of_{}", key.argument())))
            }),
        );

        let key = NodeKey::file_state("/src/main.rs");
        let value = evaluator.evaluate(&key).await.unwrap();
        let content: &String = value.expect("expected String");
        assert_eq!(content, "content_of_/src/main.rs");
    }

    #[tokio::test]
    async fn test_evaluation_with_deps() {
        let graph = Arc::new(HyperGraph::new());
        let evaluator = Evaluator::new(graph.clone());

        // File state function
        evaluator.register(
            FunctionType::FileState,
            compute_fn(|key, _ctx| async move {
                Ok(NodeValue::new(format!("file:{}", key.argument())))
            }),
        );

        // Package function that depends on file state
        evaluator.register(
            FunctionType::PackageLoad,
            compute_fn(|key, mut ctx| async move {
                let file_key = NodeKey::file_state(&format!("{}/BUILD.hb", key.argument()));
                let _file_value = ctx.require(file_key).await?;
                Ok(NodeValue::new(format!("package:{}", key.argument())))
            }),
        );

        // First, evaluate the file (so the dep is available)
        let file_key = NodeKey::file_state("//src/BUILD.hb");
        evaluator.evaluate(&file_key).await.unwrap();

        // Now evaluate the package
        let pkg_key = NodeKey::package_load("//src");
        let value = evaluator.evaluate(&pkg_key).await.unwrap();
        let pkg: &String = value.expect("expected String");
        assert_eq!(pkg, "package://src");
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let graph = Arc::new(HyperGraph::new());
        let evaluator = Evaluator::new(graph);

        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cc = call_count.clone();

        evaluator.register(
            FunctionType::FileState,
            compute_fn(move |key, _ctx| {
                let cc = cc.clone();
                async move {
                    cc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(NodeValue::new(format!("v:{}", key.argument())))
                }
            }),
        );

        let key = NodeKey::file_state("test");

        // First call — should evaluate
        evaluator.evaluate(&key).await.unwrap();
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Second call — should use cache (no re-evaluation)
        evaluator.evaluate(&key).await.unwrap();
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
