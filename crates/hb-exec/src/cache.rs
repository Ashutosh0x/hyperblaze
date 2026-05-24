//! Local disk action cache — stores action results keyed by content hash.
//!
//! On cache hit, outputs are returned in <1ms. This is what makes
//! incremental builds feel instant.

use crate::action::{Action, ActionOutput};
use hb_core::error::{HbError, HbResult};
use std::path::{Path, PathBuf};
use tracing::debug;

/// A local disk-based action cache.
///
/// Structure:
/// ```text
/// .hb-cache/
///   ac/                    # Action cache (key → result metadata)
///     ab/cd1234.json       # Sharded by first 2 hex chars
///   cas/                   # Content-addressable store (digest → file)
///     ef/gh5678            # Sharded by first 2 hex chars
/// ```
pub struct ActionCache {
    cache_dir: PathBuf,
}

impl ActionCache {
    /// Create a new action cache at the given directory.
    pub fn new(cache_dir: &Path) -> HbResult<Self> {
        let ac_dir = cache_dir.join("ac");
        let cas_dir = cache_dir.join("cas");
        std::fs::create_dir_all(&ac_dir)?;
        std::fs::create_dir_all(&cas_dir)?;
        Ok(Self {
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Look up a cached action result.
    pub fn lookup(&self, action: &Action) -> Option<ActionOutput> {
        let key = action.cache_key();
        let cache_path = self.ac_path(&key.hex());

        if !cache_path.exists() {
            return None;
        }

        match std::fs::read_to_string(&cache_path) {
            Ok(content) => match serde_json::from_str::<ActionOutput>(&content) {
                Ok(mut result) => {
                    result.cache_hit = true;
                    debug!("Cache hit for action: {}", action.description);
                    Some(result)
                }
                Err(e) => {
                    debug!("Cache entry corrupt, ignoring: {}", e);
                    None
                }
            },
            Err(_) => None,
        }
    }

    /// Store an action result in the cache.
    pub fn store(&self, action: &Action, result: &ActionOutput) -> HbResult<()> {
        if result.exit_code != 0 {
            return Ok(()); // Don't cache failures
        }

        let key = action.cache_key();
        let cache_path = self.ac_path(&key.hex());

        // Ensure parent directory exists
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(result)
            .map_err(|e| HbError::Cache(format!("Failed to serialize action result: {}", e)))?;

        std::fs::write(&cache_path, content)?;
        debug!("Cached action result: {}", action.description);
        Ok(())
    }

    /// Get the path for an action cache entry (sharded by first 2 hex chars).
    fn ac_path(&self, hex_key: &str) -> PathBuf {
        let shard = &hex_key[..2.min(hex_key.len())];
        self.cache_dir
            .join("ac")
            .join(shard)
            .join(format!("{}.json", hex_key))
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let ac_dir = self.cache_dir.join("ac");
        let mut total_entries = 0u64;
        let mut total_size = 0u64;

        if let Ok(entries) = walkdir(&ac_dir) {
            for entry in entries {
                total_entries += 1;
                total_size += entry.1;
            }
        }

        CacheStats {
            entries: total_entries,
            size_bytes: total_size,
        }
    }

    /// Clear the entire cache.
    pub fn clear(&self) -> HbResult<()> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
            std::fs::create_dir_all(self.cache_dir.join("ac"))?;
            std::fs::create_dir_all(self.cache_dir.join("cas"))?;
        }
        Ok(())
    }
}

/// Cache statistics.
#[derive(Debug)]
pub struct CacheStats {
    pub entries: u64,
    pub size_bytes: u64,
}

/// Simple recursive directory walk returning (path, size) pairs.
fn walkdir(dir: &Path) -> std::io::Result<Vec<(PathBuf, u64)>> {
    let mut results = Vec::new();
    if !dir.is_dir() {
        return Ok(results);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            results.extend(walkdir(&path)?);
        } else {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            results.push((path, size));
        }
    }
    Ok(results)
}
