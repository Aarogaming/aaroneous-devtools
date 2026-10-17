//! Versioned Contribution Lifecycle and Envelope Protocol
//!
//! Provides typed identities (`ContributionId`) and cryptographic envelopes (`ContributionEnvelope`)
//! carrying a single versioned identity through discovery, intake, ownership, worker output,
//! verification, independent review, and acceptance — with pinned origin revisions, virtual patch
//! digests, reviewer independence interlocks, and canonical Git trailers.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const SCHEMA_VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "foundation.contribution-envelope";

/// Strongly-typed, parsed Contribution Identifier: `<lane>/<producer>-<slug>-v<version>`.
///
/// Example: `C117/gemini-contribution-lifecycle-v1`
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContributionId {
    pub lane: String,
    pub producer: String,
    pub slug: String,
    pub version: u32,
}

impl ContributionId {
    /// Create a new ContributionId, validating fields.
    pub fn new(
        lane: impl Into<String>,
        producer: impl Into<String>,
        slug: impl Into<String>,
        version: u32,
    ) -> Result<Self> {
        let lane = lane.into().trim().to_string();
        let producer = producer.into().trim().to_lowercase();
        let slug = slug.into().trim().to_lowercase();

        if lane.is_empty()
            || !lane
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            bail!("invalid lane '{lane}': must be non-empty alphanumeric with optional '_' or '-'");
        }
        if producer.is_empty()
            || !producer
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            bail!(
                "invalid producer '{producer}': must be non-empty alphanumeric with optional '_' or '-'"
            );
        }
        if slug.is_empty()
            || !slug
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            bail!(
                "invalid slug '{slug}': must be non-empty alphanumeric with optional '_', '-', or '.'"
            );
        }
        if version == 0 {
            bail!("version must be >= 1");
        }

        Ok(Self {
            lane,
            producer,
            slug,
            version,
        })
    }

    /// Parse a canonical string representation: `<lane>/<producer>-<slug>-v<version>`.
    pub fn parse(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        let (lane_part, tail_part) = match trimmed.split_once('/') {
            Some((l, t)) => (l.trim(), t.trim()),
            None => bail!(
                "invalid Contribution-ID '{trimmed}': missing '/' separator (expected <lane>/<producer>-<slug>-v<version>)"
            ),
        };

        if lane_part.is_empty() {
            bail!("empty lane in Contribution-ID '{trimmed}'");
        }

        // Look for trailing "-v<digits>"
        let v_pos = match tail_part.rfind("-v") {
            Some(idx) => idx,
            None => bail!("invalid Contribution-ID '{trimmed}': missing '-v<version>' suffix"),
        };

        let ver_str = &tail_part[v_pos + 2..];
        let version: u32 = ver_str.parse().with_context(|| {
            format!("invalid version number '{ver_str}' in Contribution-ID '{trimmed}'")
        })?;
        if version == 0 {
            bail!("version must be >= 1 in Contribution-ID '{trimmed}'");
        }

        let body = &tail_part[..v_pos];
        if body.is_empty() {
            bail!("empty producer and slug in Contribution-ID '{trimmed}'");
        }

        // Handle known multi-segment producers (e.g. ubuntu-worker-01)
        let (producer, slug) = if let Some(stripped) = body.strip_prefix("ubuntu-worker-01-") {
            ("ubuntu-worker-01", stripped)
        } else {
            match body.split_once('-') {
                Some((p, s)) => (p, s),
                None => bail!(
                    "invalid producer/slug '{body}' in Contribution-ID '{trimmed}': expected '<producer>-<slug>'"
                ),
            }
        };

        if producer.is_empty() {
            bail!("empty producer in Contribution-ID '{trimmed}'");
        }
        if slug.is_empty() {
            bail!("empty slug in Contribution-ID '{trimmed}'");
        }

        Self::new(lane_part, producer, slug, version)
    }

    /// Format canonical git trailers for this contribution identity.
    pub fn as_trailers(&self, co_author: Option<(&str, &str)>) -> Vec<String> {
        let (name, email) = match co_author {
            Some((n, e)) => (n.to_string(), e.to_string()),
            None => default_seat_author(&self.producer),
        };

        vec![
            format!("Contribution-ID: {self}"),
            format!("Co-Authored-By: {name} <{email}>"),
        ]
    }
}

impl fmt::Display for ContributionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}-{}-v{}",
            self.lane, self.producer, self.slug, self.version
        )
    }
}

impl Serialize for ContributionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ContributionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ContributionId::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// Known default co-author identities by seat name.
fn default_seat_author(producer: &str) -> (String, String) {
    match producer.to_lowercase().as_str() {
        "gemini" | "antigravity" => ("Gemini".to_string(), "gemini@aarogaming.local".to_string()),
        "claude" => ("Claude".to_string(), "claude@aarogaming.local".to_string()),
        "codex" => ("Codex".to_string(), "codex@aarogaming.local".to_string()),
        "mimo" | "qwen" => ("Mimo".to_string(), "mimo@aarogaming.local".to_string()),
        "ubuntu-worker-01" => (
            "ubuntu-worker-01".to_string(),
            "ubuntu-worker-01@aarogaming.local".to_string(),
        ),
        other => (other.to_string(), format!("{other}@aarogaming.local")),
    }
}

/// Command verification record executed against this contribution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VerificationEvidence {
    pub command: String,
    pub exit_code: i32,
}

/// Independent review attestation recorded against a contribution envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewAttestation {
    pub actor: String,
    pub independent_of: String,
    pub review_sha256: String,
    pub verdict: String,
}

impl ReviewAttestation {
    pub fn new(
        actor: impl Into<String>,
        independent_of: impl Into<String>,
        review_sha256: impl Into<String>,
        verdict: impl Into<String>,
    ) -> Result<Self> {
        let attestation = Self {
            actor: actor.into().trim().to_lowercase(),
            independent_of: independent_of.into().trim().to_lowercase(),
            review_sha256: review_sha256.into().trim().to_lowercase(),
            verdict: verdict.into().trim().to_lowercase(),
        };
        attestation.validate("")?;
        Ok(attestation)
    }

    /// Revalidate attestation invariants: non-emptiness, hex hash format,
    /// case-insensitive non-self-review (`actor != independent_of`), and that
    /// the reviewer is not the producer (`actor != producer`).
    pub fn validate(&self, producer: &str) -> Result<()> {
        let actor = self.actor.trim().to_lowercase();
        let independent_of = self.independent_of.trim().to_lowercase();
        let producer = producer.trim().to_lowercase();
        let review_sha256 = self.review_sha256.trim().to_lowercase();
        let verdict = self.verdict.trim().to_lowercase();

        if actor.is_empty() {
            bail!("review actor cannot be empty");
        }
        if independent_of.is_empty() {
            bail!("review independent_of cannot be empty");
        }
        if actor == independent_of {
            bail!(
                "self-review prohibited: actor '{actor}' equals independent_of '{independent_of}'"
            );
        }
        if !producer.is_empty() && actor == producer {
            bail!("independent reviewer cannot be producer ('{actor}' == '{producer}')");
        }
        if review_sha256.len() != 64 || !review_sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("review_sha256 must be a 64-character hex string; got '{review_sha256}'");
        }
        if verdict.is_empty() {
            bail!("verdict cannot be empty");
        }
        Ok(())
    }
}

/// Cryptographically sealed contribution envelope carrying a verified contribution through the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContributionEnvelope {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub contribution_id: ContributionId,
    pub origin_revision: String,
    pub lane: String,
    pub producer: String,
    pub slug: String,
    pub version: u32,
    pub target_crate: String,
    pub target_file: String,
    pub instruction: String,
    pub evidence_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virtual_patch_set_id: Option<String>,
    pub verification_commands: Vec<VerificationEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub independent_review: Option<ReviewAttestation>,
    pub trailers: Vec<String>,
    pub created_at_secs: u64,
}

impl ContributionEnvelope {
    /// Create a new contribution envelope.
    pub fn new(
        contribution_id: ContributionId,
        origin_revision: impl Into<String>,
        target_crate: impl Into<String>,
        target_file: impl Into<String>,
        instruction: impl Into<String>,
        evidence_key: impl Into<String>,
    ) -> Result<Self> {
        let origin_revision = origin_revision.into().trim().to_lowercase();
        let target_crate = target_crate.into().trim().to_string();
        let target_file = target_file.into().trim().to_string();
        let instruction = instruction.into().trim().to_string();
        let evidence_key = evidence_key.into().trim().to_lowercase();

        if origin_revision.len() < 7
            || origin_revision.len() > 64
            || !origin_revision.chars().all(|c| c.is_ascii_hexdigit())
        {
            bail!(
                "origin_revision must be a valid hex revision between 7 and 64 characters; got '{origin_revision}'"
            );
        }
        if evidence_key.len() != 64 || !evidence_key.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("evidence_key must be a 64-character hex string; got '{evidence_key}'");
        }

        let trailers = contribution_id.as_trailers(None);
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        Ok(Self {
            schema_version: SCHEMA_VERSION,
            artifact_kind: ARTIFACT_KIND.to_string(),
            lane: contribution_id.lane.clone(),
            producer: contribution_id.producer.clone(),
            slug: contribution_id.slug.clone(),
            version: contribution_id.version,
            contribution_id,
            origin_revision,
            target_crate,
            target_file,
            instruction,
            evidence_key,
            patch_sha256: None,
            virtual_patch_set_id: None,
            verification_commands: Vec::new(),
            independent_review: None,
            trailers,
            created_at_secs: now_secs,
        })
    }

    /// Bind a virtual patch set to this envelope, locking the output digest.
    pub fn bind_virtual_patch(
        &mut self,
        patch_set: &crate::virtual_patch::VirtualPatchSet,
    ) -> Result<()> {
        if patch_set.patch_set_digest.len() != 64 {
            bail!("virtual patch set digest must be a 64-character hex hash");
        }
        self.patch_sha256 = Some(patch_set.patch_set_digest.clone());
        self.virtual_patch_set_id = Some(patch_set.patch_set_id.clone());
        Ok(())
    }

    /// Add verified command output to the evidence set.
    pub fn add_verification(&mut self, command: impl Into<String>, exit_code: i32) {
        self.verification_commands.push(VerificationEvidence {
            command: command.into(),
            exit_code,
        });
    }

    /// Record an independent review attestation.
    pub fn record_review(&mut self, attestation: ReviewAttestation) -> Result<()> {
        attestation.validate(&self.producer)?;
        self.independent_review = Some(attestation);
        Ok(())
    }

    /// Validate the envelope invariants, optionally verifying revision pinning against a local checkout.
    pub fn validate(&self, checkout: Option<&Path>) -> Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            bail!(
                "unsupported schema version {}; expected {}",
                self.schema_version,
                SCHEMA_VERSION
            );
        }
        if self.artifact_kind != ARTIFACT_KIND {
            bail!(
                "invalid artifact kind '{}'; expected '{}'",
                self.artifact_kind,
                ARTIFACT_KIND
            );
        }
        if self.lane != self.contribution_id.lane {
            bail!(
                "lane mismatch: envelope '{}' != contribution_id '{}'",
                self.lane,
                self.contribution_id.lane
            );
        }
        if self.producer != self.contribution_id.producer {
            bail!(
                "producer mismatch: envelope '{}' != contribution_id '{}'",
                self.producer,
                self.contribution_id.producer
            );
        }
        if self.slug != self.contribution_id.slug {
            bail!(
                "slug mismatch: envelope '{}' != contribution_id '{}'",
                self.slug,
                self.contribution_id.slug
            );
        }
        if self.version != self.contribution_id.version {
            bail!(
                "version mismatch: envelope '{}' != contribution_id '{}'",
                self.version,
                self.contribution_id.version
            );
        }

        // Validate origin_revision format
        if self.origin_revision.len() < 7
            || self.origin_revision.len() > 64
            || !self.origin_revision.chars().all(|c| c.is_ascii_hexdigit())
        {
            bail!("invalid origin_revision '{}'", self.origin_revision);
        }

        // If checkout provided, verify origin_revision exists in git
        if let Some(repo_root) = checkout {
            let status = Command::new("git")
                .args([
                    "cat-file",
                    "-e",
                    &format!("{}^{{commit}}", self.origin_revision),
                ])
                .current_dir(repo_root)
                .status()
                .with_context(|| {
                    format!("failed to verify git revision '{}'", self.origin_revision)
                })?;

            if !status.success() {
                bail!(
                    "origin_revision '{}' not found in repository at '{}'",
                    self.origin_revision,
                    repo_root.display()
                );
            }
        }

        // Validate evidence key
        if self.evidence_key.len() != 64
            || !self.evidence_key.chars().all(|c| c.is_ascii_hexdigit())
        {
            bail!(
                "invalid evidence_key '{}': must be 64 hex characters",
                self.evidence_key
            );
        }

        // Validate patch_sha256 if present
        if let Some(ref patch_sha) = self.patch_sha256 {
            if patch_sha.len() != 64 || !patch_sha.chars().all(|c| c.is_ascii_hexdigit()) {
                bail!("invalid patch_sha256 '{patch_sha}': must be 64 hex characters");
            }
        }

        // Validate independent review if present
        if let Some(ref rev) = self.independent_review {
            rev.validate(&self.producer)?;
        }

        // Validate trailers contain Contribution-ID trailer
        let expected_trailer = format!("Contribution-ID: {}", self.contribution_id);
        if !self
            .trailers
            .iter()
            .any(|t| t.trim().eq_ignore_ascii_case(&expected_trailer))
        {
            bail!("trailers missing expected '{}'", expected_trailer);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contribution_id_parse_and_format() {
        let cid = ContributionId::parse("C117/gemini-contribution-lifecycle-v1").unwrap();
        assert_eq!(cid.lane, "C117");
        assert_eq!(cid.producer, "gemini");
        assert_eq!(cid.slug, "contribution-lifecycle");
        assert_eq!(cid.version, 1);
        assert_eq!(cid.to_string(), "C117/gemini-contribution-lifecycle-v1");

        // Multi-segment producer
        let cid2 = ContributionId::parse("C111/ubuntu-worker-01-onboarding-v2").unwrap();
        assert_eq!(cid2.lane, "C111");
        assert_eq!(cid2.producer, "ubuntu-worker-01");
        assert_eq!(cid2.slug, "onboarding");
        assert_eq!(cid2.version, 2);
        assert_eq!(cid2.to_string(), "C111/ubuntu-worker-01-onboarding-v2");

        // Serde roundtrip
        let json = serde_json::to_string(&cid).unwrap();
        let deserialized: ContributionId = serde_json::from_str(&json).unwrap();
        assert_eq!(cid, deserialized);
    }

    #[test]
    fn test_contribution_id_parse_negative() {
        assert!(ContributionId::parse("").is_err());
        assert!(ContributionId::parse("C117").is_err());
        assert!(ContributionId::parse("C117/gemini").is_err());
        assert!(ContributionId::parse("C117/gemini-slug").is_err());
        assert!(ContributionId::parse("C117/gemini-slug-v0").is_err());
        assert!(ContributionId::parse("C117/gemini-slug-vnotanumber").is_err());
        assert!(ContributionId::parse("/gemini-slug-v1").is_err());
    }

    #[test]
    fn test_trailers_generation() {
        let cid = ContributionId::parse("C117/gemini-contribution-lifecycle-v1").unwrap();
        let trailers = cid.as_trailers(None);
        assert_eq!(trailers.len(), 2);
        assert_eq!(
            trailers[0],
            "Contribution-ID: C117/gemini-contribution-lifecycle-v1"
        );
        assert_eq!(
            trailers[1],
            "Co-Authored-By: Gemini <gemini@aarogaming.local>"
        );
    }

    #[test]
    fn test_envelope_creation_and_validation() {
        let cid = ContributionId::parse("C117/gemini-contribution-lifecycle-v1").unwrap();
        let origin_rev = "47360c238b722d3e0b4931a7f058097b830d97fe";
        let evidence_key = "a".repeat(64);

        let mut env = ContributionEnvelope::new(
            cid,
            origin_rev,
            "foundation_cli",
            "src/contribution.rs",
            "connect contribution lifecycle",
            &evidence_key,
        )
        .unwrap();

        env.add_verification("cargo test", 0);
        assert!(env.validate(None).is_ok());

        // Self-review must fail
        let self_review =
            ReviewAttestation::new("gemini", "claude", "b".repeat(64), "accept").unwrap();
        assert!(env.record_review(self_review).is_err());

        // Independent review succeeds
        let valid_review =
            ReviewAttestation::new("claude", "gemini", "b".repeat(64), "accept").unwrap();
        assert!(env.record_review(valid_review).is_ok());
        assert!(env.validate(None).is_ok());
    }

    #[test]
    fn test_deserialized_envelope_rejects_bypassed_attestation() {
        let envelope_json = serde_json::json!({
            "schema_version": 1,
            "artifact_kind": "foundation.contribution-envelope",
            "contribution_id": "C117/gemini-contribution-lifecycle-v1",
            "origin_revision": "47360c238b722d3e0b4931a7f058097b830d97fe",
            "lane": "C117",
            "producer": "gemini",
            "slug": "contribution-lifecycle",
            "version": 1,
            "target_crate": "foundation_cli",
            "target_file": "src/contribution.rs",
            "instruction": "connect contribution lifecycle",
            "evidence_key": "a".repeat(64),
            "verification_commands": [],
            "independent_review": {
                "actor": "Gemini",
                "independent_of": "claude",
                "review_sha256": "b".repeat(64),
                "verdict": "accept"
            },
            "trailers": [
                "Contribution-ID: C117/gemini-contribution-lifecycle-v1"
            ],
            "created_at_secs": 1728470000
        });

        let env: ContributionEnvelope = serde_json::from_value(envelope_json).unwrap();
        // Even though Serde bypassed ReviewAttestation::new, validate() must catch Gemini == gemini
        assert!(env.validate(None).is_err());

        // Test empty verdict in deserialization
        let mut tampered = serde_json::json!({
            "schema_version": 1,
            "artifact_kind": "foundation.contribution-envelope",
            "contribution_id": "C117/gemini-contribution-lifecycle-v1",
            "origin_revision": "47360c238b722d3e0b4931a7f058097b830d97fe",
            "lane": "C117",
            "producer": "gemini",
            "slug": "contribution-lifecycle",
            "version": 1,
            "target_crate": "foundation_cli",
            "target_file": "src/contribution.rs",
            "instruction": "connect contribution lifecycle",
            "evidence_key": "a".repeat(64),
            "verification_commands": [],
            "independent_review": {
                "actor": "claude",
                "independent_of": "gemini",
                "review_sha256": "b".repeat(64),
                "verdict": ""
            },
            "trailers": [
                "Contribution-ID: C117/gemini-contribution-lifecycle-v1"
            ],
            "created_at_secs": 1728470000
        });
        let env_empty_verdict: ContributionEnvelope =
            serde_json::from_value(tampered.clone()).unwrap();
        assert!(env_empty_verdict.validate(None).is_err());

        // Test non-hex review_sha256 in deserialization
        tampered["independent_review"]["verdict"] = serde_json::json!("accept");
        tampered["independent_review"]["review_sha256"] = serde_json::json!("not-a-valid-hex-hash");
        let env_bad_hash: ContributionEnvelope = serde_json::from_value(tampered).unwrap();
        assert!(env_bad_hash.validate(None).is_err());
    }
}
