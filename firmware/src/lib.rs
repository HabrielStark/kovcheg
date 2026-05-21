//! ARK Firmware Library
//! "Test all things; hold fast to what is good." - 1 Thessalonians 5:21
//!
//! Host-testable Rust library for the ARK defensive core firmware layer.
//! All operations are deterministic and constant-time wherever feasible.

// Bare-metal targets get `no_std`; host targets keep `std` so that
// `cargo check`/`cargo test` find a global allocator and panic handler
// without forcing the library to register its own.
#![cfg_attr(target_os = "none", no_std)]
#![deny(unsafe_op_in_unsafe_fn)]

// `alloc` is always available – on `no_std` builds it is pulled in
// explicitly, and on `std` builds `alloc` is simply re-exported by `std`.
extern crate alloc;

pub mod crypto;

pub use crypto::{CryptoContext, CryptoError, KeyType, SecureKey};

#[cfg(feature = "post-quantum")]
pub use crypto::{
    HybridEncryptedData, HybridSignature, PQAlgorithm, PQEncryptedData, PQPublicKeys,
};
