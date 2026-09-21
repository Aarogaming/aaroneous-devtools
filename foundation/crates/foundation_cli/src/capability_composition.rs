//! M47 — Internal capability composition explorer (devtools half).
//!
//! Reads Aaroneous's read-only JSON export of its `CapabilityDescriptor`
//! registry (a file contract, not a Cargo dependency — see
//! `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`'s M47 section and its
//! "Scoping decision (2026-09-20)"), generates bounded cross-category
//! candidate pairs, and emits each as a `worker-jobs/*.toml` packet asking a
//! reviewer to score it against `governance/BUILD_WORTHINESS_STANDARD.md`.
//!
//! Per the binding scoping decision, this module never attempts to
//! mechanically compute a value/feasibility score from capability metadata.
//! A text-similarity heuristic pretending to judge "is this composition
//! valuable" would overclaim what's actually happening — that judgment stays
//! with the human or agent reviewer who reads the emitted packet. This
//! mirrors `frontier_discovery::create_frontier_job`, which creates a job for
//! a reviewer to assess rather than auto-scoring external crate candidates.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Mirrors Aaroneous's `CapabilityParameter` shape. Defined locally per
/// `FOUNDATION_BOUNDARY.md` — devtools takes no Cargo dependency on
/// Aaroneous or its crates; this struct only happens to accept JSON shaped
/// the way Aaroneous's exporter produces it.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CapabilityParameter {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

/// Mirrors Aaroneous's `CapabilityDescriptor` shape. See module docs.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CapabilityDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub parameters: Vec<CapabilityParameter>,
}

/// Reads the capabilities JSON export from an explicit absolute path. No
/// ambient discovery, per `FOUNDATION_BOUNDARY.md` and `CONTRIBUTING.md`.
pub fn read_capabilities(path: &Path) -> Result<Vec<CapabilityDescriptor>> {
    if !path.is_absolute() {
        bail!(
            "capabilities path must be an absolute path: {}",
            path.display()
        );
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read capabilities export: {}", path.display()))?;
    let capabilities: Vec<CapabilityDescriptor> = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse capabilities JSON: {}", path.display()))?;
    Ok(capabilities)
}

/// A candidate cross-category composition pair.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidatePair<'a> {
    pub first: &'a CapabilityDescriptor,
    pub second: &'a CapabilityDescriptor,
}

/// Generates bounded candidate pairs from the capability list.
///
/// Bounding strategy (a judgment call for a reviewer to double-check):
/// 1. Only pairs capabilities from *different* categories. Same-category
///    pairs are far less likely to represent a novel cross-domain
///    composition (the interesting case named in the M47 plan), and
///    dropping them also cuts down the search space before the `limit`
///    check ever has to fire.
/// 2. Stops generating as soon as `limit` pairs have been produced —
///    pairs are constructed and counted one at a time during a nested
///    iteration over the (id-ordered, deduplicated-by-id) capability list,
///    not by first materializing the full O(n^2) cross product and then
///    truncating. This keeps memory and work bounded even if the upstream
///    registry grows large, at the cost of a fixed, deterministic
///    (first-id-then-second-id order) selection of *which* pairs are
///    surfaced first when the registry exceeds `limit`.
///
/// Capabilities are deduplicated by `id` (first occurrence wins) and sorted
/// by `id` before pairing, so results are deterministic regardless of the
/// input JSON's array order.
pub fn generate_candidate_pairs(
    capabilities: &[CapabilityDescriptor],
    limit: usize,
) -> Vec<CandidatePair<'_>> {
    if limit == 0 {
        return Vec::new();
    }

    // Deduplicate by id (first occurrence wins), then sort by id for a
    // deterministic pairing order.
    let mut by_id: BTreeMap<&str, &CapabilityDescriptor> = BTreeMap::new();
    for cap in capabilities {
        by_id.entry(cap.id.as_str()).or_insert(cap);
    }
    let ordered: Vec<&CapabilityDescriptor> = by_id.into_values().collect();

    let mut pairs = Vec::new();
    'outer: for (i, first) in ordered.iter().enumerate() {
        for second in ordered.iter().skip(i + 1) {
            if first.category == second.category {
                continue;
            }
            pairs.push(CandidatePair { first, second });
            if pairs.len() >= limit {
                break 'outer;
            }
        }
    }
    pairs
}

/// Sanitizes a capability id for safe use in a filename / packet id
/// component: keeps ASCII alphanumerics, replaces everything else
/// (`.`, `:`, `/`, spaces, etc.) with `_`, and uppercases for the id form.
pub(crate) fn sanitize_for_id(raw: &str) -> String {
    let s: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_uppercase();
    if s.len() > 32 { s[..32].to_string() } else { s }
}

pub(crate) fn sanitize_for_filename(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

/// Emits one `worker-jobs/*.toml` analysis packet for a candidate pair,
/// modeled directly on `frontier_discovery::create_frontier_job`. The
/// packet asks a reviewer to score the pair against
/// `governance/BUILD_WORTHINESS_STANDARD.md`'s Stage 1 gates and Stage 2
/// quadrants; this function never computes or asserts a score itself.
///
/// `evidence` lists the two capability ids plus the path to the source
/// capabilities JSON, so a reviewer can trace the packet back to the
/// exported registry it was generated from.
pub fn create_composition_job(
    pair: &CandidatePair<'_>,
    output_dir: &Path,
    capabilities_source_path: &str,
) -> Result<PathBuf> {
    if !output_dir.is_absolute() {
        bail!(
            "output_dir must be an absolute path: {}",
            output_dir.display()
        );
    }

    let first_id_sanitized = sanitize_for_id(&pair.first.id);
    let second_id_sanitized = sanitize_for_id(&pair.second.id);
    let job_id = format!("COMPOSITION-{first_id_sanitized}-{second_id_sanitized}");

    let filename = format!(
        "composition-{}-{}.toml",
        sanitize_for_filename(&pair.first.id),
        sanitize_for_filename(&pair.second.id)
    );
    let destination = output_dir.join(&filename);

    let instruction = format!(
        "Score the candidate capability composition of '{first_name}' ({first_id}, category: {first_category}) \
and '{second_name}' ({second_id}, category: {second_category}) against governance/BUILD_WORTHINESS_STANDARD.md.\n\
\n\
'{first_name}': {first_description}\n\
'{second_name}': {second_description}\n\
\n\
Evaluate whether one capability's parameter shape plausibly accepts, or could be adapted to accept, \
the other's described output or effect, and whether the resulting composition would unlock value \
neither capability provides alone.\n\
\n\
Apply BUILD_WORTHINESS_STANDARD.md's Stage 1 admissibility gates first. If the pair passes Stage 1, \
score it against Stage 2's quadratic criteria (Quadrant I capability value, Quadrant II implementation \
feasibility, Quadrant III verification rigor achievable, Quadrant IV roadmap leverage) and report the \
resulting composite score and outcome (ADMIT_TO_READY / ADMIT_WITH_SCOPING / ARCHIVE_AS_ROADMAPPED / \
REJECT_LOG_REASON). If the pair fails a Stage 1 gate, report REJECT_LOG_REASON with the failing gate \
and reason instead of proceeding to Stage 2.",
        first_name = pair.first.name,
        first_id = pair.first.id,
        first_category = pair.first.category,
        first_description = pair.first.description,
        second_name = pair.second.name,
        second_id = pair.second.id,
        second_category = pair.second.category,
        second_description = pair.second.description,
    );

    let manifest = serde_json::json!({
        "schema_version": 1,
        "id": job_id,
        "mode": "analysis",
        "evidence": [
            format!("capability:{}", pair.first.id),
            format!("capability:{}", pair.second.id),
            "governance/BUILD_WORTHINESS_STANDARD.md",
            capabilities_source_path,
        ],
        "instruction": instruction
    });

    let toml_str = toml::to_string_pretty(&manifest)?;
    std::fs::write(&destination, toml_str).with_context(|| {
        format!(
            "failed to write composition packet: {}",
            destination.display()
        )
    })?;
    Ok(destination)
}

/// Ties read -> pair -> emit together for the CLI subcommand.
pub fn run(capabilities_path: &Path, output_dir: &Path, limit: usize) -> Result<Vec<PathBuf>> {
    if !output_dir.is_absolute() {
        bail!(
            "output_dir must be an absolute path: {}",
            output_dir.display()
        );
    }
    let capabilities = read_capabilities(capabilities_path)?;
    let pairs = generate_candidate_pairs(&capabilities, limit);
    let source_path_str = capabilities_path.display().to_string();

    let mut written = Vec::with_capacity(pairs.len());
    for pair in &pairs {
        let path = create_composition_job(pair, output_dir, &source_path_str)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn cap(id: &str, category: &str) -> CapabilityDescriptor {
        CapabilityDescriptor {
            id: id.to_string(),
            name: format!("Name for {id}"),
            description: format!("Description for {id}"),
            category: category.to_string(),
            parameters: vec![CapabilityParameter {
                name: "intent".into(),
                description: "Task or goal description".into(),
                param_type: "string".into(),
                required: true,
                default_value: None,
            }],
        }
    }

    #[test]
    fn generate_candidate_pairs_only_crosses_categories() {
        let caps = vec![
            cap("specialist.a", "Specialist"),
            cap("specialist.b", "Specialist"),
            cap("devtools.c", "DevTools"),
        ];
        let pairs = generate_candidate_pairs(&caps, 100);
        // specialist.a/specialist.b share a category and must be excluded.
        assert!(!pairs.iter().any(|p| {
            (p.first.id == "specialist.a" && p.second.id == "specialist.b")
                || (p.first.id == "specialist.b" && p.second.id == "specialist.a")
        }));
        // Cross-category pairs must be present.
        assert!(
            pairs
                .iter()
                .any(|p| p.first.id == "devtools.c" || p.second.id == "devtools.c")
        );
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn generate_candidate_pairs_respects_limit() {
        let caps = vec![
            cap("a", "Specialist"),
            cap("b", "DevTools"),
            cap("c", "MemoryFabric"),
            cap("d", "ModelFoundry"),
        ];
        // 4 categories, all distinct -> 6 possible cross-category pairs.
        let all_pairs = generate_candidate_pairs(&caps, 100);
        assert_eq!(all_pairs.len(), 6);

        let limited = generate_candidate_pairs(&caps, 2);
        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn generate_candidate_pairs_no_self_or_duplicate_pairs() {
        let caps = vec![
            cap("a", "Specialist"),
            cap("a", "Specialist"), // duplicate id, should be deduplicated
            cap("b", "DevTools"),
        ];
        let pairs = generate_candidate_pairs(&caps, 100);
        // Deduplicated to 2 unique capabilities -> exactly 1 pair.
        assert_eq!(pairs.len(), 1);
        for p in &pairs {
            assert_ne!(p.first.id, p.second.id);
        }
    }

    #[test]
    fn generate_candidate_pairs_zero_limit_yields_nothing() {
        let caps = vec![cap("a", "Specialist"), cap("b", "DevTools")];
        let pairs = generate_candidate_pairs(&caps, 0);
        assert!(pairs.is_empty());
    }

    #[test]
    fn generate_candidate_pairs_empty_input() {
        let pairs = generate_candidate_pairs(&[], 10);
        assert!(pairs.is_empty());
    }

    #[test]
    fn create_composition_job_writes_parseable_toml_with_expected_fields() {
        let temp = tempdir().unwrap();
        let first = cap("specialist.dispatch_intent", "Specialist");
        let second = cap("memory.recall_fact", "MemoryFabric");
        let pair = CandidatePair {
            first: &first,
            second: &second,
        };

        let path = create_composition_job(&pair, temp.path(), "capabilities-export.json").unwrap();
        assert!(path.exists());

        let text = std::fs::read_to_string(&path).unwrap();
        let parsed: toml::Value = toml::from_str(&text).unwrap();

        assert_eq!(parsed["schema_version"].as_integer(), Some(1));
        assert_eq!(
            parsed["id"].as_str(),
            Some("COMPOSITION-SPECIALIST_DISPATCH_INTENT-MEMORY_RECALL_FACT")
        );
        assert_eq!(parsed["mode"].as_str(), Some("analysis"));

        let evidence = parsed["evidence"].as_array().unwrap();
        assert!(
            evidence
                .iter()
                .any(|v| v.as_str() == Some("capability:specialist.dispatch_intent"))
        );
        assert!(
            evidence
                .iter()
                .any(|v| v.as_str() == Some("capability:memory.recall_fact"))
        );
        assert!(
            evidence
                .iter()
                .any(|v| v.as_str() == Some("capabilities-export.json"))
        );

        let instruction = parsed["instruction"].as_str().unwrap();
        assert!(instruction.contains("BUILD_WORTHINESS_STANDARD.md"));
        assert!(instruction.contains("Stage 1"));
        assert!(instruction.contains("Stage 2"));
        assert!(instruction.contains("specialist.dispatch_intent"));
        assert!(instruction.contains("memory.recall_fact"));
    }

    #[test]
    fn create_composition_job_rejects_relative_output_dir() {
        let first = cap("a", "Specialist");
        let second = cap("b", "DevTools");
        let pair = CandidatePair {
            first: &first,
            second: &second,
        };
        let err = create_composition_job(&pair, Path::new("relative/dir"), "src.json").unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn read_capabilities_rejects_relative_path() {
        let err = read_capabilities(Path::new("relative/caps.json")).unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn read_capabilities_parses_documented_shape() {
        let temp = tempdir().unwrap();
        let json_path = temp.path().join("caps.json");
        let json = r#"[
            {
                "id": "specialist.dispatch_intent",
                "name": "Dispatch Intent to Specialist Hive",
                "description": "Decomposes and routes goals through the 9 Sovereign Specialists",
                "category": "Specialist",
                "parameters": [
                    {"name": "intent", "description": "Task or goal description", "param_type": "string", "required": true, "default_value": null}
                ]
            }
        ]"#;
        std::fs::write(&json_path, json).unwrap();

        let caps = read_capabilities(&json_path).unwrap();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].id, "specialist.dispatch_intent");
        assert_eq!(caps[0].category, "Specialist");
        assert_eq!(caps[0].parameters.len(), 1);
        assert_eq!(caps[0].parameters[0].name, "intent");
    }

    #[test]
    fn run_reads_pairs_and_emits_packets_end_to_end() {
        let temp = tempdir().unwrap();
        let json_path = temp.path().join("caps.json");
        let caps = vec![
            cap("specialist.dispatch_intent", "Specialist"),
            cap("memory.recall_fact", "MemoryFabric"),
            cap("devtools.lint", "DevTools"),
        ];
        std::fs::write(&json_path, serde_json::to_string(&caps).unwrap()).unwrap();

        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();

        let written = run(&json_path, &output_dir, 2).unwrap();
        assert_eq!(written.len(), 2);
        for path in &written {
            assert!(path.exists());
        }
    }
}
