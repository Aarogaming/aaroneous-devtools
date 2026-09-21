//! M60 Discovery Digest Generator.
//!
//! Aggregates completed local-worker internal capability compositions (`COMPOSITION-*`)
//! and external open-source frontier evaluations (`FRONTIER-*`) into a unified
//! executive dashboard (`governance/DISCOVERY_DIGEST.md`).

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryKind {
    InternalComposition,
    ExternalFrontier,
}

#[derive(Debug, Clone)]
pub struct DiscoveryRecord {
    pub job_id: String,
    pub kind: DiscoveryKind,
    pub submitted_at_secs: u64,
    pub outcome: String,
    pub observations: Vec<String>,
    pub risks: Vec<String>,
    pub recommended_next_step: String,
}

impl DiscoveryRecord {
    pub fn parse(path: &Path) -> Option<Self> {
        let text = fs::read_to_string(path).ok()?;
        let val: Value = serde_json::from_str(&text).ok()?;
        let job_id = val.get("job_id")?.as_str()?.to_string();

        let kind = if job_id.starts_with("COMPOSITION-") {
            DiscoveryKind::InternalComposition
        } else if job_id.starts_with("FRONTIER-") {
            DiscoveryKind::ExternalFrontier
        } else {
            return None;
        };

        let submitted_at_secs = val
            .get("submitted_at_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let outcome = val
            .get("outcome")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let mut observations = Vec::new();
        let mut risks = Vec::new();
        let mut recommended_next_step = String::new();

        if let Some(proposal) = val.get("proposal").and_then(|p| p.as_object()) {
            if let Some(obs) = proposal.get("observations").and_then(|v| v.as_array()) {
                for o in obs.iter().filter_map(|x| x.as_str()) {
                    observations.push(o.to_string());
                }
            }
            if let Some(r) = proposal.get("risks").and_then(|v| v.as_array()) {
                for item in r.iter().filter_map(|x| x.as_str()) {
                    risks.push(item.to_string());
                }
            }
            if let Some(step) = proposal
                .get("recommended_next_step")
                .and_then(|s| s.as_str())
            {
                recommended_next_step = step.to_string();
            }
        }

        Some(Self {
            job_id,
            kind,
            submitted_at_secs,
            outcome,
            observations,
            risks,
            recommended_next_step,
        })
    }
}

pub fn collect_discovery_records(
    results_dir: &Path,
    archive_dir: Option<&Path>,
) -> Result<Vec<DiscoveryRecord>> {
    let mut dirs = vec![results_dir.to_path_buf()];
    if let Some(a) = archive_dir {
        dirs.push(a.to_path_buf());
    }

    let mut latest_by_id: BTreeMap<String, DiscoveryRecord> = BTreeMap::new();

    for dir in dirs {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(rec) = DiscoveryRecord::parse(&path) {
                        let existing = latest_by_id.get(&rec.job_id);
                        if existing.is_none_or(|e| rec.submitted_at_secs >= e.submitted_at_secs) {
                            latest_by_id.insert(rec.job_id.clone(), rec);
                        }
                    }
                }
            }
        }
    }

    Ok(latest_by_id.into_values().collect())
}

pub fn generate_discovery_markdown(records: &[DiscoveryRecord]) -> String {
    let mut md = String::new();
    md.push_str("# Discovery & Architectural Composition Digest\n\n");
    md.push_str("> **STATUS**: AUTOMATED LOCAL-MODEL DISCOVERY SYNTHESIS\n");
    md.push_str("> **PURPOSE**: Aggregates continuous internal capability compositions (`COMPOSITION-*`) and external frontier evaluations (`FRONTIER-*`) scored against `governance/BUILD_WORTHINESS_STANDARD.md`.\n");
    md.push_str("> **GOVERNANCE NOTICE**: These records are speculative proposals (`pending-owner-review`). They do NOT claim coordination queue rows or authorize product writes.\n\n");

    let total = records.len();
    let comp_count = records
        .iter()
        .filter(|r| r.kind == DiscoveryKind::InternalComposition)
        .count();
    let frontier_count = records
        .iter()
        .filter(|r| r.kind == DiscoveryKind::ExternalFrontier)
        .count();
    let pending_count = records
        .iter()
        .filter(|r| r.outcome == "pending-owner-review")
        .count();

    md.push_str("## Executive Summary\n\n");
    md.push_str("| Metric | Value |\n");
    md.push_str("| :--- | :--- |\n");
    md.push_str(&format!("| **Total Candidates Evaluated** | {total} |\n"));
    md.push_str(&format!(
        "| **Internal Capability Compositions** | {comp_count} |\n"
    ));
    md.push_str(&format!(
        "| **External Frontier Evaluations** | {frontier_count} |\n"
    ));
    md.push_str(&format!(
        "| **Awaiting Owner Review** | {pending_count} |\n\n"
    ));

    // Internal Capability Compositions Table
    md.push_str("## Internal Capability Composition Matrix\n\n");
    md.push_str("| Composition Pair | Observations & Compatibility | Key Risks | Recommended Action | Status |\n");
    md.push_str("| :--- | :--- | :--- | :--- | :--- |\n");

    let mut comp_rows = 0;
    for r in records
        .iter()
        .filter(|r| r.kind == DiscoveryKind::InternalComposition)
    {
        comp_rows += 1;
        let display_id = r
            .job_id
            .strip_prefix("COMPOSITION-")
            .unwrap_or(&r.job_id)
            .replace('_', " ");
        let obs = if r.observations.is_empty() {
            "No observations".to_string()
        } else {
            r.observations[0].replace('|', "\\|").replace('\n', " ")
        };
        let risks = if r.risks.is_empty() {
            "None cited".to_string()
        } else {
            r.risks[0].replace('|', "\\|").replace('\n', " ")
        };
        let action = if r.recommended_next_step.is_empty() {
            "Awaiting review".to_string()
        } else {
            r.recommended_next_step
                .replace('|', "\\|")
                .replace('\n', " ")
        };

        md.push_str(&format!(
            "| **{display_id}** | {obs} | {risks} | {action} | `{}` |\n",
            r.outcome
        ));
    }
    if comp_rows == 0 {
        md.push_str("| *None evaluated yet* | - | - | - | - |\n");
    }
    md.push('\n');

    // External Frontier Candidates Table
    md.push_str("## External Frontier Candidate Matrix\n\n");
    md.push_str(
        "| Crate Candidate | Architectural Analysis | Trade-offs & Risks | Recommendation | Status |\n",
    );
    md.push_str("| :--- | :--- | :--- | :--- | :--- |\n");

    let mut frontier_rows = 0;
    for r in records
        .iter()
        .filter(|r| r.kind == DiscoveryKind::ExternalFrontier)
    {
        frontier_rows += 1;
        let display_id = r
            .job_id
            .strip_prefix("FRONTIER-")
            .unwrap_or(&r.job_id)
            .to_lowercase();
        let obs = if r.observations.is_empty() {
            "No observations".to_string()
        } else {
            r.observations[0].replace('|', "\\|").replace('\n', " ")
        };
        let risks = if r.risks.is_empty() {
            "None cited".to_string()
        } else {
            r.risks[0].replace('|', "\\|").replace('\n', " ")
        };
        let action = if r.recommended_next_step.is_empty() {
            "Awaiting review".to_string()
        } else {
            r.recommended_next_step
                .replace('|', "\\|")
                .replace('\n', " ")
        };

        md.push_str(&format!(
            "| **`{display_id}`** | {obs} | {risks} | {action} | `{}` |\n",
            r.outcome
        ));
    }
    if frontier_rows == 0 {
        md.push_str("| *None evaluated yet* | - | - | - | - |\n");
    }
    md.push('\n');

    md
}

pub fn run(results_dir: &Path, archive_dir: Option<&Path>, output_path: &Path) -> Result<()> {
    if !results_dir.is_absolute() || !output_path.is_absolute() {
        bail!("results_dir and output must be absolute paths");
    }
    if let Some(a) = archive_dir {
        if !a.is_absolute() {
            bail!("archive_dir must be an absolute path");
        }
    }

    let records = collect_discovery_records(results_dir, archive_dir)?;
    let content = generate_discovery_markdown(&records);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, content).with_context(|| {
        format!(
            "failed writing discovery digest to {}",
            output_path.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_and_generates_discovery_digest() {
        let temp = tempdir().unwrap();
        let results = temp.path().join("results");
        fs::create_dir_all(&results).unwrap();

        let comp_json = serde_json::json!({
            "schema_version": 1,
            "job_id": "COMPOSITION-A-B",
            "submitted_at_secs": 100,
            "outcome": "pending-owner-review",
            "proposal": {
                "observations": ["Comp observation"],
                "risks": ["Comp risk"],
                "recommended_next_step": "Comp next step"
            }
        });
        fs::write(
            results.join("COMPOSITION-A-B-0000.json"),
            comp_json.to_string(),
        )
        .unwrap();

        let frontier_json = serde_json::json!({
            "schema_version": 1,
            "job_id": "FRONTIER-ICED",
            "submitted_at_secs": 101,
            "outcome": "pending-owner-review",
            "proposal": {
                "observations": ["Frontier observation"],
                "risks": ["Frontier risk"],
                "recommended_next_step": "Frontier next step"
            }
        });
        fs::write(
            results.join("FRONTIER-ICED-0000.json"),
            frontier_json.to_string(),
        )
        .unwrap();

        let out = temp.path().join("DISCOVERY_DIGEST.md");
        run(&results, None, &out).unwrap();

        assert!(out.exists());
        let content = fs::read_to_string(&out).unwrap();
        assert!(content.contains("A-B"));
        assert!(content.contains("`iced`"));
        assert!(content.contains("Comp next step"));
        assert!(content.contains("Frontier next step"));
    }
}
