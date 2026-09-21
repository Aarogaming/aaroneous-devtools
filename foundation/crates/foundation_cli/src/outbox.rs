//! Local-worker outbox: a machine-readable review surface over
//! `artifacts/local-worker-service/results/`, so a reviewer -- human, or
//! another agent with file/shell access (this was built for the owner to
//! point a free local model like MiMo at) -- doesn't have to open 58
//! individual JSON files one at a time to find out what the local worker
//! has proposed.
//!
//! This module is read-only over `results/` and append-only over its own
//! verdicts file. It never promotes, claims, or deletes anything --
//! recording a verdict here is evidence a reviewer can act on, not an
//! action in itself. Actual promotion still goes through
//! `agentic_registrar::promote`, unchanged, per
//! `governance/LOCAL_AGENT_CONTROL_PLANE.md`.

use crate::local_worker::Record;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// A verdict is one of a fixed, small set so downstream tooling can
/// aggregate reliably instead of parsing free text.
const VALID_VERDICTS: &[&str] = &["accept", "reject", "needs-changes", "uncertain"];

/// Same producer-vs-bookkeeping-file exclusion `local_worker` itself uses,
/// so the outbox never tries to parse a ledger or crash-marker file as a
/// result record.
fn is_result_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    name.ends_with(".json")
        && !name.ends_with("ledger.json")
        && !name.ends_with("marker_retained.json")
}

fn result_files(results_dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut entries: Vec<_> = fs::read_dir(results_dir)
        .with_context(|| format!("reading results directory {}", results_dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry.path();
        if !is_result_file(&path) {
            continue;
        }
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            bail!("symlink result forbidden: {}", path.display());
        }
        paths.push(path);
    }
    Ok(paths)
}

/// The `artifact_kind` a real per-job `Record` carries. `results/` also
/// holds other artifact shapes from the same producer -- notably
/// `foundation.local-worker-exploration-group-summary` (an
/// `exploration_group`'s roster of member job ids, not a job result itself)
/// -- which this module deliberately skips rather than erroring on, since
/// they're legitimate, just not something the outbox lists as a reviewable
/// item.
const RESULT_ARTIFACT_KIND: &str = "foundation.local-worker-result";

/// Returns `Ok(Some(record))` for a real job result, `Ok(None)` for a
/// recognized-but-different artifact shape (skip, not an error), and
/// `Err` only when the file can't be read or isn't valid JSON at all.
fn load_record(path: &Path) -> Result<Option<Record>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading result {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("parsing result {}", path.display()))?;
    if value.get("artifact_kind").and_then(|v| v.as_str()) != Some(RESULT_ARTIFACT_KIND) {
        return Ok(None);
    }
    let record: Record = serde_json::from_value(value)
        .with_context(|| format!("parsing result {}", path.display()))?;
    Ok(Some(record))
}

/// One row of the outbox listing: a compact summary of a `Record`, not the
/// full proposal. Use [`show_entry`]/`run_show` to pull the full record for
/// one `job_id` once it looks worth a closer look.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub job_id: String,
    pub outcome: String,
    pub submitted_at_secs: u64,
    pub has_proposal: bool,
    /// `proposal.recommended_next_step` if there is a proposal, else
    /// `error` if the job failed, else a fixed placeholder. Never empty.
    pub gist: String,
    pub risk_count: usize,
    pub has_pre_proof_report: bool,
    pub reviewed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxReport {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub entry_count: usize,
    pub entries: Vec<OutboxEntry>,
}

fn entry_from_record(
    record: &Record,
    reviewed_ids: &std::collections::HashSet<String>,
) -> OutboxEntry {
    let gist = record
        .proposal
        .as_ref()
        .map(|d| d.recommended_next_step.clone())
        .filter(|s| !s.is_empty())
        .or_else(|| record.error.clone())
        .unwrap_or_else(|| "(no proposal or error recorded)".to_string());
    let risk_count = record.proposal.as_ref().map_or(0, |d| d.risks.len());
    OutboxEntry {
        job_id: record.job_id.clone(),
        outcome: record.outcome.clone(),
        submitted_at_secs: record.submitted_at_secs,
        has_proposal: record.proposal.is_some(),
        gist,
        risk_count,
        has_pre_proof_report: record.pre_proof_report.is_some(),
        reviewed: reviewed_ids.contains(&record.job_id),
    }
}

/// Reads every result in `results_dir` and returns a compact summary of
/// each, oldest first. `verdicts_path` is optional -- when given, entries
/// with at least one recorded verdict are marked `reviewed: true` so a
/// reviewer can filter to what's actually new.
pub fn list_outbox(results_dir: &Path, verdicts_path: Option<&Path>) -> Result<Vec<OutboxEntry>> {
    let reviewed_ids: std::collections::HashSet<String> = match verdicts_path {
        Some(path) if path.exists() => list_verdicts(path)?.into_iter().map(|v| v.job_id).collect(),
        _ => std::collections::HashSet::new(),
    };
    let mut entries = Vec::new();
    for path in result_files(results_dir)? {
        if let Some(record) = load_record(&path)? {
            entries.push(entry_from_record(&record, &reviewed_ids));
        }
    }
    entries.sort_by_key(|e| e.submitted_at_secs);
    Ok(entries)
}

/// `list_outbox`, written as a JSON report to `output`.
pub fn run_list(results_dir: &Path, verdicts_path: Option<&Path>, output: &Path) -> Result<()> {
    crate::require_absolute_directory("results-dir", results_dir)?;
    require_absolute_output(output)?;
    let entries = list_outbox(results_dir, verdicts_path)?;
    let report = OutboxReport {
        schema_version: 1,
        artifact_kind: "foundation.outbox-report".to_string(),
        entry_count: entries.len(),
        entries,
    };
    let data = serde_json::to_vec_pretty(&report)?;
    fs::write(output, data)
        .with_context(|| format!("writing outbox report to {}", output.display()))?;
    Ok(())
}

/// Returns the full `Record` for one `job_id`, or an error if no result
/// with that id exists in `results_dir`.
pub fn show_entry(results_dir: &Path, job_id: &str) -> Result<Record> {
    for path in result_files(results_dir)? {
        if let Some(record) = load_record(&path)?
            && record.job_id == job_id
        {
            return Ok(record);
        }
    }
    bail!(
        "no result found for job_id {job_id:?} in {}",
        results_dir.display()
    );
}

/// `show_entry`, written as a JSON report to `output`.
pub fn run_show(results_dir: &Path, job_id: &str, output: &Path) -> Result<()> {
    crate::require_absolute_directory("results-dir", results_dir)?;
    require_absolute_output(output)?;
    let record = show_entry(results_dir, job_id)?;
    let data = serde_json::to_vec_pretty(&record)?;
    fs::write(output, data).with_context(|| format!("writing record to {}", output.display()))?;
    Ok(())
}

/// A reviewer's structured verdict on one result. Never written by the
/// local worker itself -- always a separate reviewer (human, or another
/// agent) recording a judgment, same non-self-certification boundary as
/// `rejection_reason` on `Record` itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerdictEntry {
    pub job_id: String,
    pub reviewer: String,
    pub verdict: String,
    #[serde(default)]
    pub notes: String,
    pub recorded_at_secs: u64,
}

/// Appends one verdict to `verdicts_path` (JSON Lines, one record per
/// line). Validates `job_id` actually exists in `results_dir` and that
/// `verdict` is one of [`VALID_VERDICTS`] before writing anything --
/// refuses to record a verdict on a result that doesn't exist, and refuses
/// free-text verdicts that downstream tooling couldn't aggregate.
pub fn run_verdict(
    results_dir: &Path,
    verdicts_path: &Path,
    job_id: &str,
    reviewer: &str,
    verdict: &str,
    notes: &str,
) -> Result<()> {
    crate::require_absolute_directory("results-dir", results_dir)?;
    if !verdicts_path.is_absolute() {
        bail!(
            "verdicts path must be absolute: {}",
            verdicts_path.display()
        );
    }
    if reviewer.trim().is_empty() {
        bail!("reviewer must not be empty");
    }
    if !VALID_VERDICTS.contains(&verdict) {
        bail!("verdict must be one of {VALID_VERDICTS:?}, got {verdict:?}");
    }
    // Confirms the job actually exists before recording anything about it.
    show_entry(results_dir, job_id)?;

    if verdicts_path.exists()
        && fs::symlink_metadata(verdicts_path)?
            .file_type()
            .is_symlink()
    {
        bail!(
            "symlink verdicts file forbidden: {}",
            verdicts_path.display()
        );
    }
    let entry = VerdictEntry {
        job_id: job_id.to_string(),
        reviewer: reviewer.to_string(),
        verdict: verdict.to_string(),
        notes: notes.to_string(),
        recorded_at_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    let line = serde_json::to_string(&entry)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(verdicts_path)
        .with_context(|| format!("opening verdicts file {}", verdicts_path.display()))?;
    writeln!(file, "{line}")
        .with_context(|| format!("writing verdict to {}", verdicts_path.display()))?;
    Ok(())
}

/// Reads every verdict recorded in `verdicts_path`. Returns an empty list,
/// not an error, if the file doesn't exist yet -- no verdicts have been
/// recorded is a normal starting state, not a failure.
pub fn list_verdicts(verdicts_path: &Path) -> Result<Vec<VerdictEntry>> {
    if !verdicts_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(verdicts_path)
        .with_context(|| format!("reading verdicts file {}", verdicts_path.display()))?;
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        out.push(
            serde_json::from_str(line).with_context(|| format!("parsing verdict line: {line}"))?,
        );
    }
    Ok(out)
}

/// `list_verdicts`, written as a JSON array to `output`.
pub fn run_list_verdicts(verdicts_path: &Path, output: &Path) -> Result<()> {
    if !verdicts_path.is_absolute() {
        bail!(
            "verdicts path must be absolute: {}",
            verdicts_path.display()
        );
    }
    require_absolute_output(output)?;
    let verdicts = list_verdicts(verdicts_path)?;
    let data = serde_json::to_vec_pretty(&verdicts)?;
    fs::write(output, data)
        .with_context(|| format!("writing verdicts report to {}", output.display()))?;
    Ok(())
}

fn require_absolute_output(output: &Path) -> Result<()> {
    if !output.is_absolute() {
        bail!("output must be an absolute path: {}", output.display());
    }
    let parent = output
        .parent()
        .context("output path must have an existing parent directory")?;
    if !parent.is_dir() {
        bail!(
            "output parent directory does not exist: {}",
            parent.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_worker::Draft;

    fn write_record(dir: &Path, job_id: &str, gist: &str, risks: usize, error: Option<&str>) {
        let record = Record {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".to_string(),
            job_id: job_id.to_string(),
            submitted_at_secs: 1_700_000_000,
            outcome: "pending-owner-review".to_string(),
            input: "Approved job specification (data):\nid = \"x\"\n".to_string(),
            proposal: error.is_none().then(|| Draft {
                task: job_id.to_string(),
                observations: vec!["obs".to_string()],
                risks: (0..risks).map(|i| format!("risk {i}")).collect(),
                test_plan: vec!["plan".to_string()],
                recommended_next_step: gist.to_string(),
                rust_draft: None,
                solution_tree: None,
            }),
            error: error.map(str::to_string),
            pre_proof_report: None,
            rejection_reason: None,
            selected_branch: None,
            selection_rationale: None,
        };
        let path = dir.join(format!("{job_id}.json"));
        fs::write(path, serde_json::to_vec_pretty(&record).unwrap()).unwrap();
    }

    #[test]
    fn skips_exploration_group_summary_artifacts_without_erroring() {
        // Real bug caught against the live results/ directory: a second
        // artifact shape from the same producer (no job_id field at all)
        // sits in the same folder and must be skipped, not treated as a
        // parse failure for the whole listing.
        let dir = tempfile::tempdir().unwrap();
        write_record(dir.path(), "REAL", "gist", 0, None);
        fs::write(
            dir.path().join("m1-exploration-summary.json"),
            r#"{"schema_version":1,"artifact_kind":"foundation.local-worker-exploration-group-summary","exploration_group":"m1","goal":"M1","members":[]}"#,
        )
        .unwrap();

        let entries = list_outbox(dir.path(), None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].job_id, "REAL");

        // show_entry over the same directory must also skip it cleanly.
        let record = show_entry(dir.path(), "REAL").unwrap();
        assert_eq!(record.job_id, "REAL");
    }

    #[test]
    fn lists_entries_sorted_and_summarized() {
        let dir = tempfile::tempdir().unwrap();
        write_record(dir.path(), "B", "fix the thing", 2, None);
        write_record(dir.path(), "A", "", 0, Some("dispatch failed"));

        let entries = list_outbox(dir.path(), None).unwrap();
        assert_eq!(entries.len(), 2);
        // Both records share the same submitted_at_secs fixture value, so
        // sort is stable by insertion/filename order for this test's
        // purposes -- what matters is both are present and summarized.
        let b = entries.iter().find(|e| e.job_id == "B").unwrap();
        assert_eq!(b.gist, "fix the thing");
        assert_eq!(b.risk_count, 2);
        assert!(b.has_proposal);
        assert!(!b.reviewed);

        let a = entries.iter().find(|e| e.job_id == "A").unwrap();
        assert_eq!(a.gist, "dispatch failed");
        assert!(!a.has_proposal);
    }

    #[test]
    fn ignores_non_result_bookkeeping_files() {
        let dir = tempfile::tempdir().unwrap();
        write_record(dir.path(), "REAL", "gist", 0, None);
        fs::write(dir.path().join("probation_ledger.json"), "{}").unwrap();
        fs::write(dir.path().join("crash_marker_retained.json"), "{}").unwrap();

        let entries = list_outbox(dir.path(), None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].job_id, "REAL");
    }

    #[test]
    fn show_entry_finds_exact_job_and_errors_on_missing() {
        let dir = tempfile::tempdir().unwrap();
        write_record(dir.path(), "ONLY", "gist here", 1, None);

        let record = show_entry(dir.path(), "ONLY").unwrap();
        assert_eq!(record.job_id, "ONLY");

        let err = show_entry(dir.path(), "MISSING").unwrap_err();
        assert!(err.to_string().contains("MISSING"));
    }

    #[test]
    fn verdict_round_trips_and_rejects_invalid_verdict_or_missing_job() {
        let results = tempfile::tempdir().unwrap();
        write_record(results.path(), "JOB", "gist", 0, None);
        let verdicts_dir = tempfile::tempdir().unwrap();
        let verdicts_path = verdicts_dir.path().join("verdicts.jsonl");

        run_verdict(
            results.path(),
            &verdicts_path,
            "JOB",
            "mimo",
            "accept",
            "looks correct",
        )
        .unwrap();

        let recorded = list_verdicts(&verdicts_path).unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].job_id, "JOB");
        assert_eq!(recorded[0].reviewer, "mimo");
        assert_eq!(recorded[0].verdict, "accept");

        // A second verdict from a different reviewer appends, doesn't overwrite.
        run_verdict(
            results.path(),
            &verdicts_path,
            "JOB",
            "owner",
            "needs-changes",
            "double-check the overflow case",
        )
        .unwrap();
        assert_eq!(list_verdicts(&verdicts_path).unwrap().len(), 2);

        // Invalid verdict string is rejected before anything is written.
        let before = list_verdicts(&verdicts_path).unwrap().len();
        let err =
            run_verdict(results.path(), &verdicts_path, "JOB", "mimo", "meh", "").unwrap_err();
        assert!(err.to_string().contains("meh"));
        assert_eq!(list_verdicts(&verdicts_path).unwrap().len(), before);

        // Verdict on a job_id that doesn't exist in results_dir is refused.
        let err =
            run_verdict(results.path(), &verdicts_path, "NOPE", "mimo", "accept", "").unwrap_err();
        assert!(err.to_string().contains("NOPE"));
    }

    #[test]
    fn list_verdicts_on_missing_file_is_empty_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.jsonl");
        assert_eq!(list_verdicts(&path).unwrap(), Vec::new());
    }

    #[test]
    fn reviewed_flag_reflects_recorded_verdicts() {
        let results = tempfile::tempdir().unwrap();
        write_record(results.path(), "SEEN", "gist", 0, None);
        write_record(results.path(), "UNSEEN", "gist", 0, None);
        let verdicts_dir = tempfile::tempdir().unwrap();
        let verdicts_path = verdicts_dir.path().join("verdicts.jsonl");
        run_verdict(results.path(), &verdicts_path, "SEEN", "mimo", "accept", "").unwrap();

        let entries = list_outbox(results.path(), Some(&verdicts_path)).unwrap();
        let seen = entries.iter().find(|e| e.job_id == "SEEN").unwrap();
        let unseen = entries.iter().find(|e| e.job_id == "UNSEEN").unwrap();
        assert!(seen.reviewed);
        assert!(!unseen.reviewed);
    }
}
