use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::local_worker::{Record, SolutionBranch};

/// A structured record of an architectural branch decision made during
/// orchestration or human review. Captures the original prompt context, all
/// competing solution branches, the chosen branch, the rejected branches, and the
/// selection rationale.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct DecisionTrace {
    pub(crate) schema_version: u32,
    pub(crate) job_id: String,
    pub(crate) timestamp: u64,
    pub(crate) prompt: String,
    pub(crate) candidate_branches: Vec<SolutionBranch>,
    pub(crate) chosen_branch: SolutionBranch,
    pub(crate) rejected_branches: Vec<SolutionBranch>,
    pub(crate) selection_rationale: String,
}

/// Standard ChatML message for Supervised Fine-Tuning (SFT).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct ChatMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

/// A complete SFT conversation sample ready for fine-tuning.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct SftChatExample {
    pub(crate) messages: Vec<ChatMessage>,
}

/// Direct Preference Optimization (DPO) pairwise training sample.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub(crate) struct DpoPreferencePair {
    pub(crate) prompt: String,
    pub(crate) chosen: String,
    pub(crate) rejected: String,
    pub(crate) rationale: String,
}

/// Record a maintainer or orchestrator choice from a solution tree, update
/// the job result in place, and append structured training traces.
pub(crate) fn record_branch_selection(
    results_dir: &Path,
    job_id: &str,
    branch_name: &str,
    rationale: &str,
    traces_dir: &Path,
) -> Result<DecisionTrace> {
    if !results_dir.is_dir() {
        bail!("results_dir does not exist: {}", results_dir.display());
    }
    fs::create_dir_all(traces_dir)
        .with_context(|| format!("failed to create traces directory {}", traces_dir.display()))?;

    // Locate the matching result record in results_dir
    let normalized_job_id = job_id.to_uppercase();
    let entries = fs::read_dir(results_dir)
        .with_context(|| format!("failed to read results dir {}", results_dir.display()))?;

    let mut matched_path = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let fname = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_uppercase();
            if fname.starts_with(&normalized_job_id) {
                matched_path = Some(path);
                break;
            }
        }
    }

    let record_path = matched_path.with_context(|| {
        format!(
            "no result record found for job id '{job_id}' in {}",
            results_dir.display()
        )
    })?;

    let content = fs::read_to_string(&record_path)
        .with_context(|| format!("failed to read record {}", record_path.display()))?;
    let mut record: Record = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse record JSON in {}", record_path.display()))?;

    let proposal = record
        .proposal
        .as_ref()
        .context("record has no generated proposal")?;

    let branches = proposal
        .solution_tree
        .as_ref()
        .context("proposal contains no solution_tree branches to select from")?;

    if branches.is_empty() {
        bail!("solution_tree in record '{}' is empty", record.job_id);
    }

    let chosen_idx = branches
        .iter()
        .position(|b| b.branch_name.eq_ignore_ascii_case(branch_name))
        .with_context(|| {
            let available: Vec<_> = branches.iter().map(|b| b.branch_name.as_str()).collect();
            format!(
                "branch '{branch_name}' not found in solution_tree. Available branches: {:?}",
                available
            )
        })?;

    let chosen_branch = branches[chosen_idx].clone();
    let mut rejected_branches = Vec::new();
    for (i, b) in branches.iter().enumerate() {
        if i != chosen_idx {
            rejected_branches.push(b.clone());
        }
    }

    // Update the record in place
    record.selected_branch = Some(chosen_branch.branch_name.clone());
    record.selection_rationale = Some(rationale.to_string());

    let updated_json = serde_json::to_vec_pretty(&record)?;
    fs::write(&record_path, updated_json)?;

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let trace = DecisionTrace {
        schema_version: 1,
        job_id: record.job_id.clone(),
        timestamp: now_secs,
        prompt: record.input.clone(),
        candidate_branches: branches.clone(),
        chosen_branch: chosen_branch.clone(),
        rejected_branches: rejected_branches.clone(),
        selection_rationale: rationale.to_string(),
    };

    // Save individual decision trace
    let trace_file = traces_dir.join(format!("{}-decision.json", record.job_id.to_lowercase()));
    fs::write(&trace_file, serde_json::to_vec_pretty(&trace)?)?;

    // Append to DPO preference pairs (for pairwise alignment training)
    for rejected in &rejected_branches {
        let dpo_pair = DpoPreferencePair {
            prompt: record.input.clone(),
            chosen: format!(
                "Selected Architecture Paradigm: {}\nBranch: {}\nApproach: {}\nTrade-offs: {}\nBlast Radius: {}\n\nCode Sketch:\n{}",
                chosen_branch.architecture_paradigm,
                chosen_branch.branch_name,
                chosen_branch.approach,
                chosen_branch.trade_offs,
                chosen_branch.blast_radius,
                chosen_branch.code_sketch.as_deref().unwrap_or("None")
            ),
            rejected: format!(
                "Selected Architecture Paradigm: {}\nBranch: {}\nApproach: {}\nTrade-offs: {}\nBlast Radius: {}\n\nCode Sketch:\n{}",
                rejected.architecture_paradigm,
                rejected.branch_name,
                rejected.approach,
                rejected.trade_offs,
                rejected.blast_radius,
                rejected.code_sketch.as_deref().unwrap_or("None")
            ),
            rationale: rationale.to_string(),
        };

        let mut dpo_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(traces_dir.join("dpo_preference_pairs.jsonl"))?;
        writeln!(dpo_file, "{}", serde_json::to_string(&dpo_pair)?)?;
    }

    // Append to SFT decision corpus (for supervised positive completion training)
    let sft_example = SftChatExample {
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: record.input.clone(),
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: format!(
                    "Solution Tree Selection: {}\nArchitecture Paradigm: {}\nApproach: {}\nTrade-offs: {}\nBlast Radius: {}\n\nCode Sketch:\n{}",
                    chosen_branch.branch_name,
                    chosen_branch.architecture_paradigm,
                    chosen_branch.approach,
                    chosen_branch.trade_offs,
                    chosen_branch.blast_radius,
                    chosen_branch.code_sketch.as_deref().unwrap_or("None")
                ),
            },
        ],
    };

    let mut sft_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(traces_dir.join("sft_decision_corpus.jsonl"))?;
    writeln!(sft_file, "{}", serde_json::to_string(&sft_example)?)?;

    println!(
        "decision trace recorded for job '{}': selected branch '{}' with {} rejected branch(es)",
        record.job_id,
        chosen_branch.branch_name,
        rejected_branches.len()
    );

    Ok(trace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_worker::{Draft, Record};

    #[test]
    fn test_record_branch_selection_success_and_datasets() {
        let temp = tempfile::tempdir().expect("temp dir");
        let results_dir = temp.path().join("results");
        let traces_dir = temp.path().join("traces");
        fs::create_dir_all(&results_dir).expect("create results");

        let branch_a = SolutionBranch {
            branch_name: "Branch-A-StaticTypestate".to_string(),
            architecture_paradigm: "Compile-Time Parameterization".to_string(),
            approach: "Enforce state machine transitions using phantom typestate wrappers"
                .to_string(),
            blast_radius: "core/hypervisor".to_string(),
            invariant_profile: "Zero-heap, zero-alloc".to_string(),
            trade_offs: "Strict compile-time guarantees, longer compile times".to_string(),
            code_sketch: Some("pub struct Guard<T> { _inner: T }".to_string()),
        };

        let branch_b = SolutionBranch {
            branch_name: "Branch-B-RingBuffer".to_string(),
            architecture_paradigm: "Bounded Circular Buffer".to_string(),
            approach: "Isolate communication across bounded SWMR ring buffer".to_string(),
            blast_radius: "crates/ipc_bus".to_string(),
            invariant_profile: "SWMR lock-free ring".to_string(),
            trade_offs: "Simple runtime bounds, fixed memory footprint".to_string(),
            code_sketch: Some("pub struct SwmrRing<T, const N: usize> { buf: [T; N] }".to_string()),
        };

        let record = Record {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".to_string(),
            job_id: "PORTFOLIO-M8-P0002".to_string(),
            submitted_at_secs: 1000,
            outcome: "pending-owner-review".to_string(),
            input: "Task: Implement Bounded Reducer Reference Path".to_string(),
            proposal: Some(Draft {
                implementation_plan: None,
                task: "PORTFOLIO-M8-P0002".to_string(),
                observations: vec!["Reducer needs bounded execution".to_string()],
                risks: vec!["Interference with hypervisor loop".to_string()],
                test_plan: vec!["Verify deterministic transitions".to_string()],
                recommended_next_step: "Choose bounded architecture".to_string(),
                rust_draft: None,
                solution_tree: Some(vec![branch_a.clone(), branch_b.clone()]),
            }),
            error: None,
            pre_proof_report: None,
            rejection_reason: None,
            selected_branch: None,
            selection_rationale: None,
        };

        let record_file = results_dir.join("PORTFOLIO-M8-P0002-0000.json");
        fs::write(&record_file, serde_json::to_vec_pretty(&record).unwrap()).unwrap();

        let trace = record_branch_selection(
            &results_dir,
            "PORTFOLIO-M8-P0002",
            "Branch-B-RingBuffer",
            "Branch B avoids complex typestates while guaranteeing fixed-memory geometry per AGENTS.md Section 3.",
            &traces_dir,
        )
        .expect("select branch");

        assert_eq!(trace.chosen_branch.branch_name, "Branch-B-RingBuffer");
        assert_eq!(trace.rejected_branches.len(), 1);
        assert_eq!(
            trace.rejected_branches[0].branch_name,
            "Branch-A-StaticTypestate"
        );

        // Verify record on disk was updated
        let updated_text = fs::read_to_string(&record_file).unwrap();
        let updated: Record = serde_json::from_str(&updated_text).unwrap();
        assert_eq!(
            updated.selected_branch.as_deref(),
            Some("Branch-B-RingBuffer")
        );
        assert!(
            updated
                .selection_rationale
                .as_deref()
                .unwrap()
                .contains("Branch B avoids")
        );

        // Verify DPO preference pair was appended
        let dpo_content =
            fs::read_to_string(traces_dir.join("dpo_preference_pairs.jsonl")).unwrap();
        assert!(dpo_content.contains("Branch-B-RingBuffer"));
        assert!(dpo_content.contains("Branch-A-StaticTypestate"));

        // Verify SFT corpus was appended
        let sft_content = fs::read_to_string(traces_dir.join("sft_decision_corpus.jsonl")).unwrap();
        assert!(sft_content.contains("Branch-B-RingBuffer"));
        assert!(sft_content.contains("assistant"));
    }

    #[test]
    fn test_record_branch_selection_unknown_branch_fails_closed() {
        let temp = tempfile::tempdir().expect("temp dir");
        let results_dir = temp.path().join("results");
        let traces_dir = temp.path().join("traces");
        fs::create_dir_all(&results_dir).expect("create results");

        let branch_a = SolutionBranch {
            branch_name: "Branch-A".to_string(),
            architecture_paradigm: "Paradigm A".to_string(),
            approach: "Approach A".to_string(),
            blast_radius: "crate A".to_string(),
            invariant_profile: "inv A".to_string(),
            trade_offs: "trade A".to_string(),
            code_sketch: None,
        };

        let record = Record {
            schema_version: 1,
            artifact_kind: "foundation.local-worker-result".to_string(),
            job_id: "PORTFOLIO-M1-P0002".to_string(),
            submitted_at_secs: 1000,
            outcome: "pending-owner-review".to_string(),
            input: "Task: Converge active branch".to_string(),
            proposal: Some(Draft {
                implementation_plan: None,
                task: "PORTFOLIO-M1-P0002".to_string(),
                observations: vec![],
                risks: vec![],
                test_plan: vec![],
                recommended_next_step: "Pick branch".to_string(),
                rust_draft: None,
                solution_tree: Some(vec![branch_a]),
            }),
            error: None,
            pre_proof_report: None,
            rejection_reason: None,
            selected_branch: None,
            selection_rationale: None,
        };

        fs::write(
            results_dir.join("PORTFOLIO-M1-P0002-0000.json"),
            serde_json::to_vec_pretty(&record).unwrap(),
        )
        .unwrap();

        let err = record_branch_selection(
            &results_dir,
            "PORTFOLIO-M1-P0002",
            "NonExistentBranch",
            "Rationale",
            &traces_dir,
        )
        .expect_err("should fail on unknown branch");

        assert!(err.to_string().contains("not found in solution_tree"));
    }
}
