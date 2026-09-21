use crate::cloud_providers::AgentProvider;
use crate::local_worker::Draft;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum VerdictDecision {
    Approved,
    Rejected,
    NeedsRevision,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct CrossAuditVerdict {
    pub schema_version: u32,
    pub author_provider: String,
    pub author_model: String,
    pub auditor_provider: String,
    pub auditor_model: String,
    pub proposal_task: String,
    pub decision: VerdictDecision,
    pub auditor_observations: Vec<String>,
    pub security_flaws_found: Vec<String>,
    pub test_coverage_gaps: Vec<String>,
    pub recommended_action: String,
    pub verified_at_secs: u64,
}

pub(crate) fn cross_verify_proposal(
    author_provider_name: &str,
    author_model_name: &str,
    proposal: &Draft,
    evidence_input: &str,
    auditor: &dyn AgentProvider,
    now_secs: u64,
) -> Result<CrossAuditVerdict> {
    if author_provider_name == auditor.provider_name() && author_model_name == auditor.model_name()
    {
        bail!(
            "Adversarial invariant violation: an agent provider cannot cross-audit its own proposal"
        );
    }

    let proposal_json = serde_json::to_string_pretty(proposal)?;
    let prompt = format!(
        "ADVERSARIAL RED-TEAM CROSS-AUDIT DIRECTIVE:\n\
        Another AI agent ({author_provider_name} - {author_model_name}) has generated a proposal for task '{task}'.\n\
        Do NOT accept their findings on trust. Treat their proposal as potentially faulty, incomplete, or insecure.\n\n\
        Original Evidence:\n{evidence_input}\n\n\
        Author Candidate Proposal:\n{proposal_json}\n\n\
        Your Task:\n\
        Audit this proposal for hidden panics, unhandled edge cases, ambient authority leaks, forbidden stubs, or flawed test plans.\n\
        Return your review using the proposal schema with task='CROSS-AUDIT:{task}'. If you find flaws, list them in risks and recommend revision.",
        task = proposal.task
    );

    let audit_draft = auditor.generate_proposal(&prompt).with_context(|| {
        format!(
            "cross-auditor {} failed to evaluate proposal",
            auditor.provider_name()
        )
    })?;

    let has_flaws = !audit_draft.risks.is_empty()
        && audit_draft.risks.iter().any(|r| {
            let lower = r.to_lowercase();
            lower.contains("flaw")
                || lower.contains("bug")
                || lower.contains("security")
                || lower.contains("stub")
                || lower.contains("leak")
                || lower.contains("violation")
                || lower.contains("reject")
        });

    let decision = if has_flaws {
        VerdictDecision::NeedsRevision
    } else {
        VerdictDecision::Approved
    };

    Ok(CrossAuditVerdict {
        schema_version: 1,
        author_provider: author_provider_name.to_string(),
        author_model: author_model_name.to_string(),
        auditor_provider: auditor.provider_name().to_string(),
        auditor_model: auditor.model_name().to_string(),
        proposal_task: proposal.task.clone(),
        decision,
        auditor_observations: audit_draft.observations,
        security_flaws_found: audit_draft.risks,
        test_coverage_gaps: audit_draft.test_plan,
        recommended_action: audit_draft.recommended_next_step,
        verified_at_secs: now_secs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAuditor {
        provider: String,
        model: String,
        should_reject: bool,
    }

    impl AgentProvider for MockAuditor {
        fn provider_name(&self) -> &str {
            &self.provider
        }
        fn model_name(&self) -> &str {
            &self.model
        }
        fn generate_proposal(&self, _input: &str) -> Result<Draft> {
            if self.should_reject {
                Ok(Draft {
                    task: "CROSS-AUDIT:JOB-1".into(),
                    observations: vec!["Audited proposal".into()],
                    risks: vec!["Found security violation in FFI boundary".into()],
                    test_plan: vec!["Add fuzz test".into()],
                    recommended_next_step: "Reject proposal".into(),
                    rust_draft: None,
                    solution_tree: None,
                })
            } else {
                Ok(Draft {
                    task: "CROSS-AUDIT:JOB-1".into(),
                    observations: vec!["Proposal verified safe".into()],
                    risks: vec!["No critical risks detected".into()],
                    test_plan: vec!["Verification passed".into()],
                    recommended_next_step: "Proceed to promotion".into(),
                    rust_draft: None,
                    solution_tree: None,
                })
            }
        }
    }

    #[test]
    fn cross_verifier_rejects_same_agent_self_audit() {
        let auditor = MockAuditor {
            provider: "local-ollama".into(),
            model: "qwen3.5:9b-q6".into(),
            should_reject: false,
        };
        let proposal = Draft {
            task: "JOB-1".into(),
            observations: vec!["obs".into()],
            risks: vec!["risk".into()],
            test_plan: vec!["plan".into()],
            recommended_next_step: "next".into(),
            rust_draft: None,
            solution_tree: None,
        };
        assert!(
            cross_verify_proposal(
                "local-ollama",
                "qwen3.5:9b-q6",
                &proposal,
                "evidence",
                &auditor,
                1000
            )
            .is_err()
        );
    }

    #[test]
    fn cross_verifier_detects_adversarial_rejection() {
        let auditor = MockAuditor {
            provider: "anthropic-claude".into(),
            model: "claude-3-5-sonnet".into(),
            should_reject: true,
        };
        let proposal = Draft {
            task: "JOB-1".into(),
            observations: vec!["obs".into()],
            risks: vec!["risk".into()],
            test_plan: vec!["plan".into()],
            recommended_next_step: "next".into(),
            rust_draft: None,
            solution_tree: None,
        };
        let verdict = cross_verify_proposal(
            "local-ollama",
            "qwen3.5:9b-q6",
            &proposal,
            "evidence",
            &auditor,
            1000,
        )
        .expect("verdict");

        assert_eq!(verdict.decision, VerdictDecision::NeedsRevision);
        assert_eq!(verdict.author_provider, "local-ollama");
        assert_eq!(verdict.auditor_provider, "anthropic-claude");
    }
}
