//! ARK Firmware Library
//! Exports for testing and integration

#![no_std]

// Only expose these modules when testing
#[cfg(test)]
extern crate std;

pub mod crypto;

// Re-export commonly used types
pub use crypto::{CryptoContext, CryptoError, SecureKey};

#[cfg(feature = "post-quantum")]
pub use crypto::{PQAlgorithm, PQEncryptedData, HybridEncryptedData, HybridSignature, PQPublicKeys};
