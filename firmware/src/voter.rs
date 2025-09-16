//! Majority voter used by the Tri-Compute core when running in TMR mode.

/// Possible outcomes produced by the voter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoteOutcome {
    /// Approve the operation.
    Allow,
    /// Deny the operation and raise an alert.
    Deny,
    /// Trigger a purge of sensitive state.
    Purge,
}

/// Errors that can occur while performing a vote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoterError {
    /// No consensus was reached given the configured threshold.
    NoConsensus,
}

/// Result of evaluating a set of votes.
#[derive(Clone, Copy, Debug)]
pub struct VoteAssessment {
    /// Histogram of votes in ``[Allow, Deny, Purge]`` order.
    pub histogram: [u8; 3],
    /// Decision chosen when consensus is reached.
    pub decision: Option<VoteOutcome>,
}

impl VoteAssessment {
    /// Return ``true`` when a decision has been selected.
    pub fn is_consensus(&self) -> bool {
        self.decision.is_some()
    }
}

/// Triple modular redundancy voter with configurable threshold.
pub struct TripleVoter {
    threshold: u8,
}

impl TripleVoter {
    /// Create a voter requiring a 2-out-of-3 majority.
    pub const fn new() -> Self {
        Self { threshold: 2 }
    }

    /// Create a voter with a custom threshold (clamped to ``1..=3``).
    pub const fn with_threshold(threshold: u8) -> Self {
        let clamped = if threshold < 1 {
            1
        } else if threshold > 3 {
            3
        } else {
            threshold
        };
        Self { threshold: clamped }
    }

    /// Assess the provided votes and return a :struct:`VoteAssessment`.
    pub fn assess(&self, votes: [VoteOutcome; 3]) -> VoteAssessment {
        let mut histogram = [0u8; 3];
        for vote in votes.iter() {
            match vote {
                VoteOutcome::Allow => histogram[0] = histogram[0].saturating_add(1),
                VoteOutcome::Deny => histogram[1] = histogram[1].saturating_add(1),
                VoteOutcome::Purge => histogram[2] = histogram[2].saturating_add(1),
            }
        }

        let (index, count) = histogram
            .iter()
            .enumerate()
            .max_by_key(|(_, &value)| value)
            .unwrap();

        let decision = if count >= &self.threshold {
            Some(match index {
                0 => VoteOutcome::Allow,
                1 => VoteOutcome::Deny,
                _ => VoteOutcome::Purge,
            })
        } else {
            None
        };

        VoteAssessment { histogram, decision }
    }

    /// Execute a vote and return the resulting decision.
    pub fn vote(&self, votes: [VoteOutcome; 3]) -> Result<VoteOutcome, VoterError> {
        let assessment = self.assess(votes);
        match assessment.decision {
            Some(decision) => Ok(decision),
            None => Err(VoterError::NoConsensus),
        }
    }
}
