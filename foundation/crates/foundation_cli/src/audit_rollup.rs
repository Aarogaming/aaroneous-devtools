use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct DraftSummary {
    task: String,
    recommended_next_step: String,
    observations: Vec<String>,
    risks: Vec<String>,
    test_plan: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct AuditRecord {
    job_id: String,
    submitted_at_secs: u64,
    outcome: String,
    proposal: Option<DraftSummary>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InvariantSummary {
    pub base_key: String,
    pub latest_job_id: String,
    pub submitted_at_secs: u64,
    pub outcome: String,
    pub task: String,
    pub recommendation: String,
    pub observations: Vec<String>,
    pub risks: Vec<String>,
}

fn base_job_key(id: &str) -> &str {
    if let Some(pos) = id.rfind("-P") {
        let suffix = &id[pos + 2..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            return &id[..pos];
        }
    }
    if let Some(pos) = id.rfind("-p") {
        let suffix = &id[pos + 2..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            return &id[..pos];
        }
    }
    id
}

/// Discovers all JSON result records in `dir` matching `AUDIT-*` or `PREP-*`.
fn find_audit_records(dir: &Path) -> Result<Vec<AuditRecord>> {
    let mut records = Vec::new();
    if !dir.exists() {
        return Ok(records);
    }

    let entries = fs::read_dir(dir).with_context(|| format!("Failed to read {}", dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if filename.starts_with("AUDIT-") || filename.starts_with("PREP-") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(record) = serde_json::from_str::<AuditRecord>(&content) {
                        records.push(record);
                    }
                }
            }
        }
    }

    Ok(records)
}

/// Generates a structured markdown audit status document from all discovered audit records.
pub fn generate_audit_rollup(
    results_dir: &Path,
    archive_dir: &Path,
    output_file: &Path,
) -> Result<()> {
    if !results_dir.is_absolute() {
        bail!(
            "results_dir must be an absolute path: {}",
            results_dir.display()
        );
    }
    if !archive_dir.is_absolute() {
        bail!(
            "archive_dir must be an absolute path: {}",
            archive_dir.display()
        );
    }
    if !output_file.is_absolute() {
        bail!(
            "output_file must be an absolute path: {}",
            output_file.display()
        );
    }

    let mut all_records = find_audit_records(results_dir)?;
    all_records.extend(find_audit_records(archive_dir)?);

    // Group by base_key, keeping the record with the largest submitted_at_secs
    let mut latest_by_family: BTreeMap<String, AuditRecord> = BTreeMap::new();
    for rec in all_records {
        let family = base_job_key(&rec.job_id).to_string();
        let replace = match latest_by_family.get(&family) {
            Some(existing) => rec.submitted_at_secs >= existing.submitted_at_secs,
            None => true,
        };
        if replace {
            latest_by_family.insert(family, rec);
        }
    }

    let mut summaries: Vec<InvariantSummary> = Vec::new();
    for (family, rec) in latest_by_family {
        let (task, recommendation, observations, risks) = if let Some(ref d) = rec.proposal {
            (
                d.task.clone(),
                d.recommended_next_step.clone(),
                d.observations.clone(),
                d.risks.clone(),
            )
        } else {
            (
                rec.job_id.clone(),
                rec.error
                    .clone()
                    .unwrap_or_else(|| "No proposal recorded".into()),
                vec![],
                vec![],
            )
        };

        summaries.push(InvariantSummary {
            base_key: family,
            latest_job_id: rec.job_id,
            submitted_at_secs: rec.submitted_at_secs,
            outcome: rec.outcome,
            task,
            recommendation,
            observations,
            risks,
        });
    }

    let mut md = String::new();
    md.push_str("# Continuous Invariant Audit Status\n\n");
    md.push_str("> **STATUS**: AUTOMATED INVARIANT AUDIT ROLLUP\n");
    md.push_str("> **PURPOSE**: Aggregates continuous invariant and architectural audits performed by the local GPU worker (`qwen3.5:9b-q6`) across Aaroneous and devtools.\n");
    md.push_str("> **CONSTITUTION**: All findings are advisory proposals (`pending-owner-review`). Continuous audits monitor codebase health and verify compliance against AGENTS.md.\n\n");

    md.push_str("## Executive Invariant Matrix\n\n");
    md.push_str("| Invariant Family | Latest Pass | Status | Submitted | Executive Summary |\n");
    md.push_str("| :--- | :--- | :--- | :--- | :--- |\n");

    if summaries.is_empty() {
        md.push_str("| *None* | *None* | *Idle* | *N/A* | No audit records found in results or archive. |\n");
    } else {
        for s in &summaries {
            let rec_clean = s.recommendation.replace('|', "\\|").replace('\n', " ");
            let short_rec = if rec_clean.len() > 100 {
                format!("{}...", &rec_clean[..97])
            } else {
                rec_clean
            };
            md.push_str(&format!(
                "| **{}** | `{}` | `{}` | unix `{}` | {} |\n",
                s.base_key, s.latest_job_id, s.outcome, s.submitted_at_secs, short_rec
            ));
        }
    }

    md.push_str("\n---\n\n## Detailed Invariant Findings\n\n");
    for s in &summaries {
        md.push_str(&format!("### {}\n\n", s.base_key));
        md.push_str(&format!("- **Task Description**: {}\n", s.task));
        md.push_str(&format!("- **Latest Job Pass**: `{}`\n", s.latest_job_id));
        md.push_str(&format!("- **Review Status**: `{}`\n", s.outcome));
        md.push_str(&format!(
            "- **Timestamp**: unix `{}`\n\n",
            s.submitted_at_secs
        ));

        md.push_str("#### Recommended Next Step\n");
        md.push_str(&format!("{}\n\n", s.recommendation));

        if !s.observations.is_empty() {
            md.push_str("#### Key Observations\n");
            for obs in &s.observations {
                md.push_str(&format!("- {}\n", obs));
            }
            md.push('\n');
        }

        if !s.risks.is_empty() {
            md.push_str("#### Identified Risks\n");
            for r in &s.risks {
                md.push_str(&format!("- {}\n", r));
            }
            md.push('\n');
        }

        md.push_str("---\n\n");
    }

    if let Some(parent) = output_file.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let staging = output_file.with_extension("partial");
    fs::write(&staging, md.as_bytes())
        .with_context(|| format!("Failed to write staging file {}", staging.display()))?;
    fs::rename(&staging, output_file).with_context(|| {
        format!(
            "Failed to rename {} to {}",
            staging.display(),
            output_file.display()
        )
    })?;

    Ok(())
}

/// CLI entrypoint for `foundation audit-rollup`.
pub fn run_cli(results_dir: &Path, archive_dir: &Path, output: &Path) -> Result<()> {
    generate_audit_rollup(results_dir, archive_dir, output)?;
    println!("Audit status rollup generated at {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_base_job_key() {
        assert_eq!(
            base_job_key("AUDIT-01-AST-SAFETY-P0178"),
            "AUDIT-01-AST-SAFETY"
        );
        assert_eq!(
            base_job_key("AUDIT-02-HOT-PATH-ALLOC-p0001"),
            "AUDIT-02-HOT-PATH-ALLOC"
        );
        assert_eq!(
            base_job_key("PREP-02-COMPUTE-ENGINE"),
            "PREP-02-COMPUTE-ENGINE"
        );
    }

    #[test]
    fn test_generate_audit_rollup_with_records() -> Result<()> {
        let dir = tempdir()?;
        let results_dir = dir.path().join("results");
        let archive_dir = dir.path().join("archive");
        let output_file = dir.path().join("INVARIANT_AUDIT_STATUS.md");

        fs::create_dir_all(&results_dir)?;
        fs::create_dir_all(&archive_dir)?;

        let rec1 = serde_json::json!({
            "job_id": "AUDIT-01-AST-SAFETY-P0001",
            "submitted_at_secs": 100,
            "outcome": "pending-owner-review",
            "proposal": {
                "task": "AST Safety Audit",
                "recommended_next_step": "Maintain zero unsafe code in domain crates.",
                "observations": ["crates/api has deny(unsafe_code)"],
                "risks": ["Potential macro bypass"],
                "test_plan": ["cargo test"]
            }
        });
        fs::write(
            results_dir.join("AUDIT-01-AST-SAFETY-P0001-0000.json"),
            serde_json::to_string(&rec1)?,
        )?;

        generate_audit_rollup(&results_dir, &archive_dir, &output_file)?;

        assert!(output_file.exists());
        let content = fs::read_to_string(&output_file)?;
        assert!(content.contains("AUDIT-01-AST-SAFETY"));
        assert!(content.contains("Maintain zero unsafe code in domain crates."));

        Ok(())
    }
}
