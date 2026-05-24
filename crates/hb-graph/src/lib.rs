//! # HyperGraph — Async Incremental Computation Engine
//!
//! The heart of Hyperblaze. Equivalent to Bazel's Skyframe, but fundamentally
//! better:
//!
//! - **No restart protocol**: Functions suspend via async/await instead of
//!   returning null and being restarted from scratch.
//! - **Lock-free graph**: Uses DashMap for concurrent access without locks.
//! - **Work-stealing scheduler**: Efficient parallel evaluation.
//! - **Disk-backed persistence**: Graph survives daemon restarts.
//!
//! ## Architecture
//!
//! ```text
//! NodeKey ──▶ ComputeFn::compute() ──▶ NodeValue
//!                   │
//!                   ▼ (async)
//!            ctx.require(dep_key).await
//!            (suspends, doesn't restart!)
//! ```

pub mod evaluator;
pub mod function;
pub mod graph;
pub mod key;
pub mod metrics;
pub mod node;
pub mod value;

pub use evaluator::Evaluator;
pub use function::{ComputeContext, ComputeFnBox, compute_fn};
pub use graph::HyperGraph;
pub use key::NodeKey;
pub use metrics::GraphMetrics;
pub use node::{NodeEntry, NodeState};
pub use value::NodeValue;
