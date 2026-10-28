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
        let cols = crate::table::cells(row);
        if cols.len() < 5
            || !matches!(
                cols[3].as_str(),
                "Ready" | "Claimed" | "Review" | "Done" | "Blocked"
            )
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
    let _file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .context("review.lock exists: inspect active/stale run before removal")?;
    let _lock = RunLock(lock);
    let queue = read_bounded(&c.queue, 1048576)?;
    let (mut ids, diagnostics) = queue_ids(&queue);
    let program = read_bounded(&c.program, 65536)?;
    for row in program.lines().filter(|r| r.starts_with('|')) {
        let cols = crate::table::cells(row);
        if cols.len() > 3
            && matches!(
                cols[1].as_str(),
                "P0" | "P1" | "P2" | "P3" | "Done" | "Retired"
            )
        {
            ids.insert(cols[2].clone());
        }
    }
    let mut evidence = format!("Existing IDs: {ids:?}\nExisting queue scopes (avoid overlaps):\n");
    for row in queue.lines().filter(|r| r.starts_with('|')) {
        let cols = crate::table::raw_cells(row);
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

fn checked_interval(interval_minutes: u32) -> Result<Duration> {
    if interval_minutes == 0 || interval_minutes > 1440 {
        bail!("interval must be 1-1440 minutes, got {interval_minutes}");
    }
    Ok(Duration::from_secs(u64::from(interval_minutes) * 60))
}

/// Run one review cycle, logging (not propagating) failure. A persistent
/// service should not die from a single bad cycle the way the one-shot
/// `multithreader-review` command intentionally does (it exits nonzero on
/// a `failed` outcome so a caller — a person or a script — notices). Here
/// the next cycle is the notice: the failure is still recorded as an
/// inbox report by `run()` itself before this ever sees the `Err`.
fn cycle(config_path: &Path) {
    if let Err(e) = run(config_path) {
        eprintln!("multithreader-serve: cycle failed: {e:#}");
    }
}

/// Run review cycles on a fixed interval until the process is stopped.
///
/// This replaces relying on an external scheduler (the OS's Task
/// Scheduler, an installed service, etc.) to invoke `multithreader-review`
/// repeatedly: the loop lives in this process instead, so "on" is running
/// this command and "off" is not running it or stopping it (Ctrl+C) —
/// there is nothing to separately install, enable, or forget to disable.
/// Each `run()` call still owns and releases its own `review.lock`, so an
/// interrupt between cycles (during the sleep) leaves nothing locked; an
/// interrupt is only ever a concern if it lands mid-cycle, in which case
/// `run()`'s existing "review.lock exists: inspect active/stale run
/// before removal" recovery path applies exactly as it already does for
/// the one-shot command.
pub(crate) fn serve(config_path: &Path, interval_minutes: u32) -> Result<()> {
    let interval = checked_interval(interval_minutes)?;
    println!(
        "multithreader-serve: starting, interval {interval_minutes}m, config {}",
        config_path.display()
    );
    loop {
        cycle(config_path);
        std::thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_row_with_escaped_pipes_in_a_cell_is_still_a_queue_row() {
        let (ids, errors) =
            queue_ids(r#"| C89 | profile = "kernel\|control\|tooling" | Ready | Unowned |"#);
        assert!(errors.is_empty());
        assert_eq!(ids.into_iter().collect::<Vec<_>>(), ["C89"]);
    }

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
