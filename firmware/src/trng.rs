//! Deterministic software TRNG used for host-side testing.

use core::num::Wrapping;

/// Software implementation of a statistically robust PRNG used to emulate the
/// hardware TRNG during host-based tests.  The generator is based on the
/// SplitMix64 design which provides excellent equidistribution for small
/// buffers.
pub struct SoftwareTrng {
    state: Wrapping<u64>,
}

impl SoftwareTrng {
    /// Create a new TRNG seeded with ``seed``.  The seed is mixed to avoid the
    /// zero state.
    pub const fn seeded(seed: u64) -> Self {
        Self {
            state: Wrapping(seed | 1),
        }
    }

    /// Reseed the generator with ``seed``.
    pub fn reseed(&mut self, seed: u64) {
        self.state = Wrapping(seed | 1);
    }

    /// Fill ``output`` with pseudo-random bytes.
    pub fn fill_bytes(&mut self, output: &mut [u8]) {
        let mut index = 0;
        while index < output.len() {
            let value = self.next_u64();
            let bytes = value.to_le_bytes();
            for byte in bytes.iter() {
                if index >= output.len() {
                    break;
                }
                output[index] = *byte;
                index += 1;
            }
        }
    }

    /// Perform a basic health check using a monobit test.
    pub fn health_check(&mut self) -> TrngHealth {
        let mut buffer = [0u8; 64];
        self.fill_bytes(&mut buffer);

        let mut ones = 0u32;
        for byte in buffer.iter() {
            ones += byte.count_ones();
        }
        let total_bits = (buffer.len() as u32) * 8;
        let zeros = total_bits - ones;
        let imbalance = if ones > zeros {
            ones - zeros
        } else {
            zeros - ones
        };
        let imbalance_ratio = (imbalance as f32) / (total_bits as f32);

        TrngHealth {
            ones,
            zeros,
            imbalance: imbalance_ratio,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state += Wrapping(0x9E3779B97F4A7C15);
        let mut z = self.state.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
}

/// Result of :meth:`SoftwareTrng::health_check`.
#[derive(Clone, Copy, Debug)]
pub struct TrngHealth {
    /// Number of ones observed in the sample.
    pub ones: u32,
    /// Number of zeros observed in the sample.
    pub zeros: u32,
    /// Normalised imbalance between ones and zeros.
    pub imbalance: f32,
}

impl TrngHealth {
    /// Check whether the imbalance is within the provided tolerance.
    pub fn is_within_bounds(&self, tolerance: f32) -> bool {
        self.imbalance <= tolerance
    }
}
