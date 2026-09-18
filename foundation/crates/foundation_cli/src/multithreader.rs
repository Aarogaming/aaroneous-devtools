use crate::intake::{Proposal, read_bounded, relative_file};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
#[derive(Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    checkout: PathBuf,
    queue: PathBuf,
    program: PathBuf,
    evidence: Vec<String>,
    endpoint: String,
    output_directory: PathBuf,
}
struct RunLock(PathBuf);
impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn queue_ids(queue: &str) -> (BTreeSet<String>, Vec<String>) {
    let mut ids = BTreeSet::new();
    let mut errors = Vec::new();
    for row in queue.lines().filter(|s| s.starts_with('|')) {
        let cols: Vec<_> = row.split('|').map(str::trim).collect();
        if cols.len() < 5 || !matches!(cols[3], "Ready" | "Claimed" | "Review" | "Done" | "Blocked")
        {
            continue;
        }
        if !ids.insert(cols[1].to_owned()) {
            errors.push(format!("duplicate queue ID: {}", cols[1]));
        }
    }
    (ids, errors)
}
fn checked_file(root: &Path, relative: &str) -> Result<PathBuf> {
    if !relative_file(relative) {
        bail!("invalid relative file: {relative}");
    }
    let mut path = root.to_owned();
    for segment in relative.split('/') {
        path.push(segment);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            bail!("symlink scope forbidden: {relative}");
        }
    }
    if !path.is_file() {
        bail!("scope is not a file: {relative}");
    }
    Ok(path)
}
fn check_candidate(p: &Proposal, c: &Config, ids: &BTreeSet<String>, queue: &str) -> Result<()> {
    p.check()?;
    if ids.contains(&p.id) {
        bail!("candidate ID already exists");
    }
    if !c.evidence.contains(&p.source) {
        bail!("source was not declared evidence");
    }
    if p.dependencies.iter().any(|id| !ids.contains(id)) {
        bail!("unknown dependency");
    }
    for scope in &p.scope {
        checked_file(&c.checkout, scope)?;
        if queue.contains(scope) {
            bail!("scope already appears in queue: {scope}; owner reconciliation required");
        }
    }
    Ok(())
}
pub(crate) fn run(config_path: &Path) -> Result<()> {
    if !config_path.is_absolute() {
        bail!("config must be absolute");
    }
    let c: Config = toml::from_str(&read_bounded(config_path, 16384)?)?;
    if c.schema_version != 1 || c.evidence.is_empty() || c.evidence.len() > 8 {
        bail!("version 1 and 1..8 evidence files required");
    }
    crate::require_absolute_directory("checkout", &c.checkout)?;
    crate::require_absolute_directory("output directory", &c.output_directory)?;
    if !c.queue.is_absolute() || !c.program.is_absolute() {
        bail!("queue must be absolute");
    }
    if c.output_directory.starts_with(&c.checkout) {
        bail!("artifacts must be outside product checkout");
    }
    let lock = c.output_directory.join("review.lock");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let acquired = acquire_or_recover_lock(&lock, &c.output_directory, now)?;
    if !acquired {
        println!("multithreader: active review.lock present; skipping cycle");
        return Ok(());
    }
    let _lock = RunLock(lock);
    let queue = read_bounded(&c.queue, 262144)?;
    let (mut ids, diagnostics) = queue_ids(&queue);
    let program = read_bounded(&c.program, 65536)?;
    for row in program.lines().filter(|r| r.starts_with('|')) {
        let cols: Vec<_> = row.split('|').map(str::trim).collect();
        if cols.len() > 3 && matches!(cols[1], "P0" | "P1" | "P2" | "P3" | "Done" | "Retired") {
            ids.insert(cols[2].to_owned());
        }
    }
    let mut evidence = format!("Existing IDs: {ids:?}\nExisting queue scopes (avoid overlaps):\n");
    for row in queue.lines().filter(|r| r.starts_with('|')) {
        let cols: Vec<_> = row.split('|').collect();
        if cols.len() > 3 {
            evidence.push_str(&format!("{} {}\n", cols[1], cols[2]));
        }
    }
    evidence.push_str(&format!(
        "Declared existing portfolio (do not duplicate):\n{program}\n"
    ));
    for relative in &c.evidence {
        evidence.push_str(&format!(
            "\nDeclared source: {relative}\n{}\n",
            read_bounded(&checked_file(&c.checkout, relative)?, 32768)?
        ));
    }
    let mut outcome = "no-new-work";
    let mut candidate = None;
    let mut rejection = None;
    let mut failed = false;
    match crate::intake_generator::generate(&c.endpoint, &evidence) {
        Ok(Some(p)) => {
            match check_candidate(&p, &c, &ids, &queue) {
                Ok(()) => {
                    outcome = "pending-owner-review";
                    for entry in fs::read_dir(&c.output_directory)? {
                        let path = entry?.path();
                        if path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with("review-") && n.ends_with(".json"))
                        {
                            let previous: serde_json::Value =
                                serde_json::from_str(&read_bounded(&path, 131072)?)?;
                            if previous["outcome"] == "pending-owner-review"
                                && (previous["candidate"]["id"] == p.id
                                    || previous["candidate"]["scope"]
                                        == serde_json::to_value(&p.scope)?)
                            {
                                outcome = "duplicate-inbox-candidate";
                                rejection = Some(
                                    "existing pending candidate has the same ID or file scope"
                                        .to_owned(),
                                );
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    outcome = "rejected";
                    rejection = Some(e.to_string());
                }
            };
            candidate = Some(p);
        }
        Ok(None) => {}
        Err(e) => {
            outcome = "failed";
            rejection = Some(format!("{e:#}"));
            failed = true;
        }
    }
    let report = serde_json::json!({"schema_version":1,"artifact_kind":"foundation.multithreader-review","outcome":outcome,"requires_owner_review":true,"known_ids":ids,"queue_diagnostics":diagnostics,"evidence":c.evidence,"evidence_bundle":evidence,"model":"qwen3.5:9b-q6","endpoint":c.endpoint,"candidate":candidate,"rejection":rejection});
    let mut sequence = 0u32;
    loop {
        match fs::OpenOptions::new().write(true).create_new(true).open(
            c.output_directory
                .join(format!("review-{sequence:06}.json")),
        ) {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(&serde_json::to_vec_pretty(&report)?)?;
                file.sync_all()?;
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                sequence = sequence
                    .checked_add(1)
                    .context("inbox sequence exhausted")?
            }
            Err(e) => return Err(e.into()),
        }
    }
    println!("multithreader: {outcome} (inbox record {sequence})");
    if failed {
        bail!("review failed; inspect inbox report");
    }
    Ok(())
}

fn acquire_or_recover_lock(lock: &Path, output_dir: &Path, now: u64) -> Result<bool> {
    use std::io::Write;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock)
    {
        Ok(mut f) => {
            let _ = writeln!(
                f,
                "owner=foundation.multithreader\nstarted_at_secs={now}\npid={}",
                std::process::id()
            );
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let is_stale = if let Ok(meta) = fs::metadata(lock) {
                if let Ok(modified) = meta.modified() {
                    if let Ok(elapsed) = modified.elapsed() {
                        elapsed > Duration::from_secs(3600)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if is_stale {
                println!(
                    "multithreader: stale review.lock detected (>3600s); recording crash marker"
                );
                let crash_marker = output_dir.join("crash_marker_retained.json");
                let marker_data = serde_json::json!({
                    "schema_version": 1,
                    "event": "stale_lock_recovered",
                    "lock_path": lock.display().to_string(),
                    "recovered_at_secs": now,
                });
                let _ = fs::write(&crash_marker, serde_json::to_vec_pretty(&marker_data)?);
                let _ = fs::remove_file(lock);

                match fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(lock)
                {
                    Ok(mut f) => {
                        let _ = writeln!(
                            f,
                            "owner=foundation.multithreader\nstarted_at_secs={now}\npid={}",
                            std::process::id()
                        );
                        Ok(true)
                    }
                    Err(_) => Ok(false),
                }
            } else {
                Ok(false)
            }
        }
        Err(e) => Err(e.into()),
    }
}

fn checked_interval(interval_minutes: u32) -> Result<Duration> {
    if interval_minutes == 0 || interval_minutes > 1440 {
        bail!("interval must be 1-1440 minutes, got {interval_minutes}");
    }
    Ok(Duration::from_secs(u64::from(interval_minutes) * 60))
}

fn cycle(config_path: &Path) -> bool {
    if let Err(e) = run(config_path) {
        eprintln!("multithreader-serve: cycle failed: {e:#}");
        false
    } else {
        true
    }
}

pub(crate) fn serve(config_path: &Path, interval_minutes: u32) -> Result<()> {
    let interval = checked_interval(interval_minutes)?;
    println!(
        "multithreader-serve: starting, interval {interval_minutes}m, config {}",
        config_path.display()
    );
    let mut consecutive_failures = 0u32;
    loop {
        if cycle(config_path) {
            consecutive_failures = 0;
        } else {
            consecutive_failures += 1;
            eprintln!("multithreader-serve: cycle failure count: {consecutive_failures}/5");
            if consecutive_failures >= 5 {
                bail!("multithreader-serve stopped after 5 consecutive permanent failures");
            }
        }
        std::thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_queue_ids_are_visible() {
        let (ids, errors) =
            queue_ids("| M21 | README.md | Done | owner |\n| M21 | README.md | Done | another |");
        assert_eq!(ids.len(), 1);
        assert_eq!(errors.len(), 1);
    }
    #[test]
    fn existing_scope_unknown_source_and_dependency_rejected() {
        let d = tempfile::tempdir().expect("fixture");
        fs::write(d.path().join("README.md"), "fixture").expect("source");
        let c = Config {
            schema_version: 1,
            checkout: d.path().to_owned(),
            queue: d.path().join("queue.md"),
            program: d.path().join("program.md"),
            evidence: vec!["README.md".into()],
            endpoint: "http://localhost:11434".into(),
            output_directory: d.path().to_owned(),
        };
        let mut p = Proposal {
            schema_version: 1,
            id: "NEW-1".into(),
            source: "README.md".into(),
            priority: "P2".into(),
            scope: vec!["README.md".into()],
            dependencies: vec![],
            verification: vec!["observable criterion".into()],
            decision: "fixture defect".into(),
        };
        assert!(check_candidate(&p, &c, &BTreeSet::new(), "README.md").is_err());
        p.source = "other.md".into();
        assert!(check_candidate(&p, &c, &BTreeSet::new(), "").is_err());
        p.source = "README.md".into();
        p.dependencies.push("UNKNOWN".into());
        assert!(check_candidate(&p, &c, &BTreeSet::new(), "").is_err());
    }
    #[test]
    fn interval_bounds_are_enforced() {
        assert!(checked_interval(0).is_err());
        assert!(checked_interval(1441).is_err());
        assert_eq!(
            checked_interval(30).expect("valid"),
            Duration::from_secs(1800)
        );
        assert_eq!(checked_interval(1).expect("valid"), Duration::from_secs(60));
        assert_eq!(
            checked_interval(1440).expect("valid"),
            Duration::from_secs(86400)
        );
    }
    #[test]
    fn a_failing_cycle_does_not_panic() {
        // An absolute path that cannot exist: run() fails validating it,
        // cycle() must log and return rather than propagate or panic —
        // that is the whole point of using cycle() instead of run() in
        // the serve loop.
        let d = tempfile::tempdir().expect("fixture");
        cycle(&d.path().join("does-not-exist.toml"));
    }
    #[test]
    fn stale_lock_is_recovered_and_crash_marker_retained() {
        let d = tempfile::tempdir().expect("fixture");
        let lock = d.path().join("review.lock");
        fs::write(&lock, "stale").expect("lock");
        let past = std::time::SystemTime::now() - Duration::from_secs(7200);
        let f = fs::OpenOptions::new()
            .write(true)
            .open(&lock)
            .expect("open");
        f.set_times(std::fs::FileTimes::new().set_modified(past))
            .expect("set_times");
        drop(f);
        let acquired = acquire_or_recover_lock(&lock, d.path(), 1000).expect("acquire");
        assert!(acquired);
        assert!(d.path().join("crash_marker_retained.json").exists());
        let marker: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(d.path().join("crash_marker_retained.json")).expect("marker"),
        )
        .expect("json");
        assert_eq!(marker["event"], "stale_lock_recovered");
    }
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;
    use crate::intake_generator::integration_tests::mock;
    #[test]
    fn full_pipeline_preserves_queue_and_deduplicates_inbox() {
        let d = tempfile::tempdir().expect("fixture");
        let checkout = d.path().join("product");
        let inbox = d.path().join("inbox");
        fs::create_dir(&checkout).expect("checkout");
        fs::create_dir(&inbox).expect("inbox");
        fs::write(checkout.join("README.md"), "A concrete fixture failure").expect("evidence");
        let queue = d.path().join("queue.md");
        let original = "| OLD | other.rs | Done | owner |\n";
        fs::write(&queue, original).expect("queue");
        let program = d.path().join("program.md");
        fs::write(&program, "| P1 | PLANNED-1 | other task | none | owner |").expect("program");
        let proposal=serde_json::json!({"proposal":{"schema_version":1,"id":"NEW-1","source":"README.md","priority":"P2","scope":["README.md"],"dependencies":[],"verification":["fixture criterion"],"decision":"fixture defect"}}).to_string();
        for (index, outcome) in [
            (0, "pending-owner-review"),
            (1, "duplicate-inbox-candidate"),
        ] {
            let (endpoint, server) = mock(&proposal, "stop");
            let config = Config {
                schema_version: 1,
                checkout: checkout.clone(),
                queue: queue.clone(),
                program: program.clone(),
                evidence: vec!["README.md".into()],
                endpoint,
                output_directory: inbox.clone(),
            };
            let config_path = d.path().join("review.toml");
            fs::write(&config_path, toml::to_string(&config).expect("serialize")).expect("config");
            run(&config_path).expect("pipeline");
            server.join().expect("mock");
            let report: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(inbox.join(format!("review-{index:06}.json"))).expect("report"),
            )
            .expect("JSON");
            assert_eq!(report["outcome"], outcome);
            assert!(
                report["known_ids"]
                    .as_array()
                    .expect("ids")
                    .contains(&serde_json::json!("PLANNED-1"))
            );
            assert!(report["requires_owner_review"].as_bool().expect("boolean"));
            assert!(!inbox.join("review.lock").exists());
            assert_eq!(fs::read_to_string(&queue).expect("queue"), original);
        }
    }
}
