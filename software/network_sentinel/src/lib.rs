//! ARK Network Sentinel — Rust workspace placeholder.
//!
//! "The Lord watches over all who love him." — Psalm 145:20
//!
//! The full Post-Quantum TLS implementation is being migrated from prior
//! prototypes. Today this crate provides a stable type surface and a small
//! CLI so the workspace builds and downstream tooling can target it.

#![deny(missing_docs)]
#![warn(clippy::all)]

pub mod pqc_tls;

pub use pqc_tls::{PQAlgorithm, PQTlsConfig, PQTlsError, PQHandshake};

use std::net::SocketAddr;
use thiserror::Error;

/// Network Sentinel errors surfaced to the binary / library callers.
#[derive(Error, Debug)]
pub enum SentinelError {
    /// Post-quantum TLS subsystem error.
    #[error("Post-quantum TLS error: {0}")]
    PQTls(#[from] PQTlsError),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Protocol-level error.
    #[error("Protocol error: {0}")]
    Protocol(String),
}

/// Configuration container for the Network Sentinel.
#[derive(Clone, Debug)]
pub struct SentinelConfig {
    /// Listen / connect address.
    pub bind_addr: SocketAddr,
    /// PQ-TLS configuration.
    pub pq_tls_config: PQTlsConfig,
    /// Maximum concurrent connections.
    pub max_connections: usize,
    /// Connection timeout in seconds.
    pub connection_timeout: u64,
    /// Whether to require quantum-resistant cipher suites.
    pub quantum_resistant: bool,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8443".parse().expect("static literal"),
            pq_tls_config: PQTlsConfig::default(),
            max_connections: 1000,
            connection_timeout: 30,
            quantum_resistant: true,
        }
    }
}

/// Top-level handle. Placeholder for the long-running listener loop.
#[derive(Debug, Default)]
pub struct NetworkSentinel {
    config: SentinelConfig,
}

impl NetworkSentinel {
    /// Create a new Network Sentinel from a configuration.
    pub fn new(config: SentinelConfig) -> Self {
        Self { config }
    }

    /// Return the active configuration.
    pub fn config(&self) -> &SentinelConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_are_quantum_resistant() {
        let config = SentinelConfig::default();
        assert!(config.quantum_resistant);
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.connection_timeout, 30);
    }

    #[test]
    fn sentinel_round_trips_config() {
        let sentinel = NetworkSentinel::new(SentinelConfig::default());
        assert!(sentinel.config().quantum_resistant);
    }
}
