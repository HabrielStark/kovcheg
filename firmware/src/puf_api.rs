//! High-level helpers around the PUF Heart hardware interface.

use crate::boot::BootError;
use crate::crypto::CryptoError;
use crate::hardware::PufHeart;

/// Summary of entropy measurements collected from the PUF.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PufEntropyReport {
    /// Estimated entropy rate expressed in bits per second (simulated).
    pub estimated_rate_bps: u32,
    /// Lowest byte observed during sampling.
    pub min_byte: u8,
    /// Highest byte observed during sampling.
    pub max_byte: u8,
    /// Arithmetic mean of sampled bytes.
    pub mean_byte: u8,
    /// Count of bit transitions during sampling (simple runs test proxy).
    pub transition_count: u16,
}

impl PufEntropyReport {
    /// Verify that the entropy rate satisfies the minimum requirement.
    pub fn meets_requirement(&self, minimum_bps: u32) -> bool {
        self.estimated_rate_bps >= minimum_bps
    }
}

/// Convenience wrapper providing deterministic tests for the PUF Heart.
pub struct PufAnalyzer<'a> {
    puf: &'a mut PufHeart,
    buffer: [u8; 64],
}

impl<'a> PufAnalyzer<'a> {
    /// Create a new analyzer operating on ``puf``.
    pub fn new(puf: &'a mut PufHeart) -> Self {
        Self {
            puf,
            buffer: [0u8; 64],
        }
    }

    /// Derive 256-bit key material from the hardware challenge response.
    pub fn derive_key_material(&mut self, salt: &[u8; 16]) -> Result<[u8; 32], CryptoError> {
        let response = self.puf.get_challenge(salt)?;
        let mut derived = [0u8; 32];

        for (i, chunk) in response.chunks(2).enumerate() {
            if i >= derived.len() {
                break;
            }
            let left = chunk[0];
            let right = if chunk.len() > 1 { chunk[1] } else { 0 };
            derived[i] = left ^ right.rotate_left(1);
        }

        Ok(derived)
    }

    /// Collect a lightweight entropy report for health monitoring.
    pub fn collect_entropy_report(&mut self) -> Result<PufEntropyReport, CryptoError> {
        self.puf.get_entropy(&mut self.buffer)?;

        let mut min_byte = u8::MAX;
        let mut max_byte = u8::MIN;
        let mut sum = 0u32;
        let mut transitions = 0u16;
        let mut previous_bit = self.buffer[0] & 1;

        for &byte in self.buffer.iter() {
            if byte < min_byte {
                min_byte = byte;
            }
            if byte > max_byte {
                max_byte = byte;
            }
            sum += byte as u32;

            let current_bit = byte & 1;
            if current_bit != previous_bit {
                transitions = transitions.saturating_add(1);
                previous_bit = current_bit;
            }
        }

        let mean_byte = (sum / self.buffer.len() as u32) as u8;
        let estimated_rate = (self.buffer.len() as u32) * 8;

        Ok(PufEntropyReport {
            estimated_rate_bps: estimated_rate,
            min_byte,
            max_byte,
            mean_byte,
            transition_count: transitions,
        })
    }

    /// Execute the built-in entropy test sequence.
    pub fn run_startup_self_test(&mut self) -> Result<(), BootError> {
        self.puf.entropy_test()
    }
}
