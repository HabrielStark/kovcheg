//! ARK Cold-Mirror — Rust workspace placeholder.
//!
//! "But I have prayed for you, that your faith should not fail." — Luke 22:32
//!
//! The full native harm-prediction backend is being ported from the Python
//! reference shim in `src/lib.py`, which is the canonical runtime today.
//!
//! This minimal Rust facade exists so that:
//!   * `cargo check` succeeds for the whole workspace.
//!   * Downstream Rust crates can depend on `cold_mirror` without breaking
//!     compilation.
//!   * The native implementation has a stable type surface to grow into.

#![deny(missing_docs)]
#![warn(clippy::all)]

use serde::{Deserialize, Serialize};

/// Version of the Cold-Mirror system (kept in sync with Cargo.toml).
pub const COLD_MIRROR_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum batch size for inference per SRS-SW02 (≤ 50 ms / 512 events).
pub const MAX_BATCH_SIZE: usize = 512;

/// Coarse risk level used by the public Cold-Mirror API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk.
    Low,
    /// Medium risk.
    Medium,
    /// High risk.
    High,
    /// Critical risk.
    Critical,
}

/// Coarse harm category used by the public Cold-Mirror API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarmCategory {
    /// Physical harm.
    Physical,
    /// Psychological harm.
    Psychological,
    /// Spiritual harm.
    Spiritual,
    /// Unknown / unclassified.
    Unknown,
}

/// Output of a Cold-Mirror evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarmPrediction {
    /// Text snippet that was evaluated.
    pub text: String,
    /// Coarse risk level.
    pub risk_level: RiskLevel,
    /// Coarse harm category.
    pub category: HarmCategory,
}

/// Lightweight predictor placeholder. The real implementation lives in the
/// Python shim (`software/cold_mirror/src/lib.py`).
#[derive(Debug, Default)]
pub struct HarmPredictor;

impl HarmPredictor {
    /// Construct a new predictor with the default biblical filter active.
    pub fn new() -> Self {
        Self
    }

    /// Predict harm for a single text snippet. The placeholder defaults to
    /// [`RiskLevel::Low`].
    pub fn predict(&self, text: &str) -> HarmPrediction {
        HarmPrediction {
            text: text.to_owned(),
            risk_level: RiskLevel::Low,
            category: HarmCategory::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!COLD_MIRROR_VERSION.is_empty());
    }

    #[test]
    fn predictor_returns_low_risk_by_default() {
        let predictor = HarmPredictor::new();
        let prediction = predictor.predict("Peaceful greeting");
        assert_eq!(prediction.risk_level, RiskLevel::Low);
        assert_eq!(prediction.category, HarmCategory::Unknown);
        assert_eq!(prediction.text, "Peaceful greeting");
    }
}
