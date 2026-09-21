//! M54 -- Target decomposition: cluster a large source tree into bounded
//! candidates before discovery/filtration runs.
//!
//! Corrected framing (see `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`'s
//! M54 section): pointing discovery at a whole large target (e.g. Bevy ECS --
//! real, Rust, MIT/Apache-2.0, genuinely modular internally, but far past
//! `FRONTIER_FILTRATION_STANDARD_V1.md`'s G5 5,000-LOC refactoring-surface
//! gate as one candidate) fails the same way Cranelift did, for the same
//! honest reason: it was never one bounded thing to score. Nothing upstream
//! breaks a large target into individually-scorable pieces -- this module is
//! that missing step.
//!
//! Reads Aaroneous's exported `Vec<CrateSpec>` JSON (from `cratify harvest
//! --output <path>`, one entry per *file*, per
//! `adaptation_engine::source_extraction::extract_source_tree`'s own doc
//! comment), groups entries by their `source_path`'s parent directory (the
//! module-boundary proxy this milestone deliberately uses instead of a
//! `mod`-declaration-aware grouping -- directory structure closely tracks
//! Rust's module structure and a more precise grouping is explicitly out of
//! scope), sizes each group by total line count (a cheap size proxy, not a
//! full AST parse -- same pattern `component_inventory.rs` uses for its own
//! per-file line counting, generalized here to whole clusters), and emits one
//! `analysis`-mode `worker-jobs/*.toml` packet per cluster that clears the
//! G5 line-count bound. Clusters that don't clear the bound are not silently
//! dropped -- they're a real, reportable finding, so they're collected into
//! the summary report's `oversized` section instead.
//!
//! Emits `mode = "analysis"`, not `mode = "code-draft"`: a decomposed cluster
//! needs scoring against `contracts/FRONTIER_FILTRATION_STANDARD_V1.md`
//! before anyone decides to draft a port, mirroring
//! `frontier_discovery::create_frontier_job`'s analysis-mode packet shape
//! (not `cratification_intake`'s code-draft shape, which is for candidates
//! that already cleared filtration).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Worker-jobs schema (see `local_worker.rs`/`multithreader.rs`) bounds
/// `evidence` to 1..=8 entries. A cluster with more files than this cap gets
/// its evidence list truncated to the first `MAX_EVIDENCE_FILES` (sorted for
/// determinism), and the truncation is recorded as a limitation in the
/// summary report rather than silently expanding the schema or silently
/// dropping files from the finding.
const MAX_EVIDENCE_FILES: usize = 8;

/// Mirrors Aaroneous's `CrateSpec` shape as exported by
/// `cratify harvest --output <path>`. Defined locally, not imported from
/// Aaroneous -- see `cratification_intake::CrateSpec` for the same
/// precedent. This producer only needs `name`/`source_path`; `code_info` is
/// read (so parsing the real export doesn't fail) but otherwise unused.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrateSpec {
    pub name: String,
    pub source_path: String,
    #[serde(default)]
    pub code_info: serde_json::Value,
}

/// Reads the harvested `CrateSpec` JSON export from an explicit absolute
/// path. No ambient discovery, per `FOUNDATION_BOUNDARY.md` and
/// `CONTRIBUTING.md`'s "Foundation commands require explicit input and
/// output paths" rule.
pub fn read_crate_specs(path: &Path) -> Result<Vec<CrateSpec>> {
    if !path.is_absolute() {
        bail!(
            "crate specs path must be an absolute path: {}",
            path.display()
        );
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read crate specs export: {}", path.display()))?;
    let specs: Vec<CrateSpec> = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse crate specs JSON: {}", path.display()))?;
    Ok(specs)
}

/// One module-boundary-proxy cluster: every harvested file sharing a common
/// parent directory, plus its total line count.
#[derive(Debug, Clone)]
pub struct Cluster {
    /// The shared parent directory, as it appeared in `source_path` (raw
    /// string form, not filesystem-normalized -- this is also used as the
    /// packet's `perspective` field).
    pub dir: String,
    /// Every file's `source_path` in this cluster, sorted for determinism.
    pub files: Vec<String>,
    pub total_lines: usize,
}

/// Groups harvested `CrateSpec` entries by their `source_path`'s parent
/// directory (the module-boundary proxy) and computes each group's total
/// line count by actually reading every file (`fs::read_to_string` +
/// `.lines().count()`, no AST parse -- see module docs). Files that can't be
/// read (e.g. the export was taken on a different machine than this is run
/// on) fail the whole call rather than silently under-counting a cluster's
/// size, since an inaccurate size proxy would defeat the point of the G5
/// gate this feeds.
pub fn cluster_by_directory(specs: &[CrateSpec]) -> Result<Vec<Cluster>> {
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for spec in specs {
        let source = Path::new(&spec.source_path);
        let dir = source
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| ".".to_string());
        groups
            .entry(dir)
            .or_default()
            .push(spec.source_path.clone());
    }

    let mut clusters = Vec::with_capacity(groups.len());
    for (dir, mut files) in groups {
        files.sort();
        files.dedup();
        let mut total_lines = 0usize;
        for file in &files {
            let text = std::fs::read_to_string(file)
                .with_context(|| format!("failed to read source file for sizing: {file}"))?;
            total_lines += text.lines().count();
        }
        clusters.push(Cluster {
            dir,
            files,
            total_lines,
        });
    }
    Ok(clusters)
}

/// Slugify a name into an id-safe `exploration_group` key: lowercase,
/// non-alphanumeric runs collapsed to a single `_`, trimmed of leading/
/// trailing `_`.
///
/// Duplicated from (not imported from) `frontier_discovery::slugify_query` /
/// `cratification_intake::slugify_crate_name` -- both are private/non-shared
/// helpers for the same reason those two aren't shared with each other: the
/// character-class logic happens to coincide but the three producers'
/// slugging inputs are conceptually different, and promoting this to a
/// shared helper isn't justified for three ~15-line call sites.
fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_was_sep = true;
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_sep = false;
        } else if !last_was_sep {
            slug.push('_');
            last_was_sep = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    slug
}

/// Sanitizes a cluster directory path for safe use as a packet id: uppercase,
/// non-alphanumeric runs replaced with `_`, capped to fit the worker-jobs
/// schema's 64-character `id` limit (`local_worker.rs`'s
/// `job.id.len() > 64` check) alongside the `DECOMPOSE-` prefix.
///
/// Found by running this against a real harvest of Bevy ECS (2026-09-20):
/// naively truncating a long absolute path from the front collapsed every
/// cluster to an identical id, because clusters under one harvest run share
/// a long common prefix (the harvest source root) and only diverge near the
/// *end* of the path -- exactly the part a front-truncation throws away.
/// Fixed by keying on the path's last 1-2 components (the actual
/// distinguishing module name) instead of the full path, with a short
/// deterministic hash of the *complete* original path appended so two
/// clusters that happen to share a final component (e.g. two different
/// crates each having a `tests` subdirectory) still can't collide.
fn sanitize_for_id(raw: &str) -> String {
    const PREFIX_LEN: usize = "DECOMPOSE-".len();
    const HASH_LEN: usize = 8; // hex chars from the suffix hash
    let max_len = 64 - PREFIX_LEN;

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    raw.hash(&mut hasher);
    let hash_suffix = format!("{:08X}", (hasher.finish() as u32));
    debug_assert_eq!(hash_suffix.len(), HASH_LEN);

    // Prefer the last two path components (e.g. "world/entity_access") as
    // the human-readable core; fall back to whatever's available for a
    // shallow path.
    let components: Vec<&str> = raw.split('/').filter(|s| !s.is_empty()).collect();
    let readable_core = components
        .iter()
        .rev()
        .take(2)
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join("_");

    let sanitized_core: String = readable_core
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_uppercase();

    // Reserve room for "_" + the hash suffix, truncate the readable core
    // (from the front this time -- the core is already just the last 1-2
    // components, so its own *end* is the most specific part) to fit, then
    // append the hash so uniqueness never depends on truncation alone.
    let reserved = 1 + HASH_LEN;
    let core_budget = max_len.saturating_sub(reserved);
    let truncated_core = if sanitized_core.len() > core_budget {
        sanitized_core[sanitized_core.len() - core_budget..].to_string()
    } else {
        sanitized_core
    };

    format!("{truncated_core}_{hash_suffix}")
}

fn sanitize_for_filename(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

/// Emits one `mode = "analysis"` `worker-jobs/*.toml` packet for a cluster
/// that cleared the size threshold, modeled on
/// `frontier_discovery::create_frontier_job`'s analysis-mode packet shape.
///
/// `evidence` lists every file in the cluster, capped to
/// `MAX_EVIDENCE_FILES` per the worker-jobs schema's bounded-evidence-list
/// convention -- the caller is responsible for noting any truncation in the
/// summary report (`run` below does this).
pub fn create_decomposition_job(
    cluster: &Cluster,
    output_dir: &Path,
    target_name: &str,
) -> Result<PathBuf> {
    if !output_dir.is_absolute() {
        bail!(
            "output_dir must be an absolute path: {}",
            output_dir.display()
        );
    }

    let id_sanitized = sanitize_for_id(&cluster.dir);
    let job_id = format!("DECOMPOSE-{id_sanitized}");
    let filename = format!("decompose-{}.toml", sanitize_for_filename(&cluster.dir));
    let destination = output_dir.join(&filename);

    let evidence: Vec<String> = cluster
        .files
        .iter()
        .take(MAX_EVIDENCE_FILES)
        .cloned()
        .collect();

    let instruction = format!(
        "Score this module cluster from target '{target}' against \
contracts/FRONTIER_FILTRATION_STANDARD_V1.md's gates and quadrants. The cluster is the \
directory '{dir}', containing {file_count} file(s) totaling {total_lines} line(s) -- \
already confirmed under the G5 Refactoring Surface Limit (5,000 LOC) that made whole-target \
scoring impossible. Evaluate license sovereignty, kernel extractability, deterministic \
tractability, ecosystem independence, and the quadratic review criteria (algorithmic \
needle, refactoring feasibility, test portability, milestone leverage) for this cluster \
specifically, not the target as a whole. Recommend one of \
DIRECT_ADOPT_WITH_WRAPPER / CRATIFY_KERNEL_EXTRACTION / BORROW_SPEC_AND_SYNTHESIZE / \
REFACTOR_COST_PROHIBITIVE / REJECTED_AT_FILTRATION.",
        target = target_name,
        dir = cluster.dir,
        file_count = cluster.files.len(),
        total_lines = cluster.total_lines,
    );

    // M46 tagging: every cluster from one decomposition run shares a `goal`
    // (the target's own name) and `exploration_group` (a slug of it), with
    // `perspective` set to the cluster's own module/directory path -- the
    // "angle" for a decomposition analysis job is literally which piece of
    // the target is being evaluated.
    let manifest = serde_json::json!({
        "schema_version": 1,
        "id": job_id,
        "mode": "analysis",
        "evidence": evidence,
        "instruction": instruction,
        "perspective": cluster.dir,
        "goal": target_name,
        "exploration_group": slugify(target_name),
    });

    let toml_str = toml::to_string_pretty(&manifest)?;
    std::fs::write(&destination, toml_str).with_context(|| {
        format!(
            "failed to write decomposition packet: {}",
            destination.display()
        )
    })?;
    Ok(destination)
}

/// One cluster's entry in the summary report.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ClusterReportEntry {
    pub dir: String,
    pub file_count: usize,
    pub total_lines: usize,
    /// Present only for clusters that passed the threshold and got a packet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_path: Option<String>,
    /// True when this cluster had more files than `MAX_EVIDENCE_FILES` and
    /// its packet's `evidence` list was truncated to the first
    /// `MAX_EVIDENCE_FILES` (sorted) -- see module docs.
    #[serde(default)]
    pub evidence_truncated: bool,
}

/// Decomposition run summary, written alongside the emitted packets so an
/// oversized module is a visible, reportable finding rather than a silent
/// drop.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DecompositionReport {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub target_name: String,
    pub crate_specs_source: String,
    pub max_lines: usize,
    pub total_files_scanned: usize,
    pub total_clusters_found: usize,
    pub passed_count: usize,
    pub oversized_count: usize,
    pub passed: Vec<ClusterReportEntry>,
    pub oversized: Vec<ClusterReportEntry>,
}

/// Ties read -> cluster -> filter -> emit -> report together for the CLI
/// subcommand.
///
/// Clusters with `total_lines <= max_lines` become `analysis` packets in
/// `output_dir`. Clusters over the bound are not dropped: they're recorded
/// in the returned report's `oversized` list with their actual line count,
/// a real "this target isn't decomposable at the module level here" finding.
pub fn run(
    crate_specs_path: &Path,
    target_name: &str,
    max_lines: usize,
    output_dir: &Path,
    report_output: &Path,
) -> Result<DecompositionReport> {
    if !output_dir.is_absolute() {
        bail!(
            "output_dir must be an absolute path: {}",
            output_dir.display()
        );
    }
    if !report_output.is_absolute() {
        bail!(
            "report_output must be an absolute path: {}",
            report_output.display()
        );
    }

    let specs = read_crate_specs(crate_specs_path)?;
    let total_files_scanned = specs.len();
    let clusters = cluster_by_directory(&specs)?;
    let total_clusters_found = clusters.len();

    let mut passed = Vec::new();
    let mut oversized = Vec::new();

    for cluster in &clusters {
        if cluster.total_lines <= max_lines {
            let packet_path = create_decomposition_job(cluster, output_dir, target_name)?;
            passed.push(ClusterReportEntry {
                dir: cluster.dir.clone(),
                file_count: cluster.files.len(),
                total_lines: cluster.total_lines,
                packet_path: Some(packet_path.display().to_string()),
                evidence_truncated: cluster.files.len() > MAX_EVIDENCE_FILES,
            });
        } else {
            oversized.push(ClusterReportEntry {
                dir: cluster.dir.clone(),
                file_count: cluster.files.len(),
                total_lines: cluster.total_lines,
                packet_path: None,
                evidence_truncated: false,
            });
        }
    }

    let report = DecompositionReport {
        schema_version: 1,
        artifact_kind: "foundation.target-decomposition".to_string(),
        target_name: target_name.to_string(),
        crate_specs_source: crate_specs_path.display().to_string(),
        max_lines,
        total_files_scanned,
        total_clusters_found,
        passed_count: passed.len(),
        oversized_count: oversized.len(),
        passed,
        oversized,
    };

    std::fs::write(
        report_output,
        serde_json::to_vec_pretty(&report).context("failed to serialize decomposition report")?,
    )
    .with_context(|| format!("failed to write report: {}", report_output.display()))?;

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_file(dir: &Path, rel: &str, lines: usize) -> String {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let content = (0..lines)
            .map(|i| format!("// line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, content).unwrap();
        path.display().to_string()
    }

    fn write_specs_json(temp: &Path, entries: &[(String, String)]) -> PathBuf {
        let specs: Vec<serde_json::Value> = entries
            .iter()
            .map(|(name, source_path)| {
                serde_json::json!({
                    "name": name,
                    "source_path": source_path,
                    "code_info": { "functions": [], "structs": [], "enums": [] }
                })
            })
            .collect();
        let json_path = temp.join("specs.json");
        std::fs::write(&json_path, serde_json::to_string(&specs).unwrap()).unwrap();
        json_path
    }

    #[test]
    fn read_crate_specs_rejects_relative_path() {
        let err = read_crate_specs(Path::new("relative/specs.json")).unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    // (a) files in the same directory get grouped into one cluster.
    #[test]
    fn files_in_same_directory_form_one_cluster() {
        let temp = tempdir().unwrap();
        let a = write_file(temp.path(), "mod_a/one.rs", 3);
        let b = write_file(temp.path(), "mod_a/two.rs", 4);
        let specs = vec![
            CrateSpec {
                name: "one".into(),
                source_path: a,
                code_info: serde_json::Value::Null,
            },
            CrateSpec {
                name: "two".into(),
                source_path: b,
                code_info: serde_json::Value::Null,
            },
        ];
        let clusters = cluster_by_directory(&specs).unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].files.len(), 2);
        assert_eq!(clusters[0].total_lines, 7);
    }

    // (b) files in different directories produce separate clusters.
    #[test]
    fn files_in_different_directories_form_separate_clusters() {
        let temp = tempdir().unwrap();
        let a = write_file(temp.path(), "mod_a/one.rs", 3);
        let b = write_file(temp.path(), "mod_b/two.rs", 5);
        let specs = vec![
            CrateSpec {
                name: "one".into(),
                source_path: a,
                code_info: serde_json::Value::Null,
            },
            CrateSpec {
                name: "two".into(),
                source_path: b,
                code_info: serde_json::Value::Null,
            },
        ];
        let clusters = cluster_by_directory(&specs).unwrap();
        assert_eq!(clusters.len(), 2);
    }

    // (c) a cluster whose summed line count exceeds max_lines does not
    // produce a packet and appears in the oversized-report list instead.
    #[test]
    fn oversized_cluster_is_reported_not_dropped_and_no_packet_emitted() {
        let temp = tempdir().unwrap();
        let source_dir = temp.path().join("src");
        let big = write_file(&source_dir, "big_mod/huge.rs", 20);
        let json_path = write_specs_json(temp.path(), &[("huge".to_string(), big)]);

        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();
        let report_output = temp.path().join("report.json");

        let report = run(&json_path, "test-target", 5, &output_dir, &report_output).unwrap();

        assert_eq!(report.passed_count, 0);
        assert_eq!(report.oversized_count, 1);
        assert_eq!(report.oversized[0].total_lines, 20);
        assert!(report.oversized[0].packet_path.is_none());

        let entries = std::fs::read_dir(&output_dir).unwrap().count();
        assert_eq!(entries, 0);
    }

    // (d) a cluster under the threshold produces a correctly-tagged packet.
    #[test]
    fn passing_cluster_produces_correctly_tagged_analysis_packet() {
        let temp = tempdir().unwrap();
        let source_dir = temp.path().join("src");
        let small = write_file(&source_dir, "small_mod/tiny.rs", 4);
        let json_path = write_specs_json(temp.path(), &[("tiny".to_string(), small)]);

        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();
        let report_output = temp.path().join("report.json");

        let report = run(&json_path, "bevy_ecs", 5000, &output_dir, &report_output).unwrap();

        assert_eq!(report.passed_count, 1);
        assert_eq!(report.oversized_count, 0);
        let packet_path = report.passed[0].packet_path.clone().unwrap();
        let text = std::fs::read_to_string(&packet_path).unwrap();
        let parsed: toml::Value = toml::from_str(&text).unwrap();

        assert_eq!(parsed["schema_version"].as_integer(), Some(1));
        assert_eq!(parsed["mode"].as_str(), Some("analysis"));
        assert_eq!(parsed["goal"].as_str(), Some("bevy_ecs"));
        assert_eq!(parsed["exploration_group"].as_str(), Some("bevy_ecs"));
        assert!(
            parsed["perspective"]
                .as_str()
                .unwrap()
                .contains("small_mod")
        );
        assert!(
            parsed["instruction"]
                .as_str()
                .unwrap()
                .contains("FRONTIER_FILTRATION_STANDARD_V1.md")
        );
        let evidence = parsed["evidence"].as_array().unwrap();
        assert_eq!(evidence.len(), 1);
    }

    // shared exploration_group across multiple clusters from one run.
    #[test]
    fn multiple_clusters_from_one_run_share_exploration_group() {
        let temp = tempdir().unwrap();
        let source_dir = temp.path().join("src");
        let a = write_file(&source_dir, "mod_a/one.rs", 3);
        let b = write_file(&source_dir, "mod_b/two.rs", 4);
        let json_path = write_specs_json(
            temp.path(),
            &[("one".to_string(), a), ("two".to_string(), b)],
        );

        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();
        let report_output = temp.path().join("report.json");

        let report = run(&json_path, "bevy_ecs", 5000, &output_dir, &report_output).unwrap();
        assert_eq!(report.passed_count, 2);

        for entry in &report.passed {
            let text = std::fs::read_to_string(entry.packet_path.as_ref().unwrap()).unwrap();
            let parsed: toml::Value = toml::from_str(&text).unwrap();
            assert_eq!(parsed["exploration_group"].as_str(), Some("bevy_ecs"));
        }
    }

    // (e) the summary report's counts are correct for a small synthetic
    // fixture with a mix of passing and oversized clusters.
    #[test]
    fn summary_report_counts_are_correct_for_mixed_fixture() {
        let temp = tempdir().unwrap();
        let source_dir = temp.path().join("src");
        let small = write_file(&source_dir, "small_mod/tiny.rs", 4);
        let big = write_file(&source_dir, "big_mod/huge.rs", 20);
        let json_path = write_specs_json(
            temp.path(),
            &[("tiny".to_string(), small), ("huge".to_string(), big)],
        );

        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();
        let report_output = temp.path().join("report.json");

        let report = run(&json_path, "mixed-target", 10, &output_dir, &report_output).unwrap();

        assert_eq!(report.total_files_scanned, 2);
        assert_eq!(report.total_clusters_found, 2);
        assert_eq!(report.passed_count, 1);
        assert_eq!(report.oversized_count, 1);
        assert_eq!(report.max_lines, 10);

        // report file itself round-trips
        let written: DecompositionReport =
            serde_json::from_slice(&std::fs::read(&report_output).unwrap()).unwrap();
        assert_eq!(written, report);

        let output_files = std::fs::read_dir(&output_dir).unwrap().count();
        assert_eq!(output_files, 1);
    }

    #[test]
    fn evidence_list_is_truncated_and_flagged_for_oversized_cluster_file_count() {
        let temp = tempdir().unwrap();
        let source_dir = temp.path().join("src");
        let mut entries = Vec::new();
        for i in 0..10 {
            let path = write_file(&source_dir, &format!("many_mod/f{i}.rs"), 1);
            entries.push((format!("f{i}"), path));
        }
        let json_path = write_specs_json(temp.path(), &entries);

        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();
        let report_output = temp.path().join("report.json");

        let report = run(&json_path, "many-target", 5000, &output_dir, &report_output).unwrap();
        assert_eq!(report.passed_count, 1);
        assert!(report.passed[0].evidence_truncated);
        assert_eq!(report.passed[0].file_count, 10);

        let text = std::fs::read_to_string(report.passed[0].packet_path.as_ref().unwrap()).unwrap();
        let parsed: toml::Value = toml::from_str(&text).unwrap();
        let evidence = parsed["evidence"].as_array().unwrap();
        assert_eq!(evidence.len(), MAX_EVIDENCE_FILES);
    }

    #[test]
    fn run_rejects_relative_output_dir() {
        let temp = tempdir().unwrap();
        let json_path = write_specs_json(temp.path(), &[] as &[(String, String)]);
        let report_output = temp.path().join("report.json");
        let err = run(
            &json_path,
            "target",
            5000,
            Path::new("relative/dir"),
            &report_output,
        )
        .unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn run_rejects_relative_report_output() {
        let temp = tempdir().unwrap();
        let json_path = write_specs_json(temp.path(), &[] as &[(String, String)]);
        let err = run(
            &json_path,
            "target",
            5000,
            temp.path(),
            Path::new("relative/report.json"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }
}
