//! M57 -- Accumulating per-candidate metadata record ("scanning beam"
//! framing, see `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`'s M57
//! section): a per-candidate record that every applicable pass reads before
//! running and appends to after running, never overwritten, never discarded.
//! Over enough passes this becomes a real multi-dimensional representation
//! of a candidate -- functionally equivalent to an embedding, built entirely
//! from mechanical, auditable passes rather than a trained model.
//!
//! This module only adds the accumulation layer. It does not change what any
//! wrapped pass is authorized to decide (M42's read-only guarantee, M44/M55's
//! gate logic, etc. all stay exactly as they were) -- see the milestone's
//! "Explicitly out of scope" note.
//!
//! ## Storage layout
//!
//! Records live under an explicit, caller-supplied absolute `metadata_dir`,
//! alongside the existing `results`/`archive` directory convention
//! `governance/LOCAL_WORKER_SERVICE.md` already governs for this crate's
//! other persistent state -- no new storage concept.
//!
//! - Crate candidates (M55-sourced, keyed by crate name):
//!   `<metadata_dir>/crate/<sanitized-name>.json`
//! - Cluster candidates (M54-sourced, keyed by `"{target_name}/{cluster_dir}"`):
//!   `<metadata_dir>/cluster/<sanitized-target>/<sanitized-cluster>.json`
//!
//! Prefixing by `kind` into separate top-level directories (`crate/` vs.
//! `cluster/`) is what prevents a crate named the same as a cluster's
//! directory path from ever colliding -- the two kinds simply never share a
//! subtree.
//!
//! Writes are atomic: serialize to a `.partial` staging file, then rename
//! over the destination -- the same pattern `local_worker.rs`'s
//! `record_outcome`/`archive` paths and `agentic_registrar.rs` already use
//! for their own result records.
//!
//! ## Staleness / re-check policy
//!
//! `find_recent_pass` is the pure, testable "should a second sweep reuse a
//! prior pass's findings instead of repeating the work" decision -- see its
//! doc comment for the threshold this module defaults to. Kept as a pure
//! function (no I/O, no network) precisely so it's unit-testable without a
//! live network call, per this milestone's evidence requirement.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A single pass's contribution to a candidate's accumulated record.
/// `findings` is deliberately loosely typed (`serde_json::Value`) -- each
/// pass owns its own findings shape, no shared schema is forced across
/// passes (mirrors this milestone's design note that a record is a
/// collection of named, explainable fields, not one rigid struct).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PassEntry {
    pub pass_name: String,
    pub recorded_at_secs: u64,
    pub findings: serde_json::Value,
}

/// The accumulating per-candidate metadata record. `passes` is append-only:
/// nothing in this module ever removes or overwrites a prior entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateRecord {
    pub schema_version: u32,
    pub candidate_id: String,
    /// `"crate"` (M55-sourced) or `"cluster"` (M54-sourced).
    pub kind: String,
    pub created_at_secs: u64,
    pub updated_at_secs: u64,
    pub passes: Vec<PassEntry>,
}

const SCHEMA_VERSION: u32 = 1;

/// A field older than this is treated as needing re-verification on the
/// next sweep, not trusted forever -- per the milestone's "reuse the
/// existing `EVIDENCE_STANDARD.md` discipline of dated, evidenced claims"
/// design note. 24 hours: long enough that a `frontier-discover-serve` loop
/// running on its default 30-minute interval won't re-hit crates.io for the
/// same candidate on every single cycle, short enough that a license or
/// dependency change upstream gets re-checked well within a day. Not
/// sophisticated, just real and documented, per the milestone's own framing.
pub const PREFILTER_STALE_AFTER_SECS: u64 = 24 * 60 * 60;

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn sanitize_component(raw: &str) -> String {
    let sanitized: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let lowered = sanitized.to_lowercase();
    if lowered.is_empty() {
        "_".to_string()
    } else {
        lowered
    }
}

/// Derives the on-disk path for a candidate's record from `kind` +
/// `candidate_id`. See module docs for the layout and why `kind`-prefixing
/// prevents a crate/cluster name collision.
fn record_path(metadata_dir: &Path, kind: &str, candidate_id: &str) -> Result<PathBuf> {
    match kind {
        "crate" => Ok(metadata_dir
            .join("crate")
            .join(format!("{}.json", sanitize_component(candidate_id)))),
        "cluster" => {
            let (target, cluster_dir) = candidate_id.split_once('/').with_context(|| {
                format!(
                    "cluster candidate_id must be formatted '{{target_name}}/{{cluster_dir}}', got: {candidate_id}"
                )
            })?;
            Ok(metadata_dir
                .join("cluster")
                .join(sanitize_component(target))
                .join(format!("{}.json", sanitize_component(cluster_dir))))
        }
        other => bail!("unknown candidate kind '{other}' -- expected 'crate' or 'cluster'"),
    }
}

/// Reads a candidate's accumulated record. `Ok(None)` (not an error) means
/// no record exists yet -- the normal, expected shape for a candidate's
/// first-ever sweep.
pub fn read_record(
    metadata_dir: &Path,
    kind: &str,
    candidate_id: &str,
) -> Result<Option<CandidateRecord>> {
    if !metadata_dir.is_absolute() {
        bail!(
            "metadata_dir must be an absolute path: {}",
            metadata_dir.display()
        );
    }
    let path = record_path(metadata_dir, kind, candidate_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "failed to read candidate metadata record: {}",
            path.display()
        )
    })?;
    let record: CandidateRecord = serde_json::from_str(&text).with_context(|| {
        format!(
            "failed to parse candidate metadata record: {}",
            path.display()
        )
    })?;
    Ok(Some(record))
}

fn write_record_atomic(path: &Path, record: &CandidateRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("failed to create metadata directory: {}", parent.display())
        })?;
    }
    let data = serde_json::to_vec_pretty(record)
        .context("failed to serialize candidate metadata record")?;
    let staging = path.with_extension("partial");
    std::fs::write(&staging, &data).with_context(|| {
        format!(
            "failed to write staging metadata record: {}",
            staging.display()
        )
    })?;
    std::fs::rename(&staging, path)
        .with_context(|| format!("failed to publish metadata record: {}", path.display()))?;
    Ok(())
}

/// Reads any existing record for this candidate (or creates a fresh one if
/// none exists), pushes a new `PassEntry` onto `passes` (never removing or
/// overwriting prior entries), updates `updated_at_secs`, writes atomically,
/// and returns the updated record.
pub fn append_pass(
    metadata_dir: &Path,
    kind: &str,
    candidate_id: &str,
    pass_name: &str,
    findings: serde_json::Value,
) -> Result<CandidateRecord> {
    if !metadata_dir.is_absolute() {
        bail!(
            "metadata_dir must be an absolute path: {}",
            metadata_dir.display()
        );
    }
    let now = now_secs();
    let mut record =
        read_record(metadata_dir, kind, candidate_id)?.unwrap_or_else(|| CandidateRecord {
            schema_version: SCHEMA_VERSION,
            candidate_id: candidate_id.to_string(),
            kind: kind.to_string(),
            created_at_secs: now,
            updated_at_secs: now,
            passes: Vec::new(),
        });

    record.passes.push(PassEntry {
        pass_name: pass_name.to_string(),
        recorded_at_secs: now,
        findings,
    });
    record.updated_at_secs = now;

    let path = record_path(metadata_dir, kind, candidate_id)?;
    write_record_atomic(&path, &record)?;
    Ok(record)
}

/// The "intelligence" half: given a record (if one exists) and the current
/// time, decides whether a named pass's most recent recorded entry is still
/// fresh enough to reuse instead of repeating the underlying work. Pure and
/// I/O-free by design -- see module docs -- so it's unit-testable without a
/// live network call. Returns the most recent matching, still-fresh entry
/// when the caller should skip; `None` means "run the pass for real."
pub fn find_recent_pass<'a>(
    record: &'a CandidateRecord,
    pass_name: &str,
    now: u64,
    max_age_secs: u64,
) -> Option<&'a PassEntry> {
    record.passes.iter().rev().find(|p| {
        p.pass_name == pass_name && now.saturating_sub(p.recorded_at_secs) <= max_age_secs
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // (a) append_pass on a fresh candidate creates a new record with one
    // pass entry.
    #[test]
    fn append_pass_on_fresh_candidate_creates_record_with_one_entry() {
        let temp = tempdir().unwrap();
        let record = append_pass(
            temp.path(),
            "crate",
            "cranelift-codegen",
            "mechanical_prefilter",
            serde_json::json!({"license": "MIT", "license_gate": true}),
        )
        .unwrap();

        assert_eq!(record.candidate_id, "cranelift-codegen");
        assert_eq!(record.kind, "crate");
        assert_eq!(record.schema_version, 1);
        assert_eq!(record.passes.len(), 1);
        assert_eq!(record.passes[0].pass_name, "mechanical_prefilter");
    }

    // (b) a second append_pass call on the same candidate adds a second
    // entry without removing the first.
    #[test]
    fn second_append_pass_adds_entry_without_removing_first() {
        let temp = tempdir().unwrap();
        append_pass(
            temp.path(),
            "crate",
            "cranelift-codegen",
            "mechanical_prefilter",
            serde_json::json!({"license": "MIT"}),
        )
        .unwrap();

        let record = append_pass(
            temp.path(),
            "crate",
            "cranelift-codegen",
            "some_other_pass",
            serde_json::json!({"note": "second pass"}),
        )
        .unwrap();

        assert_eq!(record.passes.len(), 2);
        assert_eq!(record.passes[0].pass_name, "mechanical_prefilter");
        assert_eq!(record.passes[1].pass_name, "some_other_pass");

        // re-read from disk to confirm persistence, not just the in-memory
        // return value.
        let reread = read_record(temp.path(), "crate", "cranelift-codegen")
            .unwrap()
            .unwrap();
        assert_eq!(reread.passes.len(), 2);
    }

    // (c) read_record returns None (not an error) for a candidate with no
    // record yet.
    #[test]
    fn read_record_returns_none_for_unknown_candidate() {
        let temp = tempdir().unwrap();
        let result = read_record(temp.path(), "crate", "never-seen-before").unwrap();
        assert!(result.is_none());
    }

    // (d) the crate-vs-cluster `kind` prefixing prevents a collision between
    // a crate and a cluster sharing a name.
    #[test]
    fn crate_and_cluster_sharing_a_name_do_not_collide() {
        let temp = tempdir().unwrap();
        append_pass(
            temp.path(),
            "crate",
            "shared-name",
            "mechanical_prefilter",
            serde_json::json!({"license": "MIT"}),
        )
        .unwrap();
        append_pass(
            temp.path(),
            "cluster",
            "some_target/shared-name",
            "target_decomposition",
            serde_json::json!({"total_lines": 42}),
        )
        .unwrap();

        let crate_record = read_record(temp.path(), "crate", "shared-name")
            .unwrap()
            .unwrap();
        let cluster_record = read_record(temp.path(), "cluster", "some_target/shared-name")
            .unwrap()
            .unwrap();

        assert_eq!(crate_record.passes.len(), 1);
        assert_eq!(crate_record.passes[0].pass_name, "mechanical_prefilter");
        assert_eq!(cluster_record.passes.len(), 1);
        assert_eq!(cluster_record.passes[0].pass_name, "target_decomposition");
    }

    #[test]
    fn cluster_candidate_id_without_slash_is_rejected() {
        let temp = tempdir().unwrap();
        let err = append_pass(
            temp.path(),
            "cluster",
            "no-slash-here",
            "target_decomposition",
            serde_json::json!({}),
        )
        .unwrap_err();
        assert!(err.to_string().contains("target_name}/{cluster_dir"));
    }

    #[test]
    fn unknown_kind_is_rejected() {
        let temp = tempdir().unwrap();
        let err =
            append_pass(temp.path(), "widget", "x", "pass", serde_json::json!({})).unwrap_err();
        assert!(err.to_string().contains("unknown candidate kind"));
    }

    #[test]
    fn read_record_rejects_relative_metadata_dir() {
        let err = read_record(Path::new("relative/dir"), "crate", "x").unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn append_pass_rejects_relative_metadata_dir() {
        let err = append_pass(
            Path::new("relative/dir"),
            "crate",
            "x",
            "pass",
            serde_json::json!({}),
        )
        .unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    // (e) a recent existing mechanical_prefilter entry causes a second call
    // to skip the redundant work -- exercised here at the pure decision
    // function directly (no network involved), which is what lets M55's
    // wiring reuse a cached result without a live call in this test.
    #[test]
    fn find_recent_pass_returns_fresh_entry_within_threshold() {
        let record = CandidateRecord {
            schema_version: 1,
            candidate_id: "cranelift-codegen".into(),
            kind: "crate".into(),
            created_at_secs: 1000,
            updated_at_secs: 1000,
            passes: vec![PassEntry {
                pass_name: "mechanical_prefilter".into(),
                recorded_at_secs: 1000,
                findings: serde_json::json!({"license": "MIT"}),
            }],
        };

        // 1 hour later, well within the 24h threshold.
        let now = 1000 + 3600;
        let found = find_recent_pass(
            &record,
            "mechanical_prefilter",
            now,
            PREFILTER_STALE_AFTER_SECS,
        );
        assert!(found.is_some());
        assert_eq!(found.unwrap().findings["license"], "MIT");
    }

    #[test]
    fn find_recent_pass_returns_none_when_entry_is_stale() {
        let record = CandidateRecord {
            schema_version: 1,
            candidate_id: "cranelift-codegen".into(),
            kind: "crate".into(),
            created_at_secs: 1000,
            updated_at_secs: 1000,
            passes: vec![PassEntry {
                pass_name: "mechanical_prefilter".into(),
                recorded_at_secs: 1000,
                findings: serde_json::json!({"license": "MIT"}),
            }],
        };

        // 25 hours later, past the 24h threshold.
        let now = 1000 + (25 * 3600);
        let found = find_recent_pass(
            &record,
            "mechanical_prefilter",
            now,
            PREFILTER_STALE_AFTER_SECS,
        );
        assert!(found.is_none());
    }

    #[test]
    fn find_recent_pass_ignores_entries_for_a_different_pass_name() {
        let record = CandidateRecord {
            schema_version: 1,
            candidate_id: "cranelift-codegen".into(),
            kind: "crate".into(),
            created_at_secs: 1000,
            updated_at_secs: 1000,
            passes: vec![PassEntry {
                pass_name: "some_other_pass".into(),
                recorded_at_secs: 1000,
                findings: serde_json::json!({}),
            }],
        };

        let now = 1000 + 10;
        let found = find_recent_pass(
            &record,
            "mechanical_prefilter",
            now,
            PREFILTER_STALE_AFTER_SECS,
        );
        assert!(found.is_none());
    }

    #[test]
    fn find_recent_pass_picks_most_recent_matching_entry() {
        let record = CandidateRecord {
            schema_version: 1,
            candidate_id: "cranelift-codegen".into(),
            kind: "crate".into(),
            created_at_secs: 1000,
            updated_at_secs: 5000,
            passes: vec![
                PassEntry {
                    pass_name: "mechanical_prefilter".into(),
                    recorded_at_secs: 1000,
                    findings: serde_json::json!({"license": "OLD"}),
                },
                PassEntry {
                    pass_name: "mechanical_prefilter".into(),
                    recorded_at_secs: 5000,
                    findings: serde_json::json!({"license": "NEW"}),
                },
            ],
        };

        let now = 5000 + 10;
        let found = find_recent_pass(
            &record,
            "mechanical_prefilter",
            now,
            PREFILTER_STALE_AFTER_SECS,
        )
        .unwrap();
        assert_eq!(found.findings["license"], "NEW");
    }
}
