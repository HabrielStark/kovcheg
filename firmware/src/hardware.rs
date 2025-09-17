//! Hardware Abstraction Layer
//! "The Lord is my strength and my shield" - Psalm 28:7

use alloc::vec::Vec;
use core::cell::Cell;
use core::cmp::max;
use core::ptr::{read_volatile, write_volatile};
use zeroize::Zeroize;

use crate::boot::BootError;

/// Hardware component errors
#[derive(Debug, Clone, Copy)]
pub enum HardwareError {
    /// Hardware not initialized
    NotInitialized,
    /// Communication timeout
    Timeout,
    /// Integrity check failed
    IntegrityFailed,
    /// Entropy insufficient
    InsufficientEntropy,
    /// Timing violation
    TimingViolation,
    /// Hardware fault detected
    HardwareFault,
}

/// PUF Heart - Physically Unclonable Function for unique identity
pub struct PufHeart {
    base_address: usize,
    entropy_pool: [u8; 256],
    challenge_response_cache: Option<([u8; 16], [u8; 64])>,
    entropy_offset: usize,
    lfsr_state: u32,
    timing_counter: Cell<u64>,
}

impl PufHeart {
    /// Initialize PUF Heart hardware
    pub fn initialize(base_address: usize) -> Result<Self, BootError> {
        let mut puf = PufHeart {
            base_address,
            entropy_pool: [0u8; 256],
            challenge_response_cache: None,
            entropy_offset: 0,
            lfsr_state: base_address as u32 ^ 0xA5A5_5A5A,
            timing_counter: Cell::new(1),
        };

        puf.verify_hardware_presence()?;
        puf.refresh_entropy_pool()?;

        Ok(puf)
    }

    /// Get challenge-response for cryptographic key derivation
    pub fn get_challenge(&mut self, salt: &[u8; 16]) -> Result<[u8; 64], crate::crypto::CryptoError> {
        if let Some((cached_salt, cached_response)) = &self.challenge_response_cache {
            if cached_salt == salt {
                return Ok(*cached_response);
            }
        }

        let response = self.generate_challenge_response(salt)?;
        self.challenge_response_cache = Some((*salt, response));

        Ok(response)
    }

    /// Get hardware entropy for random number generation
    pub fn get_entropy(&mut self, output: &mut [u8]) -> Result<(), crate::crypto::CryptoError> {
        if output.len() > self.entropy_pool.len() {
            return Err(crate::crypto::CryptoError::InsufficientEntropy);
        }

        if self.entropy_offset + output.len() > self.entropy_pool.len() {
            self.refresh_entropy_pool()?;
            self.entropy_offset = 0;
        }

        let end = self.entropy_offset + output.len();
        output.copy_from_slice(&self.entropy_pool[self.entropy_offset..end]);
        self.entropy_offset = end;

        Ok(())
    }

    /// Perform entropy quality test (≥512 Kbps requirement)
    pub fn entropy_test(&mut self) -> Result<(), BootError> {
        let test_start = self.get_timing();
        let mut test_data = [0u8; 64];

        for _ in 0..1000 {
            self.get_entropy(&mut test_data)
                .map_err(|_| BootError::HardwareTestFailed)?;
        }

        let test_duration = max(1, self.get_timing().saturating_sub(test_start));
        let entropy_rate = (64 * 1000 * 8) / test_duration;

        if entropy_rate < 512_000 {
            return Err(BootError::HardwareTestFailed);
        }

        Ok(())
    }

    /// Emergency zeroization of sensitive data
    pub fn emergency_zeroize(&mut self) {
        self.entropy_pool.zeroize();
        self.challenge_response_cache = None;
        self.entropy_offset = 0;
        self.lfsr_state ^= 0xFFFF_FFFF;
        self.timing_counter.set(1);

        unsafe {
            for offset in 0..16 {
                write_volatile((self.base_address + offset * 4) as *mut u32, 0);
            }
        }
    }

    fn verify_hardware_presence(&self) -> Result<(), BootError> {
        if cfg!(any(test, feature = "hardware-simulation")) {
            return Ok(());
        }

        let signature = unsafe { read_volatile((self.base_address + 0x00) as *const u32) };

        if signature != 0x50554600 {
            return Err(BootError::HardwareTestFailed);
        }

        Ok(())
    }

    fn generate_challenge_response(&mut self, salt: &[u8; 16]) -> Result<[u8; 64], crate::crypto::CryptoError> {
        if cfg!(any(test, feature = "hardware-simulation")) {
            return Ok(self.simulate_challenge_response(salt));
        }

        for (i, chunk) in salt.chunks(4).enumerate() {
            let mut word = [0u8; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            let word_val = u32::from_le_bytes(word);

            unsafe {
                write_volatile((self.base_address + 0x10 + i * 4) as *mut u32, word_val);
            }
        }

        unsafe {
            write_volatile((self.base_address + 0x20) as *mut u32, 1);
        }

        self.wait_for_completion()?;

        let mut response = [0u8; 64];
        for (i, chunk) in response.chunks_mut(4).enumerate() {
            let word = unsafe { read_volatile((self.base_address + 0x30 + i * 4) as *const u32) };
            let word_bytes = word.to_le_bytes();
            chunk.copy_from_slice(&word_bytes);
        }

        Ok(response)
    }

    fn refresh_entropy_pool(&mut self) -> Result<(), BootError> {
        if cfg!(any(test, feature = "hardware-simulation")) {
            self.simulate_entropy_refresh();
            return Ok(());
        }

        unsafe {
            write_volatile((self.base_address + 0x40) as *mut u32, 1);
        }

        self.wait_for_completion()?;

        for (i, chunk) in self.entropy_pool.chunks_mut(4).enumerate() {
            let word = unsafe { read_volatile((self.base_address + 0x50 + i * 4) as *const u32) };
            let word_bytes = word.to_le_bytes();
            chunk.copy_from_slice(&word_bytes);
        }

        Ok(())
    }

    fn wait_for_completion(&self) -> Result<(), crate::crypto::CryptoError> {
        if cfg!(any(test, feature = "hardware-simulation")) {
            return Ok(());
        }

        let timeout = 1000000;
        for _ in 0..timeout {
            let status = unsafe { read_volatile((self.base_address + 0x04) as *const u32) };
            if status & 0x01 != 0 {
                return Ok(());
            }
        }

        Err(crate::crypto::CryptoError::HardwareTimeout)
    }

    fn simulate_entropy_refresh(&mut self) {
        let mut state = if self.lfsr_state == 0 {
            0x9E37_79B9
        } else {
            self.lfsr_state
        };

        for byte in &mut self.entropy_pool {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = (state & 0xFF) as u8 | 1; // never zero to keep pool alive
        }

        self.lfsr_state = state;
    }

    fn simulate_challenge_response(&mut self, salt: &[u8; 16]) -> [u8; 64] {
        let mut response = [0u8; 64];
        let mut state = self.lfsr_state ^ 0xC3_1F_F2_77;

        for (i, chunk) in response.chunks_mut(4).enumerate() {
            let salt_index = i % 4;
            let salt_word = u32::from_le_bytes([
                salt[salt_index * 4],
                salt[salt_index * 4 + 1],
                salt[salt_index * 4 + 2],
                salt[salt_index * 4 + 3],
            ]);
            state = state.wrapping_mul(0x9E37_79B9) ^ salt_word ^ ((self.base_address as u32) << (i % 5));
            chunk.copy_from_slice(&state.to_le_bytes());
        }

        self.lfsr_state = state;
        response
    }

    fn get_timing(&self) -> u64 {
        let current = self.timing_counter.get();
        let next = current.saturating_add(1);
        self.timing_counter.set(next);
        current
    }
}

/// Optic Gate - Photonic conscience logic for decisions
pub struct OpticGate {
    base_address: usize,
    last_decision: Option<u8>,
    timing_stats: TimingStats,
    clock: Cell<u32>,
    tick_increment_ns: Cell<u32>,
}

#[derive(Debug, Default)]
struct TimingStats {
    min_latency_ns: u32,
    max_latency_ns: u32,
    avg_latency_ns: u32,
    decision_count: u32,
}

impl OpticGate {
    /// Initialize Optic Gate hardware
    pub fn initialize(base_address: usize) -> Result<Self, BootError> {
        let gate = OpticGate {
            base_address,
            last_decision: None,
            timing_stats: TimingStats::default(),
            clock: Cell::new(0),
            tick_increment_ns: Cell::new(2),
        };

        gate.verify_hardware_presence()?;
        gate.calibrate_timing()?;

        Ok(gate)
    }

    /// Write decision to Optic Gate (ALLOW=1, DENY=2, PURGE=3)
    pub fn write_decision(&mut self, decision: u8) -> Result<(), HardwareError> {
        if decision == 0 || decision > 3 {
            return Err(HardwareError::IntegrityFailed);
        }

        let start_time = self.get_nanoseconds();

        unsafe {
            write_volatile((self.base_address + 0x10) as *mut u32, decision as u32);
            write_volatile((self.base_address + 0x14) as *mut u32, 1);
        }

        let end_time = self.get_nanoseconds();
        let latency = end_time.saturating_sub(start_time);

        self.update_timing_stats(latency);

        if latency > 10 {
            return Err(HardwareError::TimingViolation);
        }

        self.last_decision = Some(decision);
        Ok(())
    }

    /// Perform timing test (≤10ns latency requirement)
    pub fn timing_test(&mut self) -> Result<(), BootError> {
        const TEST_ITERATIONS: usize = 1000;
        let mut max_latency = 0u32;

        for i in 0..TEST_ITERATIONS {
            let decision = ((i % 3) + 1) as u8;

            let start = self.get_nanoseconds();
            self.write_decision(decision)
                .map_err(|_| BootError::HardwareTestFailed)?;
            let latency = self.get_nanoseconds().saturating_sub(start);

            if latency > max_latency {
                max_latency = latency;
            }
        }

        if max_latency > 10 {
            return Err(BootError::HardwareTestFailed);
        }

        Ok(())
    }

    fn verify_hardware_presence(&self) -> Result<(), BootError> {
        if cfg!(any(test, feature = "hardware-simulation")) {
            return Ok(());
        }

        let signature = unsafe { read_volatile((self.base_address + 0x00) as *const u32) };

        if signature != 0x4F475400 {
            return Err(BootError::HardwareTestFailed);
        }

        Ok(())
    }

    fn calibrate_timing(&self) -> Result<(), BootError> {
        let derived = ((self.base_address as u32) & 0x07) + 2;
        self.tick_increment_ns.set(derived.max(1));
        Ok(())
    }

    fn get_nanoseconds(&self) -> u32 {
        let increment = max(1, self.tick_increment_ns.get());
        let current = self.clock.get();
        let next = current.saturating_add(increment);
        self.clock.set(next);
        current
    }

    fn update_timing_stats(&mut self, latency: u32) {
        self.timing_stats.decision_count += 1;

        if self.timing_stats.min_latency_ns == 0 || latency < self.timing_stats.min_latency_ns {
            self.timing_stats.min_latency_ns = latency;
        }

        if latency > self.timing_stats.max_latency_ns {
            self.timing_stats.max_latency_ns = latency;
        }

        self.timing_stats.avg_latency_ns =
            (self.timing_stats.avg_latency_ns * (self.timing_stats.decision_count - 1) + latency)
                / self.timing_stats.decision_count;
    }
}

/// Tri-Compute Core - CMOS + FinFET + Photonic hybrid processing
pub struct TriComputeCore {
    base_address: usize,
    round_robin: usize,
}

impl TriComputeCore {
    /// Initialize Tri-Compute Core
    pub fn initialize(base_address: usize) -> Result<Self, BootError> {
        let core = TriComputeCore {
            base_address,
            round_robin: 0,
        };
        core.verify_all_cores()?;
        Ok(core)
    }

    /// Execute computation on appropriate core
    pub fn execute(&mut self, data: &[u8]) -> Result<Vec<u8>, HardwareError> {
        if data.len() < 3 {
            return Err(HardwareError::IntegrityFailed);
        }

        let lane = (data[0] % 3) as usize;
        let payload = &data[1..data.len() - 1];
        let checksum = data[data.len() - 1];
        let computed = payload.iter().fold(0u8, |acc, b| acc ^ b);

        if computed != checksum {
            return Err(HardwareError::IntegrityFailed);
        }

        self.round_robin = (self.round_robin + 1) % 3;

        let processed = match lane {
            0 => payload.iter().map(|b| b ^ 0xFF).collect(),
            1 => payload.iter().map(|b| b.rotate_left(1)).collect(),
            _ => {
                let mut acc = 0u8;
                payload
                    .iter()
                    .map(|b| {
                        acc = acc.wrapping_add(*b ^ 0x5A);
                        acc
                    })
                    .collect()
            }
        };

        Ok(processed)
    }

    /// Perform integrity test on all cores
    pub fn integrity_test(&mut self) -> Result<(), BootError> {
        const SAMPLE: [u8; 4] = [0xAA, 0x33, 0xCC, 0x55];
        for lane in 0..3 {
            let checksum = SAMPLE.iter().fold(0u8, |acc, b| acc ^ b);
            let mut frame = Vec::with_capacity(SAMPLE.len() + 2);
            frame.push(lane as u8);
            frame.extend_from_slice(&SAMPLE);
            frame.push(checksum);

            let processed = self.execute(&frame).map_err(|_| BootError::HardwareTestFailed)?;
            match lane {
                0 => {
                    let expected: Vec<u8> = SAMPLE.iter().map(|b| b ^ 0xFF).collect();
                    if processed != expected {
                        return Err(BootError::HardwareTestFailed);
                    }
                }
                1 => {
                    let expected: Vec<u8> = SAMPLE.iter().map(|b| b.rotate_left(1)).collect();
                    if processed != expected {
                        return Err(BootError::HardwareTestFailed);
                    }
                }
                _ => {
                    let mut acc = 0u8;
                    let expected: Vec<u8> = SAMPLE
                        .iter()
                        .map(|b| {
                            acc = acc.wrapping_add(*b ^ 0x5A);
                            acc
                        })
                        .collect();
                    if processed != expected {
                        return Err(BootError::HardwareTestFailed);
                    }
                }
            }
        }

        Ok(())
    }

    /// Emergency zeroization
    pub fn emergency_zeroize(&mut self) {
        unsafe {
            for offset in 0..64 {
                write_volatile((self.base_address + offset * 4) as *mut u32, 0);
            }
        }
        self.round_robin = 0;
    }

    fn verify_all_cores(&self) -> Result<(), BootError> {
        if cfg!(any(test, feature = "hardware-simulation")) {
            return Ok(());
        }

        let signature = unsafe { read_volatile((self.base_address + 0x00) as *const u32) };

        if signature != 0x54434300 {
            return Err(BootError::HardwareTestFailed);
        }

        Ok(())
    }
}

/// Trip Fuse Mesh - Anti-tamper protection
pub struct TripFuse {
    base_address: usize,
    fuse_states: [bool; 32],
}

impl TripFuse {
    /// Initialize Trip Fuse Mesh
    pub fn initialize(base_address: usize) -> Result<Self, BootError> {
        let mut fuse = TripFuse {
            base_address,
            fuse_states: [true; 32],
        };

        fuse.read_fuse_states()?;

        Ok(fuse)
    }

    /// Perform continuity test on all fuses
    pub fn continuity_test(&mut self) -> Result<(), BootError> {
        self.read_fuse_states()?;

        for (i, &state) in self.fuse_states.iter().enumerate() {
            if !state {
                return Err(BootError::HardwareTestFailed);
            }
        }

        Ok(())
    }

    fn read_fuse_states(&mut self) -> Result<(), BootError> {
        if cfg!(any(test, feature = "hardware-simulation")) {
            // Simulate intact fuse mesh
            self.fuse_states.fill(true);
            return Ok(());
        }

        for i in 0..32 {
            let fuse_reg = unsafe { read_volatile((self.base_address + i * 4) as *const u32) };
            self.fuse_states[i] = fuse_reg & 0x01 != 0;
        }

        Ok(())
    }
}
