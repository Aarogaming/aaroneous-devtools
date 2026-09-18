use crate::cross_verifier::{CrossAuditVerdict, VerdictDecision};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ConsensusStatus {
    ApprovedForPromotion,
    PendingAudits,
    RejectedByAuditor,
    FailedMechanicalChecks,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct ConsensusRecord {
    pub schema_version: u32,
    pub task_id: String,
    pub author_provider: String,
    pub author_model: String,
    pub cross_verdicts: Vec<CrossAuditVerdict>,
    pub mechanical_checks_passed: bool,
    pub consensus_status: ConsensusStatus,
    pub evaluated_at_secs: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub(crate) struct ConsensusLedger {
    pub schema_version: u32,
    pub total_consensus_evaluations: u64,
    pub approved_tasks: u64,
    pub rejected_tasks: u64,
    pub records: Vec<ConsensusRecord>,
}

pub(crate) fn evaluate_consensus(
    task_id: &str,
    author_provider: &str,
    author_model: &str,
    verdicts: &[CrossAuditVerdict],
    mechanical_passed: bool,
    required_approvals: usize,
    now_secs: u64,
) -> ConsensusRecord {
    if !mechanical_passed {
        return ConsensusRecord {
            schema_version: 1,
            task_id: task_id.to_string(),
            author_provider: author_provider.to_string(),
            author_model: author_model.to_string(),
            cross_verdicts: verdicts.to_vec(),
            mechanical_checks_passed: false,
            consensus_status: ConsensusStatus::FailedMechanicalChecks,
            evaluated_at_secs: now_secs,
        };
    }

    let approved_count = verdicts
        .iter()
        .filter(|v| v.decision == VerdictDecision::Approved)
        .count();

    let has_rejection = verdicts.iter().any(|v| {
        v.decision == VerdictDecision::Rejected || v.decision == VerdictDecision::NeedsRevision
    });

    let status = if has_rejection {
        ConsensusStatus::RejectedByAuditor
    } else if approved_count >= required_approvals {
        ConsensusStatus::ApprovedForPromotion
    } else {
        ConsensusStatus::PendingAudits
    };

    ConsensusRecord {
        schema_version: 1,
        task_id: task_id.to_string(),
        author_provider: author_provider.to_string(),
        author_model: author_model.to_string(),
        cross_verdicts: verdicts.to_vec(),
        mechanical_checks_passed: true,
        consensus_status: status,
        evaluated_at_secs: now_secs,
    }
}

pub(crate) fn update_consensus_ledger(
    results_dir: &Path,
    record: &ConsensusRecord,
) -> Result<ConsensusLedger> {
    let ledger_path = results_dir.join("consensus_ledger.json");
    let mut ledger: ConsensusLedger = if ledger_path.exists() {
        let content = fs::read_to_string(&ledger_path).with_context(|| {
            format!("failed to read consensus ledger: {}", ledger_path.display())
        })?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        ConsensusLedger {
            schema_version: 1,
            ..Default::default()
        }
    };

    ledger.total_consensus_evaluations += 1;
    match record.consensus_status {
        ConsensusStatus::ApprovedForPromotion => ledger.approved_tasks += 1,
        ConsensusStatus::RejectedByAuditor | ConsensusStatus::FailedMechanicalChecks => {
            ledger.rejected_tasks += 1
        }
        ConsensusStatus::PendingAudits => {}
    }

    ledger.records.push(record.clone());
    if ledger.records.len() > 100 {
        ledger.records.remove(0);
    }

    fs::write(&ledger_path, serde_json::to_string_pretty(&ledger)?)?;
    Ok(ledger)
}

pub(crate) fn print_consensus_status(results_dir: &Path) -> Result<()> {
    let ledger_path = results_dir.join("consensus_ledger.json");
    if !ledger_path.exists() {
        println!("=== MULTI-AGENT CONSENSUS LEDGER ===");
        println!("Status: NO CROSS-AUDIT CONSENSUS EVALUATIONS RECORDED YET");
        return Ok(());
    }

    let content = fs::read_to_string(&ledger_path)?;
    let ledger: ConsensusLedger = serde_json::from_str(&content)?;

    println!("=== MULTI-AGENT ADVERSARIAL CONSENSUS LEDGER ===");
    println!(
        "Total Evaluations:   {}",
        ledger.total_consensus_evaluations
    );
    println!("Approved Tasks:      {}", ledger.approved_tasks);
    println!("Rejected Tasks:      {}", ledger.rejected_tasks);
    println!("------------------------------------------------");
    for (idx, rec) in ledger.records.iter().rev().take(10).enumerate() {
        println!(
            "  [{}] Task {}: {:?} (Author: {}/{}, Verifier Count: {})",
            idx + 1,
            rec.task_id,
            rec.consensus_status,
            rec.author_provider,
            rec.author_model,
            rec.cross_verdicts.len()
        );
        for v in &rec.cross_verdicts {
            println!(
                "      - Auditor {}/{} -> {:?}",
                v.auditor_provider, v.auditor_model, v.decision
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consensus_requires_mechanical_checks() {
        let rec = evaluate_consensus(
            "JOB-1",
            "local-ollama",
            "qwen3.5:9b-q6",
            &[],
            false,
            2,
            1000,
        );
        assert_eq!(
            rec.consensus_status,
            ConsensusStatus::FailedMechanicalChecks
        );
    }

    #[test]
    fn consensus_approves_when_required_audits_pass() {
        let verdict = CrossAuditVerdict {
            schema_version: 1,
            author_provider: "local-ollama".into(),
            author_model: "qwen3.5:9b-q6".into(),
            auditor_provider: "anthropic-claude".into(),
            auditor_model: "claude-3-5-sonnet".into(),
            proposal_task: "JOB-1".into(),
            decision: VerdictDecision::Approved,
            auditor_observations: vec![],
            security_flaws_found: vec![],
            test_coverage_gaps: vec![],
            recommended_action: "approve".into(),
            verified_at_secs: 1000,
        };

        let rec = evaluate_consensus(
            "JOB-1",
            "local-ollama",
            "qwen3.5:9b-q6",
            &[verdict.clone(), verdict],
            true,
            2,
            1000,
        );
        assert_eq!(rec.consensus_status, ConsensusStatus::ApprovedForPromotion);
    }
}
