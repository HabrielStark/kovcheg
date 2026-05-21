//! Post-Quantum TLS skeleton.
//!
//! "A cord of three strands is not quickly broken." — Ecclesiastes 4:12
//!
//! This module defines the public surface for the future PQ-TLS handshake.
//! The real cryptographic operations are intentionally not implemented in
//! this scaffold; callers receive [`PQTlsError::NotImplemented`] until the
//! production engine lands.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Post-quantum TLS errors.
#[derive(Debug, Error)]
pub enum PQTlsError {
    /// Operation has not been implemented yet.
    #[error("post-quantum TLS operation not yet implemented")]
    NotImplemented,

    /// Wrapped protocol error.
    #[error("protocol error: {0}")]
    Protocol(String),
}

/// Supported (or planned) post-quantum algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PQAlgorithm {
    /// Hybrid X25519 + Kyber768.
    HybridX25519Kyber768,
    /// Hybrid Ed25519 + Dilithium3.
    HybridEd25519Dilithium3,
    /// Pure Kyber768.
    Kyber768,
    /// Pure Dilithium3.
    Dilithium3,
}

/// PQ-TLS configuration container.
#[derive(Debug, Clone)]
pub struct PQTlsConfig {
    /// Algorithms in preference order.
    pub supported_algorithms: Vec<PQAlgorithm>,
    /// Whether to require PQ algorithms.
    pub require_pq: bool,
}

impl Default for PQTlsConfig {
    fn default() -> Self {
        Self {
            supported_algorithms: vec![
                PQAlgorithm::HybridX25519Kyber768,
                PQAlgorithm::HybridEd25519Dilithium3,
                PQAlgorithm::Kyber768,
                PQAlgorithm::Dilithium3,
            ],
            require_pq: true,
        }
    }
}

/// Skeleton handshake handler.
#[derive(Debug, Clone)]
pub struct PQHandshake {
    config: PQTlsConfig,
    is_client: bool,
}

impl PQHandshake {
    /// Create a new handshake handler.
    pub fn new(config: PQTlsConfig, is_client: bool) -> Self {
        Self { config, is_client }
    }

    /// Return whether this handshake is operating as a client.
    pub fn is_client(&self) -> bool {
        self.is_client
    }

    /// Return the negotiated algorithm preference list (currently identical
    /// to the configured supported set).
    pub fn preferred_algorithms(&self) -> &[PQAlgorithm] {
        &self.config.supported_algorithms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_includes_hybrid_algorithms() {
        let cfg = PQTlsConfig::default();
        assert!(cfg.require_pq);
        assert!(cfg
            .supported_algorithms
            .contains(&PQAlgorithm::HybridX25519Kyber768));
    }

    #[test]
    fn handshake_records_role() {
        let handshake = PQHandshake::new(PQTlsConfig::default(), true);
        assert!(handshake.is_client());
        assert!(!handshake.preferred_algorithms().is_empty());
    }
}
