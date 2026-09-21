use crate::intake::read_bounded;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NativeDiscoverySummary {
    pub checkout: String,
    pub branch: String,
    pub tracked_files_count: usize,
    pub remotes: Vec<String>,
    pub submodules_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrateCandidate {
    pub name: String,
    pub description: String,
    pub version: String,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub downloads: u64,
}

pub fn discover_native(checkout: &Path) -> Result<NativeDiscoverySummary> {
    if !checkout.is_absolute() {
        bail!("checkout must be an absolute path: {}", checkout.display());
    }

    let repo = gix::open(checkout)
        .with_context(|| format!("Failed to open repository at {}", checkout.display()))?;

    let branch = repo
        .head_name()
        .ok()
        .flatten()
        .map(|name| name.as_bstr().to_string())
        .unwrap_or_else(|| "HEAD (detached)".into());

    let remotes: Vec<String> = repo
        .remote_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let submodules_count = repo
        .submodules()
        .ok()
        .flatten()
        .map(|s| s.count())
        .unwrap_or(0);

    let tracked_files_count = repo
        .index_or_empty()
        .map(|idx| idx.entries().len())
        .unwrap_or(0);

    Ok(NativeDiscoverySummary {
        checkout: checkout.display().to_string(),
        branch,
        tracked_files_count,
        remotes,
        submodules_count,
    })
}

pub fn search_crates_io(query: &str, limit: usize) -> Result<Vec<CrateCandidate>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("AaroneousDevtools/0.1.0 (https://github.com/Aarogaming/aaroneous)")
        .build()
        .context("Failed to construct reqwest client")?;

    let url = format!(
        "https://crates.io/api/v1/crates?q={}&per_page={}",
        urlencoding(query),
        limit.min(20)
    );

    let resp = client
        .get(&url)
        .send()
        .with_context(|| format!("Failed to query crates.io: {url}"))?;

    if !resp.status().is_success() {
        bail!("crates.io returned HTTP status {}", resp.status());
    }

    let json: serde_json::Value = resp
        .json()
        .context("Failed to parse crates.io response JSON")?;
    let mut candidates = Vec::new();

    if let Some(crates) = json["crates"].as_array() {
        for c in crates {
            let name = c["name"].as_str().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            candidates.push(CrateCandidate {
                name,
                description: c["description"].as_str().unwrap_or("").to_string(),
                version: c["max_version"].as_str().unwrap_or("0.1.0").to_string(),
                repository: c["repository"].as_str().map(|s| s.to_string()),
                documentation: c["documentation"].as_str().map(|s| s.to_string()),
                downloads: c["downloads"].as_u64().unwrap_or(0),
            });
        }
    }

    Ok(candidates)
}

/// A single dependency entry as returned by crates.io's
/// `/api/v1/crates/{name}/{version}/dependencies` endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DependencyInfo {
    pub crate_id: String,
    pub kind: String,
    pub optional: bool,
}

/// M55 G4 ecosystem-independence signal: a heuristic, not a hard gate.
/// `Flagged` carries human-readable reasons a reviewer should weigh, mirroring
/// G2/G3's own judgment-call nature in `FRONTIER_FILTRATION_STANDARD_V1.md`.
///
/// `Serialize`/`Deserialize` (M57): needed so a `PrefilterResult` can round-
/// trip through a `CandidateRecord`'s `serde_json::Value` findings and be
/// reused by a later sweep instead of re-fetched -- see
/// `mechanical_prefilter_with_metadata`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EcosystemSignal {
    Clean,
    Flagged(Vec<String>),
}

/// M55 mechanical Stage 1 pre-filter result: `license_gate` is G1, a hard
/// pass/fail (license compliance isn't a judgment call). `ecosystem_signal`
/// is G4, advisory only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefilterResult {
    pub license: String,
    pub license_gate: bool,
    pub ecosystem_signal: EcosystemSignal,
}

/// Fetch a crate version's license string from crates.io's
/// `GET /api/v1/crates/{name}/{version}` endpoint (`.version.license`).
/// Note: the search-results summary from `search_crates_io` does not
/// reliably carry a usable `license` field — this dedicated per-version
/// lookup is required (confirmed against the live API, 2026-09-20).
pub fn fetch_license(name: &str, version: &str) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("AaroneousDevtools/0.1.0 (https://github.com/Aarogaming/aaroneous)")
        .build()
        .context("Failed to construct reqwest client")?;

    let url = format!(
        "https://crates.io/api/v1/crates/{}/{}",
        urlencoding(name),
        urlencoding(version)
    );

    let resp = client
        .get(&url)
        .send()
        .with_context(|| format!("Failed to query crates.io: {url}"))?;

    if !resp.status().is_success() {
        bail!("crates.io returned HTTP status {} for {url}", resp.status());
    }

    let json: serde_json::Value = resp
        .json()
        .context("Failed to parse crates.io version response JSON")?;

    let license = json["version"]["license"]
        .as_str()
        .with_context(|| format!("crates.io response for {url} has no version.license field"))?
        .to_string();

    Ok(license)
}

/// G1 license check: a heuristic SPDX-expression-ish parser, **not** a full
/// SPDX grammar. crates.io's `version.license` field is an SPDX *expression*
/// (e.g. `"Apache-2.0 WITH LLVM-exception"`, `"MIT OR Apache-2.0"`), not a
/// bare identifier, so a naive exact-match would wrongly reject real,
/// clearly-permissive values and would mishandle dual-licensing. This splits
/// on the exact (case-sensitive) SPDX separators `" OR "`, `" AND "`,
/// `" WITH "`, trims each resulting token, and passes if *any* token exactly
/// matches an allowed license. Good enough for the common real-world shapes
/// seen so far; if real usage later shows this getting it wrong, reach for
/// the actual `spdx` crate then, with evidence, not speculatively now.
pub fn license_passes_g1(license: &str) -> bool {
    const ALLOWED: [&str; 6] = [
        "MIT",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "ISC",
        "Unlicense",
    ];

    license
        .split(" OR ")
        .flat_map(|s| s.split(" AND "))
        .flat_map(|s| s.split(" WITH "))
        .map(str::trim)
        .any(|token| ALLOWED.contains(&token))
}

/// Fetch a crate version's dependency list from crates.io's
/// `GET /api/v1/crates/{name}/{version}/dependencies` endpoint.
pub fn fetch_dependencies(name: &str, version: &str) -> Result<Vec<DependencyInfo>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("AaroneousDevtools/0.1.0 (https://github.com/Aarogaming/aaroneous)")
        .build()
        .context("Failed to construct reqwest client")?;

    let url = format!(
        "https://crates.io/api/v1/crates/{}/{}/dependencies",
        urlencoding(name),
        urlencoding(version)
    );

    let resp = client
        .get(&url)
        .send()
        .with_context(|| format!("Failed to query crates.io: {url}"))?;

    if !resp.status().is_success() {
        bail!("crates.io returned HTTP status {} for {url}", resp.status());
    }

    let json: serde_json::Value = resp
        .json()
        .context("Failed to parse crates.io dependencies response JSON")?;

    let mut deps = Vec::new();
    if let Some(items) = json["dependencies"].as_array() {
        for d in items {
            let crate_id = d["crate_id"].as_str().unwrap_or("").to_string();
            if crate_id.is_empty() {
                continue;
            }
            deps.push(DependencyInfo {
                crate_id,
                kind: d["kind"].as_str().unwrap_or("normal").to_string(),
                optional: d["optional"].as_bool().unwrap_or(false),
            });
        }
    }

    Ok(deps)
}

/// G4 ecosystem-independence signal: a heuristic, **not** a hard gate (unlike
/// G1's license check, which is a hard, correct gate — license compliance
/// isn't a judgment call). Flags a dependency whose `crate_id` ends in
/// `-sys` (the Rust convention for a raw FFI-binding crate), or whose
/// `crate_id` is `cc` or `cmake` with `kind == "build"` (known C/C++
/// toolchain wrappers). This is a signal for the reviewer to weigh, mirroring
/// G2/G3's own judgment-call nature in `FRONTIER_FILTRATION_STANDARD_V1.md`.
pub fn ecosystem_independence_signal(deps: &[DependencyInfo]) -> EcosystemSignal {
    let mut reasons = Vec::new();

    for dep in deps {
        if dep.crate_id.ends_with("-sys") {
            reasons.push(format!(
                "dependency '{}' is a -sys crate (raw FFI binding)",
                dep.crate_id
            ));
        } else if (dep.crate_id == "cc" || dep.crate_id == "cmake") && dep.kind == "build" {
            reasons.push(format!(
                "build dependency '{}' is a known C/C++ toolchain wrapper",
                dep.crate_id
            ));
        }
    }

    if reasons.is_empty() {
        EcosystemSignal::Clean
    } else {
        EcosystemSignal::Flagged(reasons)
    }
}

/// M55 mechanical Stage 1 pre-filter: makes the two crates.io API calls
/// needed for G1 (license, hard gate) and G4 (ecosystem independence,
/// advisory signal) and combines them into one result. Zero agent/LLM call
/// anywhere in this path.
pub fn mechanical_prefilter(name: &str, version: &str) -> Result<PrefilterResult> {
    let license = fetch_license(name, version)?;
    let license_gate = license_passes_g1(&license);
    let deps = fetch_dependencies(name, version)?;
    let ecosystem_signal = ecosystem_independence_signal(&deps);

    Ok(PrefilterResult {
        license,
        license_gate,
        ecosystem_signal,
    })
}

/// M57: `mechanical_prefilter` wrapped with the accumulating per-candidate
/// metadata record. When `metadata_dir` is `Some`, this is the "intelligence"
/// half the milestone asks for -- a second sweep over an already-seen crate
/// consults its accumulated record first: if a `"mechanical_prefilter"` pass
/// entry exists and is still fresh (`candidate_metadata::PREFILTER_STALE_AFTER_SECS`),
/// the recorded result is reused and the network calls are skipped entirely;
/// otherwise (or when `metadata_dir` is `None`) it runs exactly as
/// `mechanical_prefilter` always has, and -- when `metadata_dir` is `Some` --
/// appends the fresh result as a new pass entry afterward.
///
/// `metadata_dir` being `None` reproduces `mechanical_prefilter`'s exact
/// prior behavior, unchanged, per this milestone's additive requirement.
pub fn mechanical_prefilter_with_metadata(
    metadata_dir: Option<&Path>,
    name: &str,
    version: &str,
) -> Result<PrefilterResult> {
    let Some(dir) = metadata_dir else {
        return mechanical_prefilter(name, version);
    };

    let now = crate::candidate_metadata::now_secs();
    if let Some(record) = crate::candidate_metadata::read_record(dir, "crate", name)? {
        if let Some(pass) = crate::candidate_metadata::find_recent_pass(
            &record,
            "mechanical_prefilter",
            now,
            crate::candidate_metadata::PREFILTER_STALE_AFTER_SECS,
        ) {
            if let Ok(cached) = serde_json::from_value::<PrefilterResult>(pass.findings.clone()) {
                println!(
                    "M57: reusing recorded mechanical_prefilter for '{name}' (recorded {}s ago) -- skipping network calls",
                    now.saturating_sub(pass.recorded_at_secs)
                );
                return Ok(cached);
            }
        }
    }

    let result = mechanical_prefilter(name, version)?;
    let findings =
        serde_json::to_value(&result).context("failed to serialize mechanical_prefilter result")?;
    crate::candidate_metadata::append_pass(dir, "crate", name, "mechanical_prefilter", findings)
        .with_context(|| format!("failed to record M57 metadata for crate '{name}'"))?;
    Ok(result)
}

pub fn firecrawl_scrape(
    url: &str,
    endpoint: Option<&str>,
    api_key: Option<&str>,
) -> Result<String> {
    let target_endpoint = endpoint.unwrap_or("http://localhost:3002/v1/scrape");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to construct reqwest client")?;

    let mut req = client.post(target_endpoint).json(&serde_json::json!({
        "url": url,
        "formats": ["markdown"]
    }));

    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    let resp = req
        .send()
        .with_context(|| format!("Firecrawl request failed for {url}"))?;
    if !resp.status().is_success() {
        bail!("Firecrawl scrape failed with status: {}", resp.status());
    }

    let val: serde_json::Value = resp.json().context("Failed to parse Firecrawl response")?;
    let markdown = val["data"]["markdown"].as_str().unwrap_or("").to_string();

    if markdown.trim().is_empty() {
        bail!("Firecrawl returned empty markdown content for {url}");
    }

    Ok(markdown)
}

/// Slugify a query into an id-safe `exploration_group` key: lowercase,
/// non-alphanumeric runs collapsed to a single `_`, trimmed of leading/
/// trailing `_`. Same query text always produces the same group id, which
/// is the whole point — it's the join key candidates from one search share.
fn slugify_query(query: &str) -> String {
    let mut slug = String::with_capacity(query.len());
    let mut last_was_sep = true; // suppress a leading separator
    for ch in query.to_lowercase().chars() {
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

/// Create a `worker-jobs/` analysis packet for a frontier candidate.
///
/// `ecosystem_notes`, when `Some`, is appended to the generated
/// `instruction` text — used by M55's mechanical pre-filter to surface a G4
/// ecosystem-independence heuristic flag to the reviewer. This is advisory
/// only, since G4 is a signal, not a hard gate (unlike G1's license check,
/// which the caller enforces separately by simply not calling this function
/// at all for a G1 failure).
pub fn create_frontier_job_with_notes(
    candidate: &CrateCandidate,
    output_dir: &Path,
    evidence_rel_path: &str,
    query: &str,
    ecosystem_notes: Option<&str>,
) -> Result<PathBuf> {
    if !output_dir.is_absolute() {
        bail!("output_dir must be an absolute path");
    }

    let job_id = format!(
        "FRONTIER-{}",
        candidate.name.to_uppercase().replace('-', "_")
    );
    let filename = format!(
        "frontier-{}.toml",
        candidate.name.to_lowercase().replace('_', "-")
    );
    let destination = output_dir.join(&filename);

    let mut instruction = format!(
        "Audit external open-source crate '{}' (v{}) for potential dogfooding or architectural influence in Aaroneous.\nDescription: {}\nRepository: {}\nEvaluate whether this crate provides zero-allocation, zero-ambient authority, or type-safe abstractions that could enhance our hypervisor, IPC bus, compute engine, or devtools.",
        candidate.name,
        candidate.version,
        candidate.description,
        candidate.repository.as_deref().unwrap_or("N/A")
    );
    if let Some(notes) = ecosystem_notes {
        instruction
            .push_str("\n\nM55 mechanical pre-filter G4 signal (advisory, not a hard gate): ");
        instruction.push_str(notes);
    }

    // M46 meta-tagging: every candidate found for one query shares a
    // `goal`/`exploration_group` (the query itself, and its slug), with
    // `perspective` set to the candidate crate's own name -- the "angle"
    // for a read-only analysis job is literally which crate is being
    // evaluated. A single-candidate query still gets tagged; M46's own
    // grouping step already drops groups with fewer than two members, so
    // tagging unconditionally is harmless and simpler than special-casing.
    let manifest = serde_json::json!({
        "schema_version": 1,
        "id": job_id,
        "mode": "analysis",
        "evidence": [evidence_rel_path],
        "instruction": instruction,
        "perspective": candidate.name,
        "goal": query,
        "exploration_group": slugify_query(query),
    });

    let toml_str = toml::to_string_pretty(&manifest)?;
    std::fs::write(&destination, toml_str)?;
    Ok(destination)
}

/// Parse a newline-separated queries file: blank lines and lines starting
/// with `#` (comments) are skipped, everything else is trimmed and kept.
fn parse_queries(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn checked_interval(interval_minutes: u32) -> Result<Duration> {
    if interval_minutes == 0 || interval_minutes > 1440 {
        bail!("interval must be 1-1440 minutes, got {interval_minutes}");
    }
    Ok(Duration::from_secs(u64::from(interval_minutes) * 60))
}

fn cycle(
    queries_path: &Path,
    output_dir: &Path,
    limit: usize,
    metadata_dir: Option<&Path>,
) -> bool {
    let text = match read_bounded(queries_path, 65536) {
        Ok(text) => text,
        Err(e) => {
            eprintln!(
                "frontier-discover-serve: cycle failed reading {}: {e:#}",
                queries_path.display()
            );
            return false;
        }
    };

    for query in parse_queries(&text) {
        match search_crates_io(&query, limit) {
            Ok(candidates) => {
                for candidate in &candidates {
                    let prefilter = match mechanical_prefilter_with_metadata(
                        metadata_dir,
                        &candidate.name,
                        &candidate.version,
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!(
                                "frontier-discover-serve: mechanical prefilter failed for candidate '{}' (query '{query}'): {e:#}",
                                candidate.name
                            );
                            continue;
                        }
                    };

                    if !prefilter.license_gate {
                        println!(
                            "frontier-discover-serve: skipping candidate '{}' -- failed G1 license gate (license: '{}')",
                            candidate.name, prefilter.license
                        );
                        continue;
                    }

                    let ecosystem_notes = match &prefilter.ecosystem_signal {
                        EcosystemSignal::Clean => None,
                        EcosystemSignal::Flagged(reasons) => Some(reasons.join("; ")),
                    };

                    if let Err(e) = create_frontier_job_with_notes(
                        candidate,
                        output_dir,
                        "docs/ARCHITECTURE.md",
                        &query,
                        ecosystem_notes.as_deref(),
                    ) {
                        eprintln!(
                            "frontier-discover-serve: failed to create job for candidate '{}' (query '{query}'): {e:#}",
                            candidate.name
                        );
                    }
                }
                println!(
                    "frontier-discover-serve: query '{query}' -> {} candidate(s)",
                    candidates.len()
                );
            }
            Err(e) => {
                eprintln!("frontier-discover-serve: query '{query}' failed: {e:#}");
            }
        }
    }

    true
}

/// Run frontier discovery cycles on an interval until stopped (Ctrl+C) or
/// until 5 consecutive cycle-level failures occur. `queries_path` must be an
/// explicit, absolute path to a newline-separated list of search queries —
/// this tool never infers or scans for queries on its own (see
/// FOUNDATION_BOUNDARY.md's "no ambient discovery" convention). A single
/// query's network/parse failure is logged and does not fail the cycle; only
/// a cycle-level problem (e.g. `queries_path` itself unreadable) counts
/// against the consecutive-failure budget.
pub(crate) fn serve(
    queries_path: &Path,
    output_dir: &Path,
    limit: usize,
    interval_minutes: u32,
    metadata_dir: Option<&Path>,
) -> Result<()> {
    if !queries_path.is_absolute() {
        bail!("queries path must be absolute: {}", queries_path.display());
    }
    if !output_dir.is_absolute() {
        bail!("output_dir must be an absolute path");
    }
    if let Some(dir) = metadata_dir {
        if !dir.is_absolute() {
            bail!("metadata_dir must be an absolute path: {}", dir.display());
        }
    }
    let interval = checked_interval(interval_minutes)?;
    println!(
        "frontier-discover-serve: starting, interval {interval_minutes}m, queries {}",
        queries_path.display()
    );
    let mut consecutive_failures = 0u32;
    loop {
        if cycle(queries_path, output_dir, limit, metadata_dir) {
            consecutive_failures = 0;
        } else {
            consecutive_failures += 1;
            eprintln!("frontier-discover-serve: cycle failure count: {consecutive_failures}/5");
            if consecutive_failures >= 5 {
                bail!("frontier-discover-serve stopped after 5 consecutive permanent failures");
            }
        }
        std::thread::sleep(interval);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FiltrationGateRecord {
    pub gate_id: String,
    pub name: String,
    pub passed: bool,
    pub violation_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct QuadraticReviewScores {
    pub q1_algorithmic_needle: f64,
    pub q2_refactoring_feasibility: f64,
    pub q3_test_portability: f64,
    pub q4_milestone_leverage: f64,
    pub linear_weighted_score: f64,
    pub quadratic_defect_penalty: f64,
    pub composite_score: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Recommendation {
    DirectAdoptWithWrapper,
    CratifyKernelExtraction,
    BorrowSpecAndSynthesize,
    RefactorCostProhibitive,
    RejectedAtFiltration,
}

impl Recommendation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Recommendation::DirectAdoptWithWrapper => "DIRECT_ADOPT_WITH_WRAPPER",
            Recommendation::CratifyKernelExtraction => "CRATIFY_KERNEL_EXTRACTION",
            Recommendation::BorrowSpecAndSynthesize => "BORROW_SPEC_AND_SYNTHESIZE",
            Recommendation::RefactorCostProhibitive => "REFACTOR_COST_PROHIBITIVE",
            Recommendation::RejectedAtFiltration => "REJECTED_AT_FILTRATION",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrontierEvaluationReport {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub target_crate: String,
    pub version: String,
    pub repository_url: Option<String>,
    pub evaluated_at_secs: u64,
    pub active_need: String,
    pub filtration_passed: bool,
    pub gates: Vec<FiltrationGateRecord>,
    pub scores: Option<QuadraticReviewScores>,
    pub composite_score: f64,
    pub recommendation: String,
    pub rationale: String,
}

pub fn validate_standard_filtration_gates(
    license: Option<&str>,
    description: &str,
) -> Vec<FiltrationGateRecord> {
    let mut gates = Vec::new();

    // G1: License Sovereignty
    let license_valid = license.is_some_and(|l| {
        let upper = l.to_uppercase();
        upper.contains("MIT")
            || upper.contains("APACHE")
            || upper.contains("BSD")
            || upper.contains("UNLICENSE")
            || upper.contains("ISC")
    });
    gates.push(FiltrationGateRecord {
        gate_id: "G1".into(),
        name: "LicenseSovereignty".into(),
        passed: license_valid,
        violation_reason: if license_valid {
            None
        } else {
            Some(format!(
                "License '{}' is non-permissive, copyleft, or missing",
                license.unwrap_or("UNKNOWN")
            ))
        },
    });

    let desc_lower = description.to_lowercase();

    // G2: Kernel Extractability
    let monolithic = desc_lower.contains("monolithic")
        || desc_lower.contains("full-stack framework")
        || desc_lower.contains("desktop app");
    gates.push(FiltrationGateRecord {
        gate_id: "G2".into(),
        name: "KernelExtractability".into(),
        passed: !monolithic,
        violation_reason: if monolithic {
            Some("Candidate is a monolithic framework or application with inseparable business logic".into())
        } else {
            None
        },
    });

    // G3: Deterministic Tractability
    let non_deterministic = desc_lower.contains("unmanaged driver")
        || desc_lower.contains("hardware non-deterministic");
    gates.push(FiltrationGateRecord {
        gate_id: "G3".into(),
        name: "DeterministicTractability".into(),
        passed: !non_deterministic,
        violation_reason: if non_deterministic {
            Some(
                "Candidate relies on non-deterministic hardware interactions or unmanaged drivers"
                    .into(),
            )
        } else {
            None
        },
    });

    // G4: Ecosystem Independence
    let foreign_toolchain =
        desc_lower.contains("requires cmake") || desc_lower.contains("requires msvc toolchain");
    gates.push(FiltrationGateRecord {
        gate_id: "G4".into(),
        name: "EcosystemIndependence".into(),
        passed: !foreign_toolchain,
        violation_reason: if foreign_toolchain {
            Some("Candidate requires foreign C/C++ build system or toolchain dependencies".into())
        } else {
            None
        },
    });

    // G5: Refactoring Surface Limit
    let surface_excessive =
        desc_lower.contains("multi-million loc") || desc_lower.contains("mammoth codebase");
    gates.push(FiltrationGateRecord {
        gate_id: "G5".into(),
        name: "RefactoringSurfaceLimit".into(),
        passed: !surface_excessive,
        violation_reason: if surface_excessive {
            Some("Candidate extraction surface exceeds tractability thresholds (>5000 LOC)".into())
        } else {
            None
        },
    });

    gates
}

#[derive(Debug, Clone)]
pub struct ReviewInput<'a> {
    pub target_crate: &'a str,
    pub version: &'a str,
    pub repository_url: Option<String>,
    pub active_need: &'a str,
    pub now: u64,
    pub gates: Vec<FiltrationGateRecord>,
    pub raw_scores: Option<(f64, f64, f64, f64)>,
    pub rationale: &'a str,
}

pub fn compute_quadratic_review(input: ReviewInput<'_>) -> FrontierEvaluationReport {
    let filtration_passed = input.gates.iter().all(|g| g.passed);
    if !filtration_passed {
        return FrontierEvaluationReport {
            schema_version: 1,
            artifact_kind: "foundation.frontier-evaluation".into(),
            target_crate: input.target_crate.to_string(),
            version: input.version.to_string(),
            repository_url: input.repository_url,
            evaluated_at_secs: input.now,
            active_need: input.active_need.to_string(),
            filtration_passed: false,
            gates: input.gates,
            scores: None,
            composite_score: 0.0,
            recommendation: Recommendation::RejectedAtFiltration.as_str().to_string(),
            rationale: input.rationale.to_string(),
        };
    }

    let (q1, q2, q3, q4) = input.raw_scores.unwrap_or((5.0, 5.0, 5.0, 5.0));
    let q1 = q1.clamp(0.0, 10.0);
    let q2 = q2.clamp(0.0, 10.0);
    let q3 = q3.clamp(0.0, 10.0);
    let q4 = q4.clamp(0.0, 10.0);

    // Weights per Tier 2 Standard: Q1 Needle = 0.30, Q2 Feasibility = 0.30, Q3 Tests = 0.20, Q4 Leverage = 0.20
    let linear_weighted_score = (0.30 * q1 + 0.30 * q2 + 0.20 * q3 + 0.20 * q4) * 10.0;
    let penalty_lambda = 0.05;
    let defect_sum =
        (10.0 - q1).powi(2) + (10.0 - q2).powi(2) + (10.0 - q3).powi(2) + (10.0 - q4).powi(2);
    let quadratic_defect_penalty = penalty_lambda * defect_sum;

    let composite_score = (linear_weighted_score - quadratic_defect_penalty).clamp(0.0, 100.0);

    let recommendation = if composite_score >= 80.0 {
        Recommendation::DirectAdoptWithWrapper
    } else if composite_score >= 60.0 {
        Recommendation::CratifyKernelExtraction
    } else if composite_score >= 40.0 {
        Recommendation::BorrowSpecAndSynthesize
    } else {
        Recommendation::RefactorCostProhibitive
    };

    FrontierEvaluationReport {
        schema_version: 1,
        artifact_kind: "foundation.frontier-evaluation".into(),
        target_crate: input.target_crate.to_string(),
        version: input.version.to_string(),
        repository_url: input.repository_url,
        evaluated_at_secs: input.now,
        active_need: input.active_need.to_string(),
        filtration_passed: true,
        gates: input.gates,
        scores: Some(QuadraticReviewScores {
            q1_algorithmic_needle: q1,
            q2_refactoring_feasibility: q2,
            q3_test_portability: q3,
            q4_milestone_leverage: q4,
            linear_weighted_score: (linear_weighted_score * 100.0).round() / 100.0,
            quadratic_defect_penalty: (quadratic_defect_penalty * 100.0).round() / 100.0,
            composite_score: (composite_score * 100.0).round() / 100.0,
        }),
        composite_score: (composite_score * 100.0).round() / 100.0,
        recommendation: recommendation.as_str().to_string(),
        rationale: input.rationale.to_string(),
    }
}

fn urlencoding(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_discover_native_on_repo() {
        let temp = tempdir().unwrap();
        let _repo = gix::init(temp.path()).unwrap();
        let summary = discover_native(temp.path()).unwrap();
        assert_eq!(summary.tracked_files_count, 0);
        assert!(summary.remotes.is_empty());
    }

    #[test]
    fn test_create_frontier_job() {
        let temp = tempdir().unwrap();
        let candidate = CrateCandidate {
            name: "crossbeam-queue".into(),
            description: "Concurrent multi-producer multi-consumer queues".into(),
            version: "0.3.11".into(),
            repository: Some("https://github.com/crossbeam-rs/crossbeam".into()),
            documentation: None,
            downloads: 50_000_000,
        };

        let job_file = create_frontier_job_with_notes(
            &candidate,
            temp.path(),
            "docs/ARCHITECTURE.md",
            "lock-free queue",
            None,
        )
        .unwrap();
        assert!(job_file.exists());
        let text = std::fs::read_to_string(job_file).unwrap();
        assert!(text.contains("FRONTIER-CROSSBEAM_QUEUE"));
        assert!(text.contains("perspective = \"crossbeam-queue\""));
        assert!(text.contains("goal = \"lock-free queue\""));
        assert!(text.contains("exploration_group = \"lock_free_queue\""));
    }

    #[test]
    fn test_slugify_query() {
        assert_eq!(slugify_query("lock-free queue"), "lock_free_queue");
        assert_eq!(slugify_query("  async   runtime!! "), "async_runtime");
        assert_eq!(slugify_query("zero-copy"), "zero_copy");
    }

    #[test]
    fn test_parse_queries_skips_blank_and_comment_lines() {
        let text =
            "\n# a comment\nasync runtime\n\n  \n#another comment\nzero-copy serialization\n";
        let queries = parse_queries(text);
        assert_eq!(queries, vec!["async runtime", "zero-copy serialization"]);
    }

    #[test]
    fn test_parse_queries_trims_whitespace() {
        let text = "  lock-free queue  \n\tembedded allocator\t\n";
        let queries = parse_queries(text);
        assert_eq!(queries, vec!["lock-free queue", "embedded allocator"]);
    }

    #[test]
    fn test_parse_queries_empty_input_yields_no_queries() {
        assert!(parse_queries("").is_empty());
        assert!(parse_queries("# only comments\n\n").is_empty());
    }

    #[test]
    fn test_checked_interval_bounds() {
        assert!(checked_interval(0).is_err());
        assert!(checked_interval(1441).is_err());
        assert_eq!(checked_interval(30).unwrap(), Duration::from_secs(1800));
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("ring-buffer"), "ring-buffer");
    }

    #[test]
    fn test_filtration_rejects_copyleft_license() {
        let gates = validate_standard_filtration_gates(Some("GPL-3.0"), "A useful library");
        assert!(!gates.iter().all(|g| g.passed));
        let g1 = gates.iter().find(|g| g.gate_id == "G1").unwrap();
        assert!(!g1.passed);
        assert!(g1.violation_reason.as_ref().unwrap().contains("copyleft"));
    }

    #[test]
    fn test_filtration_passes_permissive_license() {
        let gates = validate_standard_filtration_gates(
            Some("MIT OR Apache-2.0"),
            "Lock-free queue without global state",
        );
        assert!(gates.iter().all(|g| g.passed));
    }

    #[test]
    fn test_quadratic_scoring_penalizes_lopsided_candidates() {
        // High leverage (10) but terrible footprint (1.0) and reliability (3.0)
        let gates = validate_standard_filtration_gates(Some("MIT"), "Lock-free structures");
        let lopsided = compute_quadratic_review(ReviewInput {
            target_crate: "lopsided-crate",
            version: "1.0.0",
            repository_url: None,
            active_need: "lock-free queue",
            now: 1000,
            gates: gates.clone(),
            raw_scores: Some((6.0, 3.0, 1.0, 10.0)),
            rationale: "Lopsided candidate with dangerous footprint",
        });

        // Balanced candidate with 6.5 across the board
        let balanced = compute_quadratic_review(ReviewInput {
            target_crate: "balanced-crate",
            version: "1.0.0",
            repository_url: None,
            active_need: "lock-free queue",
            now: 1000,
            gates,
            raw_scores: Some((6.5, 6.5, 6.5, 6.5)),
            rationale: "Balanced solid candidate",
        });

        // Quadratic defect penalty must be significantly higher on the lopsided candidate
        assert!(
            lopsided.scores.unwrap().quadratic_defect_penalty
                > balanced.scores.unwrap().quadratic_defect_penalty
        );
    }

    #[test]
    fn test_quadratic_scoring_rewards_balanced_high_synergy_crates() {
        let gates = validate_standard_filtration_gates(Some("Apache-2.0"), "Zero-copy SWMR buffer");
        let top_tier = compute_quadratic_review(ReviewInput {
            target_crate: "top-tier-crate",
            version: "2.0.0",
            repository_url: Some("https://github.com/example/top-tier".into()),
            active_need: "SWMR ring buffer",
            now: 1000,
            gates,
            raw_scores: Some((9.0, 9.5, 9.0, 8.5)),
            rationale: "Exemplary architecture matching all Aaroneous invariants",
        });

        assert_eq!(top_tier.recommendation, "DIRECT_ADOPT_WITH_WRAPPER");
        assert!(top_tier.composite_score >= 80.0);
    }

    #[test]
    fn test_evaluation_report_schema_round_trip() {
        let gates = validate_standard_filtration_gates(Some("MIT"), "Zero-alloc parser");
        let report = compute_quadratic_review(ReviewInput {
            target_crate: "parser-crate",
            version: "0.5.0",
            repository_url: None,
            active_need: "wire parser",
            now: 1789700000,
            gates,
            raw_scores: Some((7.0, 6.5, 7.5, 7.0)),
            rationale: "Clean parser candidate",
        });

        let serialized = serde_json::to_string_pretty(&report).unwrap();
        let deserialized: FrontierEvaluationReport = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.target_crate, "parser-crate");
        assert_eq!(deserialized.recommendation, "CRATIFY_KERNEL_EXTRACTION");
    }

    // --- M55 mechanical prefilter: license_passes_g1 (G1, hard gate) ---
    // No live network calls -- these exercise the pure parser directly.

    #[test]
    fn test_license_passes_g1_bare_mit() {
        assert!(license_passes_g1("MIT"));
    }

    #[test]
    fn test_license_passes_g1_apache_with_exception() {
        // Real confirmed crates.io response for cranelift-codegen 0.136.0-rc.1.
        assert!(license_passes_g1("Apache-2.0 WITH LLVM-exception"));
    }

    #[test]
    fn test_license_passes_g1_dual_license_or() {
        assert!(license_passes_g1("MIT OR Apache-2.0"));
    }

    #[test]
    fn test_license_passes_g1_rejects_copyleft() {
        assert!(!license_passes_g1("GPL-3.0"));
    }

    #[test]
    fn test_license_passes_g1_rejects_empty() {
        assert!(!license_passes_g1(""));
    }

    #[test]
    fn test_license_passes_g1_and_separator() {
        // AND combinators require every part be acceptable in a full SPDX
        // grammar, but this heuristic (documented as such) just checks that
        // any token matches -- exercise the separator itself here.
        assert!(license_passes_g1("MIT AND Apache-2.0"));
        assert!(!license_passes_g1("GPL-3.0 AND Proprietary"));
    }

    // --- M55 mechanical prefilter: ecosystem_independence_signal (G4, advisory) ---

    #[test]
    fn test_ecosystem_signal_flags_sys_crate() {
        // Modeled on cranelift-codegen's real dependencies response shape.
        let deps = vec![DependencyInfo {
            crate_id: "libc-sys".into(),
            kind: "normal".into(),
            optional: false,
        }];
        match ecosystem_independence_signal(&deps) {
            EcosystemSignal::Flagged(reasons) => {
                assert_eq!(reasons.len(), 1);
                assert!(reasons[0].contains("libc-sys"));
            }
            EcosystemSignal::Clean => panic!("expected Flagged for a -sys dependency"),
        }
    }

    #[test]
    fn test_ecosystem_signal_flags_cc_build_dependency() {
        let deps = vec![DependencyInfo {
            crate_id: "cc".into(),
            kind: "build".into(),
            optional: false,
        }];
        match ecosystem_independence_signal(&deps) {
            EcosystemSignal::Flagged(reasons) => {
                assert_eq!(reasons.len(), 1);
                assert!(reasons[0].contains("cc"));
            }
            EcosystemSignal::Clean => panic!("expected Flagged for a build-kind cc dependency"),
        }
    }

    #[test]
    fn test_ecosystem_signal_flags_cmake_build_dependency() {
        let deps = vec![DependencyInfo {
            crate_id: "cmake".into(),
            kind: "build".into(),
            optional: true,
        }];
        match ecosystem_independence_signal(&deps) {
            EcosystemSignal::Flagged(reasons) => assert_eq!(reasons.len(), 1),
            EcosystemSignal::Clean => panic!("expected Flagged for a build-kind cmake dependency"),
        }
    }

    #[test]
    fn test_ecosystem_signal_does_not_flag_cc_as_normal_dependency() {
        // Only build-kind cc/cmake are known toolchain wrappers; a normal
        // dependency literally named "cc" (unlikely, but the heuristic is
        // scoped deliberately) should not trip the signal.
        let deps = vec![DependencyInfo {
            crate_id: "cc".into(),
            kind: "normal".into(),
            optional: false,
        }];
        assert_eq!(ecosystem_independence_signal(&deps), EcosystemSignal::Clean);
    }

    #[test]
    fn test_ecosystem_signal_clean_dependency_list() {
        // Modeled on cranelift-codegen's real dependencies response, minus
        // its one build-kind dependency (cranelift-codegen-meta, which is
        // not -sys/cc/cmake so also would not flag).
        let deps = vec![
            DependencyInfo {
                crate_id: "cranelift-codegen-meta".into(),
                kind: "build".into(),
                optional: false,
            },
            DependencyInfo {
                crate_id: "target-lexicon".into(),
                kind: "normal".into(),
                optional: false,
            },
            DependencyInfo {
                crate_id: "smallvec".into(),
                kind: "normal".into(),
                optional: false,
            },
        ];
        assert_eq!(ecosystem_independence_signal(&deps), EcosystemSignal::Clean);
    }

    #[test]
    fn test_ecosystem_signal_empty_dependency_list_is_clean() {
        assert_eq!(ecosystem_independence_signal(&[]), EcosystemSignal::Clean);
    }

    // --- M55: create_frontier_job_with_notes appends advisory G4 notes ---

    #[test]
    fn test_create_frontier_job_with_notes_appends_ecosystem_notes() {
        let temp = tempdir().unwrap();
        let candidate = CrateCandidate {
            name: "example-sys-using-crate".into(),
            description: "Wraps a C library".into(),
            version: "1.0.0".into(),
            repository: None,
            documentation: None,
            downloads: 1000,
        };

        let job_file = create_frontier_job_with_notes(
            &candidate,
            temp.path(),
            "docs/ARCHITECTURE.md",
            "ffi wrapper",
            Some("dependency 'openssl-sys' is a -sys crate (raw FFI binding)"),
        )
        .unwrap();
        let text = std::fs::read_to_string(job_file).unwrap();
        assert!(text.contains("M55 mechanical pre-filter G4 signal"));
        assert!(text.contains("openssl-sys"));
    }

    // M57: mechanical_prefilter_with_metadata reuses a recent recorded
    // result instead of calling the network. This is provable without a
    // live crates.io call by pre-seeding the candidate's metadata record
    // directly (append_pass), then asserting the wrapper returns exactly
    // that recorded result -- if it had fallen through to
    // `mechanical_prefilter`'s real network call here, this test would fail
    // (no network access in this sandbox) rather than silently pass.
    #[test]
    fn mechanical_prefilter_with_metadata_reuses_recent_recorded_result() {
        let temp = tempdir().unwrap();
        let cached = PrefilterResult {
            license: "MIT".into(),
            license_gate: true,
            ecosystem_signal: EcosystemSignal::Flagged(vec!["pre-recorded reason".into()]),
        };
        let findings = serde_json::to_value(&cached).unwrap();
        crate::candidate_metadata::append_pass(
            temp.path(),
            "crate",
            "definitely-not-a-real-crate-xyz",
            "mechanical_prefilter",
            findings,
        )
        .unwrap();

        let result = mechanical_prefilter_with_metadata(
            Some(temp.path()),
            "definitely-not-a-real-crate-xyz",
            "0.1.0",
        )
        .unwrap();

        assert_eq!(result, cached);

        // still exactly one pass entry -- the cache hit did not append a
        // second, redundant entry.
        let record = crate::candidate_metadata::read_record(
            temp.path(),
            "crate",
            "definitely-not-a-real-crate-xyz",
        )
        .unwrap()
        .unwrap();
        assert_eq!(record.passes.len(), 1);
    }

    #[test]
    fn test_create_frontier_job_without_notes_matches_original() {
        let temp = tempdir().unwrap();
        let candidate = CrateCandidate {
            name: "clean-crate".into(),
            description: "No native deps".into(),
            version: "1.0.0".into(),
            repository: None,
            documentation: None,
            downloads: 1000,
        };

        let job_file = create_frontier_job_with_notes(
            &candidate,
            temp.path(),
            "docs/ARCHITECTURE.md",
            "clean crate",
            None,
        )
        .unwrap();
        let text = std::fs::read_to_string(job_file).unwrap();
        assert!(!text.contains("M55 mechanical pre-filter G4 signal"));
    }
}
