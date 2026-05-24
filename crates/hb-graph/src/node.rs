//! Graph nodes — the state machine for each computation.
//!
//! Each node in the HyperGraph has a lifecycle:
//! NeedsEvaluation → InFlight → Done (or Error)
//!
//! When a dependency changes, Done → Dirty → NeedsEvaluation.

use crate::key::NodeKey;
use crate::value::NodeValue;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, watch};

/// The lifecycle state of a graph node.
#[derive(Debug)]
pub enum NodeState {
    /// Not yet evaluated, or invalidated and needs re-evaluation
    NeedsEvaluation,
    /// Currently being evaluated by a compute function
    InFlight,
    /// Successfully computed
    Done(NodeValue),
    /// Computation failed
    Error(Arc<hb_core::error::HbError>),
    /// Previously computed but a dependency changed
    Dirty(NodeValue),
}

impl NodeState {
    /// Returns true if the node has a valid value.
    pub fn is_done(&self) -> bool {
        matches!(self, NodeState::Done(_))
    }

    /// Returns true if the node needs (re-)evaluation.
    pub fn needs_evaluation(&self) -> bool {
        matches!(self, NodeState::NeedsEvaluation | NodeState::Dirty(_))
    }

    /// Extract the value if in Done state.
    pub fn value(&self) -> Option<&NodeValue> {
        match self {
            NodeState::Done(v) | NodeState::Dirty(v) => Some(v),
            _ => None,
        }
    }
}

/// A node in the HyperGraph.
///
/// Equivalent to Bazel's NodeEntry, but designed for async evaluation
/// without the restart protocol.
pub struct NodeEntry {
    /// The key identifying this node
    pub key: NodeKey,

    /// Current state (protected by RwLock for concurrent access)
    state: RwLock<NodeState>,

    /// Notifies waiters when this node's state changes.
    notify: watch::Sender<u64>,

    /// Receiver for the notify channel
    notify_rx: watch::Receiver<u64>,

    /// Direct dependencies (keys this node depends on)
    deps: RwLock<Vec<NodeKey>>,

    /// Reverse dependencies (keys that depend on this node)
    rdeps: RwLock<Vec<NodeKey>>,

    /// Version when this node was last computed
    version: AtomicU64,

    /// Version when this node was last dirtied
    dirty_version: AtomicU64,

    /// BLAKE3 digest of the last computed value (for early cutoff).
    /// If a recomputation produces the same digest, we skip invalidating rdeps.
    value_digest: RwLock<Option<hb_core::digest::ContentDigest>>,
}

impl NodeEntry {
    /// Create a new node in NeedsEvaluation state.
    pub fn new(key: NodeKey) -> Self {
        let (tx, rx) = watch::channel(0u64);
        Self {
            key,
            state: RwLock::new(NodeState::NeedsEvaluation),
            notify: tx,
            notify_rx: rx,
            deps: RwLock::new(Vec::new()),
            rdeps: RwLock::new(Vec::new()),
            version: AtomicU64::new(0),
            dirty_version: AtomicU64::new(0),
            value_digest: RwLock::new(None),
        }
    }

    /// Get the current state (read lock).
    pub async fn state(&self) -> tokio::sync::RwLockReadGuard<'_, NodeState> {
        self.state.read().await
    }

    /// Transition to InFlight state. Returns false if already InFlight.
    pub async fn start_evaluation(&self) -> bool {
        let mut state = self.state.write().await;
        if matches!(*state, NodeState::InFlight) {
            return false; // Already being evaluated
        }
        *state = NodeState::InFlight;
        true
    }

    /// Complete evaluation with a value.
    /// Returns `true` if the value changed (no early cutoff).
    /// Returns `false` if the value is the same as before (early cutoff — skip rdep invalidation).
    pub async fn complete(
        &self,
        value: NodeValue,
        version: u64,
        digest: Option<hb_core::digest::ContentDigest>,
    ) -> bool {
        // Early cutoff: compare digest with previous value
        let value_changed = if let Some(ref new_digest) = digest {
            let old_digest = self.value_digest.read().await;
            match &*old_digest {
                Some(old) => old != new_digest,
                None => true, // No previous digest = always changed
            }
        } else {
            true // No digest provided = assume changed
        };

        // Update the value digest
        if let Some(d) = digest {
            let mut vd = self.value_digest.write().await;
            *vd = Some(d);
        }

        {
            let mut state = self.state.write().await;
            *state = NodeState::Done(value);
        }
        self.version.store(version, Ordering::Release);
        // Wake all waiters
        let _ = self.notify.send(version);

        value_changed
    }

    /// Complete evaluation with an error.
    pub async fn fail(&self, error: hb_core::error::HbError) {
        let mut state = self.state.write().await;
        *state = NodeState::Error(Arc::new(error));
        // Still wake waiters so they can observe the error
        let _ = self.notify.send(u64::MAX);
    }

    /// Mark this node as dirty (a dependency changed).
    pub async fn mark_dirty(&self, dirty_version: u64) {
        let mut state = self.state.write().await;
        match std::mem::replace(&mut *state, NodeState::NeedsEvaluation) {
            NodeState::Done(v) => *state = NodeState::Dirty(v),
            other => *state = other, // Keep current state if not Done
        }
        self.dirty_version.store(dirty_version, Ordering::Release);
    }

    /// Wait until this node is done (or errored).
    /// This is the core of the no-restart protocol!
    pub async fn wait_for_completion(&self) -> Result<NodeValue, Arc<hb_core::error::HbError>> {
        loop {
            {
                let state = self.state.read().await;
                match &*state {
                    NodeState::Done(v) => return Ok(v.clone()),
                    NodeState::Error(e) => return Err(e.clone()),
                    _ => {} // Still waiting
                }
            }

            // Subscribe and wait for notification
            let mut rx = self.notify_rx.clone();
            // This is where the magic happens: the calling task SUSPENDS
            // (yields its thread back to the tokio runtime) instead of
            // returning null and restarting like Skyframe does.
            let _ = rx.changed().await;
        }
    }

    /// Add a forward dependency (a node this one depends on).
    pub async fn add_dep(&self, key: NodeKey) {
        let mut deps = self.deps.write().await;
        if !deps.contains(&key) {
            deps.push(key);
        }
    }

    /// Set the direct dependencies of this node.
    pub async fn set_deps(&self, new_deps: Vec<NodeKey>) {
        let mut deps = self.deps.write().await;
        *deps = new_deps;
    }

    /// Get the direct dependencies.
    pub async fn deps(&self) -> Vec<NodeKey> {
        self.deps.read().await.clone()
    }

    /// Add a reverse dependency (a node that depends on this one).
    pub async fn add_rdep(&self, key: NodeKey) {
        let mut rdeps = self.rdeps.write().await;
        if !rdeps.contains(&key) {
            rdeps.push(key);
        }
    }

    /// Get the reverse dependencies.
    pub async fn rdeps(&self) -> Vec<NodeKey> {
        self.rdeps.read().await.clone()
    }

    /// Get the version when this node was last computed.
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Check if this node is dirty (needs re-evaluation).
    pub fn is_dirty(&self) -> bool {
        self.dirty_version.load(Ordering::Acquire) > self.version.load(Ordering::Acquire)
    }
}

impl std::fmt::Debug for NodeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NodeEntry({}, v={}, dirty={})",
            self.key,
            self.version.load(Ordering::Relaxed),
            self.dirty_version.load(Ordering::Relaxed),
        )
    }
}
