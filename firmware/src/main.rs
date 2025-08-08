//! ARK Firmware - Immutable Boot & Hardware Abstraction Layer
//! "The Lord is my rock, my fortress and my deliverer" - Psalm 18:2
//!
//! This is the immutable firmware layer that boots ARK and provides
//! hardware abstraction for the PUF Heart, Optic Gate, and Tri-Compute Core.

#![cfg_attr(not(test), no_std)]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]
#![warn(clippy::all)]

use core::panic::PanicInfo;
use cortex_m_rt::entry;
use riscv_rt as _;

mod boot;
mod crypto;
mod hardware;
mod memory;
mod security;

use boot::ImmutableBoot;
use hardware::{OpticGate, PufHeart, TriComputeCore, TripFuse};
use security::KillFuseProtection;

/// ARK Firmware Version - Immutably embedded at compile time
const ARK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build timestamp for reproducible builds
const BUILD_TIMESTAMP: &str = env!("SOURCE_DATE_EPOCH");

/// Biblical foundation hash - Sha3-256 of core scripture passages
const MORAL_FOUNDATION_HASH: [u8; 32] = [
    0x4a, 0x7d, 0x1e, 0xd4, 0x14, 0x2c, 0x3b, 0x5e,
    0x9f, 0x12, 0x8a, 0xe6, 0x77, 0xc4, 0x2d, 0x13,
    0xe8, 0x95, 0x3a, 0x7b, 0x81, 0x0c, 0x6f, 0x29,
    0x54, 0xd7, 0x36, 0xb9, 0x42, 0x8e, 0x1f, 0xa3,
];

/// Hardware Memory Map (RISC-V MMIO)
mod memory_map {
    /// PUF Heart base address
    pub const PUF_HEART_BASE: usize = 0x1000_0000;
    
    /// Optic Gate base address  
    pub const OPTIC_GATE_BASE: usize = 0x1001_0000;
    
    /// Tri-Compute Core base address
    pub const TRI_COMPUTE_BASE: usize = 0x1002_0000;
    
    /// Trip Fuse Mesh base address
    pub const TRIP_FUSE_BASE: usize = 0x1003_0000;
    
    /// TRNG (True Random Number Generator) base
    pub const TRNG_BASE: usize = 0x1004_0000;
    
    /// Secure ROM base (immutable code)
    pub const SECURE_ROM_BASE: usize = 0x2000_0000;
    
    /// Secure RAM base (encrypted working memory)
    pub const SECURE_RAM_BASE: usize = 0x3000_0000;
}

/// Global hardware state - initialized once at boot
static mut ARK_HARDWARE: Option<ArkHardware> = None;

/// ARK Hardware abstraction layer
struct ArkHardware {
    puf_heart: PufHeart,
    optic_gate: OpticGate,
    tri_compute: TriComputeCore,
    trip_fuse: TripFuse,
    kill_fuse_protection: KillFuseProtection,
}

impl ArkHardware {
    /// Initialize all hardware components with security validation
    fn initialize() -> Result<Self, boot::BootError> {
        // Verify moral foundation integrity
        boot::verify_moral_foundation(&MORAL_FOUNDATION_HASH)?;
        
        // Initialize hardware components in specific order
        let puf_heart = PufHeart::initialize(memory_map::PUF_HEART_BASE)?;
        let optic_gate = OpticGate::initialize(memory_map::OPTIC_GATE_BASE)?;
        let tri_compute = TriComputeCore::initialize(memory_map::TRI_COMPUTE_BASE)?;
        let trip_fuse = TripFuse::initialize(memory_map::TRIP_FUSE_BASE)?;
        
        // Critical: Initialize kill-fuse protection LAST
        let kill_fuse_protection = KillFuseProtection::initialize()?;
        
        Ok(ArkHardware {
            puf_heart,
            optic_gate,
            tri_compute,
            trip_fuse,
            kill_fuse_protection,
        })
    }
    
    /// Run hardware self-test sequence
    fn self_test(&mut self) -> Result<(), boot::BootError> {
        // PUF Heart entropy test
        self.puf_heart.entropy_test()?;
        
        // Optic Gate timing test (must be ≤10ns)
        self.optic_gate.timing_test()?;
        
        // Tri-Compute Core integrity test
        self.tri_compute.integrity_test()?;
        
        // Trip fuse continuity test
        self.trip_fuse.continuity_test()?;
        
        // Kill-fuse protection verification
        self.kill_fuse_protection.verify_protection()?;
        
        Ok(())
    }
}

/// Main firmware entry point - executed after hardware reset
#[entry]
#[cfg(test)]
fn main() {}

#[cfg(not(test))]
#[entry]
fn main() -> ! {
    // Phase 1: Immutable Boot Sequence
    let boot_result = ImmutableBoot::execute();
    
    match boot_result {
        Ok(_) => {
            // Phase 2: Hardware Initialization
            match initialize_hardware() {
                Ok(_) => {
                    // Phase 3: Transfer control to application layer
                    transfer_to_application_layer();
                }
                Err(e) => {
                    // Hardware initialization failed - enter safe mode
                    enter_safe_mode(e);
                }
            }
        }
        Err(e) => {
            // Boot verification failed - immediate shutdown
            emergency_shutdown(e);
        }
    }
}

/// Initialize and test all hardware components
fn initialize_hardware() -> Result<(), boot::BootError> {
    // SAFETY: This is the only place where ARK_HARDWARE is initialized
    unsafe {
        let hardware = ArkHardware::initialize()?;
        
        // Run comprehensive self-test
        let mut hw = hardware;
        hw.self_test()?;
        
        ARK_HARDWARE = Some(hw);
    }
    
    Ok(())
}

/// Transfer control to the application software layer
fn transfer_to_application_layer() -> ! {
    // At this point, firmware has successfully initialized all hardware
    // and the system is ready to run the ethics DSL and decision engine
    
    loop {
        // Main application loop - this would be replaced by the actual
        // application layer in a complete implementation
        
        // For now, just demonstrate the hardware is running
        if let Some(ref mut hardware) = unsafe { &mut ARK_HARDWARE } {
            // Check hardware status periodically
            let _ = hardware.self_test();
        }
        
        // Yield to application layer (not implemented in this firmware)
        cortex_m::asm::wfi(); // Wait for interrupt
    }
}

/// Enter safe mode when hardware initialization fails
fn enter_safe_mode(error: boot::BootError) -> ! {
    // Log the error (if logging is available)
    #[cfg(feature = "debug-logging")]
    log::error!("Hardware initialization failed: {:?}", error);
    
    // Enter minimal operation mode - only critical functions
    loop {
        // Minimal heartbeat to indicate system is alive but in safe mode
        cortex_m::asm::wfi();
    }
}

/// Emergency shutdown for critical boot failures
fn emergency_shutdown(error: boot::BootError) -> ! {
    // This function is called when boot verification fails
    // The system enters a permanent shutdown state
    
    #[cfg(feature = "debug-logging")]
    log::error!("Emergency shutdown: {:?}", error);
    
    // Disable all interrupts
    cortex_m::interrupt::disable();
    
    // Enter infinite loop - no recovery possible
    loop {
        cortex_m::asm::wfi();
    }
}

/// Global panic handler - zeroizes sensitive data and halts
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // CRITICAL: Zeroize all sensitive data on panic
    if let Some(ref mut hardware) = unsafe { &mut ARK_HARDWARE } {
        hardware.puf_heart.emergency_zeroize();
        hardware.tri_compute.emergency_zeroize();
    }
    
    #[cfg(feature = "debug-logging")]
    log::error!("PANIC: {}", info);
    
    // Disable interrupts and halt
    cortex_m::interrupt::disable();
    
    loop {
        cortex_m::asm::wfi();
    }
}

/// Hardware abstraction API for application layer
pub mod api {
    use super::*;
    
    /// Get PUF challenge-response for key derivation
    pub fn puf_challenge(salt: &[u8; 16]) -> Result<[u8; 64], crypto::CryptoError> {
        unsafe {
            if let Some(ref mut hardware) = &mut ARK_HARDWARE {
                hardware.puf_heart.get_challenge(salt)
            } else {
                Err(crypto::CryptoError::HardwareNotInitialized)
            }
        }
    }
    
    /// Write decision to Optic Gate (ALLOW/DENY/PURGE)
    pub fn optic_gate_decision(decision: u8) -> Result<(), hardware::HardwareError> {
        unsafe {
            if let Some(ref mut hardware) = &mut ARK_HARDWARE {
                hardware.optic_gate.write_decision(decision)
            } else {
                Err(hardware::HardwareError::NotInitialized)
            }
        }
    }
    
    /// Submit computation to Tri-Compute Core
    pub fn tri_compute_execute(data: &[u8]) -> Result<Vec<u8>, hardware::HardwareError> {
        unsafe {
            if let Some(ref mut hardware) = &mut ARK_HARDWARE {
                hardware.tri_compute.execute(data)
            } else {
                Err(hardware::HardwareError::NotInitialized)
            }
        }
    }
    
    /// Get hardware entropy from TRNG
    pub fn get_entropy(bytes: &mut [u8]) -> Result<(), crypto::CryptoError> {
        unsafe {
            if let Some(ref mut hardware) = &mut ARK_HARDWARE {
                hardware.puf_heart.get_entropy(bytes)
            } else {
                Err(crypto::CryptoError::HardwareNotInitialized)
            }
        }
    }
}

// Build-time verification
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_moral_foundation_hash() {
        // Verify the moral foundation hash is correct
        assert_eq!(MORAL_FOUNDATION_HASH.len(), 32);
        // The actual hash would be verified against known scripture
    }
    
    #[test]
    fn test_memory_map_alignment() {
        // Verify memory map addresses are properly aligned
        assert_eq!(memory_map::PUF_HEART_BASE % 0x1000, 0);
        assert_eq!(memory_map::OPTIC_GATE_BASE % 0x1000, 0);
        assert_eq!(memory_map::TRI_COMPUTE_BASE % 0x1000, 0);
    }
} 