//! Deterministic job feeder for the local support worker (C96 slice 2).
//!
//! Selection and rendering are pure functions over explicit inputs. The only I/O is the
//! thin adapter [`run_feed`], which walks a product checkout read-only and, only with
//! `write = true`, creates new manifests in an explicit jobs directory. The feeder reads
//! no environment, clock or network, never overwrites a manifest, and emits at most
//! [`MAX_LIMIT`] jobs per call, so the worker's own `max_pending` backpressure still decides
//! how fast results accumulate.
//!
//! Source: source files under `crates/<name>/src/` that contain no test at all. That is a
//! mechanical, checkable input; measured on `qwen3.5:9b-q6`, fixture-drafting jobs produced
//! parseable, rule-clean drafts while open-ended review jobs did not (see PR #26).

use anyhow::{Context, Result, bail};
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// The worker reads at most 32768 bytes of an evidence file; stay comfortably under it.
const MAX_FILE_BYTES: usize = 30_000;
/// Files smaller than this carry too little code to be worth a job.
const MIN_FILE_BYTES: usize = 200;
const MAX_WALK_FILES: usize = 20_000;
pub(crate) const MAX_LIMIT: usize = 16;
/// Crates the local-model autonomy policy keeps away from local drafting.
const EXCLUDED_CRATES: &[&str] = &[
    "ipc_bus",
    "wire",
    "hypervisor",
    "governance",
    "ast_auditor",
    "compliance_auditor",
    "platform_bridge",
    "core-contracts",
    "sdk",
];
const INSTRUCTION: &str = "Propose at most three new Rust test functions for public functions in the declared file that have no test yet. Use only items defined in the file, tempfile for any file I/O, no environment access and no clock. Output unverified test source in rust_draft and make no claim that tests pass.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeedFile {
    /// Checkout-relative path with `/` separators.
    pub relative: String,
    pub bytes: usize,
    pub has_tests: bool,
    pub has_unsafe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeedJob {
    pub id: String,
    pub evidence: String,
}

fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Stable job id for a path: `FEED-` plus 16 hex digits (only `[A-Za-z0-9-]`, well under 64).
pub(crate) fn feed_id(relative: &str) -> String {
    format!("FEED-{:016X}", fnv1a(relative))
}

fn in_excluded_crate(relative: &str) -> bool {
    relative
        .strip_prefix("crates/")
        .and_then(|rest| rest.split('/').next())
        .is_some_and(|name| EXCLUDED_CRATES.contains(&name))
}

fn eligible(file: &FeedFile) -> bool {
    file.relative.starts_with("crates/")
        && file.relative.contains("/src/")
        && file.relative.ends_with(".rs")
        && !in_excluded_crate(&file.relative)
        && !file.has_tests
        && !file.has_unsafe
        && (MIN_FILE_BYTES..=MAX_FILE_BYTES).contains(&file.bytes)
}

/// Picks up to `limit` eligible files in path order, skipping ids already seen
/// (existing manifests and past results), so repeated calls make steady progress.
pub(crate) fn select_feed_jobs(
    files: &[FeedFile],
    seen: &BTreeSet<String>,
    limit: usize,
) -> Vec<FeedJob> {
    let mut candidates: Vec<&FeedFile> = files.iter().filter(|file| eligible(file)).collect();
    candidates.sort_by(|a, b| a.relative.cmp(&b.relative));
    candidates
        .into_iter()
        .map(|file| FeedJob {
            id: feed_id(&file.relative),
            evidence: file.relative.clone(),
        })
        .filter(|job| !seen.contains(&job.id))
        .take(limit.min(MAX_LIMIT))
        .collect()
}

/// Renders the worker's job manifest (TOML, version 1, mode `test-fixture`).
pub(crate) fn render_job_toml(job: &FeedJob) -> String {
    format!(
        "schema_version = 1\nid = '{}'\nmode = 'test-fixture'\nevidence = ['{}']\ninstruction = '{}'\n",
        job.id, job.evidence, INSTRUCTION
    )
}

/// `FEED-XXXXXXXXXXXXXXXX-0000.json` (a worker result record) -> `FEED-XXXXXXXXXXXXXXXX`.
fn id_from_record_name(file_name: &str) -> Option<String> {
    let stem = file_name.strip_suffix(".json")?;
    let (id, sequence) = stem.rsplit_once('-')?;
    (sequence.len() == 4 && sequence.bytes().all(|b| b.is_ascii_digit())).then(|| id.to_string())
}

fn collect_files(
    dir: &Path,
    root: &Path,
    out: &mut Vec<FeedFile>,
    budget: &mut usize,
) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let kind = fs::symlink_metadata(&path)?.file_type();
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            collect_files(&path, root, out, budget)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if *budget == 0 {
            bail!("checkout has more than {MAX_WALK_FILES} source files; narrow the walk");
        }
        *budget -= 1;
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        out.push(FeedFile {
            relative,
            bytes: bytes.len(),
            has_tests: text.contains("#[test]") || text.contains("#[cfg(test)]"),
            has_unsafe: text.contains("unsafe "),
        });
    }
    Ok(())
}

/// Adapter: walks `checkout/crates/*/src` read-only, selects jobs and (only when `write`)
/// creates new manifests in `jobs_dir`. `history_dirs` are the worker's results and archive
/// directories, used only to recognise ids that were already attempted.
pub(crate) fn run_feed(
    checkout: &Path,
    jobs_dir: &Path,
    history_dirs: &[PathBuf],
    limit: usize,
    write: bool,
) -> Result<()> {
    if !checkout.is_absolute()
        || !jobs_dir.is_absolute()
        || history_dirs.iter().any(|d| !d.is_absolute())
    {
        bail!("checkout, jobs directory and history directories must be absolute paths");
    }
    if !checkout.is_dir() || !jobs_dir.is_dir() {
        bail!("checkout and jobs directory must exist");
    }
    if jobs_dir.starts_with(checkout) {
        bail!("the jobs directory must be outside the product checkout");
    }
    let mut seen = BTreeSet::new();
    for entry in fs::read_dir(jobs_dir)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if let Some(stem) = name.strip_suffix(".toml") {
            seen.insert(stem.to_string());
        }
    }
    for dir in history_dirs {
        if !dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(dir)? {
            if let Some(id) = id_from_record_name(&entry?.file_name().to_string_lossy()) {
                seen.insert(id);
            }
        }
    }
    let mut files = Vec::new();
    let mut budget = MAX_WALK_FILES;
    let crates = checkout.join("crates");
    if crates.is_dir() {
        collect_files(&crates, checkout, &mut files, &mut budget)?;
    }
    let jobs = select_feed_jobs(&files, &seen, limit);
    if jobs.is_empty() {
        println!("feed: no eligible new files (scanned {})", files.len());
        return Ok(());
    }
    for job in &jobs {
        if !write {
            println!(
                "feed (dry run): would create {}.toml for {}",
                job.id, job.evidence
            );
            continue;
        }
        let path = jobs_dir.join(format!("{}.toml", job.id));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("creating {}", path.display()))?;
        file.write_all(render_job_toml(job).as_bytes())?;
        file.sync_all()?;
        println!("feed: created {}.toml for {}", job.id, job.evidence);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(relative: &str, bytes: usize) -> FeedFile {
        FeedFile {
            relative: relative.into(),
            bytes,
            has_tests: false,
            has_unsafe: false,
        }
    }

    #[test]
    fn ids_are_stable_and_valid_worker_ids() {
        let id = feed_id("crates/paths/src/lib.rs");
        assert_eq!(id, feed_id("crates/paths/src/lib.rs"));
        assert_ne!(id, feed_id("crates/paths/src/other.rs"));
        assert!(id.len() <= 64 && id.starts_with("FEED-"));
        assert!(id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'));
    }

    #[test]
    fn only_testless_safe_mid_sized_files_in_allowed_crates_are_eligible() {
        let mut tested = file("crates/a/src/t.rs", 500);
        tested.has_tests = true;
        let mut unsafe_file = file("crates/a/src/u.rs", 500);
        unsafe_file.has_unsafe = true;
        let files = vec![
            file("crates/a/src/ok.rs", 500),
            tested,
            unsafe_file,
            file("crates/a/src/tiny.rs", 50),
            file("crates/a/src/huge.rs", 40_000),
            file("crates/ipc_bus/src/lib.rs", 500),
            file("crates/a/tests/it.rs", 500),
            file("core/hypervisor/src/x.rs", 500),
            file("crates/a/src/notes.md", 500),
        ];
        let jobs = select_feed_jobs(&files, &BTreeSet::new(), 10);
        assert_eq!(jobs.len(), 1, "{jobs:?}");
        assert_eq!(jobs[0].evidence, "crates/a/src/ok.rs");
    }

    #[test]
    fn selection_is_path_ordered_limited_and_skips_seen_ids() {
        let files = vec![
            file("crates/b/src/b.rs", 400),
            file("crates/a/src/a.rs", 400),
            file("crates/c/src/c.rs", 400),
        ];
        let first = select_feed_jobs(&files, &BTreeSet::new(), 2);
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].evidence, "crates/a/src/a.rs");
        assert_eq!(first[1].evidence, "crates/b/src/b.rs");
        let seen: BTreeSet<String> = first.iter().map(|job| job.id.clone()).collect();
        let next = select_feed_jobs(&files, &seen, 2);
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].evidence, "crates/c/src/c.rs");
        assert_eq!(select_feed_jobs(&files, &BTreeSet::new(), 10_000).len(), 3);
        assert!(select_feed_jobs(&files, &BTreeSet::new(), 0).is_empty());
    }

    #[test]
    fn rendered_manifest_is_valid_toml_with_the_worker_fields() {
        let job = FeedJob {
            id: feed_id("crates/a/src/a.rs"),
            evidence: "crates/a/src/a.rs".into(),
        };
        let value: toml::Value = toml::from_str(&render_job_toml(&job)).expect("toml");
        assert_eq!(value["schema_version"].as_integer(), Some(1));
        assert_eq!(value["id"].as_str(), Some(job.id.as_str()));
        assert_eq!(value["mode"].as_str(), Some("test-fixture"));
        assert_eq!(value["evidence"][0].as_str(), Some("crates/a/src/a.rs"));
        let instruction = value["instruction"].as_str().expect("instruction");
        assert!(!instruction.is_empty() && instruction.len() <= 8192);
    }

    #[test]
    fn result_record_names_map_back_to_job_ids() {
        assert_eq!(
            id_from_record_name("FEED-00AA00BB00CC00DD-0003.json").as_deref(),
            Some("FEED-00AA00BB00CC00DD")
        );
        assert_eq!(id_from_record_name("status.txt"), None);
        assert_eq!(id_from_record_name("FEED-X-abcd.json"), None);
    }

    fn write_file(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, text).expect("write");
    }

    #[test]
    fn adapter_dry_run_writes_nothing_and_write_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let checkout = dir.path().join("product");
        let jobs = dir.path().join("jobs");
        let results = dir.path().join("results");
        fs::create_dir_all(&jobs).expect("jobs");
        fs::create_dir_all(&results).expect("results");
        let body = "pub fn add(a: u32, b: u32) -> u32 { a + b }\n".repeat(10);
        write_file(&checkout.join("crates/a/src/lib.rs"), &body);
        write_file(
            &checkout.join("crates/b/src/lib.rs"),
            &format!("{body}#[cfg(test)]\nmod tests {{}}\n"),
        );
        write_file(&checkout.join("crates/ipc_bus/src/lib.rs"), &body);

        run_feed(&checkout, &jobs, std::slice::from_ref(&results), 5, false).expect("dry run");
        assert_eq!(fs::read_dir(&jobs).expect("jobs").count(), 0);

        run_feed(&checkout, &jobs, std::slice::from_ref(&results), 5, true).expect("write");
        let created: Vec<_> = fs::read_dir(&jobs).expect("jobs").collect();
        assert_eq!(created.len(), 1, "only crates/a is eligible");

        run_feed(&checkout, &jobs, &[results], 5, true).expect("again");
        assert_eq!(
            fs::read_dir(&jobs).expect("jobs").count(),
            1,
            "an existing manifest is never recreated"
        );
    }

    #[test]
    fn adapter_rejects_relative_or_nested_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let checkout = dir.path().join("product");
        fs::create_dir_all(checkout.join("jobs")).expect("nested jobs");
        assert!(run_feed(Path::new("rel"), &checkout, &[], 1, false).is_err());
        assert!(run_feed(&checkout, &checkout.join("jobs"), &[], 1, false).is_err());
    }
}
