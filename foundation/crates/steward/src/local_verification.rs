//! Explicit-input local verification adapter. No GitHub calls and no ambient credentials.
//! This executor is for owner-trusted local code, not a sandbox for public contributions.

use crate::{chain, checklist, suite, verification, wire};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const LIMIT: u64 = 8 * 1024 * 1024;
const NAMESPACE: &str = "aaroneous-local-verification-v1";

fn absolute(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("an explicit absolute path is required".into());
    }
    let result = path.canonicalize().map_err(|e| e.to_string())?;
    if fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("symlink input refused".into());
    }
    Ok(result)
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    let mut input = File::open(absolute(path)?).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut input)
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("input exceeds size limit".into());
    }
    Ok(bytes)
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn tool_digest(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 32768];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn put(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}

fn value(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&read(path)?).map_err(|e| e.to_string())
}

fn strict(value: &Value, keys: &[&str]) -> Result<(), String> {
    let map = value.as_object().ok_or("expected object")?;
    if map.len() != keys.len() || map.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err("unknown or missing fields".into());
    }
    Ok(())
}

fn git(git: &Path, repo: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut command = Command::new(absolute(git)?);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    let output = command
        .arg("-C")
        .arg(absolute(repo)?)
        .args(args)
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() || output.stdout.len() as u64 > LIMIT {
        return Err("git read failed or exceeded limit".into());
    }
    Ok(output.stdout)
}

/// Read a suite from the trusted base commit, never the candidate worktree.
pub fn pinned_suite(
    git_path: &Path,
    repo: &Path,
    request: &verification::Request,
) -> Result<suite::Suite, String> {
    if let Some(problem) = request.problem() {
        return Err(problem);
    }
    let spec = format!("{}:ci/suites/{}.toml", request.base_sha, request.suite);
    let bytes = git(git_path, repo, &["show", &spec])?;
    let suite = suite::parse_suite(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)?;
    suite::plan(request, &suite, &request.platform)?;
    Ok(suite)
}

/// Issue a deterministic job card per required platform using policy and suite from base.
pub fn issue(
    git_path: &Path,
    repo: &Path,
    name: &str,
    head: &str,
    base: &str,
    suite_name: &str,
) -> Result<Vec<verification::Request>, String> {
    let probe = verification::Request {
        repo: name.into(),
        head: head.into(),
        base_sha: base.into(),
        platform: "linux-x86_64".into(),
        suite: suite_name.into(),
        suite_digest: "0".repeat(64),
    };
    if let Some(problem) = probe.problem() {
        return Err(problem);
    }
    for revision in [head, base] {
        let spec = format!("{revision}^{{commit}}");
        if String::from_utf8(git(git_path, repo, &["rev-parse", "--verify", &spec])?)
            .map_err(|e| e.to_string())?
            .trim()
            != revision
        {
            return Err("revision is not an exact commit".into());
        }
    }
    let suite_spec = format!("{base}:ci/suites/{suite_name}.toml");
    let manifest = git(git_path, repo, &["show", &suite_spec])?;
    let suite = suite::parse_suite(std::str::from_utf8(&manifest).map_err(|e| e.to_string())?)?;
    let policy_spec = format!("{base}:ci/platform-policy.toml");
    let policy_text = git(git_path, repo, &["show", &policy_spec])?;
    let policy =
        suite::Policy::parse(std::str::from_utf8(&policy_text).map_err(|e| e.to_string())?)?;
    let paths = git(
        git_path,
        repo,
        &["diff", "--name-only", "-z", base, head, "--"],
    )?;
    let changed = paths
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| {
            std::str::from_utf8(p)
                .map(str::to_owned)
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let requests = policy
        .required(&changed)
        .into_iter()
        .map(|platform| verification::Request {
            platform,
            suite_digest: suite.digest.clone(),
            ..probe.clone()
        })
        .collect::<Vec<_>>();
    for request in &requests {
        suite::plan(request, &suite, &request.platform)?;
    }
    Ok(requests)
}

/// Filesystem spool: immutable cards, exclusive claim, monotonic attempts, immutable receipts.
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn open(root: &Path) -> Result<Self, String> {
        Ok(Self {
            root: absolute(root)?,
        })
    }
    fn directory(&self, request: &verification::Request) -> Result<PathBuf, String> {
        if let Some(problem) = request.problem() {
            return Err(problem);
        }
        let path = self.root.join(request.id());
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        let canonical = absolute(&path)?;
        if canonical.parent() != Some(self.root.as_path()) {
            return Err("job directory escapes spool".into());
        }
        Ok(canonical)
    }
    pub fn submit(&self, request: &verification::Request) -> Result<PathBuf, String> {
        let directory = self.directory(request)?;
        let path = directory.join("request.json");
        let bytes =
            serde_json::to_vec(&wire::request_to_value(request)).map_err(|e| e.to_string())?;
        if path.exists() {
            if read(&path)? != bytes {
                return Err("existing card differs or is incomplete".into());
            }
        } else {
            put(&path, &bytes)?;
        }
        Ok(path)
    }
    pub fn latest_attempt(&self, request: &verification::Request) -> Result<u32, String> {
        let directory = self.directory(request)?;
        let mut top = 0;
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let text = name.to_str().ok_or("non-UTF8 spool path")?;
            if let Some(number) = text
                .strip_prefix("attempt-")
                .and_then(|s| s.strip_suffix(".json"))
            {
                let attempt: u32 = number.parse().map_err(|_| "invalid attempt record")?;
                if attempt == 0 || attempt > 1000 {
                    return Err("invalid attempt number".into());
                }
                let record = value(&entry.path())?;
                strict(&record, &["request_id", "node", "attempt"])?;
                if record["request_id"] != request.id() || record["attempt"] != attempt {
                    return Err("attempt record mismatch".into());
                }
                top = top.max(attempt);
            }
        }
        Ok(top)
    }
    pub fn claim(&self, request: &verification::Request, node: &str) -> Result<Claim, String> {
        self.submit(request)?;
        let probe = verification::Outcome {
            request_id: request.id(),
            head: request.head.clone(),
            platform: request.platform.clone(),
            suite_digest: request.suite_digest.clone(),
            node: node.into(),
            attempt: 1,
            verdict: verification::Verdict::Error,
            signature_verified: false,
        };
        wire::outcome_from_value(&wire::outcome_to_value(&probe), false)?;
        let directory = self.directory(request)?;
        let lock = directory.join("claim.json");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
            .map_err(|_| "job already claimed or claim unavailable")?;
        let attempt = self.latest_attempt(request)? + 1;
        if attempt > 1000 {
            return Err("retry limit reached; claim requires recovery".into());
        }
        let record = json!({"request_id": request.id(), "node": node, "attempt": attempt});
        let bytes = serde_json::to_vec(&record).map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        put(&directory.join(format!("attempt-{attempt}.json")), &bytes)?;
        Ok(Claim {
            request: request.clone(),
            node: node.into(),
            attempt,
            directory,
            lock,
        })
    }
    /// Explicit crash recovery after the operator has stopped the old worker. Never auto-expire a running lease.
    pub fn recover(
        &self,
        request: &verification::Request,
        expected_node: &str,
        expected_attempt: u32,
    ) -> Result<(), String> {
        let directory = self.directory(request)?;
        let path = directory.join("claim.json");
        let record = value(&path)?;
        strict(&record, &["request_id", "node", "attempt"])?;
        if record
            != json!({"request_id": request.id(), "node": expected_node, "attempt": expected_attempt})
        {
            return Err("recovery does not match current claim".into());
        }
        if self.latest_attempt(request)? != expected_attempt {
            return Err("attempt history and claim disagree".into());
        }
        put(
            &directory.join(format!("abandoned-{expected_attempt}.json")),
            &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
        )?;
        fs::remove_file(path).map_err(|e| e.to_string())
    }
    pub fn receipt_path(
        &self,
        request: &verification::Request,
        attempt: u32,
    ) -> Result<PathBuf, String> {
        Ok(self
            .directory(request)?
            .join(format!("receipt-{attempt}.json")))
    }
}

pub struct Claim {
    pub request: verification::Request,
    pub node: String,
    pub attempt: u32,
    directory: PathBuf,
    lock: PathBuf,
}

/// Explicit executable paths and environment. Owner-trusted local execution only.
pub struct Runner {
    pub git: PathBuf,
    pub tools: BTreeMap<String, PathBuf>,
    pub environment: BTreeMap<String, String>,
}

impl Runner {
    pub fn execute(&self, claim: &Claim, repo: &Path) -> Result<PathBuf, String> {
        let suite = pinned_suite(&self.git, repo, &claim.request)?;
        let plan = suite::plan(&claim.request, &suite, &claim.request.platform)?;
        let actual_platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
        if claim.request.platform != actual_platform {
            return Err("request does not match actual host platform".into());
        }
        if absolute(repo)? == claim.directory || claim.directory.starts_with(absolute(repo)?) {
            return Err("spool must be outside the candidate checkout".into());
        }
        let head = String::from_utf8(git(&self.git, repo, &["rev-parse", "HEAD"])?)
            .map_err(|e| e.to_string())?;
        if head.trim() != claim.request.head
            || !git(
                &self.git,
                repo,
                &["status", "--porcelain", "--untracked-files=all"],
            )?
            .is_empty()
        {
            return Err("candidate must be clean at the pinned head".into());
        }
        let mut records = Vec::new();
        let mut results = Vec::new();
        for step in &plan.steps {
            let tool = absolute(
                self.tools
                    .get(&step.argv[0])
                    .ok_or("suite program has no explicit tool mapping")?,
            )?;
            if cfg!(windows)
                && tool
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_none_or(|s| !s.eq_ignore_ascii_case("exe"))
            {
                return Err("Windows tools must be native executables, not shell wrappers".into());
            }
            let cwd = absolute(&repo.join(&step.cwd))?;
            if !cwd.starts_with(absolute(repo)?) {
                return Err("step cwd escapes checkout".into());
            }
            let stdout_path = claim
                .directory
                .join(format!("{}-{}.stdout", claim.attempt, step.name));
            let stderr_path = claim
                .directory
                .join(format!("{}-{}.stderr", claim.attempt, step.name));
            let stdout = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&stdout_path)
                .map_err(|e| e.to_string())?;
            let stderr = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&stderr_path)
                .map_err(|e| e.to_string())?;
            let start = Instant::now();
            let executable_digest = tool_digest(&tool)?;
            let child = Command::new(&tool)
                .args(&step.argv[1..])
                .current_dir(cwd)
                .env_clear()
                .envs(&self.environment)
                .stdin(Stdio::null())
                .stdout(stdout)
                .stderr(stderr)
                .spawn();
            let mut result = suite::StepResult {
                name: step.name.clone(),
                started: false,
                exit: None,
                timed_out: false,
            };
            let mut log_limit = false;
            if let Ok(mut child) = child {
                result.started = true;
                loop {
                    if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                        result.exit = status.code();
                        break;
                    }
                    log_limit = fs::metadata(&stdout_path).map_err(|e| e.to_string())?.len()
                        > LIMIT
                        || fs::metadata(&stderr_path).map_err(|e| e.to_string())?.len() > LIMIT;
                    if start.elapsed() >= Duration::from_secs(u64::from(step.timeout_s))
                        || log_limit
                    {
                        result.timed_out = true;
                        child.kill().map_err(|e| e.to_string())?;
                        child.wait().map_err(|e| e.to_string())?;
                        break;
                    }
                    thread::sleep(Duration::from_millis(20));
                }
            }
            // Oversized output fails closed without importing it as valid evidence.
            if log_limit {
                return Err("log limit exceeded; claim retained for operator recovery".into());
            }
            let out = read(&stdout_path)?;
            let err = read(&stderr_path)?;
            records.push(json!({"name": step.name, "argv": step.argv, "cwd": step.cwd, "timeout_s": step.timeout_s,
                "started": result.started, "exit": result.exit, "timed_out": result.timed_out,
                "elapsed_ms": start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                "stdout_digest": hash(&out), "stderr_digest": hash(&err)}));
            records.last_mut().ok_or("step record missing")?["tool_digest"] =
                json!(executable_digest);
            let passed = result.started && result.exit == Some(0) && !result.timed_out;
            results.push(result);
            if !passed {
                break;
            }
        }
        if String::from_utf8(git(&self.git, repo, &["rev-parse", "HEAD"])?)
            .map_err(|e| e.to_string())?
            .trim()
            != claim.request.head
            || !git(
                &self.git,
                repo,
                &["status", "--porcelain", "--untracked-files=all"],
            )?
            .is_empty()
        {
            return Err("candidate changed during verification; claim retained".into());
        }
        let outcome = verification::Outcome {
            request_id: claim.request.id(),
            head: claim.request.head.clone(),
            platform: claim.request.platform.clone(),
            suite_digest: claim.request.suite_digest.clone(),
            node: claim.node.clone(),
            attempt: claim.attempt,
            verdict: suite::summarize(&plan, &results),
            signature_verified: false,
        };
        let environment_digest =
            hash(&serde_json::to_vec(&self.environment).map_err(|e| e.to_string())?);
        let receipt = json!({"v": 1, "request": wire::request_to_value(&claim.request), "outcome": wire::outcome_to_value(&outcome), "execution": "trusted-local-v1", "environment_digest": environment_digest, "steps": records});
        let path = claim
            .directory
            .join(format!("receipt-{}.json", claim.attempt));
        put(
            &path,
            &serde_json::to_vec(&receipt).map_err(|e| e.to_string())?,
        )?;
        let lock = value(&claim.lock)?;
        if lock
            != json!({"request_id": claim.request.id(), "node": claim.node, "attempt": claim.attempt})
        {
            return Err("claim changed during execution".into());
        }
        // Timeout may leave descendants on this trusted-local adapter. Keep the lease
        // until the operator has stopped them; never start a retry automatically.
        if !results.iter().any(|result| result.timed_out) {
            fs::remove_file(&claim.lock).map_err(|e| e.to_string())?;
        }
        Ok(path)
    }
}

/// Explicit OpenSSH executable; no ssh-agent or ambient key discovery.
pub fn sign(ssh_keygen: &Path, key: &Path, receipt: &Path) -> Result<PathBuf, String> {
    let receipt = absolute(receipt)?;
    let signature = receipt.with_extension("json.sig");
    if signature.exists() {
        return Err("signature already exists".into());
    }
    let status = Command::new(absolute(ssh_keygen)?)
        .args(["-Y", "sign", "-n", NAMESPACE, "-f"])
        .arg(absolute(key)?)
        .arg(&receipt)
        .env_remove("SSH_AUTH_SOCK")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("receipt signing failed".into());
    }
    absolute(&signature)
}

pub fn verify_signature(
    ssh_keygen: &Path,
    allowed_signers: &Path,
    node: &str,
    receipt: &Path,
) -> Result<(), String> {
    let bytes = read(receipt)?;
    verify_signature_bytes(ssh_keygen, allowed_signers, node, receipt, &bytes)
}

fn verify_signature_bytes(
    ssh_keygen: &Path,
    allowed_signers: &Path,
    node: &str,
    receipt: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    let signature = absolute(&receipt.with_extension("json.sig"))?;
    let mut child = Command::new(absolute(ssh_keygen)?)
        .args(["-Y", "verify", "-n", NAMESPACE, "-f"])
        .arg(absolute(allowed_signers)?)
        .args(["-I", node, "-s"])
        .arg(signature)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .ok_or("signature stdin unavailable")?
        .write_all(bytes)
        .map_err(|e| e.to_string())?;
    if !child.wait().map_err(|e| e.to_string())?.success() {
        return Err("receipt signature invalid or signer revoked".into());
    }
    Ok(())
}

/// Validate the complete receipt and log bytes, not just its claimed verdict.
pub fn validate_receipt(
    path: &Path,
    expected: &verification::Request,
    manifest: &suite::Suite,
) -> Result<verification::Outcome, String> {
    validate_receipt_bytes(path, &read(path)?, expected, manifest)
}

fn validate_receipt_bytes(
    path: &Path,
    bytes: &[u8],
    expected: &verification::Request,
    manifest: &suite::Suite,
) -> Result<verification::Outcome, String> {
    let receipt: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    strict(
        &receipt,
        &[
            "v",
            "request",
            "outcome",
            "execution",
            "environment_digest",
            "steps",
        ],
    )?;
    let is_digest = |value: &Value| {
        value.as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    };
    if !is_digest(&receipt["environment_digest"]) {
        return Err("environment digest invalid".into());
    }
    if receipt["v"] != 1
        || receipt["execution"] != "trusted-local-v1"
        || wire::request_from_value(&receipt["request"])? != *expected
    {
        return Err("receipt request, version or execution profile mismatch".into());
    }
    let outcome = wire::outcome_from_value(&receipt["outcome"], false)?;
    if outcome.request_id != expected.id()
        || outcome.head != expected.head
        || outcome.suite_digest != expected.suite_digest
        || outcome.platform != expected.platform
    {
        return Err("receipt outcome binding mismatch".into());
    }
    let plan = suite::plan(expected, manifest, &expected.platform)?;
    let records = receipt["steps"]
        .as_array()
        .ok_or("step evidence must be a list")?;
    if records.len() > plan.steps.len() {
        return Err("extra step evidence".into());
    }
    let mut results = Vec::new();
    for (record, step) in records.iter().zip(&plan.steps) {
        strict(
            record,
            &[
                "name",
                "argv",
                "cwd",
                "timeout_s",
                "started",
                "exit",
                "timed_out",
                "elapsed_ms",
                "stdout_digest",
                "stderr_digest",
                "tool_digest",
            ],
        )?;
        if record["name"] != step.name
            || record["argv"] != json!(step.argv)
            || record["cwd"] != step.cwd
            || record["timeout_s"] != step.timeout_s
            || record["elapsed_ms"].as_u64().is_none()
        {
            return Err("step evidence differs from pinned plan".into());
        }
        if !is_digest(&record["tool_digest"]) {
            return Err("tool digest invalid".into());
        }
        let directory = path.parent().ok_or("receipt directory missing")?;
        for stream in ["stdout", "stderr"] {
            let bytes =
                read(&directory.join(format!("{}-{}.{}", outcome.attempt, step.name, stream)))?;
            if record[format!("{stream}_digest")] != hash(&bytes) {
                return Err("step log digest mismatch".into());
            }
        }
        let exit = if record["exit"].is_null() {
            None
        } else {
            Some(
                i32::try_from(
                    record["exit"]
                        .as_i64()
                        .ok_or("exit must be integer or null")?,
                )
                .map_err(|_| "exit out of range")?,
            )
        };
        if exit == Some(0)
            && record["elapsed_ms"]
                .as_u64()
                .ok_or("elapsed duration missing")?
                > u64::from(step.timeout_s) * 1000
        {
            return Err("successful step exceeded its deadline".into());
        }
        results.push(suite::StepResult {
            name: step.name.clone(),
            started: record["started"].as_bool().ok_or("started must be bool")?,
            exit,
            timed_out: record["timed_out"]
                .as_bool()
                .ok_or("timed_out must be bool")?,
        });
    }
    if suite::summarize(&plan, &results) != outcome.verdict {
        return Err("claimed verdict differs from step evidence".into());
    }
    Ok(outcome)
}

/// Shadow local verification into checklist box 1, preserving every other review/policy box.
/// No merge side effects. Callers must supply fresh facts for the same immutable head and base.
pub fn shadow_gate(
    facts: &Value,
    requests: &[verification::Request],
    outcomes: &[verification::Outcome],
    authorised: &BTreeMap<String, std::collections::BTreeSet<String>>,
) -> Result<Value, String> {
    let remote = checklist::checklist(facts)?;
    let mut local = facts.clone();
    let mut problems = Vec::new();
    if requests.is_empty() {
        problems.push("no required job cards".to_owned());
    }
    for request in requests {
        if request.problem().is_some()
            || facts["head"] != request.head
            || facts["base_tip"] != request.base_sha
        {
            problems.push("facts do not match card head/base".into());
            continue;
        }
        let matching = outcomes
            .iter()
            .filter(|outcome| outcome.request_id == request.id())
            .cloned()
            .collect::<Vec<_>>();
        let evidence = verification::evidence(
            &request.head,
            &request.suite_digest,
            std::slice::from_ref(&request.platform),
            &matching,
            authorised,
        );
        if !evidence.ok {
            problems.push(format!(
                "{}:{} pending, failed or conflicting",
                request.suite, request.platform
            ));
        }
    }
    local["ci"] = json!({"state": if problems.is_empty() { "success" } else { "failure" }, "detail": if problems.is_empty() { "signed local verification receipts".into() } else { problems.join("; ") }});
    let result = checklist::checklist(&local)?;
    Ok(
        json!({"mode": "shadow", "local_would_integrate": result.would_merge, "github_would_integrate": remote.would_merge,
        "agreement": result.would_merge == remote.would_merge, "verification_problems": problems, "local_checklist": result.lines, "github_checklist": remote.lines}),
    )
}

/// Ingest only the latest issued attempts, authenticated by current allowed-signers and platform registry.
pub fn ingest(
    store: &Store,
    git_path: &Path,
    repo: &Path,
    ssh_keygen: &Path,
    allowed_signers: &Path,
    requests: &[verification::Request],
    authorised: &BTreeMap<String, std::collections::BTreeSet<String>>,
) -> Result<Vec<verification::Outcome>, String> {
    let mut outcomes = Vec::new();
    for request in requests {
        let attempt = store.latest_attempt(request)?;
        if attempt == 0 {
            continue;
        }
        let path = store.receipt_path(request, attempt)?;
        if !path.exists() {
            continue;
        }
        let manifest = pinned_suite(git_path, repo, request)?;
        let bytes = read(&path)?;
        let mut outcome = validate_receipt_bytes(&path, &bytes, request, &manifest)?;
        if outcome.attempt != attempt
            || !authorised
                .get(&outcome.node)
                .is_some_and(|platforms| platforms.contains(&outcome.platform))
        {
            return Err("receipt attempt or node authorization mismatch".into());
        }
        verify_signature_bytes(ssh_keygen, allowed_signers, &outcome.node, &path, &bytes)?;
        if store.latest_attempt(request)? != attempt {
            return Err("attempt changed while validating evidence".into());
        }
        outcome.signature_verified = true;
        outcomes.push(outcome);
    }
    Ok(outcomes)
}

/// Load a card from a bounded file, with no command fields accepted.
pub fn load_card(path: &Path) -> Result<verification::Request, String> {
    wire::request_from_value(&value(path)?)
}

/// Digest a receipt for a separate audit or display without disclosing log content.
pub fn receipt_digest(path: &Path) -> Result<String, String> {
    Ok(chain::digest(
        std::str::from_utf8(&read(path)?).map_err(|e| e.to_string())?,
    ))
}
