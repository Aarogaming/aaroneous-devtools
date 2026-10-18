//! Pipeline Router: Active Output Feedback and Staging Engine
//!
//! Replaces dead-end `archive/` disposal with active routing into the next operational loop:
//! - Evaluates candidate model results from `results/`.
//! - Validates conformance flags and code drafts.
//! - Routes approved candidates to `staged/` or `intake/` for topic-case/virtual-patch generation.
//! - Emits refinement feedback jobs when self-correction is required.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// The routing decision made for an evaluated worker result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingAction {
    /// Result passed conformance and code validation; staged for integration.
    StageForIntegration {
        job_id: String,
        target_path: String,
        has_code_draft: bool,
    },
    /// Result has conformance issues or test failures; fed back for refinement.
    FeedbackForRefinement {
        job_id: String,
        evidence: Vec<String>,
        issues: Vec<String>,
    },
    /// Result is an exploration summary or status record; recorded in ledger.
    RecordTelemetry { job_id: String },
}

/// Representation of a local worker result JSON payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResultPayload {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub job_id: String,
    pub submitted_at_secs: u64,
    pub outcome: String,
    pub input: String,
    pub proposal: Option<WorkerProposal>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub conformance_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerProposal {
    pub task: String,
    pub observations: Vec<String>,
    pub risks: Vec<String>,
    pub test_plan: Vec<String>,
    pub recommended_next_step: String,
    #[serde(default)]
    pub rust_draft: Option<String>,
}

/// Representation of a generated feedback refinement job manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedbackJobManifest {
    pub schema_version: u32,
    pub id: String,
    pub mode: String,
    pub evidence: Vec<String>,
    pub instruction: String,
}

/// Configuration for active pipeline routing.
pub struct PipelineRouterConfig {
    pub results_dir: PathBuf,
    pub staged_dir: PathBuf,
    pub feedback_jobs_dir: PathBuf,
}

impl PipelineRouterConfig {
    pub fn new(
        results_dir: impl Into<PathBuf>,
        staged_dir: impl Into<PathBuf>,
        feedback_jobs_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            results_dir: results_dir.into(),
            staged_dir: staged_dir.into(),
            feedback_jobs_dir: feedback_jobs_dir.into(),
        }
    }
}

/// Validates that a job identifier is bounded, free of path traversal sequences, and not a Windows reserved device name.
pub fn is_valid_job_id(job_id: &str) -> bool {
    if job_id.is_empty() || job_id.len() > 128 || job_id.contains("..") {
        return false;
    }
    if !job_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return false;
    }
    // Reject Windows reserved device names (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
    let base = job_id.split('.').next().unwrap_or(job_id);
    let upper = base.to_ascii_uppercase();
    let is_reserved = matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    !is_reserved
}

/// Copies a file using create_new semantics. Returns Ok(true) if copied, Ok(false) if destination already exists.
fn copy_create_new(src: &std::path::Path, dst: &std::path::Path) -> Result<bool> {
    if dst.is_dir() {
        anyhow::bail!("Destination is a directory, not a file: {}", dst.display());
    }
    let mut reader =
        fs::File::open(src).with_context(|| format!("Opening source file: {}", src.display()))?;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)
    {
        Ok(mut writer) => {
            std::io::copy(&mut reader, &mut writer)?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            eprintln!(
                "Warning: destination already exists, skipping copy to {}",
                dst.display()
            );
            Ok(false)
        }
        Err(e) => Err(e).with_context(|| format!("Creating new staged file: {}", dst.display())),
    }
}

/// Writes content to a file using create_new semantics. Returns Ok(true) if written, Ok(false) if destination already exists.
fn write_create_new(dst: &std::path::Path, content: &str) -> Result<bool> {
    use std::io::Write;
    if dst.is_dir() {
        anyhow::bail!("Destination is a directory, not a file: {}", dst.display());
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)
    {
        Ok(mut writer) => {
            writer.write_all(content.as_bytes())?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            eprintln!(
                "Warning: manifest destination already exists, skipping write to {}",
                dst.display()
            );
            Ok(false)
        }
        Err(e) => Err(e).with_context(|| format!("Creating new manifest file: {}", dst.display())),
    }
}

/// Extracts declared evidence files from the raw job input text, falling back to a default.
fn extract_evidence_from_input(input: &str) -> Vec<String> {
    for line in input.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("evidence = [") {
            let inner = rest.trim_end_matches(']').trim();
            let mut list = Vec::new();
            for item in inner.split(',') {
                let s = item.trim().trim_matches('"').trim_matches('\'');
                if !s.is_empty() {
                    list.push(s.to_string());
                }
            }
            if !list.is_empty() {
                return list;
            }
        }
    }
    vec!["AGENTS.md".to_string()]
}

/// Evaluates a single worker result and determines the next loop action.
pub fn evaluate_result(payload: &WorkerResultPayload) -> RoutingAction {
    let evidence = extract_evidence_from_input(&payload.input);

    // If there are conformance flags (e.g. unanchored citations), route back for refinement
    if !payload.conformance_flags.is_empty() {
        return RoutingAction::FeedbackForRefinement {
            job_id: payload.job_id.clone(),
            evidence,
            issues: payload.conformance_flags.clone(),
        };
    }

    if let Some(err) = &payload.error {
        return RoutingAction::FeedbackForRefinement {
            job_id: payload.job_id.clone(),
            evidence,
            issues: vec![format!("Execution error: {}", err)],
        };
    }

    if let Some(proposal) = &payload.proposal {
        let has_code = proposal
            .rust_draft
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty());
        RoutingAction::StageForIntegration {
            job_id: payload.job_id.clone(),
            target_path: proposal.task.clone(),
            has_code_draft: has_code,
        }
    } else {
        RoutingAction::RecordTelemetry {
            job_id: payload.job_id.clone(),
        }
    }
}

/// Processes all pending results in `results_dir`, routing them into active stages
/// and removing them from `results_dir` to prevent backpressure.
pub fn route_all_pending_results(config: &PipelineRouterConfig) -> Result<Vec<RoutingAction>> {
    fs::create_dir_all(&config.staged_dir)?;
    fs::create_dir_all(&config.feedback_jobs_dir)?;

    let mut actions = Vec::new();

    if !config.results_dir.exists() {
        return Ok(actions);
    }

    for entry in fs::read_dir(&config.results_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if path.file_name().and_then(|s| s.to_str()) == Some("status.txt") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Reading result file: {}", path.display()))?;

        let payload: WorkerResultPayload = match serde_json::from_str(&content) {
            Ok(p) => p,
            Err(e) => {
                eprintln!(
                    "Warning: skipping malformed result {}: {}",
                    path.display(),
                    e
                );
                continue;
            }
        };

        if !is_valid_job_id(&payload.job_id) {
            eprintln!(
                "Warning: skipping result {} with invalid or traversal job_id {:?}",
                path.display(),
                payload.job_id
            );
            continue;
        }

        let action = evaluate_result(&payload);

        match &action {
            RoutingAction::StageForIntegration { job_id, .. } => {
                let dest = config.staged_dir.join(format!("{}.staged.json", job_id));
                if !copy_create_new(&path, &dest)? {
                    continue;
                }
                fs::remove_file(&path)?;
            }
            RoutingAction::FeedbackForRefinement {
                job_id,
                evidence,
                issues,
            } => {
                let clean_id = job_id.strip_prefix("FEED-").unwrap_or(job_id);
                let manifest_id = format!("REFINE-{}", clean_id);
                if !is_valid_job_id(&manifest_id) {
                    eprintln!(
                        "Warning: skipping feedback manifest with invalid id {:?}",
                        manifest_id
                    );
                    continue;
                }

                // Stage the result record FIRST with create_new semantics
                let dest = config
                    .staged_dir
                    .join(format!("{}.needs-refinement.json", job_id));
                if !copy_create_new(&path, &dest)? {
                    // Staged record already exists: skip this result to avoid orphaning manifests or aborting
                    continue;
                }

                // Write the feedback manifest SECOND
                let manifest = FeedbackJobManifest {
                    schema_version: 1,
                    id: manifest_id.clone(),
                    mode: "code-draft".to_string(),
                    evidence: evidence.clone(),
                    instruction: format!(
                        "Refine previous proposal to address reported conformance issues: {}",
                        issues.join("; ")
                    ),
                };

                let feedback_manifest =
                    toml::to_string(&manifest).context("Serializing feedback manifest to TOML")?;
                let manifest_path = config
                    .feedback_jobs_dir
                    .join(format!("{}.toml", manifest_id));
                match write_create_new(&manifest_path, &feedback_manifest) {
                    Ok(true) => {}
                    Ok(false) => {
                        eprintln!(
                            "Warning: feedback manifest {} already exists, keeping the earlier manifest",
                            manifest_path.display()
                        );
                    }
                    Err(e) => {
                        let _ = fs::remove_file(&dest);
                        return Err(e).context(format!(
                            "Failed to write feedback manifest {} for result {}",
                            manifest_path.display(),
                            path.display()
                        ));
                    }
                }

                // Remove source result after successful staging
                fs::remove_file(&path)?;
            }
            RoutingAction::RecordTelemetry { job_id } => {
                let dest = config.staged_dir.join(format!("{}.telemetry.json", job_id));
                if !copy_create_new(&path, &dest)? {
                    continue;
                }
                fs::remove_file(&path)?;
            }
        }

        actions.push(action);
    }

    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_job_id() {
        assert!(is_valid_job_id("JOB-123.abc_456"));
        assert!(is_valid_job_id("REFINE-FIX-01"));
        assert!(!is_valid_job_id(""));
        assert!(!is_valid_job_id("../../traversal"));
        assert!(!is_valid_job_id("job/with/slashes"));
        assert!(!is_valid_job_id("job with spaces"));
        assert!(!is_valid_job_id("job\nwith\nnewlines"));
        let long_id = "a".repeat(129);
        assert!(!is_valid_job_id(&long_id));
    }

    #[test]
    fn test_invalid_job_id_skipped_during_routing() {
        let temp = tempfile::tempdir().unwrap();
        let results_dir = temp.path().join("results");
        let staged_dir = temp.path().join("staged");
        let feedback_dir = temp.path().join("feedback");
        fs::create_dir_all(&results_dir).unwrap();

        let malicious_payload = WorkerResultPayload {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".into(),
            job_id: "../../escape".into(),
            submitted_at_secs: 1000,
            outcome: "pending-owner-review".into(),
            input: "test".into(),
            proposal: None,
            error: None,
            conformance_flags: vec![],
        };
        fs::write(
            results_dir.join("bad.json"),
            serde_json::to_vec(&malicious_payload).unwrap(),
        )
        .unwrap();

        let config = PipelineRouterConfig::new(&results_dir, &staged_dir, &feedback_dir);
        let actions = route_all_pending_results(&config).unwrap();
        assert_eq!(actions.len(), 0);
        // The bad result remains unprocessed in results_dir
        assert!(results_dir.join("bad.json").exists());
        // Nothing escaped outside staged_dir
        assert!(!temp.path().join("escape.telemetry.json").exists());
    }

    #[test]
    fn test_staged_create_new_refuses_clobber() {
        let temp = tempfile::tempdir().unwrap();
        let results_dir = temp.path().join("results");
        let staged_dir = temp.path().join("staged");
        let feedback_dir = temp.path().join("feedback");
        fs::create_dir_all(&results_dir).unwrap();
        fs::create_dir_all(&staged_dir).unwrap();

        // Pre-create an existing staged file
        fs::write(staged_dir.join("JOB-1.telemetry.json"), b"existing").unwrap();

        let payload = WorkerResultPayload {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".into(),
            job_id: "JOB-1".into(),
            submitted_at_secs: 1000,
            outcome: "pending-owner-review".into(),
            input: "test".into(),
            proposal: None,
            error: None,
            conformance_flags: vec![],
        };
        fs::write(
            results_dir.join("job1.json"),
            serde_json::to_vec(&payload).unwrap(),
        )
        .unwrap();

        let config = PipelineRouterConfig::new(&results_dir, &staged_dir, &feedback_dir);
        let res = route_all_pending_results(&config);
        assert!(
            res.is_ok(),
            "route_all_pending_results must not error or wedge on existing destination"
        );
        assert!(
            res.unwrap().is_empty(),
            "No new files should have been staged"
        );
        // Verify original file is unchanged
        assert_eq!(
            fs::read(staged_dir.join("JOB-1.telemetry.json")).unwrap(),
            b"existing"
        );
    }

    #[test]
    fn test_feedback_manifest_toml_serialization() {
        let manifest = FeedbackJobManifest {
            schema_version: 1,
            id: "REFINE-TEST".into(),
            mode: "code-draft".into(),
            evidence: vec!["AGENTS.md".into(), "crates/lib.rs".into()],
            instruction: "Fix \"unescaped quote\" and [brackets] with\nnewlines".into(),
        };

        let serialized = toml::to_string(&manifest).unwrap();
        let parsed: FeedbackJobManifest = toml::from_str(&serialized).unwrap();
        assert_eq!(manifest, parsed);
    }

    #[test]
    fn test_evaluate_clean_proposal() {
        let payload = WorkerResultPayload {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".into(),
            job_id: "JOB-100".into(),
            submitted_at_secs: 1000,
            outcome: "pending-owner-review".into(),
            input: "test".into(),
            proposal: Some(WorkerProposal {
                task: "Write unit tests".into(),
                observations: vec!["Valid observation".into()],
                risks: vec![],
                test_plan: vec!["Run test".into()],
                recommended_next_step: "Proceed".into(),
                rust_draft: Some("fn test_ok() {}".into()),
            }),
            error: None,
            conformance_flags: vec![],
        };

        let action = evaluate_result(&payload);
        match action {
            RoutingAction::StageForIntegration {
                job_id,
                has_code_draft,
                ..
            } => {
                assert_eq!(job_id, "JOB-100");
                assert!(has_code_draft);
            }
            _ => panic!("Expected StageForIntegration"),
        }
    }

    #[test]
    fn test_evaluate_unanchored_observation_routes_to_refinement() {
        let payload = WorkerResultPayload {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".into(),
            job_id: "JOB-200".into(),
            submitted_at_secs: 1000,
            outcome: "pending-owner-review".into(),
            input: "test".into(),
            proposal: Some(WorkerProposal {
                task: "Write unit tests".into(),
                observations: vec!["Observation without citation".into()],
                risks: vec![],
                test_plan: vec![],
                recommended_next_step: "Fix".into(),
                rust_draft: None,
            }),
            error: None,
            conformance_flags: vec![
                "observation 1: not anchored in any declared evidence file".into(),
            ],
        };

        let action = evaluate_result(&payload);
        match action {
            RoutingAction::FeedbackForRefinement {
                job_id,
                evidence,
                issues,
            } => {
                assert_eq!(job_id, "JOB-200");
                assert_eq!(evidence, vec!["AGENTS.md"]);
                assert_eq!(issues.len(), 1);
            }
            _ => panic!("Expected FeedbackForRefinement"),
        }
    }

    #[test]
    fn test_route_all_pending_results_e2e() {
        let temp = tempfile::tempdir().unwrap();
        let results_dir = temp.path().join("results");
        let staged_dir = temp.path().join("staged");
        let feedback_dir = temp.path().join("feedback");
        fs::create_dir_all(&results_dir).unwrap();

        let clean_payload = WorkerResultPayload {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".into(),
            job_id: "JOB-CLEAN".into(),
            submitted_at_secs: 1000,
            outcome: "pending-owner-review".into(),
            input: "test".into(),
            proposal: Some(WorkerProposal {
                task: "Write test".into(),
                observations: vec!["Obs 1".into()],
                risks: vec![],
                test_plan: vec![],
                recommended_next_step: "Proceed".into(),
                rust_draft: Some("fn test_clean() {}".into()),
            }),
            error: None,
            conformance_flags: vec![],
        };
        fs::write(
            results_dir.join("JOB-CLEAN-0000.json"),
            serde_json::to_vec(&clean_payload).unwrap(),
        )
        .unwrap();

        let refine_payload = WorkerResultPayload {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".into(),
            job_id: "JOB-REFINE".into(),
            submitted_at_secs: 1000,
            outcome: "pending-owner-review".into(),
            input: "test".into(),
            proposal: Some(WorkerProposal {
                task: "Audit".into(),
                observations: vec!["Unanchored obs".into()],
                risks: vec![],
                test_plan: vec![],
                recommended_next_step: "Fix".into(),
                rust_draft: None,
            }),
            error: None,
            conformance_flags: vec!["observation 1: unanchored".into()],
        };
        fs::write(
            results_dir.join("JOB-REFINE-0000.json"),
            serde_json::to_vec(&refine_payload).unwrap(),
        )
        .unwrap();

        let config = PipelineRouterConfig::new(&results_dir, &staged_dir, &feedback_dir);
        let actions = route_all_pending_results(&config).unwrap();
        assert_eq!(actions.len(), 2);

        // Results directory is drained
        let remaining_results: Vec<_> = fs::read_dir(&results_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(remaining_results.len(), 0);

        // Staged artifacts exist
        assert!(staged_dir.join("JOB-CLEAN.staged.json").exists());
        assert!(staged_dir.join("JOB-REFINE.needs-refinement.json").exists());

        // Feedback manifest was generated for next loop pass
        assert!(feedback_dir.join("REFINE-JOB-REFINE.toml").exists());
        let toml_str = fs::read_to_string(feedback_dir.join("REFINE-JOB-REFINE.toml")).unwrap();
        let parsed: FeedbackJobManifest = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.id, "REFINE-JOB-REFINE");
    }

    #[test]
    fn test_windows_reserved_names_rejected() {
        assert!(!is_valid_job_id("CON"));
        assert!(!is_valid_job_id("con"));
        assert!(!is_valid_job_id("PRN"));
        assert!(!is_valid_job_id("aux"));
        assert!(!is_valid_job_id("NUL"));
        assert!(!is_valid_job_id("COM1"));
        assert!(!is_valid_job_id("lpt9"));
        assert!(!is_valid_job_id("NUL.txt"));

        // Valid IDs still pass
        assert!(is_valid_job_id("JOB-1"));
        assert!(is_valid_job_id("REFINE-JOB-1"));
        assert!(is_valid_job_id("FIX-concurrency"));
    }

    #[test]
    fn test_collision_on_one_result_does_not_wedge_batch_and_processes_subsequent_results() {
        let temp = tempfile::tempdir().unwrap();
        let results_dir = temp.path().join("results");
        let staged_dir = temp.path().join("staged");
        let feedback_dir = temp.path().join("feedback");
        fs::create_dir_all(&results_dir).unwrap();
        fs::create_dir_all(&staged_dir).unwrap();
        fs::create_dir_all(&feedback_dir).unwrap();

        // Pre-create staged artifact for JOB-1 (collision scenario)
        fs::write(
            staged_dir.join("JOB-1.needs-refinement.json"),
            b"{\"staged\":\"already_present\"}",
        )
        .unwrap();

        // Create two result files in results_dir: JOB-1 and JOB-2
        let payload_job1 = WorkerResultPayload {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".into(),
            job_id: "JOB-1".into(),
            submitted_at_secs: 1000,
            outcome: "pending-owner-review".into(),
            input: "test".into(),
            proposal: Some(WorkerProposal {
                task: "Audit".into(),
                observations: vec!["Obs 1".into()],
                risks: vec![],
                test_plan: vec![],
                recommended_next_step: "Fix".into(),
                rust_draft: None,
            }),
            error: None,
            conformance_flags: vec!["observation 1: unanchored".into()],
        };
        fs::write(
            results_dir.join("JOB-1.json"),
            serde_json::to_vec(&payload_job1).unwrap(),
        )
        .unwrap();

        let payload_job2 = WorkerResultPayload {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".into(),
            job_id: "JOB-2".into(),
            submitted_at_secs: 1001,
            outcome: "pending-owner-review".into(),
            input: "test".into(),
            proposal: Some(WorkerProposal {
                task: "Audit 2".into(),
                observations: vec!["Obs 2".into()],
                risks: vec![],
                test_plan: vec![],
                recommended_next_step: "Fix".into(),
                rust_draft: None,
            }),
            error: None,
            conformance_flags: vec!["observation 2: unanchored".into()],
        };
        fs::write(
            results_dir.join("JOB-2.json"),
            serde_json::to_vec(&payload_job2).unwrap(),
        )
        .unwrap();

        let config = PipelineRouterConfig::new(&results_dir, &staged_dir, &feedback_dir);

        // First routing pass: JOB-1 destination collides, but JOB-2 must proceed and be routed
        let actions = route_all_pending_results(&config).unwrap();
        assert_eq!(
            actions.len(),
            1,
            "Only JOB-2 should be successfully routed in this cycle"
        );

        // JOB-1 remains in results/ because its destination collided
        assert!(results_dir.join("JOB-1.json").exists());
        // JOB-2 was consumed and removed
        assert!(!results_dir.join("JOB-2.json").exists());

        // JOB-2 was successfully staged and feedback manifest created
        assert!(staged_dir.join("JOB-2.needs-refinement.json").exists());
        assert!(feedback_dir.join("REFINE-JOB-2.toml").exists());

        // Manifest for JOB-1 was NOT orphaned
        assert!(!feedback_dir.join("REFINE-JOB-1.toml").exists());

        // A second cycle should execute cleanly without error
        let actions_cycle_2 = route_all_pending_results(&config).unwrap();
        assert_eq!(actions_cycle_2.len(), 0);
        assert!(results_dir.join("JOB-1.json").exists());
    }

    #[test]
    fn test_manifest_write_failure_propagates_and_preserves_source_result() {
        let temp = tempfile::tempdir().unwrap();
        let results_dir = temp.path().join("results");
        let staged_dir = temp.path().join("staged");
        let feedback_dir = temp.path().join("feedback");
        fs::create_dir_all(&results_dir).unwrap();
        fs::create_dir_all(&staged_dir).unwrap();
        fs::create_dir_all(&feedback_dir).unwrap();

        // Make the target manifest path a directory so writing to it fails with an I/O error
        let blocked_manifest = feedback_dir.join("REFINE-JOB-7.toml");
        fs::create_dir(&blocked_manifest).unwrap();

        let payload = WorkerResultPayload {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".into(),
            job_id: "JOB-7".into(),
            submitted_at_secs: 1000,
            outcome: "pending-owner-review".into(),
            input: "test".into(),
            proposal: Some(WorkerProposal {
                task: "Audit".into(),
                observations: vec!["Obs 7".into()],
                risks: vec![],
                test_plan: vec![],
                recommended_next_step: "Fix".into(),
                rust_draft: None,
            }),
            error: None,
            conformance_flags: vec!["observation 7: unanchored".into()],
        };
        let result_file = results_dir.join("JOB-7.json");
        fs::write(&result_file, serde_json::to_vec(&payload).unwrap()).unwrap();

        let config = PipelineRouterConfig::new(&results_dir, &staged_dir, &feedback_dir);

        // Routing must fail with Err
        let res = route_all_pending_results(&config);
        assert!(
            res.is_err(),
            "Real I/O errors when writing manifests must propagate"
        );

        // The source result must NOT have been deleted
        assert!(
            result_file.exists(),
            "Source result must be preserved on manifest write failure"
        );

        // The staged file must NOT be left orphaned
        let staged_file = staged_dir.join("JOB-7.needs-refinement.json");
        assert!(
            !staged_file.exists(),
            "Staged record must be cleaned up if manifest write fails"
        );
    }
}
