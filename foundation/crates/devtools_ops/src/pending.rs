//! Pending Component Lifecycle Tier
//!
//! Owns canary tools, shadow reducers, and candidate capabilities under evaluation
//! prior to graduation into Aaroneous core crates.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Status of a candidate component undergoing evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationStatus {
    /// Initial drafting / incubation
    Incubating,
    /// Undergoing deterministic shadow execution
    ShadowEvaluating,
    /// Ready for graduation review into Aaroneous
    GraduationReady,
    /// Rejected due to invariant violations or regression
    Rejected,
}

/// A candidate component registered in the pending tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateComponent {
    pub name: String,
    pub proposed_cratify_profile: String,
    pub status: EvaluationStatus,
    pub shadow_runs_completed: u32,
    pub shadow_runs_passed: u32,
    pub ambient_authority_violations: u32,
}

impl CandidateComponent {
    pub fn new(name: impl Into<String>, proposed_cratify_profile: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            proposed_cratify_profile: proposed_cratify_profile.into(),
            status: EvaluationStatus::Incubating,
            shadow_runs_completed: 0,
            shadow_runs_passed: 0,
            ambient_authority_violations: 0,
        }
    }

    /// Records the result of a shadow execution run.
    pub fn record_shadow_run(&mut self, success: bool, violations: u32) {
        self.shadow_runs_completed += 1;
        if success && violations == 0 {
            self.shadow_runs_passed += 1;
        }
        self.ambient_authority_violations += violations;

        if self.ambient_authority_violations > 0 {
            self.status = EvaluationStatus::Rejected;
        } else if self.shadow_runs_passed >= 10 {
            self.status = EvaluationStatus::GraduationReady;
        } else {
            self.status = EvaluationStatus::ShadowEvaluating;
        }
    }

    /// Verifies graduation readiness according to Cratify and Aaroneous standards.
    pub fn verify_graduation_readiness(&self) -> Result<()> {
        if self.ambient_authority_violations > 0 {
            bail!(
                "Candidate '{}' has {} ambient authority violations; graduation blocked",
                self.name,
                self.ambient_authority_violations
            );
        }
        if self.shadow_runs_completed < 10 {
            bail!(
                "Candidate '{}' has only completed {}/10 required shadow runs",
                self.name,
                self.shadow_runs_completed
            );
        }
        if self.status != EvaluationStatus::GraduationReady {
            bail!(
                "Candidate '{}' status is {:?}, not GraduationReady",
                self.name,
                self.status
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_lifecycle_to_graduation() {
        let mut candidate = CandidateComponent::new("neural_tool_adapter", "tooling");
        assert_eq!(candidate.status, EvaluationStatus::Incubating);

        for _ in 0..9 {
            candidate.record_shadow_run(true, 0);
            assert_eq!(candidate.status, EvaluationStatus::ShadowEvaluating);
        }

        candidate.record_shadow_run(true, 0);
        assert_eq!(candidate.status, EvaluationStatus::GraduationReady);
        assert!(candidate.verify_graduation_readiness().is_ok());
    }

    #[test]
    fn test_candidate_fails_on_ambient_violation() {
        let mut candidate = CandidateComponent::new("clock_reader", "control");
        candidate.record_shadow_run(false, 1);
        assert_eq!(candidate.status, EvaluationStatus::Rejected);
        assert!(candidate.verify_graduation_readiness().is_err());
    }
}
