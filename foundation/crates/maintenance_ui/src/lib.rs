#![deny(unsafe_code)]

pub mod http;
pub mod packets;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    pub schema_version: u32,
    pub checkout: PathBuf,
    pub queue: PathBuf,
    pub jobs_directory: PathBuf,
    pub results_directory: PathBuf,
    pub archive_directory: PathBuf,
    pub reservation_directory: PathBuf,
    pub endpoint: String,
    pub max_pending: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_token_file: Option<PathBuf>,
}

pub fn read_text(path: &Path, limit: u64) -> Result<String> {
    let mut content = String::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_string(&mut content)?;
    if content.len() as u64 > limit {
        bail!("File exceeds size limit: {}", path.display());
    }
    Ok(content)
}

fn normalized(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    }
}
fn within(path: &Path, root: &Path) -> bool {
    let root = normalized(root).trim_end_matches('/').to_owned();
    let path = normalized(path);
    path == root || path.starts_with(&format!("{root}/"))
}
fn check_path(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        bail!(
            "Use an explicit absolute path without parent segments: {}",
            path.display()
        );
    }
    let mut ancestor = PathBuf::new();
    for component in path.components() {
        ancestor.push(component.as_os_str());
        if ancestor.exists() && fs::symlink_metadata(&ancestor)?.file_type().is_symlink() {
            bail!("Symbolic links are not allowed: {}", ancestor.display());
        }
    }
    Ok(())
}

pub fn load_config(path: &Path) -> Result<WorkerConfig> {
    check_path(path)?;
    let config: WorkerConfig = toml::from_str(&read_text(path, 16384)?)?;
    if config.schema_version != 1 || !(1..=64).contains(&config.max_pending) {
        bail!("Unsupported worker config");
    }
    for directory in [
        &config.checkout,
        &config.jobs_directory,
        &config.results_directory,
        &config.archive_directory,
        &config.reservation_directory,
    ] {
        check_path(directory)?;
        if !directory.is_dir() {
            bail!("Missing directory: {}", directory.display());
        }
    }
    check_path(&config.queue)?;
    if !config.queue.is_file() || !config.checkout.join(".git").exists() {
        bail!("Repository and coordination queue are required");
    }
    if let Some(ref token_file) = config.http_token_file {
        check_path(token_file)?;
        if !token_file.is_file() {
            bail!("Missing HTTP token file: {}", token_file.display());
        }
    }
    let outputs = [
        &config.results_directory,
        &config.archive_directory,
        &config.reservation_directory,
    ];
    for output in outputs {
        if within(output, &config.checkout) {
            bail!("Artifacts must stay outside the product checkout");
        }
    }
    if within(&config.results_directory, &config.archive_directory)
        || within(&config.archive_directory, &config.results_directory)
    {
        bail!("Results and archive must be separate");
    }
    let url = reqwest::Url::parse(&config.endpoint)?;
    if url.scheme() != "http"
        || !matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        bail!("Local endpoint must be plain loopback HTTP without credentials or paths");
    }
    Ok(config)
}

pub fn load_http_token(path: &Path) -> Result<String> {
    check_path(path)?;
    let content = read_text(path, 128)?;
    let token = content.trim();
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("HTTP token must contain exactly 64 hexadecimal characters from 32 random bytes");
    }
    Ok(token.to_owned())
}

pub struct Lock(PathBuf);
impl Lock {
    pub fn acquire(directory: &Path) -> Result<Self> {
        let path = directory.join("review.lock");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .context("Another reviewer or worker holds the reservation")?;
        let lock = Self(path);
        file.write_all(b"owner=devtools.interactive\n")?;
        Ok(lock)
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub fn list_files(directory: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|v| v.to_str()) == Some(extension) {
            check_path(&path)?;
            if !path.is_file() {
                bail!("Expected a regular file");
            }
            files.push(path);
            if files.len() > 256 {
                bail!("Too many artifacts; review retention before continuing");
            }
        }
    }
    files.sort();
    Ok(files)
}

#[derive(Serialize)]
pub struct ResultSummary {
    pub filename: String,
    pub job_id: String,
    pub outcome: String,
    pub submitted_at_secs: Option<u64>,
}
#[derive(Serialize)]
pub struct Status {
    pub schema_version: u32,
    pub checkout: String,
    pub repository_status: String,
    pub local_model: String,
    pub jobs: Vec<String>,
    pub results: Vec<ResultSummary>,
    pub last_worker_poll_secs: Option<u64>,
    pub worker_observation: String,
    pub worker_status_stale: bool,
    pub reservation_present: bool,
    pub max_pending: usize,
    pub cloud: &'static str,
    pub observed_at_secs: u64,
}

pub fn status(config: &WorkerConfig, now: u64) -> Result<Status> {
    let jobs = list_files(&config.jobs_directory, "toml")?
        .iter()
        .map(|path| {
            path.file_name()
                .map(|v| v.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .collect();
    let mut results = Vec::new();
    for path in list_files(&config.results_directory, "json")? {
        let record: serde_json::Value = serde_json::from_str(&read_text(&path, 1048576)?)?;
        results.push(ResultSummary {
            filename: path
                .file_name()
                .context("Result filename missing")?
                .to_string_lossy()
                .into_owned(),
            job_id: record["job_id"].as_str().unwrap_or("unknown").to_owned(),
            outcome: record["outcome"].as_str().unwrap_or("unknown").to_owned(),
            submitted_at_secs: record["submitted_at_secs"].as_u64(),
        });
    }
    let (last_worker_poll_secs, worker_observation) =
        match read_text(&config.results_directory.join("status.txt"), 4096) {
            Ok(text) => (
                text.lines().find_map(|l| {
                    l.strip_prefix("last_poll_secs=")
                        .and_then(|v| v.parse().ok())
                }),
                text.lines()
                    .find_map(|l| l.strip_prefix("outcome="))
                    .unwrap_or("unknown")
                    .to_owned(),
            ),
            Err(_) => (None, "No readable worker observation yet".into()),
        };
    let repository_status = match Command::new("git")
        .args(["status", "--short", "--branch"])
        .current_dir(&config.checkout)
        .output()
    {
        Ok(output) if output.status.success() && output.stdout.len() <= 32768 => {
            String::from_utf8_lossy(&output.stdout).into_owned()
        }
        _ => "Git status unavailable".into(),
    };
    Ok(Status {
        schema_version: 1,
        checkout: config.checkout.display().to_string(),
        repository_status,
        local_model: probe_model(&config.endpoint),
        jobs,
        results,
        last_worker_poll_secs,
        worker_observation,
        worker_status_stale: last_worker_poll_secs
            .is_none_or(|last| now < last || now - last > 300),
        reservation_present: config.reservation_directory.join("review.lock").exists(),
        max_pending: config.max_pending,
        cloud: "disabled: no paid provider configured",
        observed_at_secs: now,
    })
}

fn probe_model(endpoint: &str) -> String {
    let probe = || -> Result<bool> {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(2))
            .build()?;
        let mut response = client
            .get(format!("{}/api/tags", endpoint.trim_end_matches('/')))
            .send()?
            .error_for_status()?;
        let mut body = String::new();
        Read::by_ref(&mut response)
            .take(65537)
            .read_to_string(&mut body)?;
        if body.len() > 65536 {
            bail!("Model inventory too large");
        }
        let value: serde_json::Value = serde_json::from_str(&body)?;
        Ok(value["models"]
            .as_array()
            .is_some_and(|models| models.iter().any(|model| model["name"] == "qwen3.5:9b-q6")))
    };
    match probe() {
        Ok(true) => "Qwen available".into(),
        Ok(false) => "Ollama reachable; qwen3.5:9b-q6 missing".into(),
        Err(_) => "Local model unavailable".into(),
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewJob {
    pub id: String,
    pub mode: String,
    pub evidence: Vec<String>,
    pub instruction: String,
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

pub fn approve_job(config: &WorkerConfig, job: NewJob) -> Result<()> {
    if !valid_id(&job.id)
        || !matches!(
            job.mode.as_str(),
            "analysis" | "test-fixture" | "code-draft"
        )
        || !(1..=8).contains(&job.evidence.len())
        || job.instruction.trim().is_empty()
        || job.instruction.len() > 8192
    {
        bail!(
            "Invalid job: use a short ID, supported mode, 1–8 evidence paths and a bounded instruction"
        );
    }
    let _lock = Lock::acquire(&config.reservation_directory)?;
    if list_files(&config.jobs_directory, "toml")?.len() >= 256 {
        bail!("Job limit reached");
    }
    let queue = read_text(&config.queue, 262144)?;
    let mut total = job.instruction.len();
    for evidence in &job.evidence {
        let relative = Path::new(evidence);
        if relative.is_absolute()
            || evidence.contains('\\')
            || evidence.contains(':')
            || !relative
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
            || evidence.trim() != evidence
            || evidence.is_empty()
        {
            bail!("Use exact relative evidence files without parent segments");
        }
        let path = config.checkout.join(relative);
        check_path(&path)?;
        total += read_text(&path, 32768)?.len() + evidence.len();
        if queue.lines().filter(|row| row.starts_with('|')).any(|row| {
            let columns: Vec<_> = row.split('|').map(str::trim).collect();
            columns
                .get(3)
                .is_some_and(|s| matches!(*s, "Claimed" | "Review"))
                && (row.contains(evidence)
                    || row.split('`').any(|scope| {
                        scope
                            .strip_suffix("/**")
                            .is_some_and(|p| evidence.starts_with(&format!("{p}/")))
                    }))
        }) {
            bail!("Evidence is reserved by an active claim; ask its owner first");
        }
    }
    if total > 60000 {
        bail!("Evidence bundle is too large");
    }
    #[derive(Serialize)]
    struct Manifest<'a> {
        schema_version: u32,
        #[serde(flatten)]
        job: &'a NewJob,
    }
    let text = toml::to_string(&Manifest {
        schema_version: 1,
        job: &job,
    })?;
    if text.len() > 16384 {
        bail!("Serialized job too large");
    }
    let destination = config.jobs_directory.join(format!("{}.toml", job.id));
    let staging = destination.with_extension("partial");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    drop(file);
    if destination.exists() {
        let _ = fs::remove_file(staging);
        bail!("Job ID already exists; keep retry history explicit");
    }
    fs::hard_link(&staging, &destination)?;
    fs::remove_file(staging)?;
    Ok(())
}

fn result_path(config: &WorkerConfig, filename: &str) -> Result<PathBuf> {
    if filename.len() > 100
        || !filename.ends_with(".json")
        || filename
            .bytes()
            .any(|b| !b.is_ascii_alphanumeric() && !matches!(b, b'-' | b'_' | b'.'))
        || filename.contains("..")
    {
        bail!("Invalid result filename");
    }
    let path = config.results_directory.join(filename);
    check_path(&path)?;
    if !path.is_file() {
        bail!("Result not found");
    }
    Ok(path)
}

pub fn result(config: &WorkerConfig, filename: &str) -> Result<serde_json::Value> {
    Ok(serde_json::from_str(&read_text(
        &result_path(config, filename)?,
        1048576,
    )?)?)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub filename: String,
    pub disposition: String,
    pub note: String,
}
pub fn review(config: &WorkerConfig, review: Review, now: u64) -> Result<()> {
    if !matches!(
        review.disposition.as_str(),
        "accepted-for-follow-up" | "rejected"
    ) || review.note.trim().is_empty()
        || review.note.len() > 4096
    {
        bail!("Choose a disposition and provide a bounded review note");
    }
    let _lock = Lock::acquire(&config.reservation_directory)?;
    let source = result_path(config, &review.filename)?;
    let _record: serde_json::Value = serde_json::from_str(&read_text(&source, 1048576)?)?;
    if list_files(&config.archive_directory, "json")?.len() >= 256 {
        bail!("Archive limit reached; preserve idempotency history before compaction");
    }
    let destination = config.archive_directory.join(&review.filename);
    let note_path = destination.with_extension("review");
    let note = serde_json::json!({"schema_version":1,"reviewed_at_secs":now,"disposition":review.disposition,"note":review.note,"product_verified":false});
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&note_path)?;
    file.write_all(&serde_json::to_vec_pretty(&note)?)?;
    file.sync_all()?;
    drop(file);
    if let Err(error) = fs::hard_link(&source, &destination) {
        let _ = fs::remove_file(note_path);
        return Err(error.into());
    }
    fs::remove_file(source)?;
    Ok(())
}

/// Explicit runtime copy: new interactive jobs stay out of the shared Git job directory.
pub fn session_config(original: &Path, runtime: &Path) -> Result<PathBuf> {
    let mut config = load_config(original)?;
    validate_runtime(&config, original, runtime)?;
    fs::create_dir_all(runtime)?;
    let jobs = runtime.join("jobs");
    fs::create_dir_all(&jobs)?;
    check_path(&jobs)?;
    config.jobs_directory = jobs;
    let path = runtime.join("worker.toml");
    if path.exists() {
        check_path(&path)?;
    }
    let contents = toml::to_string(&config)?;
    fs::write(&path, contents)?;
    load_config(&path)?;
    Ok(path)
}

fn validate_runtime(config: &WorkerConfig, original: &Path, runtime: &Path) -> Result<()> {
    check_path(runtime)?;
    if within(runtime, &config.checkout) {
        bail!("Runtime must be outside Aaroneous");
    }
    for shared in [
        &config.jobs_directory,
        &config.results_directory,
        &config.archive_directory,
        &config.reservation_directory,
    ] {
        if within(runtime, shared) || within(shared, runtime) {
            bail!("Utility runtime must not overlap existing worker storage");
        }
    }
    if within(original, runtime) {
        bail!("Utility runtime must not contain its source configuration");
    }
    Ok(())
}

/// Held for the utility lifetime, before any session configuration is rewritten.
/// A crash leaves evidence for explicit inspection, never an automatic takeover.
pub struct RuntimeLease(PathBuf);
impl RuntimeLease {
    pub fn acquire(original: &Path, runtime: &Path) -> Result<Self> {
        let config = load_config(original)?;
        validate_runtime(&config, original, runtime)?;
        fs::create_dir_all(runtime)?;
        let path = runtime.join("utility.lock");
        check_path(&path)?;
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&path)
            .context("Utility runtime is reserved; inspect the existing instance or stale utility.lock before restarting")?;
        let lease = Self(path);
        file.write_all(format!("owner=devtools.utility\npid={}\n", std::process::id()).as_bytes())?;
        file.sync_all()?;
        Ok(lease)
    }
}
impl Drop for RuntimeLease {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_lease_blocks_duplicate_launch_without_rewriting_and_releases() {
        let (temp, config) = fixture();
        let original = temp.path().join("original.toml");
        fs::write(&original, toml::to_string(&config).expect("config")).expect("write");
        let runtime = temp.path().join("runtime");
        let lease = RuntimeLease::acquire(&original, &runtime).expect("lease");
        let sentinel = runtime.join("worker.toml");
        fs::write(&sentinel, "preserved").expect("sentinel");
        assert!(RuntimeLease::acquire(&original, &runtime).is_err());
        assert_eq!(fs::read_to_string(&sentinel).expect("read"), "preserved");
        assert!(RuntimeLease::acquire(&original, &config.checkout.join("runtime")).is_err());
        drop(lease);
        assert!(!runtime.join("utility.lock").exists());
        let _next = RuntimeLease::acquire(&original, &runtime).expect("restart");
    }
    fn fixture() -> (tempfile::TempDir, WorkerConfig) {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        for name in ["checkout/.git", "jobs", "results", "archive", "reservation"] {
            fs::create_dir_all(root.join(name)).expect("fixture dir");
        }
        fs::write(root.join("queue.md"), "| ID | Work | Status |\n").expect("queue");
        fs::write(root.join("checkout/lib.rs"), "pub fn reducer() {}\n").expect("source");
        let config = WorkerConfig {
            schema_version: 1,
            checkout: root.join("checkout"),
            queue: root.join("queue.md"),
            jobs_directory: root.join("jobs"),
            results_directory: root.join("results"),
            archive_directory: root.join("archive"),
            reservation_directory: root.join("reservation"),
            endpoint: "http://127.0.0.1:11434".into(),
            max_pending: 8,
            http_token_file: None,
        };
        (temp, config)
    }
    fn job(evidence: &str) -> NewJob {
        NewJob {
            id: "TEST".into(),
            mode: "analysis".into(),
            evidence: vec![evidence.into()],
            instruction: "Review reducer error handling".into(),
        }
    }
    #[test]
    fn rejects_traversal_and_claim_conflicts() {
        let (_temp, config) = fixture();
        assert!(approve_job(&config, job("../queue.md")).is_err());
        fs::write(&config.queue, "| C1 | `lib.rs` | Claimed | owner |\n").expect("claim");
        assert!(approve_job(&config, job("lib.rs")).is_err());
        assert!(
            list_files(&config.jobs_directory, "toml")
                .expect("files")
                .is_empty()
        );
    }
    #[test]
    fn creates_approved_manifest_without_overwriting() {
        let (_temp, config) = fixture();
        approve_job(&config, job("lib.rs")).expect("approve");
        assert!(approve_job(&config, job("lib.rs")).is_err());
        let value: toml::Value = toml::from_str(
            &read_text(&config.jobs_directory.join("TEST.toml"), 16384).expect("text"),
        )
        .expect("manifest");
        assert_eq!(value["schema_version"].as_integer(), Some(1));
    }
    #[test]
    fn reviews_preserve_worker_history_and_do_not_verify() {
        let (_temp, config) = fixture();
        let source = config.results_directory.join("TEST-0000.json");
        let record =
            r#"{"schema_version":1,"job_id":"TEST","input":"snapshot","outcome":"failed"}"#;
        fs::write(&source, record).expect("record");
        review(
            &config,
            Review {
                filename: "TEST-0000.json".into(),
                disposition: "rejected".into(),
                note: "Malformed response; retain history".into(),
            },
            42,
        )
        .expect("review");
        assert!(!source.exists());
        assert_eq!(
            read_text(&config.archive_directory.join("TEST-0000.json"), 4096).expect("archive"),
            record
        );
        let note: serde_json::Value = serde_json::from_str(
            &read_text(&config.archive_directory.join("TEST-0000.review"), 8192).expect("note"),
        )
        .expect("json");
        assert_eq!(note["product_verified"], false);
    }
    #[test]
    fn reservation_and_existing_archive_prevent_loss() {
        let (_temp, config) = fixture();
        let lock = Lock::acquire(&config.reservation_directory).expect("lock");
        assert!(approve_job(&config, job("lib.rs")).is_err());
        drop(lock);
        let source = config.results_directory.join("TEST-0000.json");
        fs::write(&source, "{}").expect("source");
        fs::write(config.archive_directory.join("TEST-0000.json"), "original").expect("archive");
        assert!(
            review(
                &config,
                Review {
                    filename: "TEST-0000.json".into(),
                    disposition: "rejected".into(),
                    note: "review".into()
                },
                1
            )
            .is_err()
        );
        assert!(source.exists());
        assert_eq!(
            read_text(&config.archive_directory.join("TEST-0000.json"), 100).expect("archive"),
            "original"
        );
    }
    #[test]
    fn rejects_remote_endpoint_and_nested_storage() {
        let (temp, mut config) = fixture();
        config.endpoint = "https://example.com".into();
        let path = temp.path().join("config.toml");
        fs::write(&path, toml::to_string(&config).expect("config")).expect("write");
        assert!(load_config(&path).is_err());
        config.endpoint = "http://127.0.0.1:11434".into();
        config.results_directory = config.checkout.clone();
        fs::write(&path, toml::to_string(&config).expect("config")).expect("write");
        assert!(load_config(&path).is_err());
    }
    #[test]
    fn test_token_file_validation_and_constant_time_loading() {
        let (temp, mut config) = fixture();
        let token_path = temp.path().join("token.txt");
        let valid_token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        fs::write(&token_path, valid_token).expect("write token");
        assert_eq!(
            load_http_token(&token_path).expect("load token"),
            valid_token
        );

        let invalid_token_path = temp.path().join("invalid_token.txt");
        fs::write(&invalid_token_path, "short-token").expect("write invalid token");
        assert!(load_http_token(&invalid_token_path).is_err());

        config.http_token_file = Some(token_path);
        let path = temp.path().join("config.toml");
        fs::write(&path, toml::to_string(&config).expect("config")).expect("write");
        assert!(load_config(&path).is_ok());

        config.http_token_file = Some(temp.path().join("non_existent_token.txt"));
        fs::write(&path, toml::to_string(&config).expect("config")).expect("write");
        assert!(load_config(&path).is_err());
    }
}
