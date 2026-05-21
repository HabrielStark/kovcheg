//! ARK Ethics DSL — Rust workspace placeholder.
//!
//! "Test everything; hold fast what is good." — 1 Thessalonians 5:21
//!
//! The full Rust implementation of the Ethics DSL is still being ported from
//! the Python reference shim in `src/lib.py`. The Python implementation is
//! the canonical runtime used by the project's integration and unit tests
//! today.
//!
//! This minimal Rust facade exists so that:
//!   * `cargo check` and the workspace-wide build complete successfully.
//!   * Downstream Rust crates (e.g. `patch_orchestrator`) can name the
//!     `ethics_dsl` crate in their dependency graph without compilation
//!     errors.
//!   * The future, fully native engine has a stable type surface to grow
//!     into.

#![deny(missing_docs)]
#![warn(clippy::all)]

use serde::{Deserialize, Serialize};

/// Version of the Ethics DSL (kept in sync with Cargo.toml).
pub const DSL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Output of an Ethics DSL evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    /// Action/content is permitted.
    Allow,
    /// Action/content is denied.
    Deny,
    /// Action/content must be removed.
    Purge,
}

/// Lightweight engine placeholder. The full implementation lives in the
/// Python shim (`software/ethics_dsl/src/lib.py`).
#[derive(Debug, Default)]
pub struct EthicsEngine;

impl EthicsEngine {
    /// Construct a new engine seeded with the Biblical moral foundation.
    pub fn new_with_biblical_foundation() -> Self {
        Self
    }

    /// Stable evaluation entry point — currently defaults to [`Decision::Allow`].
    pub fn evaluate(&self, _payload: &str) -> Decision {
        Decision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!DSL_VERSION.is_empty());
    }

    #[test]
    fn engine_defaults_to_allow() {
        let engine = EthicsEngine::new_with_biblical_foundation();
        assert_eq!(engine.evaluate("anything"), Decision::Allow);
    }
}
