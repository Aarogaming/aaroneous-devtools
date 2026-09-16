use crate::intake::{Proposal, read_bounded, relative_file};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
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
    let _file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .context("review.lock exists: inspect active/stale run before removal")?;
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
