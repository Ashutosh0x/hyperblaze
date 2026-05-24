//! File watcher — detects filesystem changes and triggers graph invalidation.
//!
//! Uses the `notify` crate for cross-platform file watching
//! (inotify on Linux, FSEvents on macOS, ReadDirectoryChanges on Windows).

use crate::error::{HbError, HbResult};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{info, warn};

/// File change events emitted by the watcher.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: FileChangeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Async file watcher that monitors a directory tree for changes.
pub struct FileWatcher {
    /// The watcher handle (kept alive to maintain watches)
    _watcher: RecommendedWatcher,
    /// Channel receiver for file change events
    rx: mpsc::UnboundedReceiver<FileChange>,
    /// Root directory being watched
    root: PathBuf,
}

impl FileWatcher {
    /// Create a new file watcher monitoring the given root directory.
    pub fn new(root: &Path) -> HbResult<Self> {
        let (tx, rx) = mpsc::unbounded_channel();

        let sender = tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    let kind = match event.kind {
                        EventKind::Create(_) => Some(FileChangeKind::Created),
                        EventKind::Modify(_) => Some(FileChangeKind::Modified),
                        EventKind::Remove(_) => Some(FileChangeKind::Deleted),
                        _ => None,
                    };

                    if let Some(kind) = kind {
                        for path in event.paths {
                            // Skip build output and cache directories
                            let path_str = path.to_string_lossy();
                            if path_str.contains(".hb-out")
                                || path_str.contains(".hb-cache")
                                || path_str.contains(".git")
                                || path_str.contains("target")
                            {
                                continue;
                            }

                            let _ = sender.send(FileChange { path, kind });
                        }
                    }
                }
                Err(e) => {
                    warn!("File watcher error: {}", e);
                }
            }
        })
        .map_err(|e| HbError::Internal(format!("Failed to create file watcher: {}", e)))?;

        watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(|e| HbError::Internal(format!("Failed to watch directory: {}", e)))?;

        info!("File watcher started on: {}", root.display());

        Ok(Self {
            _watcher: watcher,
            rx,
            root: root.to_path_buf(),
        })
    }

    /// Receive the next file change event (async).
    pub async fn next_change(&mut self) -> Option<FileChange> {
        self.rx.recv().await
    }

    /// Drain all pending changes (non-blocking).
    /// Useful for batching multiple rapid changes into one invalidation cycle.
    pub fn drain_pending(&mut self) -> Vec<FileChange> {
        let mut changes = Vec::new();
        while let Ok(change) = self.rx.try_recv() {
            changes.push(change);
        }
        // Deduplicate by path (keep last change kind)
        let mut seen = std::collections::HashMap::new();
        for change in changes {
            seen.insert(change.path.clone(), change);
        }
        seen.into_values().collect()
    }

    /// Get the root directory being watched.
    pub fn root(&self) -> &Path {
        &self.root
    }
}
