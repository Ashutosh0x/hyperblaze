//! Virtual Filesystem abstraction.
//!
//! Provides a unified interface for file operations, supporting:
//! - Real filesystem (default)
//! - Content-addressable views
//! - File change tracking

use crate::digest::ContentDigest;
use dashmap::DashMap;
use std::path::{Path, PathBuf};

/// Cached file metadata for change detection.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Full path
    pub path: PathBuf,
    /// Content digest (lazily computed)
    pub digest: Option<ContentDigest>,
    /// Last modified time (for fast change detection)
    pub mtime: u64,
    /// File size in bytes
    pub size: u64,
}

/// A thread-safe file metadata cache for fast change detection.
///
/// Instead of re-hashing files on every build, we cache (path → mtime + digest)
/// and only re-hash when mtime changes. This is the same strategy Bazel uses
/// in its FileStateValue, but without the JVM overhead.
pub struct FileCache {
    entries: DashMap<PathBuf, FileInfo>,
}

impl FileCache {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    /// Get or compute the digest for a file.
    /// Returns cached digest if mtime hasn't changed, otherwise re-hashes.
    pub fn digest_of(&self, path: &Path) -> std::io::Result<ContentDigest> {
        let metadata = std::fs::metadata(path)?;
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let size = metadata.len();

        // Check cache
        if let Some(entry) = self.entries.get(path)
            && entry.mtime == mtime
            && entry.size == size
            && let Some(ref digest) = entry.digest
        {
            return Ok(digest.clone());
        }

        // Cache miss — compute digest
        let digest = ContentDigest::of_file(path)?;

        self.entries.insert(
            path.to_path_buf(),
            FileInfo {
                path: path.to_path_buf(),
                digest: Some(digest.clone()),
                mtime,
                size,
            },
        );

        Ok(digest)
    }

    /// Invalidate a specific path (called when file watcher detects a change).
    pub fn invalidate(&self, path: &Path) {
        self.entries.remove(path);
    }

    /// Invalidate all cached entries.
    pub fn invalidate_all(&self) {
        self.entries.clear();
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for FileCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively collect all files in a directory matching a glob pattern.
pub fn glob_files(dir: &Path, pattern: &str) -> std::io::Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    glob_files_recursive(dir, pattern, &mut results)?;
    results.sort();
    Ok(results)
}

fn glob_files_recursive(
    dir: &Path,
    pattern: &str,
    results: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectories (handle ** patterns)
            if pattern.contains("**") {
                glob_files_recursive(&path, pattern, results)?;
            }
        } else if matches_glob(&path, pattern) {
            results.push(path);
        }
    }

    Ok(())
}

/// Simple glob matching (supports *, **, and file extensions).
fn matches_glob(path: &Path, pattern: &str) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Handle simple extension patterns like "*.rs"
    if let Some(ext_pattern) = pattern.strip_prefix("*.") {
        return path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext == ext_pattern);
    }

    // Handle ** patterns like "**/*.rs"
    if let Some(suffix) = pattern.strip_prefix("**/") {
        return matches_glob(path, suffix);
    }

    // Exact match
    file_name == pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_glob_extension() {
        assert!(matches_glob(Path::new("src/main.rs"), "*.rs"));
        assert!(!matches_glob(Path::new("src/main.go"), "*.rs"));
    }

    #[test]
    fn test_matches_glob_recursive() {
        assert!(matches_glob(Path::new("src/lib/foo.rs"), "**/*.rs"));
    }

    #[test]
    fn test_file_cache_empty() {
        let cache = FileCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }
}
