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
//! separately-invoked action that re-verifies (compile + clippy) before
//! committing, and rolls back correctly on failure whether the target file
//! pre-existed or was newly created.
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

    println!(
        "Status breakdown: {pending} Pending | {review} Awaiting Review | {verified} Verified | {failed} Failed"
    );

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
/// the pending record, re-validates target/crate correlation, writes the
/// draft, and requires `cargo check` **and** `cargo clippy -D warnings` to
/// pass (a real bar, not the bare compile check the pre-hardening version
/// used) before committing. On failure, rolls back correctly whether the
/// file pre-existed (`git checkout --`) or was newly created (`remove_file`
/// — `git checkout --` is a no-op for a file git has never seen, which
/// used to leave broken new files sitting in the tree indefinitely).
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

    validate_target(&config.checkout, &record.crate_name, &record.target_file)?;
    let full_target_path = config.checkout.join(&record.target_file);
    let pre_existing = full_target_path.exists();

    if let Some(parent) = full_target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&full_target_path, &draft.rust_source)?;

    println!(
        "agentic registrar: promoting {task_id} — running cargo check + clippy for '{}'...",
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
            archive_record(config, &record_path, "promoted")?;
            update_task_status(config, task_id, "Verified")?;
            println!("agentic registrar: promoted and committed {task_id} ({hash})");
            Ok(())
        }
        Err(err_msg) => {
            if pre_existing {
                git_rollback_existing_target(&config.checkout, &record.target_file)?;
            } else {
                let _ = fs::remove_file(&full_target_path);
            }
            archive_record(config, &record_path, "promotion-failed")?;
            update_task_status(config, task_id, "Failed")?;
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

pub fn auto_register_ide_mcp_config(port: u16) -> Result<Vec<PathBuf>> {
    let mut updated_paths = Vec::new();

    let appdata = std::env::var("APPDATA").ok();
    let userprofile = std::env::var("USERPROFILE").ok();

    let mut target_paths = Vec::new();
    if let Some(ref app) = appdata {
        target_paths.push(
            PathBuf::from(app)
                .join("Cursor")
                .join("User")
                .join("globalStorage")
                .join("mcp.json"),
        );
        target_paths.push(
            PathBuf::from(app)
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("mcp.json"),
        );
    }
    if let Some(ref user) = userprofile {
        target_paths.push(PathBuf::from(user).join(".cursor").join("mcp.json"));
    }

    let mcp_entry = serde_json::json!({
        "url": format!("http://127.0.0.1:{port}/mcp"),
        "transport": "http"
    });

    for path in target_paths {
        if let Some(parent) = path.parent() {
            if parent.exists() {
                let mut doc: serde_json::Value = if path.exists() {
                    let content = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
                } else {
                    serde_json::json!({})
                };

                if !doc.is_object() {
                    doc = serde_json::json!({});
                }

                if doc.get("mcpServers").is_none() {
                    doc["mcpServers"] = serde_json::json!({});
                }

                doc["mcpServers"]["aaroneous-mcp"] = mcp_entry.clone();

                let formatted = serde_json::to_string_pretty(&doc)?;
                fs::write(&path, formatted)?;
                updated_paths.push(path);
            }
        }
    }

    Ok(updated_paths)
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
            Ok(format!(
                "Checkout: {}\nModel: {}\nEndpoint: {}\nTotal Tasks: {}\nPending: {}\nAwaiting Review: {}\nVerified: {}\nFailed: {}",
                config.checkout.display(),
                config.model,
                config.endpoint,
                manifest.tasks.len(),
                pending,
                review,
                verified,
                failed
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

/// Serve REST + MCP over 127.0.0.1. Each connection is handled on its own
/// thread with read/write timeouts, so one stalled client can no longer
/// block every other caller indefinitely (the pre-hardening version
/// accepted connections on a single thread with no timeout at all).
///
/// Known remaining gap, not fixed here: there is still no request
/// authentication, and CORS is wildcard, so any local process (including a
/// browser tab, since the OPTIONS preflight is answered permissively) can
/// call every tool while `serve` is running. The blast radius is now much
/// smaller — nothing it can trigger writes to the checkout or commits
/// without a separate, explicit `promote` call — but a shared-secret
/// header check would still be a worthwhile follow-up before this runs
/// unattended on a shared machine.
pub fn serve(config_path: &Path, port: u16) -> Result<()> {
    use std::net::TcpListener;

    if let Ok(paths) = auto_register_ide_mcp_config(port) {
        for p in paths {
            println!(
                "Agentic Registrar: Auto-registered MCP endpoint in {}",
                p.display()
            );
        }
    }

    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("Failed to bind server to 127.0.0.1:{port}"))?;
    println!("Agentic Registrar Dual REST + MCP Server listening on http://127.0.0.1:{port}");

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let config_path = config_path.to_path_buf();
        std::thread::spawn(move || {
            let _ = handle_connection(&config_path, stream);
        });
    }

    Ok(())
}

fn handle_connection(config_path: &Path, mut stream: std::net::TcpStream) -> Result<()> {
    use std::io::{Read, Write};

    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let mut buffer = [0u8; 8192];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return Ok(()),
    };

    let req_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = req_str.lines();
    let request_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    let body_str = if let Some(pos) = req_str.find("\r\n\r\n") {
        &req_str[pos + 4..]
    } else {
        ""
    };

    if method == "GET" && path == "/sse" {
        let sse_body = "event: endpoint\r\ndata: http://127.0.0.1:8080/mcp\r\n\r\n";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nAccess-Control-Allow-Origin: *\r\nConnection: keep-alive\r\nContent-Length: {}\r\n\r\n{}",
            sse_body.len(),
            sse_body
        );
        let _ = stream.write_all(response.as_bytes());
        return Ok(());
    }

    let (status_code, body) = match (method, path) {
        ("OPTIONS", _) => ("200 OK".to_string(), "{}".to_string()),
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
                Ok(serde_json::json!({
                    "checkout": cfg.checkout.display().to_string(),
                    "model": cfg.model,
                    "endpoint": cfg.endpoint,
                    "total_tasks": manifest.tasks.len(),
                    "pending": pending,
                    "review": review,
                    "verified": verified,
                    "failed": failed
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

    let response = format!(
        "HTTP/1.1 {status_code}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    let _ = stream.write_all(response.as_bytes());
    Ok(())
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

    fn fixture() -> (tempfile::TempDir, RegistrarConfig, PathBuf) {
        let d = tempfile::tempdir().expect("fixture");
        let checkout = d.path().join("checkout");
        let crate_dir = checkout.join("crates").join("demo");
        fs::create_dir_all(crate_dir.join("src")).expect("crate dir");
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
        fs::create_dir_all(checkout.join(".git")).expect("git marker");

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
            "crates/demo/src/new_module.rs",
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
            target_file: "crates/demo/src/new_module.rs".into(),
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

        let target = config.checkout.join("crates/demo/src/new_module.rs");
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
