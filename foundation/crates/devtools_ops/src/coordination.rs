//! Coordination Component Lifecycle Tier
//!
//! Owns multi-agent consensus, seat attestation, and coordination queue arbitration.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

/// Represents an operational seat identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatIdentity {
    pub seat_id: String,
    pub role: String,
    pub public_key_fingerprint: String,
}

impl SeatIdentity {
    pub fn new(
        seat_id: impl Into<String>,
        role: impl Into<String>,
        fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            seat_id: seat_id.into(),
            role: role.into(),
            public_key_fingerprint: fingerprint.into(),
        }
    }
}

/// Represents an exact-path claim within the coordination queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneClaim {
    pub lane_id: String,
    pub owner_seat: String,
    pub branch: String,
    pub start_timestamp_utc: String,
    pub owned_paths: Vec<String>,
}

impl LaneClaim {
    pub fn new(
        lane_id: impl Into<String>,
        owner_seat: impl Into<String>,
        branch: impl Into<String>,
        start_timestamp_utc: impl Into<String>,
        owned_paths: Vec<String>,
    ) -> Result<Self> {
        let lane_id = lane_id.into();
        let owner_seat = owner_seat.into();
        let branch = branch.into();
        let start_timestamp_utc = start_timestamp_utc.into();

        if owned_paths.is_empty() {
            bail!("Lane claim '{lane_id}' must specify exact owned paths");
        }

        Ok(Self {
            lane_id,
            owner_seat,
            branch,
            start_timestamp_utc,
            owned_paths,
        })
    }

    /// Checks if a proposed file path conflicts with this claim.
    pub fn conflicts_with_path(&self, path: &str) -> bool {
        self.owned_paths.iter().any(|owned| owned == path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lane_claim_conflict_detection() {
        let claim = LaneClaim::new(
            "C89",
            "ubuntu-worker-01",
            "seat/ubuntu-worker-01-c89",
            "2026-10-17T19:47:00Z",
            vec![
                "xtask/src/check_deps.rs".into(),
                "docs/CRATIFY_SPEC.md".into(),
            ],
        )
        .expect("claim must be valid");

        assert!(claim.conflicts_with_path("xtask/src/check_deps.rs"));
        assert!(!claim.conflicts_with_path("crates/capabilities/src/lib.rs"));
    }

    #[test]
    fn test_claim_without_paths_fails() {
        let res = LaneClaim::new("C99", "codex", "codex/c99", "2026-10-17T19:00:00Z", vec![]);
        assert!(res.is_err());
    }
}
