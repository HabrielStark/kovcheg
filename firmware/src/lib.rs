#![cfg_attr(not(test), no_std)]

#[cfg(test)]
extern crate std;

// Only include heavy hardware/crypto modules in non-test builds to allow host testing
#[cfg(not(test))]
pub mod boot;
#[cfg(not(test))]
pub mod crypto;
#[cfg(not(test))]
pub mod hardware;
#[cfg(not(test))]
pub mod security;

// Optional empty stubs for modules referenced elsewhere (non-test only)
#[cfg(not(test))]
pub mod optic_gate;
#[cfg(not(test))]
pub mod puf_api;
#[cfg(not(test))]
pub mod trng;
#[cfg(not(test))]
pub mod voter;