//! Guild autonomous scheduler and task-card dispatcher (lane C113).
//!
//! Rust service evolution of the Steward (`steward_run.py` and `steward_loop.py`).
//! Consumes pure reducers and contracts from `foundation/crates/steward`:
//! - `steward::cards::build_cards`: deterministic per-seat task card dispatch
//! - `steward::chain`: cryptographic SHA-256 chained tamper-evident event log
//! - `steward::engagement`: stall and activity tracking
//! - `steward::checklist` and `steward::coverage`: merge-readiness and verification
//!
//! Exposes CLI actions:
//! - `tick`: run a single deterministic scheduler tick
//! - `verify-log`: cryptographically verify the SHA-256 event log chain
//! - `status`: inspect tip hash, active leases and outbox cards
//! - `daemon`: run persistent scheduling loop on an interval

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use crate::maintenance_pipeline::{format_iso8601, unix_secs_now};

#[derive(Subcommand, Debug, Clone)]
pub enum Action {
    /// Execute one Guild tick (card dispatch + tamper-evident event log append)
    Tick(TickArgs),
    /// Verify the cryptographic SHA-256 chain of an event log
    VerifyLog(VerifyLogArgs),
    /// Inspect the latest Guild state, tip hash, active leases and outbox cards
    Status(StatusArgs),
    /// Run the persistent Guild scheduler loop on an interval
    Daemon(DaemonArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct TickArgs {
    /// Repository slug (default: Aarogaming/aaroneous-devtools)
    #[arg(long, default_value = "Aarogaming/aaroneous-devtools")]
    pub repo: String,
    /// Absolute path to repository checkout
    #[arg(long)]
    pub repo_dir: Option<PathBuf>,
    /// Path to identities.json
    #[arg(long)]
    pub identities: Option<PathBuf>,
    /// Path to the append-only event log (e.g. steward-events.jsonl)
    #[arg(long)]
    pub log: PathBuf,
    /// Directory where per-seat task cards (<seat>.md) are written
    #[arg(long)]
    pub outbox: Option<PathBuf>,
    /// Optional path to pre-collected snapshot JSON (deterministic offline run)
    #[arg(long)]
    pub snapshot: Option<PathBuf>,
    /// Explicit tick number (if omitted, derived from event log)
    #[arg(long)]
    pub tick: Option<i64>,
    /// Inactive ticks before escalating a seat (default: 3)
    #[arg(long, default_value_t = 3)]
    pub stall_after: i64,
    /// Output cards as JSON to stdout
    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug, Clone)]
pub struct VerifyLogArgs {
    /// Path to the event log file
    #[arg(long)]
    pub log: PathBuf,
    /// Optional expected tip hash (SHA-256) to verify against
    #[arg(long)]
    pub expect_tip: Option<String>,
}

#[derive(Parser, Debug, Clone)]
pub struct StatusArgs {
    /// Path to the event log file
    #[arg(long)]
    pub log: PathBuf,
    /// Path to the outbox directory
    #[arg(long)]
    pub outbox: Option<PathBuf>,
}

#[derive(Parser, Debug, Clone)]
pub struct DaemonArgs {
    /// Repository slug
    #[arg(long, default_value = "Aarogaming/aaroneous-devtools")]
    pub repo: String,
    /// Path to repository checkout
    #[arg(long)]
    pub repo_dir: PathBuf,
    /// Path to identities.json
    #[arg(long)]
    pub identities: PathBuf,
    /// Path to the event log
    #[arg(long)]
    pub log: PathBuf,
    /// Path to outbox directory
    #[arg(long)]
    pub outbox: PathBuf,
    /// Interval between ticks in seconds (default: 600 = 10 min)
    #[arg(long, default_value_t = 600)]
    pub interval_seconds: u64,
    /// Path to file that triggers graceful stop when touched
    #[arg(long)]
    pub stop_file: Option<PathBuf>,
    /// Path to lock file preventing concurrent runs
    #[arg(long)]
    pub lock_file: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickOutcome {
    pub tick: i64,
    pub tip: String,
    pub cards_count: usize,
    pub written_outbox_files: Vec<String>,
    pub timestamp_utc: String,
}

/// Render a single card value into human-readable task card markdown matching the contract.
pub fn render_card(c: &Value) -> String {
    let agent = c.get("agent").and_then(Value::as_str).unwrap_or("");
    let role = c.get("role").and_then(Value::as_str).unwrap_or("");
    let tick = c.get("tick").and_then(Value::as_i64).unwrap_or(0);
    let priority = c.get("priority").and_then(Value::as_i64).unwrap_or(0);

    let mut head = format!("[{agent}] role={role} tick={tick} p{priority}");
    if let Some(lens) = c.get("lens").and_then(Value::as_str) {
        if !lens.is_empty() {
            let attempt = c.get("attempt").and_then(Value::as_i64).unwrap_or(1);
            head.push_str(&format!(" lens={lens} attempt={attempt}"));
        }
    }
    if let Some(lease) = c.get("lease") {
        if let Some(expires) = lease.get("expires_tick").and_then(Value::as_i64) {
            head.push_str(&format!(" lease->tick {expires}"));
        }
    }

    let mut lines = vec![head];
    if let Some(task) = c.get("task").and_then(Value::as_str) {
        lines.push(format!("  TASK: {task}"));
    }
    if let Some(state) = c.get("state").and_then(Value::as_str) {
        if !state.is_empty() {
            lines.push(format!("  STATE NOW: {state}"));
        }
    }
    if let Some(new_since) = c.get("new_since_last").and_then(Value::as_str) {
        if !new_since.is_empty() {
            lines.push(format!("  NEW SINCE YOUR LAST CARD: {new_since}"));
        }
    }
    if let Some(lens_text) = c.get("lens_text").and_then(Value::as_str) {
        if !lens_text.is_empty() {
            lines.push(format!("  APPROACH: {lens_text}"));
        }
    }
    if let Some(rejected) = c.get("rejected").and_then(Value::as_array) {
        for r in rejected {
            let cid = r
                .get("comment_id")
                .map(|v| v.to_string())
                .unwrap_or_default();
            let reason = r.get("reason").and_then(Value::as_str).unwrap_or("");
            lines.push(format!("  REJECTED: comment {cid}: {reason}"));
        }
    }
    if let Some(note) = c.get("rejected_note").and_then(Value::as_array) {
        for r in note {
            let cid = r
                .get("comment_id")
                .map(|v| v.to_string())
                .unwrap_or_default();
            let reason = r.get("reason").and_then(Value::as_str).unwrap_or("");
            lines.push(format!("  NOTE: comment {cid} on this PR was not counted ({reason}); if it is yours, repost it"));
        }
    }
    if let Some(recipe) = c.get("recipe").and_then(Value::as_array) {
        for (n, step) in recipe.iter().enumerate() {
            let s = step.as_str().unwrap_or("");
            lines.push(format!("  STEP {}: {s}", n + 1));
        }
    }
    if let Some(acc) = c.get("acceptance").and_then(Value::as_str) {
        lines.push(format!("  DONE WHEN: {acc}"));
    }
    if let Some(then) = c.get("then").and_then(Value::as_str) {
        if !then.is_empty() {
            lines.push(format!("  THEN: {then}"));
        }
    }
    if let Some(blocked) = c.get("if_blocked").and_then(Value::as_str) {
        lines.push(format!("  IF BLOCKED: {blocked}"));
    }
    lines.join("\n")
}

#[derive(Deserialize)]
struct LogCheckpoint {
    count: u64,
    tip: String,
}

/// Validate any retained checkpoint, including when the log has disappeared.
/// Missing checkpoints remain supported for first runs and unanchored legacy logs.
/// A mismatch is a recovery hold, never permission to rewrite the checkpoint.
fn verify_checkpoint(log_path: &Path, count: u64, tip: &str) -> Result<()> {
    let tip_path = PathBuf::from(format!("{}.tip", log_path.display()));
    let raw = match fs::read_to_string(&tip_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("cannot read checkpoint {}", tip_path.display()));
        }
    };
    let trimmed = raw.trim();
    // The verify-log command historically also accepted a plain SHA-256 digest.
    let recorded_tip = if trimmed.len() == 64 && trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        trimmed.to_owned()
    } else {
        let checkpoint: LogCheckpoint = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "invalid checkpoint {}; preserve the log and checkpoint for recovery",
                tip_path.display()
            )
        })?;
        if checkpoint.count != count {
            bail!(
                "checkpoint count mismatch at {}: log has {count}, checkpoint has {}; preserve both for recovery",
                tip_path.display(),
                checkpoint.count
            );
        }
        checkpoint.tip
    };
    if recorded_tip != tip {
        bail!(
            "checkpoint tip mismatch at {}; preserve the log and checkpoint for recovery",
            tip_path.display()
        );
    }
    Ok(())
}

/// Read and verify the SHA-256 chain and any retained checkpoint.
pub fn read_and_verify_log(log_path: &Path) -> Result<(Vec<Value>, String)> {
    let content = match fs::read_to_string(log_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read event log at {}", log_path.display()));
        }
    };

    let mut events = Vec::new();
    let mut expected_prev = steward::chain::GENESIS.to_owned();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let val: Value = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "malformed JSON on line {} in {}",
                line_num + 1,
                log_path.display()
            )
        })?;

        let prev = val.get("prev").and_then(Value::as_str).unwrap_or("");
        if prev != expected_prev {
            bail!(
                "chain broken at line {} in {}: expected prev '{}', got '{}'",
                line_num + 1,
                log_path.display(),
                expected_prev,
                prev
            );
        }
        expected_prev = steward::chain::digest(trimmed);
        events.push(val);
    }

    verify_checkpoint(log_path, events.len() as u64, &expected_prev)?;
    Ok((events, expected_prev))
}

/// Append one event line, then update its checkpoint in a separate write.
/// These writes are not an atomic transaction; a mismatch must halt for recovery.
pub fn append_event_line(
    log_path: &Path,
    seq: u64,
    event_type: &str,
    data: &Value,
    ts: &str,
    prev_tip: &str,
) -> Result<String> {
    let previous_count = seq
        .checked_sub(1)
        .context("checkpoint append sequence must start at one")?;
    verify_checkpoint(log_path, previous_count, prev_tip)?;
    let tip_path = PathBuf::from(format!("{}.tip", log_path.display()));

    let line = steward::chain::event_line(seq, ts, event_type, data, prev_tip);
    let new_tip = steward::chain::digest(&line);

    // Ensure parent directory exists
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Append to log file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("failed to open log file for append: {}", log_path.display()))?;
    writeln!(file, "{line}")?;
    file.flush()?;

    // Update <log_path>.tip sidecar matching steward_run.py convention: {"count": seq, "tip": "..."}
    let tip_json = json!({
        "count": seq,
        "tip": new_tip,
    });
    fs::write(
        &tip_path,
        format!("{}\n", serde_json::to_string(&tip_json)?),
    )?;

    Ok(new_tip)
}

/// Write a per-seat card markdown file atomically into the outbox directory.
pub fn write_card_atomically(
    outbox: &Path,
    seat: &str,
    card_body: &str,
    ts: &str,
    tick: i64,
) -> Result<PathBuf> {
    if seat.contains('/') || seat.contains('\\') || seat.contains("..") || seat.is_empty() {
        bail!("invalid agent name in card: {}", seat);
    }
    fs::create_dir_all(outbox)?;
    let target = outbox.join(format!("{seat}.md"));
    let temp = outbox.join(format!("{seat}.md.tmp"));

    let full_content = format!(
        "Steward card for {seat}, generated {ts}, tick {tick}.\n\
         This replaces any earlier card. If it says standby or healthy idle, do nothing and stay quiet.\n\n\
         {card_body}\n"
    );

    fs::write(&temp, full_content.as_bytes())?;
    fs::rename(&temp, &target)?;
    Ok(target)
}

/// Execute one Guild tick.
pub fn run_tick(args: &TickArgs) -> Result<TickOutcome> {
    let now_ts = format_iso8601(unix_secs_now());
    let (events, mut current_tip) = read_and_verify_log(&args.log)?;

    // Determine current tick count from events if not explicitly passed
    let derived_tick = if let Some(t) = args.tick {
        t
    } else {
        events
            .iter()
            .filter_map(|e| {
                e.get("data")
                    .and_then(|d| d.get("tick"))
                    .and_then(Value::as_i64)
            })
            .max()
            .map(|t| t + 1)
            .unwrap_or(1)
    };

    if let Some(ref identities) = args.identities {
        if !identities.exists() {
            bail!("identities file does not exist: {}", identities.display());
        }
    }

    // Load state from snapshot file if provided, otherwise invoke live collector
    let state: Value = if let Some(ref snapshot_path) = args.snapshot {
        let content = fs::read_to_string(snapshot_path)
            .with_context(|| format!("failed to read snapshot at {}", snapshot_path.display()))?;
        let parsed: Value = serde_json::from_str(&content)
            .with_context(|| format!("malformed JSON in snapshot {}", snapshot_path.display()))?;
        // If snapshot has a top-level "state" key, unwrap it
        if let Some(inner) = parsed.get("state") {
            inner.clone()
        } else {
            parsed
        }
    } else if let Some(ref repo_dir) = args.repo_dir {
        let local_snapshot = repo_dir.join(".guild_snapshot.json");
        if local_snapshot.exists() {
            let content = fs::read_to_string(&local_snapshot)?;
            serde_json::from_str(&content)?
        } else {
            // Native Rust live collector with fallback to Python script if gh fails or unavailable
            match crate::guild_collector::collect_live_state(
                &args.repo,
                repo_dir,
                args.identities.as_deref(),
                false,
            ) {
                Ok(collected) => {
                    if let Some(inner) = collected.get("state") {
                        inner.clone()
                    } else {
                        collected
                    }
                }
                Err(native_err) => {
                    let script = repo_dir.join("scripts").join("collect_state.py");
                    if script.exists() {
                        let python_bin =
                            if Command::new("python3").arg("--version").output().is_ok() {
                                "python3"
                            } else {
                                "python"
                            };
                        let out = Command::new(python_bin)
                            .arg(&script)
                            .arg("--repo")
                            .arg(&args.repo)
                            .arg("--repo-dir")
                            .arg(repo_dir)
                            .output()
                            .with_context(|| {
                                format!("native collector failed ({native_err:#}); fallback to {python_bin} {script:?} also failed")
                            })?;
                        if !out.status.success() {
                            bail!(
                                "collect state failed: native error: {native_err:#}; python error (exit {}): {}",
                                out.status,
                                String::from_utf8_lossy(&out.stderr)
                            );
                        }
                        let parsed: Value = serde_json::from_slice(&out.stdout)
                            .with_context(|| "malformed JSON from collect_state.py")?;
                        if let Some(inner) = parsed.get("state") {
                            inner.clone()
                        } else {
                            parsed
                        }
                    } else {
                        bail!("native state collection failed: {native_err:#}");
                    }
                }
            }
        }
    } else {
        bail!("Either --snapshot or --repo-dir must be provided to run a Guild tick.");
    };

    // Build deterministic task cards via steward::cards::build_cards with stall detection and lease support
    let mut state = state;
    let engagement_events: Vec<steward::engagement::Event> = events
        .iter()
        .map(|e| {
            let kind = e
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let tick = e
                .get("data")
                .and_then(|d| d.get("tick"))
                .and_then(Value::as_i64);
            let agents = e
                .get("data")
                .and_then(|d| d.get("agents"))
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            steward::engagement::Event {
                kind,
                tick,
                agents,
                per_seat: Default::default(),
                pushes: Default::default(),
                unattributed: 0,
            }
        })
        .collect();

    let agent_names: Vec<String> = state
        .get("agents")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("name").and_then(Value::as_str))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let stalled =
        steward::engagement::stalled_seats(&engagement_events, &agent_names, args.stall_after);
    if let Some(obj) = state.as_object_mut() {
        obj.insert("stalled".to_string(), json!(stalled));
        if !obj.contains_key("leases") {
            obj.insert("leases".to_string(), json!({}));
        }
    }

    let cards = steward::cards::build_cards(&state, derived_tick)
        .map_err(|e| anyhow::anyhow!("steward card dispatcher failed: {e}"))?;

    // Append "cards" event to event log FIRST before publishing cards
    let actionable: Vec<&str> = cards
        .iter()
        .filter(|c| {
            let kind = c
                .get("inputs")
                .and_then(|i| i.get("kind"))
                .and_then(Value::as_str)
                .unwrap_or("");
            kind != "healthy_idle" && kind != "standby"
        })
        .filter_map(|c| c.get("agent").and_then(Value::as_str))
        .collect();

    let tasks: Vec<&str> = cards
        .iter()
        .filter_map(|c| c.get("task").and_then(Value::as_str))
        .collect();

    let cards_data = json!({
        "tick": derived_tick,
        "agents": actionable,
        "tasks": tasks,
    });

    let next_seq = (events.len() + 1) as u64;
    current_tip = append_event_line(
        &args.log,
        next_seq,
        "cards",
        &cards_data,
        &now_ts,
        &current_tip,
    )?;

    // Publish cards only after the validated append and checkpoint write succeeded.
    // This ordering alone is not a power-loss durability guarantee.
    let mut written_files = Vec::new();
    if let Some(ref outbox_dir) = args.outbox {
        for card in &cards {
            if let Some(agent) = card.get("agent").and_then(Value::as_str) {
                let card_body = render_card(card);
                let path =
                    write_card_atomically(outbox_dir, agent, &card_body, &now_ts, derived_tick)?;
                written_files.push(path.display().to_string());
            }
        }
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&cards)?);
    } else {
        println!(
            "Guild tick {} complete: generated {} card(s), log tip: {}",
            derived_tick,
            cards.len(),
            current_tip
        );
        for f in &written_files {
            println!("  Wrote card: {f}");
        }
    }

    Ok(TickOutcome {
        tick: derived_tick,
        tip: current_tip,
        cards_count: cards.len(),
        written_outbox_files: written_files,
        timestamp_utc: now_ts,
    })
}

/// Verify the SHA-256 chain of an event log.
pub fn run_verify_log(args: &VerifyLogArgs) -> Result<()> {
    let (events, tip) = read_and_verify_log(&args.log)?;
    println!(
        "Log at {}: verified {} event(s). Chain tip: {}",
        args.log.display(),
        events.len(),
        tip
    );
    if let Some(ref expected) = args.expect_tip {
        if &tip != expected {
            bail!("tip mismatch: expected '{}', got '{}'", expected, tip);
        }
        println!("Tip matches expected hash exactly.");
    }
    Ok(())
}

/// Inspect the status of Guild event log and outbox.
pub fn run_status(args: &StatusArgs) -> Result<()> {
    let (events, tip) = read_and_verify_log(&args.log)?;
    let tick_count = events
        .iter()
        .filter(|e| e.get("type").and_then(Value::as_str) == Some("cards"))
        .count();

    println!("Guild Status:");
    println!("  Log path:       {}", args.log.display());
    println!("  Total events:   {}", events.len());
    println!("  Total ticks:    {}", tick_count);
    println!("  Chain tip:      {}", tip);

    if let Some(ref outbox) = args.outbox {
        println!("  Outbox path:    {}", outbox.display());
        if outbox.exists() {
            let mut entries = Vec::new();
            if let Ok(dir) = fs::read_dir(outbox) {
                for entry in dir.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("md") {
                        entries.push(path);
                    }
                }
            }
            println!("  Active cards:   {}", entries.len());
            for e in entries {
                println!(
                    "    - {}",
                    e.file_name().unwrap_or_default().to_string_lossy()
                );
            }
        } else {
            println!("  Outbox does not exist yet.");
        }
    }
    Ok(())
}

/// Run the persistent Guild daemon loop.
pub fn run_daemon(args: &DaemonArgs) -> Result<()> {
    if args.interval_seconds == 0 {
        bail!("interval-seconds must be at least 1");
    }
    if !args.identities.exists() {
        bail!(
            "identities file does not exist: {}",
            args.identities.display()
        );
    }

    println!(
        "Starting Guild persistent daemon for {} (interval: {}s)...",
        args.repo, args.interval_seconds
    );

    // Handle exclusive lock file
    let lock_path = args
        .lock_file
        .clone()
        .unwrap_or_else(|| args.log.with_extension("loop.lock"));

    let mut lock_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to acquire exclusive lock {}: another daemon is running or stale lock exists ({e})",
                lock_path.display()
            )
        })?;
    writeln!(lock_file, "pid={}", std::process::id())?;
    lock_file.flush()?;

    let tick_args = TickArgs {
        repo: args.repo.clone(),
        repo_dir: Some(args.repo_dir.clone()),
        identities: Some(args.identities.clone()),
        log: args.log.clone(),
        outbox: Some(args.outbox.clone()),
        snapshot: None,
        tick: None,
        stall_after: 3,
        json: false,
    };

    let mut consecutive_failures = 0;

    loop {
        // Check for stop file
        if let Some(ref stop_file) = args.stop_file {
            if stop_file.exists() {
                println!(
                    "Stop file {} detected. Guild daemon shutting down gracefully.",
                    stop_file.display()
                );
                let _ = fs::remove_file(stop_file);
                let _ = fs::remove_file(&lock_path);
                break;
            }
        }

        println!("--- Guild Tick at {} ---", format_iso8601(unix_secs_now()));
        match run_tick(&tick_args) {
            Ok(outcome) => {
                consecutive_failures = 0;
                println!(
                    "Tick {} successful: {} card(s) issued. Tip: {}",
                    outcome.tick, outcome.cards_count, outcome.tip
                );
            }
            Err(err) => {
                consecutive_failures += 1;
                eprintln!("Tick error (consecutive failures: {consecutive_failures}): {err}");
                if consecutive_failures >= 5 {
                    eprintln!("Exceeded 5 consecutive failures. Exiting loop for safety.");
                    let _ = fs::remove_file(&lock_path);
                    return Err(err);
                }
            }
        }

        println!("Sleeping for {}s...", args.interval_seconds);
        sleep(Duration::from_secs(args.interval_seconds));
    }

    let _ = fs::remove_file(&lock_path);
    Ok(())
}

pub fn dispatch(action: Action) -> Result<()> {
    match action {
        Action::Tick(args) => {
            run_tick(&args)?;
            Ok(())
        }
        Action::VerifyLog(args) => run_verify_log(&args),
        Action::Status(args) => run_status(&args),
        Action::Daemon(args) => run_daemon(&args),
    }
}
