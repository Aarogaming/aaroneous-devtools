//! Agentic Queue Registrar & Dispatch Control Plane.
//!
//! Generates candidate Rust code via a local Ollama model for registered
//! tasks, exactly like `local_worker.rs`'s job dispatcher: every dispatch
//! produces a versioned, `pending-owner-review` JSON record outside the
//! product checkout (`results_directory`) and never touches the real
//! checkout or git. `governance/LOCAL_WORKER_SERVICE.md` states the rule
//! this module now follows: "No draft is executed, formatted, compiled,
//! applied, merged or pushed automatically. A successful generation means
//! pending-owner-review, not a successful product fix." The only path that
//! writes into the checkout and commits is [`promote`], an explicit,
//! separately-invoked action.
//!
//! `promote` never commits onto whatever branch happens to be checked out.
//! It requires a clean working tree, creates an isolated `local-agent/{task_id}`
//! branch, writes the draft, re-verifies (compile + clippy) there, commits,
//! and pushes that branch to `origin` — then always restores the original
//! branch, success or failure. It does **not** open a pull request itself
//! (see `governance/LOCAL_MODEL_AUTONOMY_POLICY.md`: opening the PR and
//! reviewing it is a cloud agent's job, not a new GitHub credential inside
//! this binary). Two further gates sit in front of the push: a high-risk
//! allowlist (`is_high_risk`) that refuses to promote a change touching a
//! safety/security-critical crate or containing generated `unsafe` code —
//! that class of change is blocked outright, not just flagged, and needs a
//! cloud agent to implement it directly — and a circuit breaker
//! (`check_circuit_breaker`/`record_circuit_breaker_outcome`) that disables
//! further promotion after repeated verification failures until explicitly
//! reset.
//!
//! `task_queue.json` is protected by the same exclusive `review.lock`
//! idiom `local_worker.rs`/`multithreader.rs` use (a `create_new(true)`
//! marker file, held for one full operation via a `Reservation` RAII
//! guard), so `register_task`, `dispatch`, and `promote` never race each
//! other even when driven by a scheduled task and a manual/MCP call at the
//! same time.

use crate::intake::{read_bounded, relative_file};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrarConfig {
    pub schema_version: u32,
    pub checkout: PathBuf,
    pub queue: PathBuf,
    pub results_directory: PathBuf,
    pub archive_directory: PathBuf,
    pub reservation_directory: PathBuf,
    pub progress_log: PathBuf,
    pub endpoint: String,
    pub model: String,
    pub max_pending: usize,
    /// Required only by Serve. Never discover credentials from the environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_token_file: Option<PathBuf>,
    /// Where the promotion circuit-breaker's state is persisted. Defaults to
    /// `reservation_directory/circuit_breaker.json` when unset, so existing
    /// configs need no change. Deliberately not archive_directory, which
    /// callers scan for `*.json` and treat every match as an archived task
    /// record. Never inside the product checkout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circuit_breaker_path: Option<PathBuf>,
    /// Absolute path to this devtools repo's own working copy. When set,
    /// `check_circuit_breaker` also fast-forward-pulls this repo's `main`
    /// and checks `governance/circuit_breaker_override.toml` — the channel
    /// a cloud agent uses to trip the breaker remotely after finding a
    /// problem the local-only breaker state has no way to see (e.g. a
    /// merged local-agent PR later reverted). Optional: omitted, this
    /// check is skipped entirely and no network access is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub devtools_repo_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QueueManifest {
    pub schema_version: u32,
    pub tasks: Vec<TaskItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskItem {
    pub id: String,
    pub crate_name: String,
    pub task: String,
    pub target_file: String,
    #[serde(default)]
    pub instruction: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneratedDraft {
    pub rust_source: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Record {
    pub schema_version: u32,
    pub artifact_kind: String,
    pub task_id: String,
    pub crate_name: String,
    pub target_file: String,
    pub submitted_at_secs: u64,
    pub outcome: String,
    pub instruction: String,
    pub existing_code_excerpt: String,
    pub proposal: Option<GeneratedDraft>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModelRequest {
    model: String,
    stream: bool,
    think: bool,
    options: ModelOptions,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModelOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// RAII exclusive lock: dropping the guard removes the marker file. Same
/// idiom as `local_worker.rs`'s `Reservation` / `multithreader.rs`'s
/// `RunLock` — each module keeps its own copy rather than sharing one,
/// which is this codebase's existing convention.
struct Reservation(PathBuf);
impl Drop for Reservation {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn now_secs() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

/// Case-insensitive, separator-normalized "is `path` inside `root`" check
/// (Windows paths are case-insensitive and mix `/`/`\`) — copied from
/// `local_worker.rs`.
fn within(path: &Path, root: &Path) -> bool {
    #[cfg(windows)]
    {
        PathBuf::from(path.to_string_lossy().replace('\\', "/").to_lowercase()).starts_with(
            PathBuf::from(root.to_string_lossy().replace('\\', "/").to_lowercase()),
        )
    }
    #[cfg(not(windows))]
    {
        path.starts_with(root)
    }
}

fn files(directory: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some(extension) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Acquire the shared `review.lock`, run `action`, then release it
/// (via `Reservation`'s `Drop`) regardless of outcome. Fails fast with a
/// clear "busy" error instead of blocking if another registrar operation
/// (or the local worker / multithreader review, which share the same lock
/// per `governance/LOCAL_WORKER_SERVICE.md`) already holds it.
fn with_reservation<T>(
    reservation_dir: &Path,
    owner: &str,
    now: u64,
    action: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let lock = reservation_dir.join("review.lock");
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            bail!("agentic registrar: busy (review.lock held) — try again shortly");
        }
        Err(e) => return Err(e.into()),
    };
    let _reservation = Reservation(lock);
    {
        use std::io::Write;
        let _ = file.write_all(format!("owner={owner}\nstarted_at_secs={now}\n").as_bytes());
    }
    action()
}

/// Find the nearest ancestor directory (walking up from `target_file`'s
/// parent toward `checkout`) that contains a `Cargo.toml`, i.e. the crate
/// that owns `target_file` under normal Cargo layout.
fn crate_root_for(checkout: &Path, target_file: &str) -> Option<PathBuf> {
    let full = checkout.join(target_file);
    let mut dir = full.parent()?.to_path_buf();
    loop {
        if dir.join("Cargo.toml").is_file() {
            return Some(dir);
        }
        if !within(&dir, checkout) || !dir.pop() {
            return None;
        }
    }
}

fn declared_crate_name(cargo_toml: &Path) -> Option<String> {
    let manifest = fs::read_to_string(cargo_toml).ok()?;
    manifest.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("name")
            .and_then(|rest| rest.trim_start().strip_prefix('='))
            .map(|value| value.trim().trim_matches('"').to_string())
    })
}

/// Reject a task before it ever reaches generation or promotion unless:
/// `target_file` is an exact, traversal-free relative path (`intake::relative_file`),
/// `crate_name` is a plain identifier, and `target_file` actually resolves
/// (via the nearest ancestor `Cargo.toml`) to a crate named `crate_name` —
/// closing the gap where an unrelated, always-compiling crate name could
/// be paired with an unrelated target file to defeat per-crate verification.
fn validate_target(checkout: &Path, crate_name: &str, target_file: &str) -> Result<()> {
    if !relative_file(target_file) {
        bail!("target_file must be an exact relative path with no traversal: {target_file}");
    }
    if crate_name.trim().is_empty()
        || crate_name.len() > 64
        || !crate_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        bail!("crate_name must be a plain identifier: {crate_name}");
    }
    let declared = crate_root_for(checkout, target_file)
        .map(|dir| dir.join("Cargo.toml"))
        .and_then(|manifest| declared_crate_name(&manifest));
    if declared.as_deref() != Some(crate_name) {
        bail!(
            "target_file '{target_file}' does not belong to a crate named '{crate_name}' (found: {declared:?})"
        );
    }
    Ok(())
}

/// Crates where local-model-authored changes are never eligible for local
/// promotion, regardless of how the draft looks: safety/security-critical,
/// unsafe-heavy, or the audit tooling that other promotions are trusted
/// against. See `governance/LOCAL_MODEL_AUTONOMY_POLICY.md`'s high-risk
/// allowlist — this list is deliberately conservative and crate-name based
/// (not a content heuristic), so it stays correct even as file layouts move.
const HIGH_RISK_CRATES: &[&str] = &[
    "ipc_bus",
    "wire",
    "hypervisor",
    "governance",
    "ast_auditor",
    "compliance_auditor",
    "platform_bridge",
    "core_contracts",
    "sdk",
];

/// `true` if this change must never be locally promoted: either the target
/// crate is on the high-risk allowlist, or the generated source itself
/// contains the `unsafe` keyword (a cheap, deliberately-over-inclusive
/// heuristic — a false positive just means one more change routed to a
/// cloud agent instead of pushed unattended, which is the safe direction
/// to be wrong in).
fn is_high_risk(crate_name: &str, draft_source: &str) -> bool {
    HIGH_RISK_CRATES.contains(&crate_name) || draft_source.contains("unsafe")
}

fn rs_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            continue;
        }
        if path.is_dir() {
            rs_files_recursive(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
            if out.len() > 2048 {
                bail!("crate src/ tree exceeds 2048 .rs files; too large to scan for mod wiring");
            }
        }
    }
    Ok(())
}

/// Cargo does not auto-discover arbitrary new files under `src/` the way it
/// auto-discovers every top-level `tests/*.rs` file as its own integration
/// test binary. A brand-new `src/`-rooted file that isn't referenced by a
/// `mod <name>;` declaration anywhere in the crate is simply never part of
/// what `cargo check`/`clippy` compiles — so a "verification passed" result
/// for it would mean nothing, silently. Refuse to promote that case rather
/// than trust a check that never actually ran against the new code.
/// Exempt: files under `tests/` (auto-discovered) and any file that already
/// existed (already reachable by construction, or this would have already
/// been failing every promotion of an edit to an existing file).
fn ensure_new_file_is_reachable(crate_root: &Path, full_target_path: &Path) -> Result<()> {
    let relative = full_target_path
        .strip_prefix(crate_root)
        .context("target file is not inside its crate root")?;
    if relative.starts_with("tests") {
        return Ok(());
    }
    let stem = full_target_path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("target file has no valid file stem")?;

    let mut candidates = Vec::new();
    rs_files_recursive(&crate_root.join("src"), &mut candidates)?;

    let wired = candidates.iter().any(|path| {
        if path == full_target_path {
            return false;
        }
        let Ok(content) = fs::read_to_string(path) else {
            return false;
        };
        content.lines().any(|line| {
            let line = line
                .trim()
                .trim_start_matches("pub(crate)")
                .trim_start_matches("pub")
                .trim();
            line.strip_prefix("mod ")
                .map(|rest| rest.trim_end_matches(';').trim() == stem)
                .unwrap_or(false)
        })
    });

    if !wired {
        bail!(
            "new file '{}' is not referenced by a `mod {stem};` declaration anywhere in the \
             crate's src/ tree, so Cargo would never actually compile or check it — \
             verification would be meaningless. Either place new files under tests/ \
             (auto-discovered) or have the draft's instruction also wire in the mod \
             declaration.",
            full_target_path.display()
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CircuitBreakerState {
    consecutive_failures: u32,
    tripped: bool,
}

/// Consecutive local-verification failures before the breaker trips and
/// further promotion is refused until an operator explicitly clears it via
/// `reset_circuit_breaker`. Two, not one: a single failure is expected and
/// already retried with the error fed back at the dispatch layer
/// (`local_worker.rs`/`dispatch_locked`); it's a second *promotion-time*
/// failure in a row that indicates something systematic, not a one-off.
const CIRCUIT_BREAKER_THRESHOLD: u32 = 2;

fn circuit_breaker_path(config: &RegistrarConfig) -> PathBuf {
    // Deliberately not archive_directory: callers scan that directory for
    // `*.json` and treat every match as an archived task record (see
    // `archived.len()` style assertions/dashboards) — a breaker state file
    // living there would silently inflate that count.
    config
        .circuit_breaker_path
        .clone()
        .unwrap_or_else(|| config.reservation_directory.join("circuit_breaker.json"))
}

fn load_circuit_breaker(config: &RegistrarConfig) -> Result<CircuitBreakerState> {
    let path = circuit_breaker_path(config);
    if !path.exists() {
        return Ok(CircuitBreakerState::default());
    }
    serde_json::from_str(&read_bounded(&path, 4096)?).context("invalid circuit_breaker.json")
}

fn save_circuit_breaker(config: &RegistrarConfig, state: &CircuitBreakerState) -> Result<()> {
    let path = circuit_breaker_path(config);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let staging = path.with_extension("json.tmp");
    fs::write(&staging, serde_json::to_vec_pretty(state)?)?;
    fs::rename(&staging, &path)?;
    Ok(())
}

const CIRCUIT_BREAKER_OVERRIDE_RELATIVE: &str = "governance/circuit_breaker_override.toml";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CircuitBreakerOverride {
    schema_version: u32,
    tripped: bool,
    #[serde(default)]
    reason: String,
}

/// Best-effort fast-forward pull of the devtools repo's own `main`, so a
/// remote trip is seen promptly. Failure here fails the whole check
/// **closed** (bails, refusing promotion) rather than silently promoting
/// against possibly-stale state: the entire point of this channel is that
/// a cloud agent can stop promotion remotely by committing to a tracked
/// file, and a network hiccup or dirty local checkout must never silently
/// defeat that.
fn check_remote_circuit_breaker_override(devtools_repo_root: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["pull", "--ff-only", "origin", "main"])
        .current_dir(devtools_repo_root)
        .status()
        .context("failed to run git pull for the remote circuit-breaker override check")?;
    if !status.success() {
        bail!(
            "could not fast-forward {} from origin/main to check the remote circuit-breaker \
             override; refusing to promote rather than act on possibly-stale state",
            devtools_repo_root.display()
        );
    }

    let override_path = devtools_repo_root.join(CIRCUIT_BREAKER_OVERRIDE_RELATIVE);
    if !override_path.exists() {
        return Ok(());
    }
    let state: CircuitBreakerOverride = toml::from_str(&read_bounded(&override_path, 4096)?)
        .context("invalid governance/circuit_breaker_override.toml")?;
    if state.tripped {
        bail!(
            "remote circuit-breaker override is tripped ({}): {}",
            CIRCUIT_BREAKER_OVERRIDE_RELATIVE,
            if state.reason.trim().is_empty() {
                "no reason given"
            } else {
                state.reason.trim()
            }
        );
    }
    Ok(())
}

/// Refuses to proceed if either breaker is tripped: the local one (tracks
/// verification failures at promotion time) or, when `devtools_repo_root`
/// is configured, the remote override in this repo's own tracked
/// `governance/circuit_breaker_override.toml` — the channel a cloud agent
/// uses to stop promotion after finding a problem the local-only state has
/// no way to see, such as a merged local-agent PR reverted days later.
fn check_circuit_breaker(config: &RegistrarConfig) -> Result<()> {
    let state = load_circuit_breaker(config)?;
    if state.tripped {
        bail!(
            "promotion circuit breaker is tripped ({} consecutive verification failures) — \
             investigate before clearing it with `agentic-registrar reset-circuit-breaker`",
            state.consecutive_failures
        );
    }
    if let Some(devtools_repo_root) = &config.devtools_repo_root {
        check_remote_circuit_breaker_override(devtools_repo_root)?;
    }
    Ok(())
}

fn record_circuit_breaker_outcome(config: &RegistrarConfig, succeeded: bool) -> Result<()> {
    let mut state = load_circuit_breaker(config)?;
    if succeeded {
        state.consecutive_failures = 0;
    } else {
        state.consecutive_failures += 1;
        if state.consecutive_failures >= CIRCUIT_BREAKER_THRESHOLD {
            state.tripped = true;
        }
    }
    save_circuit_breaker(config, &state)
}

/// Explicitly clears a tripped breaker. `reason` is recorded only in the
/// printed confirmation today (the state file itself stays minimal); callers
/// driving this from a cloud-agent review should still leave their own
/// record of why in the PR/queue they were investigating.
pub fn reset_circuit_breaker(config_path: &Path, reason: &str) -> Result<()> {
    let config = load_config(config_path)?;
    save_circuit_breaker(&config, &CircuitBreakerState::default())?;
    println!("agentic registrar: circuit breaker reset ({reason})");
    Ok(())
}

fn load_config(config_path: &Path) -> Result<RegistrarConfig> {
    if !config_path.is_absolute() {
        bail!("Config path must be absolute: {}", config_path.display());
    }
    let content = read_bounded(config_path, 16384)?;
    let config: RegistrarConfig =
        toml::from_str(&content).context("Invalid registrar_config.toml")?;
    if config.schema_version != 1 {
        bail!(
            "Unsupported config schema version {}",
            config.schema_version
        );
    }
    if !(1..=64).contains(&config.max_pending) {
        bail!("max_pending must be between 1 and 64");
    }
    for (label, dir) in [
        ("checkout", &config.checkout),
        ("results", &config.results_directory),
        ("archive", &config.archive_directory),
        ("reservation", &config.reservation_directory),
    ] {
        crate::require_absolute_directory(label, dir)?;
    }
    for output in [
        &config.results_directory,
        &config.archive_directory,
        &config.reservation_directory,
    ] {
        if within(output, &config.checkout) {
            bail!("registrar artifacts must stay outside the product checkout");
        }
    }
    if !config.queue.is_absolute() {
        bail!("queue path must be absolute");
    }
    Ok(config)
}

fn load_manifest(queue_path: &Path) -> Result<QueueManifest> {
    if !queue_path.exists() {
        return Ok(QueueManifest {
            schema_version: 1,
            tasks: Vec::new(),
        });
    }
    // Bounded well above the prior 256 KiB cap: unlike local_worker's
    // externally-curated queue.md, this file accumulates task history
    // in-process with no archival step yet (tracked as follow-up work).
    let content = read_bounded(queue_path, 2 * 1024 * 1024)?;
    let manifest: QueueManifest =
        serde_json::from_str(&content).context("Invalid task_queue.json manifest")?;
    if manifest.schema_version != 1 {
        bail!(
            "unsupported task_queue.json schema version {}",
            manifest.schema_version
        );
    }
    Ok(manifest)
}

/// Atomic replace: write to a temp path then rename, so a crash mid-write
/// can never leave `task_queue.json` truncated or corrupt.
fn save_manifest(queue_path: &Path, manifest: &QueueManifest) -> Result<()> {
    let json = serde_json::to_vec_pretty(manifest)?;
    let staging = queue_path.with_extension("json.tmp");
    fs::write(&staging, &json)?;
    fs::rename(&staging, queue_path)?;
    Ok(())
}

/// Persist a `Record` as `{task_id}-{sequence}.json` via staged write +
/// `sync_all` + atomic rename — identical durability pattern to
/// `local_worker.rs`'s `persist()`.
fn persist(dir: &Path, record: &Record) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let data = serde_json::to_vec_pretty(record)?;
    if data.len() > 1_048_576 {
        bail!("result exceeds 1 MiB storage limit");
    }
    for sequence in 0..10000u32 {
        let destination = dir.join(format!("{}-{sequence:04}.json", record.task_id));
        if destination.exists() {
            continue;
        }
        let staging = destination.with_extension("partial");
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
        {
            Ok(mut file) => {
                use std::io::Write;
                if let Err(error) = file.write_all(&data).and_then(|()| file.sync_all()) {
                    drop(file);
                    let _ = fs::remove_file(&staging);
                    return Err(error.into());
                }
                drop(file);
                fs::rename(&staging, &destination)?;
                return Ok(destination);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    bail!("result sequence exhausted")
}

fn archive_record(config: &RegistrarConfig, record_path: &Path, outcome_label: &str) -> Result<()> {
    fs::create_dir_all(&config.archive_directory)?;
    let mut data: serde_json::Value = serde_json::from_str(&read_bounded(record_path, 1_048_576)?)?;
    if let Some(obj) = data.as_object_mut() {
        obj.insert(
            "archived_outcome".into(),
            serde_json::Value::String(outcome_label.into()),
        );
        obj.insert(
            "archived_at_secs".into(),
            serde_json::Value::from(now_secs()?),
        );
    }
    let file_name = record_path
        .file_name()
        .context("record path has no file name")?;
    fs::write(
        config.archive_directory.join(file_name),
        serde_json::to_vec_pretty(&data)?,
    )?;
    fs::remove_file(record_path)?;
    Ok(())
}

fn update_task_status(config: &RegistrarConfig, task_id: &str, status: &str) -> Result<()> {
    let mut manifest = load_manifest(&config.queue)?;
    if let Some(item) = manifest.tasks.iter_mut().find(|t| t.id == task_id) {
        item.status = status.to_string();
    }
    save_manifest(&config.queue, &manifest)
}

pub fn generate_dashboard_html(config_path: &Path, output_path: &Path) -> Result<()> {
    let config = load_config(config_path)?;
    let manifest = load_manifest(&config.queue)?;

    let pending = manifest
        .tasks
        .iter()
        .filter(|t| t.status == "Pending")
        .count();
    let review = manifest
        .tasks
        .iter()
        .filter(|t| t.status == "Review")
        .count();
    let verified = manifest
        .tasks
        .iter()
        .filter(|t| t.status == "Verified")
        .count();
    let failed = manifest
        .tasks
        .iter()
        .filter(|t| t.status == "Failed")
        .count();

    let mut task_rows = String::new();
    for t in &manifest.tasks {
        let badge_class = match t.status.as_str() {
            "Verified" => "bg-emerald-500/10 text-emerald-500 border-emerald-500/20",
            "Review" => "bg-blue-500/10 text-blue-400 border-blue-500/20",
            "Failed" => "bg-rose-500/10 text-rose-400 border-rose-500/20",
            "Blocked" => "bg-orange-500/10 text-orange-400 border-orange-500/20",
            _ => "bg-amber-500/10 text-amber-400 border-amber-500/20",
        };
        task_rows.push_str(&format!(
            r#"<div class="border border-[var(--border)] bg-[var(--background)] p-4 rounded-lg flex flex-col md:flex-row justify-between items-start md:items-center gap-4 hover:border-emerald-500/50 transition">
  <div class="space-y-1">
    <div class="flex items-center gap-2">
      <span class="font-mono text-xs font-bold px-2 py-0.5 rounded bg-blue-500/10 text-blue-400 border border-blue-500/20">{}</span>
      <span class="text-sm font-semibold text-[var(--foreground)]">{}</span>
    </div>
    <p class="text-xs font-mono text-[var(--muted-foreground)]">Target: {}</p>
    <p class="text-xs text-[var(--muted-foreground)]">{}</p>
  </div>
  <div><span class="border text-xs font-semibold px-2.5 py-1 rounded-full {}">{}</span></div>
</div>"#,
            t.id,
            t.crate_name,
            t.target_file,
            if t.instruction.is_empty() { &t.task } else { &t.instruction },
            badge_class,
            t.status
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Aaroneous DevTools Agentic Dashboard</title>
  <script src="https://www.gstatic.com/antigravity/web/dev/tailwindcss.min.js"></script>
</head>
<body class="bg-transparent text-[var(--foreground)] antialiased p-6">
  <div class="max-w-6xl mx-auto space-y-6">
    <div class="bg-[var(--card)] border border-[var(--border)] rounded-xl p-6 shadow-sm flex flex-col md:flex-row justify-between items-start md:items-center gap-4">
      <div>
        <h1 class="text-2xl font-bold text-[var(--foreground)]">Aaroneous DevTools Dashboard</h1>
        <p class="text-sm text-[var(--muted-foreground)] mt-1">Checkout: {} | Model: {}</p>
      </div>
    </div>
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <div class="bg-[var(--card)] border border-[var(--border)] p-4 rounded-xl shadow-sm"><div class="text-xs font-semibold text-[var(--muted-foreground)]">PENDING</div><div class="text-3xl font-extrabold text-amber-400 mt-1">{}</div></div>
      <div class="bg-[var(--card)] border border-[var(--border)] p-4 rounded-xl shadow-sm"><div class="text-xs font-semibold text-[var(--muted-foreground)]">AWAITING REVIEW</div><div class="text-3xl font-extrabold text-blue-400 mt-1">{}</div></div>
      <div class="bg-[var(--card)] border border-[var(--border)] p-4 rounded-xl shadow-sm"><div class="text-xs font-semibold text-[var(--muted-foreground)]">VERIFIED &amp; COMMITTED</div><div class="text-3xl font-extrabold text-emerald-500 mt-1">{}</div></div>
      <div class="bg-[var(--card)] border border-[var(--border)] p-4 rounded-xl shadow-sm"><div class="text-xs font-semibold text-[var(--muted-foreground)]">FAILED</div><div class="text-3xl font-extrabold text-rose-400 mt-1">{}</div></div>
    </div>
    <div class="bg-[var(--card)] border border-[var(--border)] rounded-xl p-6 shadow-sm space-y-4">
      <h2 class="text-lg font-semibold text-[var(--foreground)]">Task Queue Overview</h2>
      <div class="space-y-3">{}</div>
    </div>
  </div>
</body>
</html>"#,
        config.checkout.display(),
        config.model,
        pending,
        review,
        verified,
        failed,
        task_rows
    );

    fs::write(output_path, html)?;
    println!("Wrote DevTools Dashboard HTML: {}", output_path.display());
    Ok(())
}

pub fn print_status(config_path: &Path) -> Result<()> {
    let config = load_config(config_path)?;
    let manifest = load_manifest(&config.queue)?;

    println!("=== Agentic Registrar Status ===");
    println!("Checkout: {}", config.checkout.display());
    println!("Queue: {}", config.queue.display());
    println!("Total Tasks: {}", manifest.tasks.len());

    let pending = manifest
        .tasks
        .iter()
        .filter(|t| t.status == "Pending")
        .count();
    let review = manifest
        .tasks
        .iter()
        .filter(|t| t.status == "Review")
        .count();
    let verified = manifest
        .tasks
        .iter()
        .filter(|t| t.status == "Verified")
        .count();
    let failed = manifest
        .tasks
        .iter()
        .filter(|t| t.status == "Failed")
        .count();
    let blocked = manifest
        .tasks
        .iter()
        .filter(|t| t.status == "Blocked")
        .count();

    println!(
        "Status breakdown: {pending} Pending | {review} Awaiting Review | {verified} Verified | {failed} Failed | {blocked} Blocked"
    );

    let breaker = load_circuit_breaker(&config)?;
    if breaker.tripped {
        println!(
            "Circuit breaker: TRIPPED ({} consecutive verification failures) — \
             no further promotion will run until `reset-circuit-breaker`",
            breaker.consecutive_failures
        );
    } else if breaker.consecutive_failures > 0 {
        println!(
            "Circuit breaker: OK ({} consecutive verification failure(s) so far, trips at {})",
            breaker.consecutive_failures, CIRCUIT_BREAKER_THRESHOLD
        );
    } else {
        println!("Circuit breaker: OK");
    }

    for task in &manifest.tasks {
        println!(
            "  - [{}] {} ({}) -> {}",
            task.status, task.id, task.crate_name, task.task
        );
    }

    Ok(())
}

pub fn register_task(
    config_path: &Path,
    id: &str,
    crate_name: &str,
    task_desc: &str,
    target_file: &str,
    instruction: &str,
) -> Result<()> {
    let config = load_config(config_path)?;
    validate_target(&config.checkout, crate_name, target_file)?;
    if id.trim().is_empty()
        || id.len() > 64
        || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        bail!("task id must be a short alphanumeric/hyphen identifier: {id}");
    }
    if instruction.trim().is_empty() || instruction.len() > 8192 {
        bail!("instruction must be nonempty and at most 8192 bytes");
    }
    let now = now_secs()?;
    with_reservation(
        &config.reservation_directory,
        "foundation.agentic-registrar.register",
        now,
        || {
            let mut manifest = load_manifest(&config.queue)?;
            if manifest.tasks.iter().any(|t| t.id == id) {
                bail!("Task with ID '{id}' already exists in queue");
            }
            let outstanding = manifest
                .tasks
                .iter()
                .filter(|t| matches!(t.status.as_str(), "Pending" | "Review"))
                .count();
            if outstanding >= config.max_pending {
                bail!(
                    "queue backpressure: {outstanding} task(s) already outstanding (max_pending = {})",
                    config.max_pending
                );
            }
            manifest.tasks.push(TaskItem {
                id: id.to_string(),
                crate_name: crate_name.to_string(),
                task: task_desc.to_string(),
                target_file: target_file.to_string(),
                instruction: instruction.to_string(),
                status: "Pending".to_string(),
            });
            save_manifest(&config.queue, &manifest)?;
            println!("Registered new task '{id}' for crate '{crate_name}'");
            Ok(())
        },
    )
}

pub fn dispatch(config_path: &Path, target_task_id: Option<&str>) -> Result<()> {
    let config = load_config(config_path)?;
    let now = now_secs()?;
    with_reservation(
        &config.reservation_directory,
        "foundation.agentic-registrar.dispatch",
        now,
        || dispatch_locked(&config, target_task_id, now),
    )
}

fn dispatch_locked(config: &RegistrarConfig, target_task_id: Option<&str>, now: u64) -> Result<()> {
    let pending_results = files(&config.results_directory, "json")?.len();
    if pending_results >= config.max_pending {
        println!(
            "agentic registrar: backpressure — {pending_results} result(s) awaiting owner review"
        );
        return Ok(());
    }

    let manifest = load_manifest(&config.queue)?;
    let task = match manifest.tasks.iter().find(|t| {
        if let Some(tid) = target_task_id {
            t.id == tid && matches!(t.status.as_str(), "Pending" | "Failed")
        } else {
            t.status == "Pending"
        }
    }) {
        Some(t) => t.clone(),
        None => {
            println!("agentic registrar: no eligible pending tasks found");
            return Ok(());
        }
    };

    validate_target(&config.checkout, &task.crate_name, &task.target_file)?;

    let full_target_path = config.checkout.join(&task.target_file);
    let existing_code = if full_target_path.exists() {
        read_bounded(&full_target_path, 65536).unwrap_or_default()
    } else {
        String::new()
    };
    let instruction = if task.instruction.trim().is_empty() {
        task.task.clone()
    } else {
        task.instruction.clone()
    };

    let mut record = Record {
        schema_version: 1,
        artifact_kind: "foundation.agentic-registrar-result".into(),
        task_id: task.id.clone(),
        crate_name: task.crate_name.clone(),
        target_file: task.target_file.clone(),
        submitted_at_secs: now,
        outcome: "pending-owner-review".into(),
        instruction: instruction.clone(),
        existing_code_excerpt: existing_code.chars().take(4000).collect(),
        proposal: None,
        error: None,
    };

    let system_prompt = "You are a senior Rust systems programmer for the Aaroneous workspace. \
Do NOT output any thinking trace, reasoning, or markdown commentary outside code tags. \
Write clean, robust Rust code satisfying all zero-allocation, type-safety, and AGENTS.md rules. \
Output must be a single fenced ```rust code block containing the complete file and nothing else.";
    let user_prompt = format!(
        "Task ID: {}\nTarget Crate: {}\nTarget File: {}\nInstruction: {instruction}\n\n\
Existing Code Context:\n```rust\n{existing_code}\n```\n\n\
Synthesize the complete, pure Rust source for file '{}'. This is an unverified draft for owner \
review — it will not be applied or committed automatically.",
        task.id, task.crate_name, task.target_file, task.target_file
    );

    match invoke_ollama(&config.endpoint, &config.model, system_prompt, &user_prompt)
        .and_then(|raw| extract_rust_code(&raw))
    {
        Ok(rust_source) => record.proposal = Some(GeneratedDraft { rust_source }),
        Err(e) => {
            record.outcome = "failed".into();
            record.error = Some(e.to_string());
        }
    }

    persist(&config.results_directory, &record)?;
    append_progress_log(
        &config.progress_log,
        &task.id,
        &task.crate_name,
        &task.target_file,
        &record.outcome,
    )?;

    // Only after the record is durably persisted do we advance the task's
    // status. If generation, persistence, or the process itself failed
    // before this point, the task is untouched and stays "Pending" (or
    // "Failed", which is retryable) — never stuck in an unrecoverable
    // "Claimed" limbo with nothing to show for it.
    let mut manifest = load_manifest(&config.queue)?;
    if let Some(item) = manifest.tasks.iter_mut().find(|t| t.id == task.id) {
        item.status = if record.outcome == "failed" {
            "Failed".into()
        } else {
            "Review".into()
        };
    }
    save_manifest(&config.queue, &manifest)?;

    if record.outcome == "failed" {
        println!(
            "agentic registrar: task {} failed generation; see progress log",
            task.id
        );
    } else {
        println!(
            "agentic registrar: task {} -> pending owner review in {}",
            task.id,
            config.results_directory.display()
        );
    }
    Ok(())
}

pub fn daemon(config_path: &Path) -> Result<()> {
    let config = load_config(config_path)?;
    let manifest = load_manifest(&config.queue)?;

    let pending_ids: Vec<String> = manifest
        .tasks
        .iter()
        .filter(|t| t.status == "Pending")
        .map(|t| t.id.clone())
        .collect();

    if pending_ids.is_empty() {
        println!("agentic registrar daemon: queue is clean (0 pending tasks)");
        return Ok(());
    }

    println!(
        "agentic registrar daemon: processing {} pending task(s)...",
        pending_ids.len()
    );
    for id in pending_ids {
        if let Err(e) = dispatch(config_path, Some(&id)) {
            println!("agentic registrar daemon: task {id} dispatch returned error: {e}");
        }
    }

    Ok(())
}

/// The only function in this module that writes into the real checkout or
/// touches git — an explicit, separately-invoked acceptance step. Re-reads
/// the pending record, re-validates target/crate correlation, refuses a
/// high-risk change outright, checks the circuit breaker, then writes the
/// draft on an isolated branch and requires `cargo check` **and**
/// `cargo clippy -D warnings` to pass there before committing and pushing.
/// See the module doc comment for the full flow and why it stops at
/// pushing a branch rather than opening a pull request itself.
pub fn promote(config_path: &Path, task_id: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let now = now_secs()?;
    with_reservation(
        &config.reservation_directory,
        "foundation.agentic-registrar.promote",
        now,
        || promote_locked(&config, task_id),
    )
}

fn promote_locked(config: &RegistrarConfig, task_id: &str) -> Result<()> {
    check_circuit_breaker(config)?;

    let record_path = files(&config.results_directory, "json")?
        .into_iter()
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&format!("{task_id}-")))
        })
        .with_context(|| format!("no pending-owner-review result found for task {task_id}"))?;

    let record: Record = serde_json::from_str(&read_bounded(&record_path, 1_048_576)?)?;
    if record.outcome != "pending-owner-review" {
        bail!(
            "task {task_id} result outcome is '{}', not eligible for promotion",
            record.outcome
        );
    }
    let draft = record
        .proposal
        .as_ref()
        .context("record has no generated proposal to promote")?;

    if is_high_risk(&record.crate_name, &draft.rust_source) {
        archive_record(config, &record_path, "blocked-high-risk")?;
        update_task_status(config, task_id, "Blocked")?;
        bail!(
            "task {task_id} blocked by the high-risk allowlist (crate '{}' or generated \
             `unsafe` code) — local promotion is disabled for this class of change; escalate \
             to a cloud agent to implement it directly instead",
            record.crate_name
        );
    }

    validate_target(&config.checkout, &record.crate_name, &record.target_file)?;
    let full_target_path = config.checkout.join(&record.target_file);
    let pre_existing = full_target_path.exists();

    if !pre_existing {
        let crate_root = crate_root_for(&config.checkout, &record.target_file)
            .context("could not resolve crate root for target_file")?;
        ensure_new_file_is_reachable(&crate_root, &full_target_path)?;
    }

    let original_branch = git_current_branch(&config.checkout)?;
    if !git_working_tree_clean(&config.checkout)? {
        bail!(
            "checkout has uncommitted changes on branch '{original_branch}'; refusing to \
             promote onto a dirty working tree"
        );
    }

    let branch_name = format!("local-agent/{task_id}");
    git_checkout_new_branch(&config.checkout, &branch_name)
        .with_context(|| format!("failed to create isolated branch '{branch_name}'"))?;

    let outcome: Result<PromoteOutcome> = (|| {
        if let Some(parent) = full_target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_target_path, &draft.rust_source)?;

        println!(
            "agentic registrar: promoting {task_id} on branch '{branch_name}' — running \
             cargo check + clippy for '{}'...",
            record.crate_name
        );
        let verification = check_crate_compilation(&config.checkout, &record.crate_name)
            .and_then(|()| check_crate_clippy(&config.checkout, &record.crate_name));

        match verification {
            Ok(()) => {
                let hash = git_stage_and_commit(
                    &config.checkout,
                    &record.task_id,
                    &record.crate_name,
                    &record.target_file,
                    &record.instruction,
                )?;
                match git_push_branch(&config.checkout, &branch_name) {
                    Ok(()) => Ok(PromoteOutcome::Pushed(hash)),
                    Err(push_err) => Ok(PromoteOutcome::CommittedNotPushed(
                        hash,
                        push_err.to_string(),
                    )),
                }
            }
            Err(err_msg) => {
                if pre_existing {
                    git_rollback_existing_target(&config.checkout, &record.target_file)?;
                } else {
                    let _ = fs::remove_file(&full_target_path);
                }
                Ok(PromoteOutcome::VerificationFailed(err_msg))
            }
        }
    })();

    // Always attempt to restore the original branch before deciding what to
    // report, whatever happened above — a promotion attempt must never
    // leave the checkout sitting on a throwaway branch.
    let restore = git_checkout_branch(&config.checkout, &original_branch);
    let outcome = outcome?;
    restore.with_context(|| {
        format!(
            "promotion result computed but failed to restore original branch '{original_branch}'"
        )
    })?;

    match outcome {
        PromoteOutcome::Pushed(hash) => {
            archive_record(config, &record_path, "pushed-for-review")?;
            update_task_status(config, task_id, "Review")?;
            record_circuit_breaker_outcome(config, true)?;
            println!(
                "agentic registrar: pushed {task_id} to branch '{branch_name}' ({hash}) — \
                 open a PR for full CI + cloud-agent review"
            );
            Ok(())
        }
        PromoteOutcome::CommittedNotPushed(hash, push_err) => {
            // A push/network failure is not evidence the model or its
            // output is bad, so this does not count against the circuit
            // breaker. The commit and branch are left in place locally for
            // a manual `git push` retry rather than discarded.
            archive_record(config, &record_path, "committed-push-failed")?;
            update_task_status(config, task_id, "Blocked")?;
            bail!(
                "task {task_id} committed ({hash}) to local branch '{branch_name}' but push \
                 to origin failed, branch left in place for retry: {push_err}"
            );
        }
        PromoteOutcome::VerificationFailed(err_msg) => {
            git_delete_local_branch(&config.checkout, &branch_name);
            archive_record(config, &record_path, "promotion-failed")?;
            update_task_status(config, task_id, "Failed")?;
            record_circuit_breaker_outcome(config, false)?;
            bail!("promotion verification failed for {task_id}, rolled back:\n{err_msg}");
        }
    }
}

pub fn run_pipeline(config_path: &Path) -> Result<()> {
    println!("=== Running Closed-Loop Autonomous Development Pipeline ===");
    let config = load_config(config_path)?;

    println!(
        "Stage 1: Scanning codebase gaps & Git discovery in {}...",
        config.checkout.display()
    );
    let tracked_files = git_discover_repository(&config.checkout)?;
    let manifest = load_manifest(&config.queue)?;
    println!(
        "Stage 2: Queue registrar active with {} tasks. Tracked repository files: {}",
        manifest.tasks.len(),
        tracked_files.len()
    );

    println!("Stage 3: Running batch daemon dispatch (generation only, no auto-commit)...");
    daemon(config_path)?;

    println!(
        "Stage 4: Pipeline run complete. Generated drafts are pending owner review in {} — \
nothing was written to the checkout or committed. Run `promote --task-id <id>` to accept one.",
        config.results_directory.display()
    );
    Ok(())
}

pub fn git_discover_repository(checkout: &Path) -> Result<Vec<String>> {
    if !checkout.is_absolute() || !checkout.join(".git").exists() {
        bail!(
            "Checkout must be an absolute Git repository path: {}",
            checkout.display()
        );
    }

    let output = Command::new("git")
        .args(["ls-files"])
        .current_dir(checkout)
        .output()
        .context("Failed to run git ls-files for repository discovery")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git ls-files failed: {stderr}");
    }

    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    println!(
        "Git Discovery: Found {} tracked files in {}",
        files.len(),
        checkout.display()
    );
    Ok(files)
}

pub fn register_task_scheduler(
    binary_path: &Path,
    task_name: &str,
    interval_minutes: u32,
) -> Result<()> {
    if !cfg!(windows) {
        bail!("Task Scheduler registration is supported on Windows host platforms only");
    }

    let config_path = binary_path
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("config").join("registrar_config.toml"))
        .context("Failed to derive registrar_config.toml path relative to binary")?;

    let ps_script = format!(
        r#"$action = New-ScheduledTaskAction -Execute '{bin}' -Argument 'agentic-registrar daemon --config "{config}"'
$trigger = New-ScheduledTaskTrigger -Once -At (Get-Date) -RepetitionInterval (New-TimeSpan -Minutes {interval})
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable
Register-ScheduledTask -TaskName '{name}' -Action $action -Trigger $trigger -Settings $settings -Force
"#,
        bin = binary_path.display(),
        config = config_path.display(),
        interval = interval_minutes,
        name = task_name
    );

    let status = Command::new("pwsh")
        .args(["-Command", &ps_script])
        .status()
        .context("Failed to execute PowerShell Task Scheduler registration command")?;

    if status.success() {
        println!(
            "Successfully registered Windows Scheduled Task '{task_name}' to invoke foundation daemon every {interval_minutes} minutes."
        );
    } else {
        bail!(
            "PowerShell Task Scheduler registration failed with exit code {:?}",
            status.code()
        );
    }

    Ok(())
}

pub fn auto_register_ide_mcp_config(_port: u16) -> Result<Vec<PathBuf>> {
    bail!(
        "Implicit IDE configuration is retired. Configure the explicit registrar URL and X-Devtools-Token header in your chosen client; no settings were changed."
    )
}

fn handle_mcp_json_rpc(config_path: &Path, body: &str) -> (String, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => {
            return (
                "400 Bad Request".into(),
                serde_json::json!({"error": "Invalid JSON"}).to_string(),
            );
        }
    };

    let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

    let response = match method {
        "initialize" => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": { "listChanged": false }
                    },
                    "serverInfo": {
                        "name": "aaroneous-mcp-foundation",
                        "version": "0.1.0"
                    }
                }
            })
        }
        "notifications/initialized" => {
            return ("200 OK".into(), "".into());
        }
        "ping" => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {}
            })
        }
        "tools/list" => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "verify_gate_suite",
                            "description": "Run full 11-gate cargo xtask gate verification suite on Aaroneous codebase",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "get_repository_status",
                            "description": "Query repository branch, task queue status, and gate health",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "register_vibe_task",
                            "description": "Register a new task into the background execution queue. Produces an unverified draft awaiting explicit owner promotion — never applied or committed automatically.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "crate_name": { "type": "string" },
                                    "target_file": { "type": "string" },
                                    "instruction": { "type": "string" }
                                },
                                "required": ["crate_name", "target_file", "instruction"]
                            }
                        },
                        {
                            "name": "dispatch_queue_tick",
                            "description": "Generate a draft for the next pending task via local model synthesis. Writes a pending-owner-review record only; does not touch the checkout or git.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "audit_ast_invariants",
                            "description": "Run zero-alloc AST audit scanner on Aaroneous codebase",
                            "inputSchema": { "type": "object", "properties": {} }
                        }
                    ]
                }
            })
        }
        "tools/call" => {
            let params = req.get("params");
            let name = params
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");
            let args = params
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or(serde_json::json!({}));

            let outcome_text = match execute_mcp_tool(config_path, name, &args) {
                Ok(msg) => msg,
                Err(err) => format!("Error executing tool '{name}': {err}"),
            };

            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": outcome_text
                        }
                    ]
                }
            })
        }
        _ => {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("Method '{method}' not found")
                }
            })
        }
    };

    ("200 OK".into(), response.to_string())
}

fn execute_mcp_tool(
    config_path: &Path,
    tool_name: &str,
    args: &serde_json::Value,
) -> Result<String> {
    let config = load_config(config_path)?;
    match tool_name {
        "verify_gate_suite" => {
            let out = Command::new("cargo")
                .args(["run", "-p", "xtask", "--", "gate"])
                .current_dir(&config.checkout)
                .output()?;
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            Ok(format!(
                "Status Code: {}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}",
                out.status
            ))
        }
        "get_repository_status" => {
            let manifest = load_manifest(&config.queue)?;
            let pending = manifest
                .tasks
                .iter()
                .filter(|t| t.status == "Pending")
                .count();
            let review = manifest
                .tasks
                .iter()
                .filter(|t| t.status == "Review")
                .count();
            let verified = manifest
                .tasks
                .iter()
                .filter(|t| t.status == "Verified")
                .count();
            let failed = manifest
                .tasks
                .iter()
                .filter(|t| t.status == "Failed")
                .count();
            let blocked = manifest
                .tasks
                .iter()
                .filter(|t| t.status == "Blocked")
                .count();
            let breaker = load_circuit_breaker(&config)?;
            Ok(format!(
                "Checkout: {}\nModel: {}\nEndpoint: {}\nTotal Tasks: {}\nPending: {}\nAwaiting Review: {}\nVerified: {}\nFailed: {}\nBlocked: {}\nCircuit breaker tripped: {}",
                config.checkout.display(),
                config.model,
                config.endpoint,
                manifest.tasks.len(),
                pending,
                review,
                verified,
                failed,
                blocked,
                breaker.tripped
            ))
        }
        "register_vibe_task" => {
            let crate_name = args
                .get("crate_name")
                .and_then(|v| v.as_str())
                .context("Missing crate_name")?;
            let target_file = args
                .get("target_file")
                .and_then(|v| v.as_str())
                .context("Missing target_file")?;
            let instruction = args
                .get("instruction")
                .and_then(|v| v.as_str())
                .context("Missing instruction")?;
            let task_id = format!("TASK-{}", now_secs()?);
            register_task(
                config_path,
                &task_id,
                crate_name,
                instruction,
                target_file,
                instruction,
            )?;
            Ok(format!(
                "Successfully registered task {task_id} into queue (pending owner review after dispatch)"
            ))
        }
        "dispatch_queue_tick" => {
            dispatch(config_path, None)?;
            Ok("Dispatch tick completed; any generated draft is pending owner review, not applied.".to_string())
        }
        "audit_ast_invariants" => {
            let out = Command::new("cargo")
                .args([
                    "run",
                    "-p",
                    "ast_auditor",
                    "--",
                    "audit",
                    "core/",
                    "crates/",
                    "dev/",
                ])
                .current_dir(&config.checkout)
                .output()?;
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            Ok(format!(
                "AST Audit Status Code: {}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}",
                out.status
            ))
        }
        _ => bail!("Unknown MCP tool: '{tool_name}'"),
    }
}

const MAX_HTTP_HEADER: usize = 8192;
const MAX_HTTP_BODY: usize = 65536;
const MAX_HTTP_CONNECTIONS: usize = 8;

struct HttpAuth {
    host: String,
    origin: String,
    token: String,
}
impl HttpAuth {
    fn load(config: &RegistrarConfig, port: u16) -> Result<Self> {
        if port == 0 {
            bail!("A nonzero loopback port is required");
        }
        let path = config
            .http_token_file
            .as_ref()
            .context("Serve requires an explicit http_token_file")?;
        if !path.is_absolute()
            || path.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
            || within(path, &config.checkout)
        {
            bail!("HTTP token file must be an absolute path outside the product without traversal");
        }
        let mut ancestor = PathBuf::new();
        for component in path.components() {
            ancestor.push(component.as_os_str());
            if fs::symlink_metadata(&ancestor)?.file_type().is_symlink() {
                bail!("Token path must not contain symbolic links");
            }
        }
        let token = read_bounded(path, 128)?.trim().to_owned();
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            bail!("HTTP token must contain exactly 64 hexadecimal characters from 32 random bytes");
        }
        let host = format!("127.0.0.1:{port}");
        Ok(Self {
            origin: format!("http://{host}"),
            host,
            token,
        })
    }
    fn permits(&self, request: &HttpRequest) -> bool {
        request.headers.get("host") == Some(&self.host)
            && request
                .headers
                .get("x-devtools-token")
                .is_some_and(|value| constant_time_eq(value.as_bytes(), self.token.as_bytes()))
            && request
                .headers
                .get("origin")
                .is_none_or(|value| value == &self.origin)
    }
}

/// Constant-time byte comparison for the credential check above. `==` on
/// `&str`/`String` short-circuits on the first mismatch, leaking a timing
/// signal proportional to how many leading bytes of `X-Devtools-Token` an
/// attacker already guessed correctly — the same class of bug fixed via
/// `subtle::ConstantTimeEq` for `core/hypervisor`'s own API key check
/// (CHANGELOG SEC-04). Lengths differ openly (tokens are a fixed 64 hex
/// characters; length alone isn't secret), but every byte is compared.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
struct HttpRequest {
    method: String,
    path: String,
    headers: std::collections::BTreeMap<String, String>,
    body: String,
}
fn read_http(reader: &mut impl std::io::Read) -> Result<HttpRequest> {
    let mut raw = Vec::new();
    while !raw.ends_with(b"\r\n\r\n") {
        if raw.len() >= MAX_HTTP_HEADER {
            bail!("HTTP header exceeds limit");
        }
        let mut byte = [0u8; 1];
        reader
            .read_exact(&mut byte)
            .context("Incomplete HTTP header")?;
        raw.push(byte[0]);
    }
    let text = std::str::from_utf8(&raw).context("Invalid HTTP header encoding")?;
    let mut lines = text[..text.len() - 4].split("\r\n");
    let parts: Vec<_> = lines
        .next()
        .context("Request line missing")?
        .split(' ')
        .collect();
    if parts.len() != 3
        || parts[2] != "HTTP/1.1"
        || !matches!(parts[0], "GET" | "POST" | "OPTIONS")
        || !parts[1].starts_with('/')
    {
        bail!("Unsupported HTTP request line");
    }
    let mut headers = std::collections::BTreeMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').context("Malformed HTTP header")?;
        if name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !value.bytes().all(|b| b == b'\t' || (32..=126).contains(&b))
        {
            bail!("Invalid HTTP header");
        }
        if headers
            .insert(name.to_ascii_lowercase(), value.trim().to_owned())
            .is_some()
        {
            bail!("Duplicate HTTP header");
        }
    }
    if headers.contains_key("transfer-encoding") || headers.contains_key("expect") {
        bail!("Transfer encoding and Expect are unsupported");
    }
    let length = match headers.get("content-length") {
        Some(value) if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
            value.parse::<usize>()?
        }
        Some(_) => bail!("Invalid Content-Length"),
        None if parts[0] == "POST" => bail!("POST requires Content-Length"),
        None => 0,
    };
    if length > MAX_HTTP_BODY || (parts[0] != "POST" && length != 0) {
        bail!("HTTP body exceeds allowed bound");
    }
    if parts[0] == "POST"
        && headers.get("content-type").map(String::as_str) != Some("application/json")
    {
        bail!("POST requires application/json");
    }
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .context("Incomplete HTTP body")?;
    Ok(HttpRequest {
        method: parts[0].into(),
        path: parts[1].into(),
        headers,
        body: String::from_utf8(body)?,
    })
}
struct DeadlineReader<'a> {
    stream: &'a mut std::net::TcpStream,
    deadline: std::time::Instant,
}
impl std::io::Read for DeadlineReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self
            .deadline
            .checked_duration_since(std::time::Instant::now())
            .filter(|time| !time.is_zero())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "HTTP deadline exceeded")
            })?;
        self.stream.set_read_timeout(Some(remaining))?;
        std::io::Read::read(self.stream, buffer)
    }
}
fn http_response(stream: &mut std::net::TcpStream, status: &str, body: &str) -> Result<()> {
    use std::io::Write;
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}
struct ConnectionSlot(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl ConnectionSlot {
    fn acquire(active: &std::sync::Arc<std::sync::atomic::AtomicUsize>) -> Option<Self> {
        use std::sync::atomic::Ordering;
        let mut count = active.load(Ordering::Acquire);
        loop {
            if count >= MAX_HTTP_CONNECTIONS {
                return None;
            }
            match active.compare_exchange_weak(
                count,
                count + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(Self(std::sync::Arc::clone(active))),
                Err(current) => count = current,
            }
        }
    }
}
impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }
}
/// Authenticated loopback REST/MCP. No IDE settings are discovered or written.
pub fn serve(config_path: &Path, port: u16) -> Result<()> {
    use std::{
        net::TcpListener,
        sync::{Arc, atomic::AtomicUsize},
    };
    let auth = Arc::new(HttpAuth::load(&load_config(config_path)?, port)?);
    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("Failed to bind server to 127.0.0.1:{port}"))?;
    println!(
        "Authenticated registrar listening on http://127.0.0.1:{port}; configure the X-Devtools-Token header explicitly."
    );
    let active = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        let Some(slot) = ConnectionSlot::acquire(&active) else {
            // Close immediately rather than allowing an overloaded client to stall acceptance.
            drop(stream);
            continue;
        };
        let config_path = config_path.to_path_buf();
        let auth = Arc::clone(&auth);
        std::thread::spawn(move || {
            let _slot = slot;
            let _ = handle_connection(&config_path, stream, &auth);
        });
    }
    Ok(())
}
fn handle_connection(
    config_path: &Path,
    mut stream: std::net::TcpStream,
    auth: &HttpAuth,
) -> Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let request = match read_http(&mut std::io::BufReader::new(DeadlineReader {
        stream: &mut stream,
        deadline: std::time::Instant::now() + Duration::from_secs(5),
    })) {
        Ok(request) => request,
        Err(_) => {
            return http_response(
                &mut stream,
                "400 Bad Request",
                r#"{"error":"Invalid, incomplete or oversized HTTP request"}"#,
            );
        }
    };
    if !auth.permits(&request) {
        return http_response(
            &mut stream,
            "403 Forbidden",
            r#"{"error":"Explicit token, exact loopback Host and matching Origin required"}"#,
        );
    }
    let method = request.method.as_str();
    let path = request.path.as_str();
    let body_str = request.body.as_str();

    let (status_code, body) = match (method, path) {
        ("OPTIONS", _) => ("405 Method Not Allowed".to_string(), "{}".to_string()),
        ("POST", "/mcp") | ("POST", "/") => handle_mcp_json_rpc(config_path, body_str),
        ("GET", "/mcp") | ("GET", "/health") => (
            "200 OK".to_string(),
            serde_json::json!({
                "status": "ok",
                "mcp": "Anthropic Model Context Protocol 2024-11-05",
                "service": "aaroneous-mcp-foundation"
            })
            .to_string(),
        ),
        ("GET", "/api/status") => {
            match load_config(config_path).and_then(|cfg| {
                let manifest = load_manifest(&cfg.queue)?;
                let pending = manifest
                    .tasks
                    .iter()
                    .filter(|t| t.status == "Pending")
                    .count();
                let review = manifest
                    .tasks
                    .iter()
                    .filter(|t| t.status == "Review")
                    .count();
                let verified = manifest
                    .tasks
                    .iter()
                    .filter(|t| t.status == "Verified")
                    .count();
                let failed = manifest
                    .tasks
                    .iter()
                    .filter(|t| t.status == "Failed")
                    .count();
                let blocked = manifest
                    .tasks
                    .iter()
                    .filter(|t| t.status == "Blocked")
                    .count();
                let breaker = load_circuit_breaker(&cfg)?;
                Ok(serde_json::json!({
                    "checkout": cfg.checkout.display().to_string(),
                    "model": cfg.model,
                    "endpoint": cfg.endpoint,
                    "total_tasks": manifest.tasks.len(),
                    "pending": pending,
                    "review": review,
                    "verified": verified,
                    "failed": failed,
                    "blocked": blocked,
                    "circuit_breaker_tripped": breaker.tripped
                }))
            }) {
                Ok(val) => ("200 OK".to_string(), val.to_string()),
                Err(e) => (
                    "500 Internal Error".to_string(),
                    serde_json::json!({"error": e.to_string()}).to_string(),
                ),
            }
        }
        ("GET", "/api/tasks") => {
            match load_config(config_path).and_then(|cfg| load_manifest(&cfg.queue)) {
                Ok(manifest) => (
                    "200 OK".to_string(),
                    serde_json::to_string(&manifest.tasks).unwrap_or_default(),
                ),
                Err(e) => (
                    "500 Internal Error".to_string(),
                    serde_json::json!({"error": e.to_string()}).to_string(),
                ),
            }
        }
        ("GET", "/api/logs") => {
            match load_config(config_path).map(|cfg| {
                let content = if cfg.progress_log.exists() {
                    read_bounded(&cfg.progress_log, 262144).unwrap_or_default()
                } else {
                    String::new()
                };
                serde_json::json!({ "log": content })
            }) {
                Ok(val) => ("200 OK".to_string(), val.to_string()),
                Err(e) => (
                    "500 Internal Error".to_string(),
                    serde_json::json!({"error": e.to_string()}).to_string(),
                ),
            }
        }
        _ => (
            "404 Not Found".to_string(),
            serde_json::json!({"error": "Endpoint not found"}).to_string(),
        ),
    };

    http_response(&mut stream, &status_code, &body)
}

/// Explicit loopback-only Ollama chat call: rejects anything but a bare
/// `http://{localhost,127.0.0.1,[::1]}` origin (no credentials, query, or
/// fragment) before ever dialing out, and bounds the response to 1 MiB —
/// same discipline `local_worker.rs`'s `delegate()` already uses.
fn invoke_ollama(endpoint: &str, model: &str, system: &str, user: &str) -> Result<String> {
    let url = reqwest::Url::parse(endpoint).context("invalid endpoint URL")?;
    if url.scheme() != "http"
        || !matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("explicit loopback HTTP endpoint required: {endpoint}");
    }

    let req = ModelRequest {
        model: model.to_string(),
        stream: false,
        think: false,
        options: ModelOptions {
            temperature: 0.0,
            num_predict: 4096,
        },
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user.to_string(),
            },
        ],
    };

    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(120))
        .build()?;

    let response = client
        .post(format!("{}/api/chat", endpoint.trim_end_matches('/')))
        .json(&req)
        .send()?
        .error_for_status()?;

    use std::io::Read;
    let mut body = String::new();
    response.take(1_048_577).read_to_string(&mut body)?;
    if body.len() > 1_048_576 {
        bail!("model response exceeds 1 MiB");
    }

    let value: serde_json::Value =
        serde_json::from_str(&body).context("invalid Ollama response JSON")?;
    let content = value["message"]["content"]
        .as_str()
        .context("Missing content field in Ollama chat response")?;

    Ok(content.to_string())
}

/// Extract the generated Rust source from a model response. Uses the
/// *last* closing fence rather than the first one after the opening fence,
/// so a nested ``` inside a rustdoc example within the generated code
/// (a common, idiomatic pattern) doesn't truncate the real function body.
/// Returns an error rather than silently truncating when no usable code
/// can be identified, so a bad extraction shows up as a "failed" record
/// instead of a plausible-looking but broken draft.
fn extract_rust_code(text: &str) -> Result<String> {
    for marker in ["```rust", "```rs", "```"] {
        if let Some(start) = text.find(marker) {
            let code_start = start + marker.len();
            if let Some(rel_end) = text[code_start..].rfind("```") {
                let code = text[code_start..code_start + rel_end].trim();
                if !code.is_empty() {
                    return Ok(code.to_string());
                }
            }
        }
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        bail!("model response contained no extractable Rust code");
    }
    Ok(trimmed.to_string())
}

fn git_stage_and_commit(
    checkout: &Path,
    task_id: &str,
    crate_name: &str,
    target_file: &str,
    instruction: &str,
) -> Result<String> {
    let add_status = Command::new("git")
        .args(["add", "--", target_file])
        .current_dir(checkout)
        .status()
        .context("Failed to run git add")?;

    if !add_status.success() {
        bail!("git add failed for target file: {target_file}");
    }

    let commit_msg = format!("feat({crate_name}): {instruction} ({task_id})");
    let commit_out = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(checkout)
        .output()
        .context("Failed to run git commit")?;

    if !commit_out.status.success() {
        let stderr = String::from_utf8_lossy(&commit_out.stderr);
        bail!("git commit note: {stderr}");
    }

    let rev_out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(checkout)
        .output()
        .context("Failed to get git commit hash")?;

    let hash = String::from_utf8_lossy(&rev_out.stdout).trim().to_string();
    Ok(hash)
}

/// Roll back a target file that git already had tracked. Callers must
/// branch on whether the file pre-existed *before* calling this: `git
/// checkout -- {file}` is a silent no-op for a file git has never seen,
/// so it must never be used for a newly-created file (see `promote_locked`,
/// which uses `fs::remove_file` for that case instead).
fn git_rollback_existing_target(checkout: &Path, target_file: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["checkout", "--", target_file])
        .current_dir(checkout)
        .status()
        .context("Failed to execute git checkout rollback")?;

    if status.success() {
        println!("agentic registrar: git rollback SUCCEEDED for {target_file}");
        Ok(())
    } else {
        bail!("git rollback failed for {target_file}");
    }
}

/// Outcome of the verify-and-push closure inside `promote_locked`, kept
/// distinct from a hard `Err` (an I/O failure in the closure itself) so the
/// caller can still always attempt to restore the original branch before
/// deciding what to report.
enum PromoteOutcome {
    Pushed(String),
    CommittedNotPushed(String, String),
    VerificationFailed(String),
}

fn git_current_branch(checkout: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(checkout)
        .output()
        .context("Failed to run git rev-parse")?;
    if !output.status.success() {
        bail!(
            "git rev-parse --abbrev-ref HEAD failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        bail!("checkout is in a detached HEAD state; check out a real branch before promoting");
    }
    Ok(branch)
}

fn git_working_tree_clean(checkout: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(checkout)
        .output()
        .context("Failed to run git status")?;
    if !output.status.success() {
        bail!(
            "git status failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output.stdout.is_empty())
}

fn git_checkout_new_branch(checkout: &Path, branch: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(checkout)
        .status()
        .context("Failed to run git checkout -b")?;
    if !status.success() {
        bail!("git checkout -b {branch} failed");
    }
    Ok(())
}

/// Restore the branch that was checked out before `promote_locked` started.
/// Called on every path out of `promote_locked` — success, verification
/// failure, or push failure — so a promotion attempt never leaves the
/// user's checkout sitting on a throwaway `local-agent/*` branch.
fn git_checkout_branch(checkout: &Path, branch: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["checkout", branch])
        .current_dir(checkout)
        .status()
        .context("Failed to run git checkout (restore)")?;
    if !status.success() {
        bail!("failed to restore original branch '{branch}' after promotion attempt");
    }
    Ok(())
}

/// Best-effort cleanup of a throwaway local branch after a failed
/// promotion. Never called while `branch` is still checked out. Failure
/// here is logged, not propagated — a leftover local branch is untidy, not
/// unsafe, and must never mask the real verification failure that's already
/// being reported.
fn git_delete_local_branch(checkout: &Path, branch: &str) {
    let status = Command::new("git")
        .args(["branch", "-D", branch])
        .current_dir(checkout)
        .status();
    if !matches!(status, Ok(s) if s.success()) {
        eprintln!("agentic registrar: warning: failed to delete leftover local branch '{branch}'");
    }
}

fn git_push_branch(checkout: &Path, branch: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["push", "-u", "origin", branch])
        .current_dir(checkout)
        .status()
        .context("Failed to run git push")?;
    if !status.success() {
        bail!("git push -u origin {branch} failed");
    }
    Ok(())
}

fn check_crate_compilation(checkout: &Path, crate_name: &str) -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["check", "-p", crate_name, "--all-targets"])
        .current_dir(checkout)
        .output()
        .map_err(|e| format!("Failed to spawn cargo check: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Err(format!("cargo check stderr:\n{stderr}\nstdout:\n{stdout}"))
    }
}

fn check_crate_clippy(checkout: &Path, crate_name: &str) -> Result<(), String> {
    let output = Command::new("cargo")
        .args([
            "clippy",
            "-p",
            crate_name,
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .current_dir(checkout)
        .output()
        .map_err(|e| format!("Failed to spawn cargo clippy: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Err(format!("cargo clippy stderr:\n{stderr}\nstdout:\n{stdout}"))
    }
}

fn append_progress_log(
    log_path: &Path,
    task_id: &str,
    crate_name: &str,
    target_file: &str,
    outcome: &str,
) -> Result<()> {
    let now = now_secs()?;
    let entry = format!("\n- [{now}] Task {task_id} ({crate_name} -> {target_file}): {outcome}\n");

    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    file.write_all(entry.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn http_framing_requires_complete_bounded_unambiguous_requests() {
        fn parse(raw: &str) -> Result<HttpRequest> {
            read_http(&mut std::io::Cursor::new(raw.as_bytes()))
        }
        let request = parse("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:8080\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}").expect("complete");
        assert_eq!(request.body, "{}");
        for raw in [
            "GET /health HTTP/1.1\r\nHost: a\r\nHost: b\r\n\r\n",
            "GET /health HTTP/1.1\r\nHost: a\r\nhOsT: b\r\n\r\n",
            "POST /mcp HTTP/1.1\r\nContent-Type: application/json\r\n\r\n{}",
            "POST /mcp HTTP/1.1\r\nContent-Length: 2\r\n\r\n{}",
            "POST /mcp HTTP/1.1\r\nContent-Length: 3\r\nContent-Type: application/json\r\n\r\n{}",
            "POST /mcp HTTP/1.1\r\nContent-Length: 65537\r\nContent-Type: application/json\r\n\r\n",
            "POST /mcp HTTP/1.1\r\nContent-Length: +2\r\nContent-Type: application/json\r\n\r\n{}",
            "POST /mcp HTTP/1.1\r\nContent-Length: 2\r\nTransfer-Encoding: chunked\r\nContent-Type: application/json\r\n\r\n{}",
            "GET /health HTTP/1.1\r\nContent-Length: 1\r\n\r\nx",
            "GET /health HTTP/1.0\r\n\r\n",
            "GET /health HTTP/1.1\r\nX: folded\r\n continuation\r\n\r\n",
        ] {
            assert!(parse(raw).is_err(), "{raw}");
        }
        assert!(
            parse(&format!(
                "GET /health HTTP/1.1\r\nX: {}\r\n\r\n",
                "x".repeat(MAX_HTTP_HEADER)
            ))
            .is_err()
        );
    }
    #[test]
    fn serve_requires_explicit_valid_token_and_legacy_setup_never_writes() {
        let (temp, mut config, _) = fixture();
        assert!(HttpAuth::load(&config, 8080).is_err());
        config.http_token_file = Some(PathBuf::from("relative-token"));
        assert!(HttpAuth::load(&config, 8080).is_err());
        let path = temp.path().join("token");
        config.http_token_file = Some(path.clone());
        fs::write(&path, "weak").expect("token");
        assert!(HttpAuth::load(&config, 8080).is_err());
        fs::write(&path, "a".repeat(64)).expect("token");
        assert!(HttpAuth::load(&config, 0).is_err());
        assert!(HttpAuth::load(&config, 8080).is_ok());
        config.http_token_file = Some(config.checkout.join("token"));
        assert!(HttpAuth::load(&config, 8080).is_err());
        assert!(auto_register_ide_mcp_config(8080).is_err());
    }
    #[test]
    fn connection_slots_are_bounded_and_released() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let active = Arc::new(AtomicUsize::new(0));
        let mut slots: Vec<_> = (0..MAX_HTTP_CONNECTIONS)
            .map(|_| ConnectionSlot::acquire(&active).expect("slot"))
            .collect();
        assert!(ConnectionSlot::acquire(&active).is_none());
        slots.pop();
        assert!(ConnectionSlot::acquire(&active).is_some());
        drop(slots);
        assert_eq!(active.load(Ordering::Acquire), 0);
    }
    #[test]
    fn auth_requires_exact_host_token_and_optional_same_origin() {
        let auth = HttpAuth {
            host: "127.0.0.1:8080".into(),
            origin: "http://127.0.0.1:8080".into(),
            token: "a".repeat(64),
        };
        for (host, token, origin, expected) in [
            ("127.0.0.1:8080", "a".repeat(64), "", true),
            (
                "127.0.0.1:8080",
                "a".repeat(64),
                "Origin: http://127.0.0.1:8080\r\n",
                true,
            ),
            ("localhost:8080", "a".repeat(64), "", false),
            ("attacker.example:8080", "a".repeat(64), "", false),
            ("127.0.0.1:8080", "wrong".into(), "", false),
            ("127.0.0.1:8080", "a".repeat(64), "Origin: null\r\n", false),
            (
                "127.0.0.1:8080",
                "a".repeat(64),
                "Origin: http://127.0.0.1:5173\r\n",
                false,
            ),
        ] {
            let raw = format!(
                "GET /health HTTP/1.1\r\nHost: {host}\r\nX-Devtools-Token: {token}\r\n{origin}\r\n"
            );
            let request = read_http(&mut std::io::Cursor::new(raw)).expect("request");
            assert_eq!(auth.permits(&request), expected, "{host} {origin}");
        }
    }
    #[test]
    fn expired_read_deadline_fails_without_waiting_for_client() {
        use std::io::Read;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
        let _client =
            std::net::TcpStream::connect(listener.local_addr().expect("address")).expect("client");
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = DeadlineReader {
            stream: &mut stream,
            deadline: std::time::Instant::now(),
        };
        assert_eq!(
            reader.read(&mut [0]).expect_err("expired").kind(),
            std::io::ErrorKind::TimedOut
        );
    }
    #[test]
    fn loopback_http_authentication_precedes_rpc_and_has_no_cors_bypass() {
        use std::{
            io::{Read, Write},
            net::{Shutdown, TcpListener, TcpStream},
        };
        let (_temp, config, config_path) = fixture();
        fn exchange(
            config_path: &Path,
            headers: &str,
            method: &str,
            path: &str,
            body: &str,
        ) -> String {
            let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
            let address = listener.local_addr().expect("address");
            let auth = HttpAuth {
                host: address.to_string(),
                origin: format!("http://{address}"),
                token: "a".repeat(64),
            };
            let config_path = config_path.to_owned();
            let server = std::thread::spawn(move || {
                let (stream, _) = listener.accept().expect("accept");
                handle_connection(&config_path, stream, &auth).expect("connection");
            });
            let request = format!(
                "{method} {path} HTTP/1.1\r\nHost: {address}\r\n{headers}Content-Length: {}\r\n\r\n{body}",
                body.len()
            );
            let mut client = TcpStream::connect(address).expect("connect");
            client
                .set_read_timeout(Some(Duration::from_secs(10)))
                .expect("timeout");
            // Feed separate fragments: the server must not assume one read is one request.
            for fragment in request.as_bytes().chunks(7) {
                client.write_all(fragment).expect("write");
            }
            client.shutdown(Shutdown::Write).expect("shutdown");
            let mut response = String::new();
            client.read_to_string(&mut response).expect("response");
            server.join().expect("server");
            response
        }
        let token = format!("X-Devtools-Token: {}\r\n", "a".repeat(64));
        let positive = exchange(&config_path, &token, "GET", "/health", "");
        assert!(positive.starts_with("HTTP/1.1 200 OK"));
        assert!(!positive.contains("Access-Control-Allow"));
        assert!(positive.contains("Cache-Control: no-store"));
        let rpc = exchange(
            &config_path,
            &format!("{token}Content-Type: application/json\r\n"),
            "POST",
            "/mcp",
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        );
        assert!(rpc.starts_with("HTTP/1.1 200 OK"));
        assert!(rpc.contains("serverInfo"));
        for headers in [
            String::new(),
            "X-Devtools-Token: wrong\r\n".into(),
            format!("{token}Origin: https://attacker.example\r\n"),
        ] {
            let response = exchange(
                &config_path,
                &format!("{headers}Content-Type: application/json\r\n"),
                "POST",
                "/mcp",
                r#"{"method":"tools/call","params":{"name":"register_task"}}"#,
            );
            assert!(response.starts_with("HTTP/1.1 403 Forbidden"));
        }
        assert!(
            !config.queue.exists(),
            "Unauthenticated RPC must not create queue state"
        );
        let options = exchange(&config_path, &token, "OPTIONS", "/mcp", "");
        assert!(options.starts_with("HTTP/1.1 405 Method Not Allowed"));
        assert!(!options.contains("Access-Control-Allow"));
        let sse = exchange(&config_path, &token, "GET", "/sse", "");
        assert!(sse.starts_with("HTTP/1.1 404 Not Found"));
    }

    /// Initializes a real git repository at `checkout` (not just a `.git`
    /// marker directory) with an initial commit on `main`, and adds a bare
    /// repository under `scratch_root` as its `origin` remote, so
    /// `promote`'s branch/commit/push path is exercised for real against a
    /// local remote rather than needing network access in tests.
    fn init_git_fixture(checkout: &Path, scratch_root: &Path) {
        let run = |args: &[&str], cwd: &Path| {
            let status = Command::new("git")
                .args(args)
                .current_dir(cwd)
                .status()
                .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));
            assert!(status.success(), "git {args:?} failed in {}", cwd.display());
        };

        run(&["init", "-q", "-b", "main"], checkout);
        run(&["config", "user.email", "test@example.com"], checkout);
        run(&["config", "user.name", "Test"], checkout);
        run(&["add", "-A"], checkout);
        run(&["commit", "-q", "-m", "initial"], checkout);

        let origin = scratch_root.join("origin.git");
        fs::create_dir_all(&origin).expect("origin dir");
        run(&["init", "-q", "--bare", "-b", "main"], &origin);
        run(
            &[
                "remote",
                "add",
                "origin",
                origin.to_str().expect("utf8 path"),
            ],
            checkout,
        );
        run(&["push", "-q", "-u", "origin", "main"], checkout);
    }

    fn fixture() -> (tempfile::TempDir, RegistrarConfig, PathBuf) {
        let d = tempfile::tempdir().expect("fixture");
        let checkout = d.path().join("checkout");
        let crate_dir = checkout.join("crates").join("demo");
        fs::create_dir_all(crate_dir.join("src")).expect("crate dir");
        fs::write(
            checkout.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/demo\"]\nresolver = \"2\"\n",
        )
        .expect("workspace cargo toml");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("cargo toml");
        fs::write(
            crate_dir.join("src").join("lib.rs"),
            "pub fn existing() {}\n",
        )
        .expect("lib.rs");
        // Real Aaroneous checkouts already gitignore this; without it here,
        // the very first `cargo check`/`clippy` promote runs would create
        // an untracked target/ dir and make every later
        // `git_working_tree_clean` check see a permanently dirty tree.
        fs::write(checkout.join(".gitignore"), "/target\n/Cargo.lock\n").expect(".gitignore");
        init_git_fixture(&checkout, d.path());

        let config = RegistrarConfig {
            schema_version: 1,
            checkout: checkout.clone(),
            queue: d.path().join("task_queue.json"),
            results_directory: d.path().join("results"),
            archive_directory: d.path().join("archive"),
            reservation_directory: d.path().join("reservation"),
            progress_log: d.path().join("progress.md"),
            endpoint: "http://127.0.0.1:1".into(),
            model: "test-model".into(),
            max_pending: 4,
            http_token_file: None,
            circuit_breaker_path: None,
            devtools_repo_root: None,
        };
        fs::create_dir_all(&config.results_directory).expect("results dir");
        fs::create_dir_all(&config.archive_directory).expect("archive dir");
        fs::create_dir_all(&config.reservation_directory).expect("reservation dir");

        let config_path = d.path().join("registrar.toml");
        fs::write(
            &config_path,
            toml::to_string(&config).expect("serialize config"),
        )
        .expect("write config");
        (d, config, config_path)
    }

    #[test]
    fn extract_rust_code_handles_nested_fences() {
        let raw = "```rust\n/// Example:\n/// ```rust\n/// foo();\n/// ```\nfn real_code() {}\n```";
        let extracted = extract_rust_code(raw).expect("extraction");
        assert!(extracted.contains("fn real_code"), "extracted: {extracted}");
    }

    #[test]
    fn extract_rust_code_rejects_empty_response() {
        assert!(extract_rust_code("   ").is_err());
    }

    #[test]
    fn extract_rust_code_handles_plain_unfenced_output() {
        assert_eq!(extract_rust_code("fn pure() {}").unwrap(), "fn pure() {}");
    }

    #[test]
    fn validate_target_accepts_matching_crate_and_file() {
        let (_d, config, _cfg_path) = fixture();
        validate_target(&config.checkout, "demo", "crates/demo/src/lib.rs").expect("valid pair");
    }

    #[test]
    fn validate_target_rejects_mismatched_crate_name() {
        let (_d, config, _cfg_path) = fixture();
        let err = validate_target(
            &config.checkout,
            "unrelated_crate",
            "crates/demo/src/lib.rs",
        )
        .expect_err("mismatch must be rejected");
        assert!(err.to_string().contains("does not belong"));
    }

    #[test]
    fn validate_target_rejects_path_traversal() {
        let (_d, config, _cfg_path) = fixture();
        assert!(validate_target(&config.checkout, "demo", "../../etc/passwd").is_err());
        assert!(validate_target(&config.checkout, "demo", "C:/Windows/System32/evil.rs").is_err());
    }

    #[test]
    fn validate_target_rejects_dash_prefixed_git_option_like_paths() {
        let (_d, config, _cfg_path) = fixture();
        // relative_file() forbids no dash specifically, but this must still
        // fail because it can never resolve to a real crate directory.
        assert!(validate_target(&config.checkout, "demo", "--interactive").is_err());
    }

    #[test]
    fn register_task_enforces_backpressure() {
        let (_d, config, cfg_path) = fixture();
        for i in 0..config.max_pending {
            register_task(
                &cfg_path,
                &format!("T{i}"),
                "demo",
                "task",
                "crates/demo/src/lib.rs",
                "do work",
            )
            .unwrap_or_else(|e| panic!("register {i}: {e}"));
        }
        let err = register_task(
            &cfg_path,
            "T-overflow",
            "demo",
            "task",
            "crates/demo/src/lib.rs",
            "do work",
        )
        .expect_err("backpressure must reject further registration");
        assert!(err.to_string().contains("backpressure"));
    }

    #[test]
    fn register_task_rejects_duplicate_ids() {
        let (_d, _config, cfg_path) = fixture();
        register_task(
            &cfg_path,
            "DUP",
            "demo",
            "task",
            "crates/demo/src/lib.rs",
            "do work",
        )
        .expect("first");
        let err = register_task(
            &cfg_path,
            "DUP",
            "demo",
            "task",
            "crates/demo/src/lib.rs",
            "do work",
        )
        .expect_err("duplicate id must be rejected");
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn dispatch_never_touches_checkout_or_git_on_generation_failure() {
        let (_d, config, cfg_path) = fixture();
        register_task(
            &cfg_path,
            "FAIL-1",
            "demo",
            "task",
            "crates/demo/src/lib.rs",
            "do work",
        )
        .expect("register");
        // endpoint is an unreachable loopback port, so generation fails —
        // dispatch must still succeed overall (it records the failure).
        dispatch(&cfg_path, Some("FAIL-1")).expect("dispatch records failure, does not error");

        let original =
            fs::read_to_string(config.checkout.join("crates/demo/src/lib.rs")).expect("read");
        assert_eq!(
            original, "pub fn existing() {}\n",
            "checkout must be untouched"
        );

        let manifest = load_manifest(&config.queue).expect("manifest");
        let task = manifest
            .tasks
            .iter()
            .find(|t| t.id == "FAIL-1")
            .expect("task");
        assert_eq!(task.status, "Failed");

        let results = files(&config.results_directory, "json").expect("results");
        assert_eq!(
            results.len(),
            1,
            "a failed record must still be persisted for review"
        );
    }

    #[test]
    fn reservation_lock_prevents_concurrent_operations() {
        let (_d, config, _cfg_path) = fixture();
        fs::write(
            config.reservation_directory.join("review.lock"),
            "owner=test\n",
        )
        .expect("preexisting lock");
        let err = with_reservation(&config.reservation_directory, "test", 1, || {
            Ok::<(), anyhow::Error>(())
        })
        .expect_err("held lock must be respected");
        assert!(err.to_string().contains("busy"));
    }

    #[test]
    fn reservation_is_released_after_use() {
        let (_d, config, _cfg_path) = fixture();
        with_reservation(&config.reservation_directory, "test", 1, || {
            Ok::<(), anyhow::Error>(())
        })
        .expect("first use");
        assert!(!config.reservation_directory.join("review.lock").exists());
        with_reservation(&config.reservation_directory, "test", 2, || {
            Ok::<(), anyhow::Error>(())
        })
        .expect("second use after release");
    }

    #[test]
    fn promote_rolls_back_new_file_by_removing_it_not_git_checkout() {
        let (_d, config, cfg_path) = fixture();
        register_task(
            &cfg_path,
            "NEWFILE",
            "demo",
            "task",
            "crates/demo/tests/new_module_test.rs",
            "add broken code",
        )
        .expect("register");

        // Directly persist a record with source that will fail clippy (an
        // unused variable) rather than depending on a live Ollama endpoint.
        let now = now_secs().unwrap();
        let record = Record {
            schema_version: 1,
            artifact_kind: "foundation.agentic-registrar-result".into(),
            task_id: "NEWFILE".into(),
            crate_name: "demo".into(),
            target_file: "crates/demo/tests/new_module_test.rs".into(),
            submitted_at_secs: now,
            outcome: "pending-owner-review".into(),
            instruction: "add broken code".into(),
            existing_code_excerpt: String::new(),
            proposal: Some(GeneratedDraft {
                rust_source: "pub fn broken() { let unused = 1; }\n".into(),
            }),
            error: None,
        };
        persist(&config.results_directory, &record).expect("persist");

        let target = config.checkout.join("crates/demo/tests/new_module_test.rs");
        assert!(!target.exists(), "file must not exist before promotion");

        let err = promote(&cfg_path, "NEWFILE").expect_err("clippy must fail on unused variable");
        assert!(err.to_string().contains("rolled back"));
        assert!(
            !target.exists(),
            "newly-created file must be removed on failed promotion, not left behind"
        );

        let manifest = load_manifest(&config.queue).expect("manifest");
        let task = manifest
            .tasks
            .iter()
            .find(|t| t.id == "NEWFILE")
            .expect("task");
        assert_eq!(task.status, "Failed");
        assert_eq!(
            git_current_branch(&config.checkout).expect("branch"),
            "main",
            "a failed promotion must leave the checkout back on its original branch"
        );
    }

    fn record_for(task_id: &str, crate_name: &str, target_file: &str, source: &str) -> Record {
        Record {
            schema_version: 1,
            artifact_kind: "foundation.agentic-registrar-result".into(),
            task_id: task_id.into(),
            crate_name: crate_name.into(),
            target_file: target_file.into(),
            submitted_at_secs: now_secs().unwrap(),
            outcome: "pending-owner-review".into(),
            instruction: "test instruction".into(),
            existing_code_excerpt: String::new(),
            proposal: Some(GeneratedDraft {
                rust_source: source.into(),
            }),
            error: None,
        }
    }

    #[test]
    fn is_high_risk_flags_denylisted_crates_and_unsafe_source() {
        assert!(is_high_risk("ipc_bus", "pub fn f() {}"));
        assert!(is_high_risk("hypervisor", "pub fn f() {}"));
        assert!(is_high_risk(
            "demo",
            "pub fn f() { unsafe { core::ptr::null::<u8>(); } }"
        ));
        assert!(!is_high_risk("demo", "pub fn f() {}"));
    }

    #[test]
    fn promote_blocks_high_risk_crate_without_touching_git() {
        let (_d, config, cfg_path) = fixture();
        register_task(
            &cfg_path,
            "RISKY",
            "demo",
            "task",
            "crates/demo/tests/new_module_test.rs",
            "touch something risky",
        )
        .expect("register");
        persist(
            &config.results_directory,
            &record_for(
                "RISKY",
                "demo",
                "crates/demo/tests/new_module_test.rs",
                "pub fn f() { unsafe { core::ptr::null::<u8>(); } }\n",
            ),
        )
        .expect("persist");

        let err = promote(&cfg_path, "RISKY").expect_err("high-risk must be blocked");
        assert!(err.to_string().contains("high-risk allowlist"), "{err}");
        assert!(
            !config
                .checkout
                .join("crates/demo/tests/new_module_test.rs")
                .exists(),
            "a blocked task must never write into the checkout"
        );

        let manifest = load_manifest(&config.queue).expect("manifest");
        let task = manifest
            .tasks
            .iter()
            .find(|t| t.id == "RISKY")
            .expect("task");
        assert_eq!(task.status, "Blocked");
    }

    #[test]
    fn promote_refuses_a_dirty_working_tree() {
        let (_d, config, cfg_path) = fixture();
        register_task(
            &cfg_path,
            "DIRTY",
            "demo",
            "task",
            "crates/demo/tests/new_module_test.rs",
            "add code",
        )
        .expect("register");
        persist(
            &config.results_directory,
            &record_for(
                "DIRTY",
                "demo",
                "crates/demo/tests/new_module_test.rs",
                "pub fn f() {}\n",
            ),
        )
        .expect("persist");

        // Dirty the working tree directly, bypassing promote entirely.
        fs::write(
            config.checkout.join("crates/demo/src/lib.rs"),
            "pub fn existing() {} // uncommitted local edit\n",
        )
        .expect("dirty tree");

        let err = promote(&cfg_path, "DIRTY").expect_err("dirty tree must be refused");
        assert!(err.to_string().contains("uncommitted changes"), "{err}");
        assert_eq!(
            git_current_branch(&config.checkout).expect("branch"),
            "main"
        );
    }

    #[test]
    fn promote_pushes_isolated_branch_and_restores_original_branch_on_success() {
        let (_d, config, cfg_path) = fixture();
        register_task(
            &cfg_path,
            "GOODFILE",
            "demo",
            "task",
            "crates/demo/tests/new_module_test.rs",
            "add a trivial function",
        )
        .expect("register");
        persist(
            &config.results_directory,
            &record_for(
                "GOODFILE",
                "demo",
                "crates/demo/tests/new_module_test.rs",
                "pub fn added() -> u32 { 42 }\n",
            ),
        )
        .expect("persist");

        promote(&cfg_path, "GOODFILE").expect("clean code must promote");

        assert_eq!(
            git_current_branch(&config.checkout).expect("branch"),
            "main",
            "a successful promotion must still restore the original branch"
        );
        assert!(
            !config
                .checkout
                .join("crates/demo/tests/new_module_test.rs")
                .exists(),
            "restoring 'main' must not leave the new file behind — it only exists on the \
             pushed local-agent branch, never on main"
        );
        let show = Command::new("git")
            .args([
                "show",
                "local-agent/GOODFILE:crates/demo/tests/new_module_test.rs",
            ])
            .current_dir(&config.checkout)
            .output()
            .expect("git show on the local branch");
        assert!(
            show.status.success(),
            "the committed file must be present on the local-agent branch itself"
        );
        assert!(String::from_utf8_lossy(&show.stdout).contains("pub fn added"));

        let manifest = load_manifest(&config.queue).expect("manifest");
        let task = manifest
            .tasks
            .iter()
            .find(|t| t.id == "GOODFILE")
            .expect("task");
        assert_eq!(task.status, "Review");

        let archived = files(&config.archive_directory, "json").expect("archive");
        assert_eq!(archived.len(), 1);
        let archived_content: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&archived[0]).expect("read archive"))
                .expect("parse archive");
        assert_eq!(archived_content["archived_outcome"], "pushed-for-review");

        // The pushed branch must actually exist on the bare "origin" remote.
        let branches = Command::new("git")
            .args(["branch", "-r"])
            .current_dir(&config.checkout)
            .output()
            .expect("git branch -r");
        let branches = String::from_utf8_lossy(&branches.stdout);
        assert!(
            branches.contains("origin/local-agent/GOODFILE"),
            "expected pushed branch on origin, got: {branches}"
        );
    }

    #[test]
    fn circuit_breaker_trips_after_threshold_and_reset_clears_it() {
        let (_d, config, cfg_path) = fixture();
        for i in 0..CIRCUIT_BREAKER_THRESHOLD {
            let id = format!("FAIL{i}");
            register_task(
                &cfg_path,
                &id,
                "demo",
                "task",
                "crates/demo/tests/new_module_test.rs",
                "add broken code",
            )
            .expect("register");
            persist(
                &config.results_directory,
                &record_for(
                    &id,
                    "demo",
                    "crates/demo/tests/new_module_test.rs",
                    "pub fn broken() { let unused = 1; }\n",
                ),
            )
            .expect("persist");
            let _ = promote(&cfg_path, &id);
        }

        let breaker = load_circuit_breaker(&config).expect("breaker");
        assert!(
            breaker.tripped,
            "breaker must trip after {CIRCUIT_BREAKER_THRESHOLD} consecutive failures"
        );

        register_task(
            &cfg_path,
            "AFTER-TRIP",
            "demo",
            "task",
            "crates/demo/tests/another_test.rs",
            "add code after trip",
        )
        .expect("register");
        persist(
            &config.results_directory,
            &record_for(
                "AFTER-TRIP",
                "demo",
                "crates/demo/tests/another_test.rs",
                "pub fn added() -> u32 { 1 }\n",
            ),
        )
        .expect("persist");
        let err = promote(&cfg_path, "AFTER-TRIP")
            .expect_err("tripped breaker must refuse further promotion, even for good code");
        assert!(err.to_string().contains("circuit breaker"), "{err}");

        reset_circuit_breaker(&cfg_path, "test reset").expect("reset");
        let breaker = load_circuit_breaker(&config).expect("breaker after reset");
        assert!(!breaker.tripped);
        assert_eq!(breaker.consecutive_failures, 0);

        promote(&cfg_path, "AFTER-TRIP").expect("promotion must work again after reset");
    }

    /// A minimal real git repo standing in for the devtools repo's own
    /// working copy, with a bare `origin` remote so `git pull --ff-only`
    /// has something real to talk to.
    fn devtools_fixture() -> (tempfile::TempDir, PathBuf) {
        let d = tempfile::tempdir().expect("devtools fixture");
        let repo = d.path().join("devtools-repo");
        fs::create_dir_all(repo.join("governance")).expect("governance dir");
        fs::write(repo.join("README.md"), "placeholder\n").expect("readme");
        init_git_fixture(&repo, d.path());
        (d, repo)
    }

    fn write_override(repo: &Path, tripped: bool, reason: &str) {
        let content = format!("schema_version = 1\ntripped = {tripped}\nreason = {reason:?}\n");
        fs::write(repo.join(CIRCUIT_BREAKER_OVERRIDE_RELATIVE), content).expect("write override");
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(repo)
                .status()
                .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"));
            assert!(status.success(), "git {args:?} failed");
        };
        run(&["add", "-A"]);
        run(&["commit", "-q", "-m", "update override"]);
        run(&["push", "-q", "origin", "main"]);
    }

    #[test]
    fn remote_override_absent_is_not_tripped() {
        let (_d, repo) = devtools_fixture();
        check_remote_circuit_breaker_override(&repo).expect("no override file -> ok");
    }

    #[test]
    fn remote_override_false_is_not_tripped() {
        let (_d, repo) = devtools_fixture();
        write_override(&repo, false, "");
        check_remote_circuit_breaker_override(&repo).expect("explicitly untripped -> ok");
    }

    #[test]
    fn remote_override_true_trips_with_reason() {
        let (_d, repo) = devtools_fixture();
        write_override(&repo, true, "reverted PR #7, investigate before resuming");
        let err = check_remote_circuit_breaker_override(&repo).expect_err("must refuse");
        assert!(
            err.to_string().contains("reverted PR #7"),
            "error should surface the recorded reason: {err}"
        );
    }

    #[test]
    fn remote_override_check_fails_closed_when_pull_is_impossible() {
        let d = tempfile::tempdir().expect("dir");
        let repo = d.path().join("no-origin-repo");
        fs::create_dir_all(&repo).expect("dir");
        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&repo)
                .status()
                .unwrap_or_else(|e| panic!("failed to run git {args:?}: {e}"))
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        fs::write(repo.join("f.txt"), "x").expect("file");
        run(&["add", "-A"]);
        run(&["commit", "-q", "-m", "initial"]);
        // Deliberately no `origin` remote configured.

        let err = check_remote_circuit_breaker_override(&repo)
            .expect_err("an impossible pull must fail closed, not silently pass");
        assert!(
            err.to_string().contains("refusing to promote"),
            "must fail closed with a clear reason: {err}"
        );
    }

    #[test]
    fn promote_refuses_when_remote_circuit_breaker_override_is_tripped() {
        let (_d, mut config, cfg_path) = fixture();
        let (_devtools_d, devtools_repo) = devtools_fixture();
        write_override(&devtools_repo, true, "owner paused the pipeline");
        config.devtools_repo_root = Some(devtools_repo);
        fs::write(&cfg_path, toml::to_string(&config).expect("config")).expect("rewrite config");

        register_task(
            &cfg_path,
            "GOOD",
            "demo",
            "task",
            "crates/demo/tests/good_test.rs",
            "add trivial code",
        )
        .expect("register");
        persist(
            &config.results_directory,
            &record_for(
                "GOOD",
                "demo",
                "crates/demo/tests/good_test.rs",
                "pub fn f() {}\n",
            ),
        )
        .expect("persist");

        let err = promote(&cfg_path, "GOOD")
            .expect_err("remote override must block promotion even for clean code");
        assert!(
            err.to_string().contains("remote circuit-breaker override"),
            "{err}"
        );
    }

    #[test]
    fn mcp_initialize_and_tools_list() {
        let temp = tempfile::tempdir().unwrap();
        let cfg_path = temp.path().join("config.toml");

        let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let (status, resp_str) = handle_mcp_json_rpc(&cfg_path, init_req);
        assert_eq!(status, "200 OK");
        let v: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(v["result"]["protocolVersion"], "2024-11-05");

        let tools_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
        let (status, resp_str) = handle_mcp_json_rpc(&cfg_path, tools_req);
        assert_eq!(status, "200 OK");
        let v: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
        let tools = v["result"]["tools"].as_array().unwrap();
        assert!(tools.iter().any(|t| t["name"] == "verify_gate_suite"));
        assert!(tools.iter().any(|t| t["name"] == "get_repository_status"));
    }
}
