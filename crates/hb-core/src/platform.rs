//! Platform abstraction layer.
//!
//! Detects CPU count, available memory, OS capabilities,
//! and GPU availability for resource-aware scheduling.

use serde::{Deserialize, Serialize};

/// Information about the current platform and its resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Operating system
    pub os: Os,
    /// CPU architecture
    pub arch: Arch,
    /// Number of logical CPU cores
    pub cpu_count: usize,
    /// Total system memory in bytes
    pub total_memory: u64,
    /// Available system memory in bytes (approximate)
    pub available_memory: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Os {
    Linux,
    MacOS,
    Windows,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Arch {
    X86_64,
    Aarch64,
    Unknown,
}

impl PlatformInfo {
    /// Detect current platform information.
    pub fn detect() -> Self {
        let os = if cfg!(target_os = "linux") {
            Os::Linux
        } else if cfg!(target_os = "macos") {
            Os::MacOS
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Unknown
        };

        let arch = if cfg!(target_arch = "x86_64") {
            Arch::X86_64
        } else if cfg!(target_arch = "aarch64") {
            Arch::Aarch64
        } else {
            Arch::Unknown
        };

        let cpu_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        // Approximate memory detection
        let (total_memory, available_memory) = detect_memory();

        Self {
            os,
            arch,
            cpu_count,
            total_memory,
            available_memory,
        }
    }

    /// Returns a human-readable string for the platform.
    pub fn display_string(&self) -> String {
        format!(
            "{:?}-{:?} ({} CPUs, {} RAM)",
            self.os,
            self.arch,
            self.cpu_count,
            format_bytes(self.total_memory),
        )
    }

    /// Returns whether namespace sandboxing is supported.
    pub fn supports_namespace_sandbox(&self) -> bool {
        self.os == Os::Linux
    }

    /// Returns whether sandbox-exec is supported.
    pub fn supports_sandbox_exec(&self) -> bool {
        self.os == Os::MacOS
    }
}

impl std::fmt::Display for PlatformInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_string())
    }
}

/// Detect total and available system memory.
fn detect_memory() -> (u64, u64) {
    // For MVP, use a reasonable default. In production, we'd use
    // sysinfo or platform-specific APIs.
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            let mut total = 0u64;
            let mut available = 0u64;
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    total = parse_meminfo_kb(line) * 1024;
                } else if line.starts_with("MemAvailable:") {
                    available = parse_meminfo_kb(line) * 1024;
                }
            }
            if total > 0 {
                return (total, available);
            }
        }
    }
    // Default fallback: 8 GB total, 4 GB available
    (8 * 1024 * 1024 * 1024, 4 * 1024 * 1024 * 1024)
}

#[cfg(target_os = "linux")]
fn parse_meminfo_kb(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Format bytes into human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
