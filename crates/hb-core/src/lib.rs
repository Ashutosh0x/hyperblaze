//! # hb-core — Hyperblaze Core Runtime
//!
//! Foundation crate providing:
//! - Configuration system (HYPERBLAZE.toml)
//! - Platform abstractions (Linux, macOS, Windows)
//! - Virtual filesystem with content-addressable storage
//! - BLAKE3 content hashing
//! - Error types and diagnostics
//! - File watching for incremental builds

pub mod config;
pub mod digest;
pub mod error;
pub mod platform;
pub mod vfs;
pub mod watcher;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::config::HyperblazeConfig;
    pub use crate::digest::ContentDigest;
    pub use crate::error::{HbError, HbResult};
    pub use crate::platform::PlatformInfo;
}

/// Hyperblaze version — baked in at compile time
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Product name
pub const PRODUCT_NAME: &str = "Hyperblaze";

/// ASCII art banner
pub const BANNER: &str = r#"
  ╦ ╦╦ ╦╔═╗╔═╗╦═╗╔╗ ╦  ╔═╗╔═╗╔═╗
  ╠═╣╚╦╝╠═╝║╣ ╠╦╝╠╩╗║  ╠═╣╔═╝║╣ 
  ╩ ╩ ╩ ╩  ╚═╝╩╚═╚═╝╩═╝╩ ╩╚═╝╚═╝
"#;
