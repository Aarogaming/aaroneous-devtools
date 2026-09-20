//! Batch-level scope-overlap check for freshly-generated candidate packets
//! (M43, corrected scope — see `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`).
//!
//! `local_worker::scope_reserved` already checks one job's declared evidence
//! against `governance/COORDINATION_QUEUE.md`'s active rows before dispatch.
//! What is missing is a check *within* a single batch: `frontier_discovery`
//! and `capability_composition` can each emit several candidate `.toml`
//! packets per cycle, and nothing today checks whether two of those
//! candidates would overlap each other's declared evidence before any of
//! them are written to `worker-jobs/`.
//!
//! This module is read-only and report-only: it never modifies, moves, or
//! deletes the candidate files it reads, and it never touches
//! `COORDINATION_QUEUE.md`. It only reports which candidate pairs overlap,
//! so a batch can be trimmed to non-crossing candidates by a human or a
//! later, separate step before anything lands in `worker-jobs/`.

use crate::local_worker::scope_matches_path;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A minimal candidate packet: just the fields relevant to overlap
/// detection. This mirrors the shape `create_frontier_job` /
/// `create_composition_job` already write (`schema_version, id, mode,
/// evidence, instruction`) but only deserializes what this check needs;
/// unknown fields (`mode`, `instruction`, ...) are ignored rather than
/// rejected, since this module's job is read-only comparison, not schema
/// enforcement of the packet format (that already happens where the
/// packets are consumed for dispatch).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CandidatePacket {
    pub id: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

/// One overlapping pair found in a batch, plus the evidence entries that
/// caused the overlap (as matched pairs: an entry from `a`'s evidence and
/// the entry from `b`'s evidence it overlapped with, which may be equal or
/// may be a directory-glob relationship such as `crates/paths/**` against
/// `crates/paths/src/lib.rs`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlapPair {
    pub a_id: String,
    pub b_id: String,
    pub shared_evidence: Vec<(String, String)>,
}

/// Report written by [`check_directory`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlapReport {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub candidates_checked: usize,
    pub overlaps: Vec<OverlapPair>,
}

/// Returns every evidence-entry pair between `a` and `b` that overlap,
/// using the same exact-path/directory-glob semantics as
/// `local_worker::scope_reserved` (via the shared
/// [`scope_matches_path`](crate::local_worker::scope_matches_path)
/// primitive). Checks both directions, since either candidate's evidence
/// entry may itself be the directory-glob (e.g. `crates/paths/**`).
fn evidence_overlap(a: &[String], b: &[String]) -> Vec<(String, String)> {
    let mut hits = Vec::new();
    for x in a {
        for y in b {
            if scope_matches_path(x, y) || scope_matches_path(y, x) {
                hits.push((x.clone(), y.clone()));
            }
        }
    }
    hits
}

/// Computes all overlapping pairs among a batch of candidate packets.
/// Returns pairs in the order candidates were provided, each pair listed
/// once (`a` before `b` in the input slice).
pub fn find_overlaps(candidates: &[CandidatePacket]) -> Vec<OverlapPair> {
    let mut overlaps = Vec::new();
    for i in 0..candidates.len() {
        for j in (i + 1)..candidates.len() {
            let shared = evidence_overlap(&candidates[i].evidence, &candidates[j].evidence);
            if !shared.is_empty() {
                overlaps.push(OverlapPair {
                    a_id: candidates[i].id.clone(),
                    b_id: candidates[j].id.clone(),
                    shared_evidence: shared,
                });
            }
        }
    }
    overlaps
}

fn load_candidates(candidates_dir: &Path) -> Result<Vec<CandidatePacket>> {
    let mut candidates = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(candidates_dir)
        .with_context(|| format!("reading candidates directory {}", candidates_dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            bail!("symlink candidate forbidden: {}", path.display());
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading candidate {}", path.display()))?;
        let candidate: CandidatePacket = toml::from_str(&text)
            .with_context(|| format!("parsing candidate {}", path.display()))?;
        candidates.push(candidate);
    }
    Ok(candidates)
}

/// Reads every `.toml` candidate packet in `candidates_dir`, computes
/// pairwise evidence overlaps, and writes a JSON report to `output`.
/// Read-only over `candidates_dir`: never writes, moves, or deletes any
/// candidate file.
pub fn check_directory(candidates_dir: &Path, output: &Path) -> Result<()> {
    crate::require_absolute_directory("candidates-dir", candidates_dir)?;
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

    let candidates = load_candidates(candidates_dir)?;
    let overlaps = find_overlaps(&candidates);
    let report = OverlapReport {
        schema_version: 1,
        artifact_kind: "batch-overlap-report".to_string(),
        candidates_checked: candidates.len(),
        overlaps,
    };
    let data = serde_json::to_vec_pretty(&report)?;
    fs::write(output, data)
        .with_context(|| format!("writing overlap report to {}", output.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, evidence: &[&str]) -> CandidatePacket {
        CandidatePacket {
            id: id.to_string(),
            evidence: evidence.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn identical_evidence_paths_are_flagged() {
        let candidates = vec![
            candidate("A", &["crates/foo/src/lib.rs"]),
            candidate("B", &["crates/foo/src/lib.rs"]),
        ];
        let overlaps = find_overlaps(&candidates);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].a_id, "A");
        assert_eq!(overlaps[0].b_id, "B");
        assert_eq!(
            overlaps[0].shared_evidence,
            vec![(
                "crates/foo/src/lib.rs".to_string(),
                "crates/foo/src/lib.rs".to_string()
            )]
        );
    }

    #[test]
    fn disjoint_evidence_is_not_flagged() {
        let candidates = vec![
            candidate("A", &["crates/foo/src/lib.rs"]),
            candidate("B", &["crates/bar/src/lib.rs"]),
        ];
        assert!(find_overlaps(&candidates).is_empty());
    }

    #[test]
    fn evidence_under_directory_glob_scope_is_flagged() {
        let candidates = vec![
            candidate("A", &["crates/paths/**"]),
            candidate("B", &["crates/paths/src/lib.rs"]),
        ];
        let overlaps = find_overlaps(&candidates);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].a_id, "A");
        assert_eq!(overlaps[0].b_id, "B");
    }

    #[test]
    fn three_candidates_report_only_overlapping_pairs() {
        let candidates = vec![
            candidate("A", &["crates/foo/src/lib.rs"]),
            candidate("B", &["crates/foo/src/lib.rs"]),
            candidate("C", &["crates/unrelated/src/lib.rs"]),
        ];
        let overlaps = find_overlaps(&candidates);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].a_id, "A");
        assert_eq!(overlaps[0].b_id, "B");
    }

    #[test]
    fn check_directory_is_read_only_and_writes_report() {
        let dir = tempfile::tempdir().expect("dir");
        let candidates_dir = dir.path().join("candidates");
        fs::create_dir(&candidates_dir).expect("candidates dir");
        fs::write(
            candidates_dir.join("a.toml"),
            r#"schema_version = 1
id = "A"
mode = "analysis"
evidence = ["crates/foo/src/lib.rs"]
instruction = "look at foo"
"#,
        )
        .expect("write a");
        fs::write(
            candidates_dir.join("b.toml"),
            r#"schema_version = 1
id = "B"
mode = "analysis"
evidence = ["crates/foo/src/lib.rs"]
instruction = "also look at foo"
"#,
        )
        .expect("write b");
        let output = dir.path().join("report.json");

        check_directory(&candidates_dir, &output).expect("check");

        let contents_a_before =
            fs::read_to_string(candidates_dir.join("a.toml")).expect("read a again");
        assert!(contents_a_before.contains("id = \"A\""));
        let mut entries: Vec<_> = fs::read_dir(&candidates_dir)
            .expect("list candidates dir")
            .map(|e| e.expect("entry").file_name())
            .collect();
        entries.sort();
        assert_eq!(entries, vec!["a.toml", "b.toml"]);

        let report: OverlapReport =
            serde_json::from_str(&fs::read_to_string(&output).expect("read report"))
                .expect("parse report");
        assert_eq!(report.candidates_checked, 2);
        assert_eq!(report.overlaps.len(), 1);
        assert_eq!(report.overlaps[0].a_id, "A");
        assert_eq!(report.overlaps[0].b_id, "B");
    }
}
