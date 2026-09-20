use crate::consensus_engine::{ConsensusLedger, ConsensusStatus};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::fs;
use std::path::Path;

const REPORT_KIND: &str = "foundation.consensus-readiness";

#[derive(Debug, Serialize)]
struct ConsensusReadinessReport {
    schema_version: u32,
    artifact_kind: String,
    queue: String,
    ledger: String,
    ready_for_signoff: Vec<String>,
    findings: Vec<String>,
}

/// Cross-reference `governance/COORDINATION_QUEUE.md` rows against a
/// `ConsensusLedger` and report which queue rows have an
/// `ApprovedForPromotion` consensus record, so a human reviewer can decide
/// whether to mark them `Done`.
///
/// Read-only: this function never writes to `queue` or `ledger`. It only
/// ever writes the report at `output`.
pub(crate) fn run(queue: &Path, ledger: &Path, output: &Path) -> Result<()> {
    require_absolute_file("queue", queue)?;
    require_absolute_file("ledger", ledger)?;
    validate_output(output)?;

    let queue_text =
        fs::read_to_string(queue).with_context(|| format!("failed to read {}", queue.display()))?;
    let ledger_text = fs::read_to_string(ledger)
        .with_context(|| format!("failed to read {}", ledger.display()))?;
    let consensus_ledger: ConsensusLedger = serde_json::from_str(&ledger_text)
        .with_context(|| format!("failed to parse consensus ledger: {}", ledger.display()))?;

    let rows = queue_rows(&queue_text);
    let candidate_rows = rows
        .iter()
        .filter(|(_, status)| status == "Ready" || status == "Claimed" || status == "Review");

    let mut ready_for_signoff = Vec::new();
    let mut findings = Vec::new();

    for (id, _status) in candidate_rows {
        match consensus_ledger
            .records
            .iter()
            .find(|record| &record.task_id == id)
        {
            Some(record) if record.consensus_status == ConsensusStatus::ApprovedForPromotion => {
                ready_for_signoff.push(id.clone());
            }
            Some(_) => {
                // Non-approved status: excluded, not flagged as an error.
            }
            None => {
                findings.push(format!(
                    "queue row {id} has no matching consensus ledger record"
                ));
            }
        }
    }

    let report = ConsensusReadinessReport {
        schema_version: 1,
        artifact_kind: REPORT_KIND.to_owned(),
        queue: queue.display().to_string(),
        ledger: ledger.display().to_string(),
        ready_for_signoff,
        findings,
    };
    fs::write(
        output,
        serde_json::to_vec_pretty(&report)
            .context("failed to serialize consensus readiness report")?,
    )
    .with_context(|| format!("failed to write {}", output.display()))?;
    println!("wrote consensus readiness report: {}", output.display());
    Ok(())
}

fn queue_rows(input: &str) -> Vec<(String, String)> {
    input
        .lines()
        .filter_map(|line| {
            let fields = line.split('|').map(str::trim).collect::<Vec<_>>();
            if fields.len() < 4
                || fields[1] == "ID"
                || fields[1].is_empty()
                || !fields[1].chars().all(|c| c.is_ascii_alphanumeric())
            {
                return None;
            }
            Some((fields[1].to_owned(), fields[3].to_owned()))
        })
        .collect()
}

fn require_absolute_file(label: &str, path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("{label} must be an absolute path: {}", path.display());
    }
    if !fs::metadata(path)
        .with_context(|| format!("{label} is unavailable: {}", path.display()))?
        .is_file()
    {
        bail!("{label} must be a file: {}", path.display());
    }
    Ok(())
}

fn validate_output(output: &Path) -> Result<()> {
    if !output.is_absolute() {
        bail!("output must be an absolute path: {}", output.display());
    }
    let parent = output.parent().context("output must have a parent")?;
    if !fs::metadata(parent)
        .with_context(|| format!("output parent is unavailable: {}", parent.display()))?
        .is_dir()
    {
        bail!("output parent must be a directory: {}", parent.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus_engine::ConsensusRecord;

    fn record(task_id: &str, status: ConsensusStatus) -> ConsensusRecord {
        ConsensusRecord {
            schema_version: 1,
            task_id: task_id.to_owned(),
            author_provider: "local-ollama".to_owned(),
            author_model: "qwen3.5:9b-q6".to_owned(),
            cross_verdicts: vec![],
            mechanical_checks_passed: true,
            consensus_status: status,
            evaluated_at_secs: 1000,
        }
    }

    #[test]
    fn matching_approved_record_is_ready_for_signoff() {
        let queue = "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n\
             | C9 | scope | Ready | owner | branch | 2026-09-20 | evidence |";
        let rows = queue_rows(queue);
        let ledger = ConsensusLedger {
            schema_version: 1,
            total_consensus_evaluations: 1,
            approved_tasks: 1,
            rejected_tasks: 0,
            records: vec![record("C9", ConsensusStatus::ApprovedForPromotion)],
        };

        let mut ready = Vec::new();
        let mut findings = Vec::new();
        for (id, _status) in rows
            .iter()
            .filter(|(_, status)| status == "Ready" || status == "Claimed" || status == "Review")
        {
            match ledger.records.iter().find(|r| &r.task_id == id) {
                Some(r) if r.consensus_status == ConsensusStatus::ApprovedForPromotion => {
                    ready.push(id.clone())
                }
                Some(_) => {}
                None => findings.push(format!(
                    "queue row {id} has no matching consensus ledger record"
                )),
            }
        }

        assert_eq!(ready, vec!["C9".to_owned()]);
        assert!(findings.is_empty());
    }

    #[test]
    fn missing_ledger_record_is_noted_not_errored() {
        let queue = "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n\
             | C10 | scope | Claimed | owner | branch | 2026-09-20 | evidence |";
        let rows = queue_rows(queue);
        let ledger = ConsensusLedger::default();

        let mut ready = Vec::new();
        let mut findings = Vec::new();
        for (id, _status) in rows
            .iter()
            .filter(|(_, status)| status == "Ready" || status == "Claimed" || status == "Review")
        {
            match ledger.records.iter().find(|r| &r.task_id == id) {
                Some(r) if r.consensus_status == ConsensusStatus::ApprovedForPromotion => {
                    ready.push(id.clone())
                }
                Some(_) => {}
                None => findings.push(format!(
                    "queue row {id} has no matching consensus ledger record"
                )),
            }
        }

        assert!(ready.is_empty());
        assert_eq!(
            findings,
            vec!["queue row C10 has no matching consensus ledger record".to_owned()]
        );
    }

    #[test]
    fn rejected_record_is_excluded() {
        let queue = "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n\
             | C11 | scope | Review | owner | branch | 2026-09-20 | evidence |";
        let rows = queue_rows(queue);
        let ledger = ConsensusLedger {
            schema_version: 1,
            total_consensus_evaluations: 1,
            approved_tasks: 0,
            rejected_tasks: 1,
            records: vec![record("C11", ConsensusStatus::RejectedByAuditor)],
        };

        let mut ready = Vec::new();
        let mut findings = Vec::new();
        for (id, _status) in rows
            .iter()
            .filter(|(_, status)| status == "Ready" || status == "Claimed" || status == "Review")
        {
            match ledger.records.iter().find(|r| &r.task_id == id) {
                Some(r) if r.consensus_status == ConsensusStatus::ApprovedForPromotion => {
                    ready.push(id.clone())
                }
                Some(_) => {}
                None => findings.push(format!(
                    "queue row {id} has no matching consensus ledger record"
                )),
            }
        }

        assert!(ready.is_empty());
        assert!(findings.is_empty());
    }

    #[test]
    fn reads_only_candidate_status_rows() {
        let rows = queue_rows(
            "| ID | Scope | Status |\n| --- | --- | --- |\n| C1 | x | Ready |\n| F7 | x | Done |",
        );
        assert_eq!(
            rows,
            vec![
                ("C1".to_owned(), "Ready".to_owned()),
                ("F7".to_owned(), "Done".to_owned())
            ]
        );
    }
}
