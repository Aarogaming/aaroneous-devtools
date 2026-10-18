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

/// Validates that a job identifier is bounded and free of path traversal sequences.
pub fn is_valid_job_id(job_id: &str) -> bool {
    !job_id.is_empty()
        && job_id.len() <= 128
        && !job_id.contains("..")
        && job_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// Copies a file using create_new semantics to prevent overwriting existing staged artifacts.
fn copy_create_new(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    let mut reader =
        fs::File::open(src).with_context(|| format!("Opening source file: {}", src.display()))?;
    let mut writer = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)
        .with_context(|| format!("Creating new staged file: {}", dst.display()))?;
    std::io::copy(&mut reader, &mut writer)?;
    Ok(())
}

/// Writes content to a file using create_new semantics.
fn write_create_new(dst: &std::path::Path, content: &str) -> Result<()> {
    use std::io::Write;
    let mut writer = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)
        .with_context(|| format!("Creating new manifest file: {}", dst.display()))?;
    writer.write_all(content.as_bytes())?;
    Ok(())
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
                copy_create_new(&path, &dest)?;
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
                write_create_new(&manifest_path, &feedback_manifest)?;

                // Move consumed result into staged feedback record with create_new semantics
                let dest = config
                    .staged_dir
                    .join(format!("{}.needs-refinement.json", job_id));
                copy_create_new(&path, &dest)?;
                fs::remove_file(&path)?;
            }
            RoutingAction::RecordTelemetry { job_id } => {
                let dest = config.staged_dir.join(format!("{}.telemetry.json", job_id));
                copy_create_new(&path, &dest)?;
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
            res.is_err(),
            "create_new must error on existing destination"
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
}
