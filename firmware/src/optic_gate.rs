//! Diagnostics helpers for the Optic Gate hardware component.

use crate::boot::BootError;
use crate::hardware::{HardwareError, OpticGate};

const SAMPLE_LIMIT: usize = 32;

/// Result of replaying a decision pattern through the Optic Gate.
#[derive(Clone, Copy, Debug)]
pub struct OpticGateReport {
    /// Number of decisions successfully applied.
    pub applied: u8,
    /// Number of rejected decisions.
    pub failures: u8,
}

impl OpticGateReport {
    /// Percentage of successful decisions as a value between ``0.0`` and ``1.0``.
    pub fn success_rate(&self) -> f32 {
        if self.applied == 0 {
            return 0.0;
        }
        let successes = (self.applied as i32) - (self.failures as i32);
        (successes.max(0) as f32) / (self.applied as f32)
    }
}

/// Captured result for each decision in a diagnostic run.
#[derive(Clone, Copy, Debug)]
pub struct OpticGateSample {
    /// Decision written to the hardware (1=ALLOW, 2=DENY, 3=PURGE).
    pub decision: u8,
    /// Optional hardware error captured for this decision.
    pub error: Option<HardwareError>,
}

impl Default for OpticGateSample {
    fn default() -> Self {
        Self {
            decision: 0,
            error: None,
        }
    }
}

/// Utility struct providing deterministic diagnostics over the Optic Gate API.
pub struct OpticGateDiagnostics<'a> {
    gate: &'a mut OpticGate,
    samples: [OpticGateSample; SAMPLE_LIMIT],
    count: usize,
}

impl<'a> OpticGateDiagnostics<'a> {
    /// Create a new diagnostics helper around ``gate``.
    pub fn new(gate: &'a mut OpticGate) -> Self {
        Self {
            gate,
            samples: [OpticGateSample::default(); SAMPLE_LIMIT],
            count: 0,
        }
    }

    /// Execute the provided decision ``pattern`` and capture success metrics.
    pub fn exercise(&mut self, pattern: &[u8]) -> OpticGateReport {
        self.count = 0;
        let mut failures = 0u8;

        for &decision in pattern.iter().take(SAMPLE_LIMIT) {
            let index = self.count;
            self.samples[index] = OpticGateSample {
                decision,
                error: self.gate.write_decision(decision).err(),
            };
            if self.samples[index].error.is_some() {
                failures = failures.saturating_add(1);
            }
            self.count += 1;
        }

        OpticGateReport {
            applied: self.count as u8,
            failures,
        }
    }

    /// Run the embedded timing self-test routine.
    pub fn verify_latency(&mut self) -> Result<(), BootError> {
        self.gate.timing_test()
    }

    /// Return the collected samples from the last :meth:`exercise` run.
    pub fn samples(&self) -> &[OpticGateSample] {
        &self.samples[..self.count]
    }
}
