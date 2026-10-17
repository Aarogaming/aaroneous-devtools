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

        let action = evaluate_result(&payload);

        match &action {
            RoutingAction::StageForIntegration { job_id, .. } => {
                let dest = config.staged_dir.join(format!("{}.staged.json", job_id));
                fs::copy(&path, &dest)?;
                fs::remove_file(&path)?;
            }
            RoutingAction::FeedbackForRefinement {
                job_id,
                evidence,
                issues,
            } => {
                // Generate a feedback job manifest matching strict Job schema
                let ev_str = serde_json::to_string(evidence)
                    .unwrap_or_else(|_| "[\"AGENTS.md\"]".to_string());
                let feedback_manifest = format!(
                    "schema_version = 1\nid = \"REFINE-{}\"\nmode = \"code-draft\"\nevidence = {}\ninstruction = \"Refine previous proposal to address reported conformance issues: {}\"\n",
                    job_id.replace("FEED-", ""),
                    ev_str,
                    issues.join("; ")
                );
                let manifest_path = config
                    .feedback_jobs_dir
                    .join(format!("REFINE-{}.toml", job_id.replace("FEED-", "")));
                fs::write(&manifest_path, feedback_manifest)?;

                // Move consumed result into staged feedback record
                let dest = config
                    .staged_dir
                    .join(format!("{}.needs-refinement.json", job_id));
                fs::copy(&path, &dest)?;
                fs::remove_file(&path)?;
            }
            RoutingAction::RecordTelemetry { job_id } => {
                let dest = config.staged_dir.join(format!("{}.telemetry.json", job_id));
                fs::copy(&path, &dest)?;
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
    }
}
