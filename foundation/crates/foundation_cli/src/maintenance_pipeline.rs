use crate::component_inventory;
use crate::local_worker;
use crate::virtual_patch::sha256_digest;
use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Subcommand, Debug, Clone)]
pub enum Action {
    /// Refresh component inventory facts, source file index, and evidence captures.
    Refresh {
        #[arg(long)]
        runtime_root: PathBuf,
        #[arg(long)]
        checkout: PathBuf,
        #[arg(long = "component")]
        components: Vec<String>,
    },
    /// Mechanically validate a model proposal JSON against stage contract bounds.
    CheckStage {
        #[arg(long)]
        result_path: PathBuf,
        #[arg(long, default_value = "plan")]
        stage: String,
        #[arg(long, default_value = "inventory-integrity")]
        task_kind: String,
    },
    /// Advance the pipeline state machine, publish jobs, validate decisions and archive outputs.
    Advance {
        #[arg(long)]
        runtime_root: PathBuf,
    },
    /// Collect read-only housekeeping retention telemetry and generate housekeeping.json report.
    Housekeeping {
        #[arg(long)]
        runtime_root: PathBuf,
    },
    /// Continuous maintenance daemon runner.
    Daemon {
        #[arg(long)]
        runtime_root: PathBuf,
        #[arg(long)]
        checkout: Option<PathBuf>,
        #[arg(long)]
        serve: bool,
    },
    /// Read-only status report across loop lease, control, pipeline state, and last cycle.
    Status {
        #[arg(long)]
        runtime_root: PathBuf,
    },
    /// Update loop-control state (run, pause, restart).
    Control {
        #[arg(long)]
        runtime_root: PathBuf,
        #[arg(long)]
        state: String,
    },
}

pub fn run(action: Action) -> Result<()> {
    match action {
        Action::Refresh {
            runtime_root,
            checkout,
            components,
        } => {
            let comps = if components.is_empty() {
                default_components()
            } else {
                components
            };
            let refresh = refresh_inventory(&runtime_root, &checkout, &comps)?;
            println!(
                "evidence_digest={}; new_capture={}",
                refresh.evidence_digest, refresh.new_capture
            );
            Ok(())
        }
        Action::CheckStage {
            result_path,
            stage,
            task_kind,
        } => {
            let report = check_stage_contract(&result_path, &stage, &task_kind)?;
            let json = serde_json::to_string_pretty(&report)?;
            println!("{json}");
            if !report.mechanical_pass {
                bail!("stage contract check failed");
            }
            Ok(())
        }
        Action::Advance { runtime_root } => {
            let msg = advance_pipeline(&runtime_root)?;
            println!("{msg}");
            Ok(())
        }
        Action::Housekeeping { runtime_root } => {
            let report = housekeeping(&runtime_root)?;
            println!(
                "housekeeping report only; pending={}; deferred={}; deletion=false",
                report.pending_results, report.deferred_findings
            );
            Ok(())
        }
        Action::Daemon {
            runtime_root,
            checkout,
            serve,
        } => daemon(&runtime_root, checkout.as_deref(), serve),
        Action::Status { runtime_root } => status(&runtime_root),
        Action::Control {
            runtime_root,
            state,
        } => control(&runtime_root, &state),
    }
}

pub fn default_components() -> Vec<String> {
    vec![
        "crates/local_inference".to_string(),
        "crates/llm_gateway".to_string(),
        "crates/paths".to_string(),
    ]
}

// ============================================================================
// Time & Hashing Utilities
// ============================================================================

pub fn unix_secs_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn format_iso8601(secs: u64) -> String {
    let days = secs / 86400;
    let rem_secs = secs % 86400;
    let hours = rem_secs / 3600;
    let mins = (rem_secs % 3600) / 60;
    let s = rem_secs % 60;

    let mut y = 1970;
    let mut d = days;
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
    let days_in_months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 1;
    for &dim in &days_in_months {
        if d < dim {
            break;
        }
        d -= dim;
        m += 1;
    }
    let day = d + 1;
    format!("{y:04}-{m:02}-{day:02}T{hours:02}:{mins:02}:{s:02}Z")
}

pub fn format_compact_stamp(secs: u64) -> String {
    let days = secs / 86400;
    let rem_secs = secs % 86400;
    let hours = rem_secs / 3600;
    let mins = (rem_secs % 3600) / 60;
    let s = rem_secs % 60;

    let mut y = 1970;
    let mut d = days;
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
    let days_in_months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 1;
    for &dim in &days_in_months {
        if d < dim {
            break;
        }
        d -= dim;
        m += 1;
    }
    let day = d + 1;
    format!("{y:04}{m:02}{day:02}T{hours:02}{mins:02}{s:02}Z")
}

/// Validate seat identifier: 1-32 chars, lowercase alphanumeric, hyphens, or underscores.
pub fn is_valid_seat_id(seat: &str) -> bool {
    let trimmed = seat.trim();
    if trimmed.is_empty() || trimmed.len() > 32 {
        return false;
    }
    trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(sha256_digest(&bytes))
}

pub fn sha256_file_uppercase(path: &Path) -> Result<String> {
    Ok(sha256_file(path)?.to_uppercase())
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("target file must have a parent directory")?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        "{}.{}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("artifact"),
        sha256_digest(format!("{:?}-{}", path, unix_secs_now()).as_bytes())
            .get(0..8)
            .unwrap_or("tmp")
    ));
    fs::write(&tmp, bytes).with_context(|| format!("failed to write {}", tmp.display()))?;
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e).with_context(|| {
            format!(
                "failed to replace {} with {}",
                path.display(),
                tmp.display()
            )
        });
    }
    Ok(())
}

pub fn save_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_vec_pretty(value)?;
    write_atomic(path, &json)
}

struct FileLockGuard {
    path: PathBuf,
    file: Option<File>,
}

impl FileLockGuard {
    fn try_acquire(path: &Path) -> Result<Option<Self>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        match File::options().write(true).create_new(true).open(path) {
            Ok(file) => Ok(Some(Self {
                path: path.to_path_buf(),
                file: Some(file),
            })),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        drop(self.file.take());
        let _ = fs::remove_file(&self.path);
    }
}

// Missing files permit initialization; every other read/parse failure preserves
// the artifact and stops the caller rather than resetting persistent decisions.
fn read_optional_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("cannot inspect {}", path.display()));
        }
        Ok(metadata) if !metadata.is_file() => {
            bail!(
                "expected regular JSON file at {}; retained for recovery",
                path.display()
            );
        }
        Ok(_) => {}
    }
    let text = crate::intake::read_bounded(path, 65536)
        .with_context(|| format!("cannot read {}; retained for recovery", path.display()))?;
    serde_json::from_str(&text)
        .map(Some)
        .with_context(|| format!("invalid JSON at {}; retained for recovery", path.display()))
}

fn load_pipeline_state(path: &Path) -> Result<Option<PipelineState>> {
    let state: Option<PipelineState> = read_optional_json(path)?;
    if let Some(state) = &state {
        if state.schema_version != 1
            || state.attempt > 1
            || !matches!(
                state.stage.as_str(),
                "plan-ready"
                    | "brief-ready"
                    | "awaiting-independent-review"
                    | "blocked-transport"
                    | "blocked-contract"
                    | "blocked-calibration"
                    | "handed-off"
            )
        {
            bail!(
                "unsupported pipeline state at {}; retained for recovery",
                path.display()
            );
        }
    }
    Ok(state)
}

fn load_loop_control(path: &Path) -> Result<Option<LoopControl>> {
    let control: Option<LoopControl> = read_optional_json(path)?;
    if let Some(control) = &control {
        if !matches!(control.state.as_str(), "run" | "pause" | "restart") {
            bail!(
                "invalid loop control state at {}; retained for recovery",
                path.display()
            );
        }
    }
    Ok(control)
}

fn daemon_control(root: &Path) -> Result<LoopControl> {
    Ok(
        load_loop_control(&root.join("loop-control.json"))?.unwrap_or(LoopControl {
            state: "run".to_string(),
            requested_at_utc: None,
            route: None,
        }),
    )
}

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceIndexEntry {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceIndex {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub checkout: String,
    pub scope: Vec<String>,
    pub files: Vec<SourceIndexEntry>,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InlineTestMarker {
    pub path: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentFacts {
    pub component: String,
    pub rust_source_files: usize,
    pub separate_test_files: usize,
    pub declared_dependency_names: usize,
    pub inline_test_marker_count: usize,
    pub inline_test_markers: Vec<InlineTestMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryFacts {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub total_inline_test_markers: usize,
    pub components: Vec<ComponentFacts>,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshCommandLog {
    pub executable: String,
    pub arguments: Vec<String>,
    pub exit_code: i32,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastRefresh {
    pub schema_version: u32,
    pub observed_at_utc: String,
    pub evidence_digest: String,
    pub new_capture: bool,
    pub commands: Vec<RefreshCommandLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageContractCheckReport {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub result_sha256: String,
    pub job_id: String,
    pub stage: String,
    pub task_kind: String,
    pub mechanical_pass: bool,
    pub findings: Vec<String>,
    pub semantic_acceptance: bool,
    pub limits: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    pub schema_version: u32,
    pub task_kind: String,
    pub cycle_id: String,
    pub evidence_key: String,
    pub stage: String,
    pub attempt: u32,
    pub plan_job: String,
    pub brief_job: String,
    pub plan_result: String,
    pub brief_result: String,
    pub plan_sha256: String,
    pub brief_sha256: String,
    pub decision_sha256: String,
    pub next_action: String,
    pub whole_job_accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HousekeepingReport {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub observed_at_utc: String,
    pub pipeline_stage: String,
    pub pending_results: usize,
    pub archived_results: usize,
    pub deferred_findings: usize,
    pub reviewed_handoffs: usize,
    pub retained_files: usize,
    pub retained_bytes: u64,
    pub partial_files: Vec<String>,
    pub deletion_enabled: bool,
    pub limits: Vec<String>,
    pub recommended_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopControl {
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_at_utc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopLease {
    pub pid: u32,
    pub started_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionReceipt {
    pub actor: String,
    pub independent_of: String,
    pub cycle_id: String,
    pub evidence_key: String,
    pub plan_sha256: String,
    pub brief_sha256: String,
    pub verdict: String,
    pub review_path: String,
    pub review_sha256: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub archive_reviewed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MechanicalCountEntry {
    pub component: String,
    pub source_files: usize,
    pub separate_test_files: usize,
    pub dependencies: usize,
    pub inline_attributes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTicket {
    pub schema_version: u32,
    pub cycle_id: String,
    pub responsibility: String,
    pub authority: String,
    pub non_overlap: String,
    pub evidence_key: String,
    pub mechanical_counts: Vec<MechanicalCountEntry>,
    pub limits: Vec<String>,
    pub independent_review: serde_json::Value,
    pub review_sha256: String,
    pub feedback: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub housekeeping_snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawProposal {
    task: String,
    observations: Vec<String>,
    risks: Vec<String>,
    test_plan: Vec<String>,
    recommended_next_step: String,
    rust_draft: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawRecord {
    job_id: String,
    outcome: String,
    #[serde(default)]
    submitted_at_secs: u64,
    proposal: Option<RawProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InventoryOutput {
    rust_sources: Vec<String>,
    test_sources: Vec<String>,
    dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobManifest {
    schema_version: u32,
    id: String,
    mode: String,
    evidence: Vec<String>,
    instruction: String,
}

// ============================================================================
// 1. Refresh Inventory & Source Index
// ============================================================================

pub fn is_test_marker(line: &str) -> bool {
    let t = line.trim_start();
    if !t.starts_with("#[") {
        return false;
    }
    let rest = t[2..].trim_start();
    if let Some(after) = rest.strip_prefix("cfg(test)") {
        after.trim_start().starts_with(']')
    } else if let Some(after) = rest.strip_prefix("test") {
        after.trim_start().starts_with(']')
    } else if let Some(after) = rest.strip_prefix("tokio::test") {
        after.trim_start().starts_with(']')
    } else {
        false
    }
}

pub fn refresh_inventory(
    runtime_root: &Path,
    checkout: &Path,
    components: &[String],
) -> Result<LastRefresh> {
    if !runtime_root.is_absolute() {
        bail!(
            "runtime_root must be an absolute path: {}",
            runtime_root.display()
        );
    }
    if !checkout.is_absolute() {
        bail!("checkout must be an absolute path: {}", checkout.display());
    }

    let evidence_dir = runtime_root.join("evidence");
    fs::create_dir_all(&evidence_dir)?;

    let deferred_findings_src = runtime_root.join("DEFERRED_FINDINGS.md");
    if deferred_findings_src.exists() {
        let dest = evidence_dir.join("deferred-findings.md");
        fs::copy(&deferred_findings_src, &dest)?;
    }

    let mut commands = Vec::new();
    let mut index_entries = Vec::new();
    let mut all_facts = Vec::new();

    for comp in components {
        let label = comp
            .split(['/', '\\'])
            .next_back()
            .context("component has no path segment")?;
        let report_path = evidence_dir.join(format!("{label}-inventory.json"));

        // Direct in-process execution of component_inventory
        component_inventory::run(checkout, Path::new(comp), &report_path)?;

        commands.push(RefreshCommandLog {
            executable: "foundation".to_string(),
            arguments: vec![
                "component-inventory".to_string(),
                "--checkout".to_string(),
                checkout.display().to_string(),
                "--crate-path".to_string(),
                comp.clone(),
                "--output".to_string(),
                report_path.display().to_string(),
            ],
            exit_code: 0,
            output: format!("wrote component inventory: {}", report_path.display()),
        });

        let report_data = fs::read_to_string(&report_path)?;
        let report: InventoryOutput = serde_json::from_str(&report_data)?;

        // Files to index: Cargo.toml + rust_sources + test_sources
        let mut rel_files = vec!["Cargo.toml".to_string()];
        rel_files.extend(report.rust_sources.iter().cloned());
        rel_files.extend(report.test_sources.iter().cloned());
        rel_files.sort();
        rel_files.dedup();

        for rel in rel_files {
            let exact_rel = format!("{comp}/{rel}");
            let file_path = checkout.join(&exact_rel);
            let sha = sha256_file(&file_path)?;
            index_entries.push(SourceIndexEntry {
                path: exact_rel,
                sha256: sha,
            });
        }

        // Inline test marker scanning
        let mut markers = Vec::new();
        for rel in &report.rust_sources {
            let file_path = checkout.join(comp).join(rel);
            if file_path.is_file() {
                let content = fs::read_to_string(&file_path)?;
                for (i, line) in content.lines().enumerate() {
                    if is_test_marker(line) {
                        markers.push(InlineTestMarker {
                            path: format!("{comp}/{rel}"),
                            line: i + 1,
                            text: line.trim().to_string(),
                        });
                    }
                }
            }
        }

        all_facts.push(ComponentFacts {
            component: comp.clone(),
            rust_source_files: report.rust_sources.len(),
            separate_test_files: report.test_sources.len(),
            declared_dependency_names: report.dependencies.len(),
            inline_test_marker_count: markers.len(),
            inline_test_markers: markers,
        });
    }

    let source_index = SourceIndex {
        schema_version: 1,
        artifact_kind: "maintenance.source-index".to_string(),
        checkout: checkout.display().to_string(),
        scope: components.to_vec(),
        files: index_entries,
        limits: vec![
            "Read-only inventory of declared components, not whole workspace.".to_string(),
            "test_sources lists separate test files; inline tests may be in rust_sources."
                .to_string(),
            "public_item_lines and hot_path_markers are lexical counts, not behavioral or compliance proof.".to_string(),
            "No cache retention, deletion, implementation or test execution is authorized by these reports.".to_string(),
        ],
    };
    save_json_atomic(&evidence_dir.join("source-index.json"), &source_index)?;

    let total_markers: usize = all_facts.iter().map(|f| f.inline_test_marker_count).sum();
    let inventory_facts = InventoryFacts {
        schema_version: 1,
        artifact_kind: "maintenance.inventory-facts".to_string(),
        total_inline_test_markers: total_markers,
        components: all_facts,
        limits: vec![
            "Inline attributes are lexical evidence of test declarations, not execution or coverage.".to_string(),
            "Dependency names include workspace and third-party dependencies; no complexity conclusion follows.".to_string(),
            "Zero separate test files does not mean zero tests or zero test coverage.".to_string(),
        ],
    };
    save_json_atomic(&evidence_dir.join("inventory-facts.json"), &inventory_facts)?;

    // Compute evidence digest across all evidence files
    let mut entries = Vec::new();
    for entry in fs::read_dir(&evidence_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            entries.push((name, path));
        }
    }
    entries.sort_by_key(|a| a.0.to_lowercase());

    let mut digest_lines = Vec::new();
    for (name, path) in &entries {
        let hash_upper = sha256_file_uppercase(path)?;
        digest_lines.push(format!("{name}:{hash_upper}"));
    }
    let digest_input = digest_lines.join("\n");
    let evidence_digest = sha256_digest(digest_input.as_bytes());

    let capture_dir = runtime_root.join("captures").join(&evidence_digest);
    let new_capture = !capture_dir.exists();
    if new_capture {
        fs::create_dir_all(&capture_dir)?;
        for (name, path) in &entries {
            fs::copy(path, capture_dir.join(name))?;
        }
    }

    let refresh = LastRefresh {
        schema_version: 1,
        observed_at_utc: format_iso8601(unix_secs_now()),
        evidence_digest,
        new_capture,
        commands,
    };
    save_json_atomic(&runtime_root.join("last-refresh.json"), &refresh)?;

    Ok(refresh)
}

// ============================================================================
// 2. Check Stage Contract
// ============================================================================

pub fn check_stage_contract(
    result_path: &Path,
    stage: &str,
    task_kind: &str,
) -> Result<StageContractCheckReport> {
    if !result_path.is_file() {
        bail!("result file does not exist: {}", result_path.display());
    }
    let content = fs::read_to_string(result_path)?;
    let rec: RawRecord = serde_json::from_str(&content)
        .with_context(|| format!("invalid JSON in {}", result_path.display()))?;

    let result_sha256 = sha256_digest(content.as_bytes());
    let mut findings = Vec::new();

    if rec.outcome == "failed" {
        findings.push("No completed proposal".to_string());
    } else if let Some(p) = &rec.proposal {
        if p.task != rec.job_id {
            findings.push("Proposal task does not bind exact native job_id".to_string());
        }
        if p.rust_draft.is_some() {
            findings.push("Analysis-only contract requires rust_draft null".to_string());
        }
        if p.observations.len() > 3 || p.risks.len() > 3 {
            findings.push("Stage exceeds approved observation/risk bounds".to_string());
        }
        if p.test_plan.is_empty() || p.test_plan.iter().any(|t| !t.starts_with("NOT RUN")) {
            findings.push("All proposed tests require NOT RUN prefix".to_string());
        }
        if stage == "brief" {
            if !p
                .recommended_next_step
                .to_lowercase()
                .contains("independent frontier review")
            {
                findings.push("Downstream must request independent frontier review".to_string());
            }
            if task_kind == "inventory-integrity" {
                if p.observations.len() != 3 {
                    findings.push("Inventory brief requires three proposed cases".to_string());
                }
                for obs in &p.observations {
                    for tag in ["CASE:", "INPUT:", "EXPECTED:", "REVIEW:"] {
                        if !obs.contains(tag) {
                            findings.push(format!("Case missing {tag}"));
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(StageContractCheckReport {
        schema_version: 1,
        artifact_kind: "maintenance.stage-contract-check".to_string(),
        result_sha256,
        job_id: rec.job_id,
        stage: stage.to_string(),
        task_kind: task_kind.to_string(),
        mechanical_pass: findings.is_empty(),
        findings,
        semantic_acceptance: false,
        limits: "Mechanical shape/binding only. Correctness, causality, completeness and authority require independent evidence review.".to_string(),
    })
}

// ============================================================================
// 3. Advance Pipeline
// ============================================================================

pub fn advance_pipeline(runtime_root: &Path) -> Result<String> {
    if !runtime_root.is_absolute() {
        bail!(
            "runtime_root must be an absolute path: {}",
            runtime_root.display()
        );
    }
    let pipe_dir = runtime_root.join("pipeline");
    fs::create_dir_all(&pipe_dir)?;

    let router_lock_path = pipe_dir.join("router.lock");
    let _router_lease = match FileLockGuard::try_acquire(&router_lock_path)? {
        Some(guard) => guard,
        None => return Ok("pipeline router already active".to_string()),
    };

    // Shared reservation check
    let res_dir = runtime_root
        .parent()
        .map(|p| p.join("reservation"))
        .unwrap_or_else(|| runtime_root.join("reservation"));
    let reservation_lock_path = res_dir.join("review.lock");
    let _reservation_guard = match FileLockGuard::try_acquire(&reservation_lock_path)? {
        Some(guard) => guard,
        None => return Ok("pipeline reserved".to_string()),
    };

    let state_path = pipe_dir.join("state.json");
    let mut state = load_pipeline_state(&state_path)?;

    let facts_path = runtime_root.join("evidence").join("inventory-facts.json");
    let index_path = runtime_root.join("evidence").join("source-index.json");
    if !facts_path.is_file() || !index_path.is_file() {
        bail!("evidence files missing; run refresh first");
    }

    let index_txt = fs::read_to_string(&index_path)?;
    let index: SourceIndex = serde_json::from_str(&index_txt)?;

    // Verify source files have not changed since capture
    let checkout = Path::new(&index.checkout);
    let mut stale = Vec::new();
    for entry in &index.files {
        let orig = checkout.join(&entry.path);
        if !orig.is_file() {
            stale.push(entry.path.clone());
        } else {
            let sha = sha256_file(&orig)?;
            if sha != entry.sha256 {
                stale.push(entry.path.clone());
            }
        }
    }
    if !stale.is_empty() {
        bail!(
            "Source changed after capture; dispatch blocked: {}",
            stale.join(", ")
        );
    }

    let seed_path = runtime_root
        .join("archive")
        .join("MAINTENANCE-INVENTORY-FRESHNESS-0000-1791060269.review.md");
    let seed_sha = if seed_path.is_file() {
        sha256_file(&seed_path)?
    } else {
        "".to_string()
    };
    let seed_content = if seed_path.is_file() {
        fs::read_to_string(&seed_path)?
    } else {
        "".to_string()
    };

    let facts_sha = sha256_file(&facts_path)?;
    let index_sha = sha256_file(&index_path)?;
    let mut key = sha256_digest(
        format!("connected-maintenance-v1|{facts_sha}|{index_sha}|{seed_sha}").as_bytes(),
    );
    let inventory_key = key.clone();

    if let Some(s) = &mut state {
        if s.task_kind.is_empty() {
            s.task_kind = "inventory-integrity".to_string();
        }
        if s.task_kind == "housekeeping-brief" {
            key = s.evidence_key.clone();
        }
    }

    let before_state = state.clone();
    let mut changed = false;

    let completed_path = pipe_dir.join("completed.json");
    let mut completed: BTreeMap<String, serde_json::Value> = if completed_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&completed_path)?).unwrap_or_default()
    } else {
        BTreeMap::new()
    };

    let house_path = pipe_dir.join("housekeeping.json");

    if let Some(s) = &state {
        if s.stage == "handed-off" || s.stage == "blocked-calibration" {
            completed.insert(
                s.evidence_key.clone(),
                serde_json::json!({
                    "task_kind": s.task_kind,
                    "stage": s.stage,
                    "cycle_id": s.cycle_id,
                    "successful_support": s.stage == "handed-off"
                }),
            );
            save_json_atomic(&completed_path, &completed)?;

            if house_path.is_file() {
                let house_data = fs::read_to_string(&house_path)?;
                let house: HousekeepingReport = serde_json::from_str(&house_data)?;
                let house_key = sha256_digest(
                    format!(
                        "housekeeping-v1|{}|backpressure={}|deletion={}",
                        house.partial_files.join("|"),
                        house.pending_results >= 8,
                        house.deletion_enabled
                    )
                    .as_bytes(),
                );
                if (s.task_kind == "inventory-integrity" || s.task_kind == "housekeeping-brief")
                    && !completed.contains_key(&house_key)
                {
                    state = None;
                    key = house_key;
                } else if s.task_kind == "housekeeping-brief"
                    && !completed.contains_key(&inventory_key)
                {
                    state = None;
                    key = inventory_key.clone();
                }
            }
        }
    }

    let results_dir = runtime_root.join("results");
    fs::create_dir_all(&results_dir)?;
    let pending_count = fs::read_dir(&results_dir)?
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .count();

    if let Some(s) = &mut state {
        if s.evidence_key != key && pending_count >= 8 {
            s.next_action = "Backpressure: independently review/archive retained outputs before staging changed evidence".to_string();
            save_json_atomic(&state_path, s)?;
            return Ok("pipeline backpressure; no new manifest".to_string());
        }
        if s.evidence_key != key {
            clear_jobs(runtime_root, s)?;
            let stamp = format_compact_stamp(unix_secs_now());
            let superseded_path = pipe_dir
                .join("runs")
                .join(&s.cycle_id)
                .join(format!("superseded-{stamp}.json"));
            save_json_atomic(
                &superseded_path,
                &serde_json::json!({
                    "prior_state": s,
                    "reason": "Declared evidence changed; no acceptance granted",
                    "replacement_key": key
                }),
            )?;
            state = None;
        }
    }

    let task_kind = if let Some(s) = &state {
        s.task_kind.clone()
    } else if key != inventory_key {
        "housekeeping-brief".to_string()
    } else {
        "inventory-integrity".to_string()
    };

    if state.is_none() {
        if pending_count >= 8 {
            return Ok("pipeline backpressure; independent review required".to_string());
        }
        let cycle = key.get(0..20).unwrap_or(&key).to_string();
        let run_dir = pipe_dir.join("runs").join(&cycle);
        fs::create_dir_all(&run_dir)?;

        let facts_txt = fs::read_to_string(&facts_path)?;
        let facts: InventoryFacts = serde_json::from_str(&facts_txt)?;
        let counts: Vec<MechanicalCountEntry> = facts
            .components
            .iter()
            .map(|c| MechanicalCountEntry {
                component: c.component.clone(),
                source_files: c.rust_source_files,
                separate_test_files: c.separate_test_files,
                dependencies: c.declared_dependency_names,
                inline_attributes: c.inline_test_markers.len(),
            })
            .collect();

        let mut ticket = PipelineTicket {
            schema_version: 1,
            cycle_id: cycle.clone(),
            responsibility: "M01 inconsistent inventory assertions".to_string(),
            authority:
                "unclaimed read-only calibration; generated reports only; no source lane"
                    .to_string(),
            non_overlap:
                "Only runtime reports and retained reviews; no implementation paths reserved or changed"
                    .to_string(),
            evidence_key: key.clone(),
            mechanical_counts: counts,
            limits: vec![
                "Attributes are not executed tests or coverage".to_string(),
                "No product implementation or test execution".to_string(),
                "Model output remains untrusted".to_string(),
            ],
            independent_review: serde_json::Value::String(seed_content),
            review_sha256: seed_sha,
            feedback: None,
            housekeeping_snapshot: None,
        };

        if task_kind == "housekeeping-brief" {
            ticket.responsibility = "Runtime housekeeping retention/backlog handoff".to_string();
            ticket.mechanical_counts = Vec::new();
            ticket.independent_review = serde_json::Value::String(
                "Owner requested housekeeping audit; model remains an unverified interpreter"
                    .to_string(),
            );
            ticket.review_sha256 = "".to_string();
            if house_path.is_file() {
                ticket.housekeeping_snapshot =
                    serde_json::from_str(&fs::read_to_string(&house_path)?).ok();
            }
        }

        fs::copy(&facts_path, run_dir.join("inventory-facts.json"))?;
        fs::copy(&index_path, run_dir.join("source-index.json"))?;
        save_json_atomic(&run_dir.join("ticket.json"), &ticket)?;
        fs::copy(
            run_dir.join("ticket.json"),
            runtime_root.join("evidence").join("pipeline-ticket.json"),
        )?;

        let mut s = PipelineState {
            schema_version: 1,
            task_kind: task_kind.clone(),
            cycle_id: cycle,
            evidence_key: key.clone(),
            stage: "plan-ready".to_string(),
            attempt: 0,
            plan_job: "".to_string(),
            brief_job: "".to_string(),
            plan_result: "".to_string(),
            brief_result: "".to_string(),
            plan_sha256: "".to_string(),
            brief_sha256: "".to_string(),
            decision_sha256: "".to_string(),
            next_action: if task_kind == "housekeeping-brief" {
                "Native worker interprets bounded housekeeping snapshot".to_string()
            } else {
                "Native worker drafts bounded repair plan".to_string()
            },
            whole_job_accepted: false,
        };
        publish_plan(runtime_root, &mut s)?;
        state = Some(s);
        changed = true;
    }

    let state_ref = state.as_mut().unwrap();

    if state_ref.stage == "plan-ready" {
        if let Some(record) = find_record(runtime_root, &state_ref.plan_job)? {
            state_ref.plan_result = record.path.display().to_string();
            state_ref.plan_sha256 = sha256_file(&record.path)?;

            if record.raw.outcome == "failed" || record.raw.proposal.is_none() {
                clear_jobs(runtime_root, state_ref)?;
                state_ref.stage = "blocked-transport".to_string();
                state_ref.next_action =
                    "Frontier inspect retained failure; no unchanged automatic retry".to_string();
            } else {
                let contract = check_stage_contract(&record.path, "plan", &state_ref.task_kind)?;
                let contract_path = pipe_dir
                    .join("runs")
                    .join(&state_ref.cycle_id)
                    .join(format!("contract-plan-R{}.json", state_ref.attempt));
                save_json_atomic(&contract_path, &contract)?;

                if !contract.mechanical_pass {
                    clear_jobs(runtime_root, state_ref)?;
                    state_ref.stage = "blocked-contract".to_string();
                    state_ref.next_action =
                        "Independent review required for retained mechanical contract violation"
                            .to_string();
                } else {
                    let upstream = serde_json::json!({
                        "artifact_kind": "unverified.upstream-plan",
                        "result_sha256": state_ref.plan_sha256,
                        "proposal": record.raw.proposal,
                        "limits": "Untrusted input for specification synthesis; not an independent review"
                    });
                    let ev_up = runtime_root.join("evidence").join("upstream-plan.json");
                    save_json_atomic(&ev_up, &upstream)?;
                    fs::copy(
                        &ev_up,
                        pipe_dir
                            .join("runs")
                            .join(&state_ref.cycle_id)
                            .join(format!("upstream-plan-R{}.json", state_ref.attempt)),
                    )?;
                    publish_brief(runtime_root, state_ref)?;
                    state_ref.next_action = if state_ref.task_kind == "housekeeping-brief" {
                        "Native worker develops unverified operational handoff brief".to_string()
                    } else {
                        "Native worker expands plan into unverified regression-case brief"
                            .to_string()
                    };
                }
            }
            changed = true;
        }
    }

    if state_ref.stage == "brief-ready" {
        if let Some(record) = find_record(runtime_root, &state_ref.brief_job)? {
            state_ref.brief_result = record.path.display().to_string();
            state_ref.brief_sha256 = sha256_file(&record.path)?;
            clear_jobs(runtime_root, state_ref)?;

            if record.raw.outcome == "failed" || record.raw.proposal.is_none() {
                state_ref.stage = "blocked-transport".to_string();
            } else {
                let contract = check_stage_contract(&record.path, "brief", &state_ref.task_kind)?;
                let contract_path = pipe_dir
                    .join("runs")
                    .join(&state_ref.cycle_id)
                    .join(format!("contract-brief-R{}.json", state_ref.attempt));
                save_json_atomic(&contract_path, &contract)?;

                if !contract.mechanical_pass {
                    state_ref.stage = "blocked-contract".to_string();
                } else {
                    state_ref.stage = "awaiting-independent-review".to_string();
                }
            }
            state_ref.next_action =
                "Frontier independently review both outputs and cited evidence; publish bound decision receipt"
                    .to_string();
            changed = true;
        }
    }

    let decision_path = pipe_dir.join("decisions").join(format!(
        "{}-R{}.json",
        state_ref.cycle_id, state_ref.attempt
    ));
    if matches!(
        state_ref.stage.as_str(),
        "awaiting-independent-review" | "blocked-transport" | "blocked-contract"
    ) && decision_path.is_file()
    {
        let dec_txt = fs::read_to_string(&decision_path)?;
        let decision: DecisionReceipt = serde_json::from_str(&dec_txt)?;

        let actor = decision.actor.trim().to_lowercase();
        let independent_of = decision.independent_of.trim().to_lowercase();
        if actor.is_empty()
            || independent_of.is_empty()
            || actor == independent_of
            || !is_valid_seat_id(&actor)
            || !is_valid_seat_id(&independent_of)
            || decision.cycle_id != state_ref.cycle_id
            || decision.plan_sha256 != state_ref.plan_sha256
            || decision.brief_sha256 != state_ref.brief_sha256
            || decision.evidence_key != state_ref.evidence_key
            || decision.review_path.trim().is_empty()
        {
            bail!("Unbound or invalid independent decision");
        }

        let review = runtime_root.join(&decision.review_path);
        if !review.is_file() || sha256_file(&review)? != decision.review_sha256 {
            bail!("Review receipt path/hash mismatch");
        }
        if decision.verdict != "revise" && decision.verdict != "handoff" {
            bail!("Decision must be revise or handoff");
        }
        if matches!(
            state_ref.stage.as_str(),
            "blocked-transport" | "blocked-contract"
        ) && decision.verdict == "handoff"
        {
            bail!("Failed transport cannot be handed off as successful support");
        }

        if decision.verdict == "revise" {
            let def_path = pipe_dir
                .join("runs")
                .join(&state_ref.cycle_id)
                .join(format!("deferred-R{}.json", state_ref.attempt));
            save_json_atomic(
                &def_path,
                &serde_json::json!({
                    "schema_version": 1,
                    "artifact_kind": "maintenance.deferred-finding",
                    "classification": "independent-rejection",
                    "decision_sha256": sha256_file(&decision_path)?,
                    "notes": decision.notes,
                    "successful": false,
                    "next_action": "Bounded corrective retry or coordinated escalation"
                }),
            )?;
        }

        if decision.archive_reviewed == Some(true) {
            let archive_dir = runtime_root.join("archive");
            fs::create_dir_all(&archive_dir)?;

            for (kind, job_id, exp_sha) in [
                ("plan", &state_ref.plan_job, &state_ref.plan_sha256),
                ("brief", &state_ref.brief_job, &state_ref.brief_sha256),
            ] {
                if job_id.is_empty() {
                    continue;
                }
                if let Some(entry) = find_record(runtime_root, job_id)? {
                    if sha256_file(&entry.path)? != *exp_sha {
                        bail!("Archive source hash mismatch");
                    }
                    if entry.path.parent() == Some(&archive_dir) {
                        if kind == "plan" {
                            state_ref.plan_result = entry.path.display().to_string();
                        } else {
                            state_ref.brief_result = entry.path.display().to_string();
                        }
                        continue;
                    }
                    let stem = entry
                        .path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("result");
                    let dest = archive_dir.join(format!(
                        "{}-{}-{}.json",
                        stem,
                        entry.raw.submitted_at_secs,
                        exp_sha.get(0..8).unwrap_or("00000000")
                    ));
                    if dest.exists() {
                        bail!("Archive collision refused: {}", dest.display());
                    }
                    let sidecar = entry.path.with_extension("review.md");
                    if !sidecar.is_file() {
                        bail!(
                            "Independent sibling review required before archival: {}",
                            sidecar.display()
                        );
                    }
                    let dest_review = dest.with_extension("review.md");
                    if dest_review.exists() {
                        bail!(
                            "Review archive collision refused: {}",
                            dest_review.display()
                        );
                    }
                    fs::copy(&sidecar, &dest_review)?;
                    fs::rename(&entry.path, &dest)?;
                    if kind == "plan" {
                        state_ref.plan_result = dest.display().to_string();
                    } else {
                        state_ref.brief_result = dest.display().to_string();
                    }
                }
            }
        }

        state_ref.decision_sha256 = sha256_file(&decision_path)?;

        match decision.verdict.as_str() {
            "revise" => {
                if state_ref.attempt >= 1 {
                    state_ref.stage = "blocked-calibration".to_string();
                    state_ref.next_action =
                        "Two reviewed attempts exhausted; retain handoff for coordinated native repair"
                            .to_string();
                } else {
                    state_ref.attempt += 1;
                    let ticket_path = runtime_root.join("evidence").join("pipeline-ticket.json");
                    let mut ticket: PipelineTicket =
                        serde_json::from_str(&fs::read_to_string(&ticket_path)?)?;
                    ticket.feedback = Some(serde_json::json!({
                        "review_path": decision.review_path,
                        "review_sha256": decision.review_sha256,
                        "notes": decision.notes
                    }));
                    if state_ref.task_kind == "inventory-integrity" {
                        let mechanically_correct_total: usize = ticket
                            .mechanical_counts
                            .iter()
                            .map(|c| c.inline_attributes)
                            .sum();
                        ticket.independent_review = serde_json::json!({
                            "scope": "Historical rejected model assertion, not current ticket facts",
                            "original_review_sha256": ticket.review_sha256,
                            "historical_wrong_total": 72,
                            "mechanically_correct_total": mechanically_correct_total,
                            "verdict": "Old 72-count claim rejected; current mechanical_counts agree with inventory-facts",
                            "unsupported_recommendation": "Historical gemini investigation was rejected; do not revive it",
                            "next_task": "Specify prevention and regression cases, not a new diagnosis of current inputs"
                        });
                    }
                    save_json_atomic(&ticket_path, &ticket)?;
                    publish_plan(runtime_root, state_ref)?;
                    state_ref.next_action =
                        "One bounded retry from independently reviewed corrective feedback"
                            .to_string();
                }
            }
            "handoff" => {
                state_ref.stage = "handed-off".to_string();
                state_ref.next_action =
                    "Retained reviewed support brief; source implementation remains separately coordinated"
                        .to_string();
                let handoffs_dir = pipe_dir.join("handoffs");
                fs::create_dir_all(&handoffs_dir)?;
                save_json_atomic(
                    &handoffs_dir.join(format!("{}.json", state_ref.cycle_id)),
                    &serde_json::json!({
                        "schema_version": 1,
                        "cycle_id": state_ref.cycle_id,
                        "decision": decision,
                        "plan_result": state_ref.plan_result,
                        "brief_result": state_ref.brief_result,
                        "scope": "Reviewed support specification only; not product compliance, implementation acceptance or release approval"
                    }),
                )?;
            }
            _ => bail!("Decision must be revise or handoff"),
        }
        changed = true;
    }

    if matches!(
        state_ref.stage.as_str(),
        "blocked-transport" | "blocked-contract" | "blocked-calibration"
    ) {
        let finding_path = pipe_dir
            .join("runs")
            .join(&state_ref.cycle_id)
            .join(format!("deferred-R{}.json", state_ref.attempt));
        if !finding_path.exists() {
            save_json_atomic(
                &finding_path,
                &serde_json::json!({
                    "schema_version": 1,
                    "artifact_kind": "maintenance.deferred-finding",
                    "id": format!("{}-R{}", state_ref.cycle_id, state_ref.attempt),
                    "classification": state_ref.stage,
                    "observed": true,
                    "source_state": state_ref,
                    "disposition": "Retained, pending independent review; not archived or successful",
                    "next_action": state_ref.next_action
                }),
            )?;
        }
    }

    if changed {
        save_json_atomic(&state_path, state_ref)?;
        let stamp = format_compact_stamp(unix_secs_now());
        let transition_path = pipe_dir
            .join("runs")
            .join(&state_ref.cycle_id)
            .join(format!("transition-{stamp}.json"));
        save_json_atomic(
            &transition_path,
            &serde_json::json!({
                "before": before_state,
                "after": state_ref,
                "command": "foundation maintenance advance",
                "exit_code": 0
            }),
        )?;
    }

    let summary = format!(
        "pipeline stage={}; attempt={}; pending={}; next={}",
        state_ref.stage, state_ref.attempt, pending_count, state_ref.next_action
    );
    Ok(summary)
}

struct FoundRecord {
    path: PathBuf,
    raw: RawRecord,
}

fn find_record(runtime_root: &Path, job_id: &str) -> Result<Option<FoundRecord>> {
    let mut candidates = Vec::new();
    for dir_name in ["results", "archive"] {
        let dir = runtime_root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|x| x.to_str()) == Some("json") {
                if let Ok(txt) = fs::read_to_string(&path) {
                    if let Ok(rec) = serde_json::from_str::<RawRecord>(&txt) {
                        if rec.job_id == job_id {
                            candidates.push(FoundRecord { path, raw: rec });
                        }
                    }
                }
            }
        }
    }
    candidates.sort_by_key(|c| c.raw.submitted_at_secs);
    Ok(candidates.pop())
}

fn clear_jobs(runtime_root: &Path, state: &PipelineState) -> Result<()> {
    let jobs_dir = runtime_root.join("jobs");
    if !jobs_dir.is_dir() {
        return Ok(());
    }
    let pipe_dir = runtime_root.join("pipeline");
    for entry in fs::read_dir(&jobs_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with("route-") && name.ends_with(".toml") {
            let saved = pipe_dir
                .join("runs")
                .join(&state.cycle_id)
                .join(format!("{}-R{}-{}", state.stage, state.attempt, name));
            if !saved.exists() {
                let _ = fs::copy(&path, saved);
            }
            let _ = fs::remove_file(&path);
        }
    }
    Ok(())
}

fn publish_plan(runtime_root: &Path, state: &mut PipelineState) -> Result<()> {
    clear_jobs(runtime_root, state)?;
    let short_id = state.cycle_id.get(0..12).unwrap_or(&state.cycle_id);
    let id = format!("MAINTENANCE-PLAN-{short_id}-R{}", state.attempt);
    state.plan_result = "".to_string();
    state.plan_sha256 = "".to_string();
    state.brief_job = "".to_string();
    state.brief_result = "".to_string();
    state.brief_sha256 = "".to_string();
    state.plan_job = id.clone();
    state.stage = "plan-ready".to_string();

    let instruction = if state.task_kind == "housekeeping-brief" {
        "Unclaimed read-only housekeeping interpretation of generated runtime reports only. Read housekeeping_snapshot in pipeline-ticket.json. Summarize pending/archive/deferred counts and any partial files at that snapshot. Deletion is disabled. Do not infer cache waste, compliance or product coverage. Return existing JSON contract, at most 3 observations, at most 3 risks, one NOT RUN test_plan, rust_draft:null, under 180 words. Recommend one bounded retention/backlog handoff for independent review. No commands, repository changes, deletion, self-review, claims, approvals, merges, promotion or settings changes.".to_string()
    } else {
        "Unclaimed read-only calibration, generated evidence only. Draft one bounded M01 repair plan using pipeline-ticket.json and inventory-facts.json. Treat the supplied independent review as evidence, not your own verdict. Mechanical facts are authoritative; do not recompute counts or claim coverage. Return the existing JSON contract, observations at most 3, risks at most 3, one test_plan starting NOT RUN, rust_draft:null. Recommend a regression-case specification as the next stage. No commands, repository changes, deletion, verification claims, self-review, claims, approval, merges, promotion or credentials/settings changes. One responsibility: outline how an inconsistent numeric assertion should be prevented from reaching acceptance. Keep under 220 words.".to_string()
    };

    let evidence = if state.task_kind == "housekeeping-brief" {
        vec!["pipeline-ticket.json".to_string()]
    } else {
        vec![
            "pipeline-ticket.json".to_string(),
            "inventory-facts.json".to_string(),
        ]
    };

    let job = JobManifest {
        schema_version: 1,
        id,
        mode: "analysis".to_string(),
        evidence,
        instruction,
    };

    let jobs_dir = runtime_root.join("jobs");
    fs::create_dir_all(&jobs_dir)?;
    let toml_content = toml::to_string_pretty(&job)?;
    write_atomic(&jobs_dir.join("route-plan.toml"), toml_content.as_bytes())?;
    Ok(())
}

fn publish_brief(runtime_root: &Path, state: &mut PipelineState) -> Result<()> {
    clear_jobs(runtime_root, state)?;
    let short_id = state.cycle_id.get(0..12).unwrap_or(&state.cycle_id);
    let id = format!("MAINTENANCE-BRIEF-{short_id}-R{}", state.attempt);
    state.brief_job = id.clone();
    state.stage = "brief-ready".to_string();

    let instruction = if state.task_kind == "housekeeping-brief" {
        "Unclaimed read-only housekeeping handoff, generated snapshot only. Develop a new bounded operational brief, not a copy. Observations: 1 explicitly dated snapshot facts, 2 disposition of pending, failed, archived and deferred artifacts without counting archive as success, 3 remaining independent review action and why no deletion is authorized. Do not request enabling cleanup without a demonstrated need, explicit roots, retention rules and an approved tool. Use housekeeping_snapshot in pipeline-ticket.json as authoritative. Do not review or approve the upstream model. Return existing JSON contract with at most 3 observations, at most 3 risks, one NOT RUN test_plan, rust_draft:null, under 180 words. recommended_next_step must request independent frontier review of this brief. No commands, deletion, repository changes, verification/compliance claims, self-review, claims, approval, merges, promotion or settings changes.".to_string()
    } else {
        "Unclaimed read-only regression-fixture specification, generated evidence only. Expand the unverified upstream-plan.json into one bounded regression-case brief for M01. Do not review, endorse or score the upstream model. Use pipeline-ticket.json mechanical facts and retained independent review as constraints. Cases: mismatched count, matching count without coverage inference, unsupported coverage claim. Use exactly three observations entries with CASE:, INPUT:, EXPECTED:, REVIEW: labels. REVIEW specifies remaining independent review obligation, never your verdict. Specify input and proposed mechanical rejection/allow-to-review outcome. Return existing JSON contract with at most 3 observations and 3 risks, one NOT RUN test_plan, rust_draft:null, under 260 words. No source changes, commands, cache deletion, verification claims, self-review, claims, approvals, merges or promotion. recommended_next_step must request independent frontier review of this brief.".to_string()
    };

    let evidence = if state.task_kind == "housekeeping-brief" {
        vec![
            "pipeline-ticket.json".to_string(),
            "upstream-plan.json".to_string(),
        ]
    } else {
        vec![
            "pipeline-ticket.json".to_string(),
            "upstream-plan.json".to_string(),
            "inventory-facts.json".to_string(),
        ]
    };

    let job = JobManifest {
        schema_version: 1,
        id,
        mode: "analysis".to_string(),
        evidence,
        instruction,
    };

    let jobs_dir = runtime_root.join("jobs");
    fs::create_dir_all(&jobs_dir)?;
    let toml_content = toml::to_string_pretty(&job)?;
    write_atomic(&jobs_dir.join("route-brief.toml"), toml_content.as_bytes())?;
    Ok(())
}

// ============================================================================
// 4. Housekeeping Telemetry
// ============================================================================

pub fn housekeeping(runtime_root: &Path) -> Result<HousekeepingReport> {
    if !runtime_root.is_absolute() {
        bail!(
            "runtime_root must be an absolute path: {}",
            runtime_root.display()
        );
    }
    let pipe_dir = runtime_root.join("pipeline");
    let state_path = pipe_dir.join("state.json");
    let stage = load_pipeline_state(&state_path)?
        .map(|s| s.stage)
        .unwrap_or_else(|| "unknown".to_string());

    let count_json = |dir: &Path| -> usize {
        if !dir.is_dir() {
            return 0;
        }
        fs::read_dir(dir)
            .map(|rd| {
                rd.filter_map(std::result::Result::ok)
                    .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
                    .count()
            })
            .unwrap_or(0)
    };

    let pending_results = count_json(&runtime_root.join("results"));
    let archived_results = count_json(&runtime_root.join("archive"));
    let reviewed_handoffs = count_json(&pipe_dir.join("handoffs"));

    // Recurse for deferred findings in pipeline/runs
    let mut deferred_findings = 0;
    let runs_dir = pipe_dir.join("runs");
    if runs_dir.is_dir() {
        collect_deferred_findings(&runs_dir, &mut deferred_findings)?;
    }

    // Recurse entire runtime_root for files, bytes, and partials
    let mut retained_files = 0;
    let mut retained_bytes = 0u64;
    let mut partial_files = Vec::new();

    collect_file_metrics(
        runtime_root,
        &mut retained_files,
        &mut retained_bytes,
        &mut partial_files,
    )?;

    let recommended_next_action = if !partial_files.is_empty() {
        "Frontier inspect partial-file ownership before any authorized recovery".to_string()
    } else if pending_results >= 8 {
        "Frontier review retained backlog; native backpressure remains active".to_string()
    } else {
        "Retain history; request explicit retention roots/rules only if cleanup is needed"
            .to_string()
    };

    let report = HousekeepingReport {
        schema_version: 1,
        artifact_kind: "maintenance.housekeeping".to_string(),
        observed_at_utc: format_iso8601(unix_secs_now()),
        pipeline_stage: stage,
        pending_results,
        archived_results,
        deferred_findings,
        reviewed_handoffs,
        retained_files,
        retained_bytes,
        partial_files,
        deletion_enabled: false,
        limits: vec![
            "Inventory only; no deletion, cache eviction, settings change or lock repair"
                .to_string(),
            "Failed tasks are not successful; archive is history, not acceptance".to_string(),
            "Source changes and product gates remain separately coordinated".to_string(),
        ],
        recommended_next_action,
    };

    let house_path = pipe_dir.join("housekeeping.json");
    save_json_atomic(&house_path, &report)?;
    Ok(report)
}

fn collect_deferred_findings(dir: &Path, count: &mut usize) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_deferred_findings(&path, count)?;
        } else if path.is_file() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with("deferred-") && name.ends_with(".json") {
                *count += 1;
            }
        }
    }
    Ok(())
}

fn collect_file_metrics(
    dir: &Path,
    total_files: &mut usize,
    total_bytes: &mut u64,
    partial_files: &mut Vec<String>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_file_metrics(&path, total_files, total_bytes, partial_files)?;
        } else if path.is_file() {
            *total_files += 1;
            if let Ok(meta) = fs::metadata(&path) {
                *total_bytes += meta.len();
            }
            if let Some(ext) = path.extension().and_then(|x| x.to_str()) {
                if ext == "tmp" || ext == "partial" {
                    partial_files.push(path.display().to_string());
                }
            }
        }
    }
    Ok(())
}

// ============================================================================
// 5. Daemon, Status & Control
// ============================================================================

pub fn daemon(runtime_root: &Path, checkout_dir: Option<&Path>, serve: bool) -> Result<()> {
    if !runtime_root.is_absolute() {
        bail!(
            "runtime_root must be an absolute path: {}",
            runtime_root.display()
        );
    }
    let config = runtime_root.join("worker.toml");
    if !config.is_file() {
        bail!("worker.toml not found at {}", config.display());
    }

    let lock_path = runtime_root.join("maintenance-loop.lock");
    let lease_guard = match FileLockGuard::try_acquire(&lock_path)? {
        Some(guard) => guard,
        None => {
            println!("maintenance daemon: loop lease already held");
            return Ok(());
        }
    };

    let pid = std::process::id();
    let lease_info = LoopLease {
        pid,
        started_at_utc: format_iso8601(unix_secs_now()),
    };
    let _ = fs::write(&lock_path, serde_json::to_vec(&lease_info)?);

    println!(
        "maintenance daemon started pid={pid} runtime={}",
        runtime_root.display()
    );

    loop {
        let control = daemon_control(runtime_root)?;
        load_pipeline_state(&runtime_root.join("pipeline/state.json"))?;
        if control.state == "pause" || control.state == "restart" {
            println!("daemon exiting on control state: {}", control.state);
            break;
        }
        if control.state != "run" {
            bail!("Invalid loop control state: {}", control.state);
        }

        let stamp = format_compact_stamp(unix_secs_now());
        let mut commands_log = Vec::new();
        let outcome: String;

        let res_dir = runtime_root
            .parent()
            .map(|p| p.join("reservation"))
            .unwrap_or_else(|| runtime_root.join("reservation"));
        if res_dir.join("review.lock").is_file() {
            outcome = "shared-reservation-active".to_string();
        } else {
            // Step 1: Refresh inventory
            if let Some(checkout) = checkout_dir {
                let comps = default_components();
                match refresh_inventory(runtime_root, checkout, &comps) {
                    Ok(ref_res) => {
                        commands_log.push(serde_json::json!({
                            "command": "refresh_inventory",
                            "evidence_digest": ref_res.evidence_digest,
                            "exit_code": 0
                        }));
                    }
                    Err(e) => {
                        commands_log.push(serde_json::json!({
                            "command": "refresh_inventory",
                            "error": e.to_string(),
                            "exit_code": 1
                        }));
                    }
                }
            } else {
                commands_log.push(serde_json::json!({
                    "command": "refresh_inventory",
                    "notice": "no --checkout provided; skipped inventory refresh",
                    "exit_code": 0
                }));
            }

            // Step 2: Advance pipeline
            match advance_pipeline(runtime_root) {
                Ok(adv_res) => {
                    commands_log.push(serde_json::json!({
                        "command": "advance_pipeline",
                        "result": adv_res,
                        "exit_code": 0
                    }));
                }
                Err(e) => {
                    commands_log.push(serde_json::json!({
                        "command": "advance_pipeline",
                        "error": e.to_string(),
                        "exit_code": 1
                    }));
                }
            }

            // Step 3: Housekeeping
            match housekeeping(runtime_root) {
                Ok(_) => {
                    commands_log.push(serde_json::json!({
                        "command": "housekeeping",
                        "exit_code": 0
                    }));
                }
                Err(e) => {
                    commands_log.push(serde_json::json!({
                        "command": "housekeeping",
                        "error": e.to_string(),
                        "exit_code": 1
                    }));
                }
            }

            // Recheck persisted state after advance and before any model dispatch.
            load_pipeline_state(&runtime_root.join("pipeline/state.json"))?;
            // Step 4: Tick local worker
            let ctrl = daemon_control(runtime_root)?;
            if ctrl.state == "run" {
                let now = unix_secs_now();
                let minute = ((now / 60) % 60) as u32;
                match local_worker::tick(&config, minute, now) {
                    Ok(()) => {
                        commands_log.push(serde_json::json!({
                            "command": "local_worker::tick",
                            "exit_code": 0
                        }));
                        outcome = "tick-returned".to_string();
                    }
                    Err(e) => {
                        commands_log.push(serde_json::json!({
                            "command": "local_worker::tick",
                            "error": e.to_string(),
                            "exit_code": 1
                        }));
                        outcome = "tick-failed".to_string();
                    }
                }
            } else {
                outcome = "drain-requested-after-refresh".to_string();
            }
        }

        let receipt = serde_json::json!({
            "started_at_utc": format_iso8601(unix_secs_now()),
            "loop_pid": pid,
            "commands": commands_log,
            "outcome": outcome
        });

        let cap_dir = runtime_root.join("captures");
        fs::create_dir_all(&cap_dir)?;
        save_json_atomic(&cap_dir.join(format!("cycle-{stamp}.json")), &receipt)?;
        save_json_atomic(&runtime_root.join("last-cycle.json"), &receipt)?;

        if !serve {
            break;
        }

        // Adaptive sleep in 10s slices up to 120s
        for _ in 0..12 {
            let ctrl = daemon_control(runtime_root)?;
            if ctrl.state != "run" {
                break;
            }
            std::thread::sleep(Duration::from_secs(10));
        }
    }

    drop(lease_guard);
    Ok(())
}

pub fn status(runtime_root: &Path) -> Result<()> {
    if !runtime_root.is_absolute() {
        bail!(
            "runtime_root must be an absolute path: {}",
            runtime_root.display()
        );
    }
    let lock_path = runtime_root.join("maintenance-loop.lock");
    let lease: Option<LoopLease> = if lock_path.is_file() {
        fs::read_to_string(&lock_path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
    } else {
        None
    };

    let control_path = runtime_root.join("loop-control.json");
    let control = load_loop_control(&control_path)?;

    let results_dir = runtime_root.join("results");
    let pending = if results_dir.is_dir() {
        fs::read_dir(&results_dir)?
            .filter_map(std::result::Result::ok)
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .count()
    } else {
        0
    };

    let archive_dir = runtime_root.join("archive");
    let archived = if archive_dir.is_dir() {
        fs::read_dir(&archive_dir)?
            .filter_map(std::result::Result::ok)
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .count()
    } else {
        0
    };

    let state_path = runtime_root.join("pipeline").join("state.json");
    let pipeline = load_pipeline_state(&state_path)?;

    let last_cycle_path = runtime_root.join("last-cycle.json");
    let last_cycle: Option<serde_json::Value> = if last_cycle_path.is_file() {
        fs::read_to_string(&last_cycle_path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
    } else {
        None
    };

    let status_obj = serde_json::json!({
        "route": "Status",
        "loop_lease": lease,
        "control": control,
        "pending": pending,
        "archived": archived,
        "pipeline": pipeline,
        "last_cycle": last_cycle
    });

    println!("{}", serde_json::to_string_pretty(&status_obj)?);
    Ok(())
}

pub fn control(runtime_root: &Path, state: &str) -> Result<()> {
    if !runtime_root.is_absolute() {
        bail!(
            "runtime_root must be an absolute path: {}",
            runtime_root.display()
        );
    }
    match state {
        "run" | "pause" | "restart" => {}
        _ => bail!("invalid control state '{state}'; must be run, pause, or restart"),
    }

    let control_path = runtime_root.join("loop-control.json");
    let control_obj = LoopControl {
        state: state.to_string(),
        requested_at_utc: Some(format_iso8601(unix_secs_now())),
        route: Some("Control".to_string()),
    };
    save_json_atomic(&control_path, &control_obj)?;
    println!("loop control set to '{state}'");
    Ok(())
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_test_marker_detection() {
        assert!(is_test_marker("#[cfg(test)]"));
        assert!(is_test_marker("  #[test]"));
        assert!(is_test_marker("    #[tokio::test]"));
        assert!(is_test_marker("#[ cfg(test) ]"));
        assert!(is_test_marker("#[ test ]"));
        assert!(is_test_marker(" #[tokio::test] // async test"));
        assert!(!is_test_marker("#[test_case]"));
        assert!(!is_test_marker("#[derive(Test)]"));
        assert!(!is_test_marker("fn test() {}"));
        assert!(!is_test_marker(""));
    }

    #[test]
    fn test_iso8601_formatting() {
        // 0 secs = 1970-01-01T00:00:00Z
        assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_compact_stamp(0), "19700101T000000Z");

        // 1700000000 = 2023-11-14T22:13:20Z
        assert_eq!(format_iso8601(1700000000), "2023-11-14T22:13:20Z");
        assert_eq!(format_compact_stamp(1700000000), "20231114T221320Z");
    }

    #[test]
    fn test_stage_contract_validation() {
        let temp = tempfile::tempdir().expect("tempdir");
        let result_path = temp.path().join("res.json");

        // Valid plan proposal
        let valid_plan = serde_json::json!({
            "schema_version": 1,
            "artifact_kind": "foundation.local-worker-result",
            "job_id": "TEST-PLAN-001",
            "submitted_at_secs": 1000,
            "outcome": "pending-owner-review",
            "input": "...",
            "proposal": {
                "task": "TEST-PLAN-001",
                "observations": ["obs 1", "obs 2"],
                "risks": ["risk 1"],
                "test_plan": ["NOT RUN fixture check"],
                "recommended_next_step": "Draft regression brief",
                "rust_draft": null
            }
        });
        fs::write(&result_path, serde_json::to_string(&valid_plan).unwrap()).unwrap();

        let report = check_stage_contract(&result_path, "plan", "inventory-integrity").unwrap();
        assert!(report.mechanical_pass);
        assert!(report.findings.is_empty());

        // Invalid: test_plan without NOT RUN
        let invalid_plan = serde_json::json!({
            "schema_version": 1,
            "artifact_kind": "foundation.local-worker-result",
            "job_id": "TEST-PLAN-001",
            "submitted_at_secs": 1000,
            "outcome": "pending-owner-review",
            "input": "...",
            "proposal": {
                "task": "TEST-PLAN-001",
                "observations": ["obs 1"],
                "risks": [],
                "test_plan": ["RUN cargo test"],
                "recommended_next_step": "Next step",
                "rust_draft": null
            }
        });
        fs::write(&result_path, serde_json::to_string(&invalid_plan).unwrap()).unwrap();
        let report = check_stage_contract(&result_path, "plan", "inventory-integrity").unwrap();
        assert!(!report.mechanical_pass);
        assert_eq!(
            report.findings,
            vec!["All proposed tests require NOT RUN prefix"]
        );

        // Valid brief with CASE: INPUT: EXPECTED: REVIEW:
        let valid_brief = serde_json::json!({
            "schema_version": 1,
            "artifact_kind": "foundation.local-worker-result",
            "job_id": "TEST-BRIEF-001",
            "submitted_at_secs": 1001,
            "outcome": "pending-owner-review",
            "input": "...",
            "proposal": {
                "task": "TEST-BRIEF-001",
                "observations": [
                    "CASE: mismatch INPUT: 72 EXPECTED: reject REVIEW: independent review needed",
                    "CASE: exact match INPUT: 79 EXPECTED: allow REVIEW: verify counts",
                    "CASE: coverage claim INPUT: full EXPECTED: reject REVIEW: reject coverage claim"
                ],
                "risks": ["risk 1"],
                "test_plan": ["NOT RUN check cases"],
                "recommended_next_step": "Request independent frontier review",
                "rust_draft": null
            }
        });
        fs::write(&result_path, serde_json::to_string(&valid_brief).unwrap()).unwrap();
        let report = check_stage_contract(&result_path, "brief", "inventory-integrity").unwrap();
        assert!(report.mechanical_pass);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn test_housekeeping_file_metrics() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();

        fs::create_dir_all(root.join("results")).unwrap();
        fs::create_dir_all(root.join("archive")).unwrap();
        fs::create_dir_all(root.join("pipeline/handoffs")).unwrap();
        fs::create_dir_all(root.join("pipeline/runs/c1")).unwrap();

        fs::write(root.join("results/r1.json"), "{}").unwrap();
        fs::write(root.join("archive/a1.json"), "{}").unwrap();
        fs::write(root.join("archive/a2.json"), "{}").unwrap();
        fs::write(root.join("pipeline/handoffs/h1.json"), "{}").unwrap();
        fs::write(root.join("pipeline/runs/c1/deferred-R0.json"), "{}").unwrap();
        fs::write(root.join("temp.tmp"), "partial").unwrap();

        let report = housekeeping(root).expect("housekeeping succeeds");
        assert_eq!(report.pending_results, 1);
        assert_eq!(report.archived_results, 2);
        assert_eq!(report.reviewed_handoffs, 1);
        assert_eq!(report.deferred_findings, 1);
        assert_eq!(report.partial_files.len(), 1);
        assert!(
            report
                .recommended_next_action
                .contains("partial-file ownership")
        );
    }
}
