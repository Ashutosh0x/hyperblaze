//! BLAKE3-based content hashing for Hyperblaze.
//!
//! BLAKE3 is 3-5x faster than SHA256. We use it as the default digest
//! algorithm for content-addressable storage and action cache keys.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// A content digest — the hash of file contents or action inputs.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentDigest {
    /// The raw hash bytes (32 bytes for BLAKE3)
    bytes: [u8; 32],
    /// Size of the hashed content in bytes
    size: u64,
}

impl ContentDigest {
    /// Hash a byte slice.
    pub fn of_bytes(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self {
            bytes: *hash.as_bytes(),
            size: data.len() as u64,
        }
    }

    /// Hash a file on disk.
    pub fn of_file(path: &Path) -> std::io::Result<Self> {
        let data = std::fs::read(path)?;
        Ok(Self::of_bytes(&data))
    }

    /// Hash a file asynchronously.
    pub async fn of_file_async(path: &Path) -> std::io::Result<Self> {
        let data = tokio::fs::read(path).await?;
        // BLAKE3 is so fast that spawning a blocking task isn't worth it
        // for files under ~100MB
        Ok(Self::of_bytes(&data))
    }

    /// Create a digest for an empty content.
    pub fn empty() -> Self {
        Self::of_bytes(b"")
    }

    /// Combine multiple digests into a single digest (for action cache keys).
    pub fn combine(digests: &[&ContentDigest]) -> Self {
        let mut hasher = blake3::Hasher::new();
        for d in digests {
            hasher.update(&d.bytes);
            hasher.update(&d.size.to_le_bytes());
        }
        let hash = hasher.finalize();
        let total_size: u64 = digests.iter().map(|d| d.size).sum();
        Self {
            bytes: *hash.as_bytes(),
            size: total_size,
        }
    }

    /// Hash a string (for command lines, env vars, etc.)
    pub fn of_str(s: &str) -> Self {
        Self::of_bytes(s.as_bytes())
    }

    /// Get the hex representation of the hash.
    pub fn hex(&self) -> String {
        hex_encode(&self.bytes)
    }

    /// Get the size of the hashed content.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl fmt::Debug for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({}:{} bytes)", &self.hex()[..16], self.size)
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hex())
    }
}

/// Fast hex encoding without allocating a formatter.
fn hex_encode(bytes: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        result.push(HEX_CHARS[(b >> 4) as usize] as char);
        result.push(HEX_CHARS[(b & 0x0f) as usize] as char);
    }
    result
}

/// Compute the action cache key for a build action.
///
/// Key = BLAKE3(sorted_input_digests + argv + sorted_env_vars)
pub fn action_cache_key(
    input_digests: &[ContentDigest],
    argv: &[String],
    env: &[(String, String)],
) -> ContentDigest {
    let mut hasher = blake3::Hasher::new();

    // Domain separator
    hasher.update(b"hyperblaze-action-v1");

    // Sorted input digests
    let mut sorted_inputs: Vec<_> = input_digests.iter().collect();
    sorted_inputs.sort_by_key(|d| d.bytes);
    for d in &sorted_inputs {
        hasher.update(&d.bytes);
        hasher.update(&d.size.to_le_bytes());
    }

    // Command line
    hasher.update(b"argv:");
    for arg in argv {
        hasher.update(arg.as_bytes());
        hasher.update(b"\0");
    }

    // Sorted environment
    let mut sorted_env: Vec<_> = env.iter().collect();
    sorted_env.sort_by_key(|(k, _)| k.as_str());
    hasher.update(b"env:");
    for (k, v) in &sorted_env {
        hasher.update(k.as_bytes());
        hasher.update(b"=");
        hasher.update(v.as_bytes());
        hasher.update(b"\0");
    }

    let hash = hasher.finalize();
    ContentDigest {
        bytes: *hash.as_bytes(),
        size: 0, // Size doesn't apply to action keys
    }
}

/// Convenience: hash a file on disk. Returns error as HbError.
pub fn digest_file(path: &Path) -> Result<ContentDigest, std::io::Error> {
    ContentDigest::of_file(path)
}

/// Convenience: combine multiple digests into one.
pub fn combine_digests(digests: &[ContentDigest]) -> ContentDigest {
    let refs: Vec<&ContentDigest> = digests.iter().collect();
    ContentDigest::combine(&refs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digest_deterministic() {
        let d1 = ContentDigest::of_bytes(b"hello world");
        let d2 = ContentDigest::of_bytes(b"hello world");
        assert_eq!(d1, d2);
        assert_eq!(d1.hex(), d2.hex());
    }

    #[test]
    fn test_digest_different_content() {
        let d1 = ContentDigest::of_bytes(b"hello");
        let d2 = ContentDigest::of_bytes(b"world");
        assert_ne!(d1, d2);
    }

    #[test]
    fn test_combine_digests() {
        let d1 = ContentDigest::of_bytes(b"hello");
        let d2 = ContentDigest::of_bytes(b"world");
        let combined = ContentDigest::combine(&[&d1, &d2]);
        assert_eq!(combined.size(), d1.size() + d2.size());
    }

    #[test]
    fn test_action_cache_key_deterministic() {
        let inputs = vec![ContentDigest::of_bytes(b"input1")];
        let argv = vec!["rustc".to_string(), "main.rs".to_string()];
        let env = vec![("PATH".to_string(), "/usr/bin".to_string())];

        let key1 = action_cache_key(&inputs, &argv, &env);
        let key2 = action_cache_key(&inputs, &argv, &env);
        assert_eq!(key1, key2);
    }
}
