//! Local integration facts from base-pinned principals and signed records.
//! Report-only: authenticates evidence and reads local refs, never merges or calls GitHub.
use crate::{chain, local_verification, verification};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const NAMESPACE: &str = "aaroneous-local-authority-v1";
const LIMIT: u64 = 2 * 1024 * 1024;
static TEMP: AtomicU64 = AtomicU64::new(0);

fn absolute(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err("explicit absolute non-symlink input required".into());
    }
    path.canonicalize().map_err(|e| e.to_string())
}
fn bytes(path: &Path) -> Result<Vec<u8>, String> {
    let mut buffer = Vec::new();
    fs::File::open(absolute(path)?)
        .map_err(|e| e.to_string())?
        .take(LIMIT + 1)
        .read_to_end(&mut buffer)
        .map_err(|e| e.to_string())?;
    if buffer.len() as u64 > LIMIT {
        return Err("authority input exceeds limit".into());
    }
    Ok(buffer)
}
fn object(value: &Value, fields: &[&str]) -> Result<(), String> {
    let map = value.as_object().ok_or("authority record must be object")?;
    if map.len() != fields.len() || map.keys().any(|key| !fields.contains(&key.as_str())) {
        return Err("unknown or missing authority fields".into());
    }
    Ok(())
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("{key} must be text"))
}
fn label(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 48
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b))
}
fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn list(value: &Value) -> Result<Vec<String>, String> {
    let list = value.as_array().ok_or("expected label list")?;
    if list.len() > 64 {
        return Err("label list exceeds limit".into());
    }
    let mut seen = BTreeSet::new();
    for entry in list {
        let entry = entry.as_str().filter(|s| label(s)).ok_or("invalid label")?;
        if !seen.insert(entry.to_owned()) {
            return Err("duplicate label".into());
        }
    }
    Ok(seen.into_iter().collect())
}

#[derive(Debug, Clone)]
pub struct Principal {
    pub family: String,
    pub roles: BTreeSet<String>,
    pub platforms: BTreeSet<String>,
    public_key: String,
}
#[derive(Debug)]
pub struct Policy {
    pub digest: String,
    pub integration_ref: String,
    pub suites: Vec<String>,
    pub minimum_reviewers: usize,
    pub minimum_families: usize,
    pub principals: BTreeMap<String, Principal>,
}
impl Policy {
    pub fn parse(source: &str) -> Result<Self, String> {
        let table: toml::Value = source
            .parse()
            .map_err(|e| format!("integration policy: {e}"))?;
        let value = serde_json::to_value(table).map_err(|e| e.to_string())?;
        object(
            &value,
            &[
                "version",
                "integration_ref",
                "required_suites",
                "minimum_reviewers",
                "minimum_families",
                "principals",
            ],
        )?;
        if value["version"] != 1 {
            return Err("unsupported integration policy version".into());
        }
        let integration_ref = text(&value, "integration_ref")?;
        if !integration_ref.starts_with("refs/heads/")
            || integration_ref.chars().any(char::is_control)
        {
            return Err("policy requires a local integration branch ref".into());
        }
        let suites = list(&value["required_suites"])?;
        if suites.is_empty() {
            return Err("at least one required suite is mandatory".into());
        }
        let count = |key: &str| -> Result<usize, String> {
            let n = value[key]
                .as_u64()
                .filter(|n| (2..=16).contains(n))
                .ok_or("review thresholds must be 2..16")?;
            Ok(n as usize)
        };
        let minimum_reviewers = count("minimum_reviewers")?;
        let minimum_families = count("minimum_families")?;
        if minimum_families > minimum_reviewers {
            return Err("family threshold exceeds reviewer threshold".into());
        }
        let mut principals = BTreeMap::new();
        let mut keys = BTreeSet::new();
        for principal in value["principals"]
            .as_array()
            .filter(|a| a.len() <= 64)
            .ok_or("bounded principals list required")?
        {
            object(
                principal,
                &["id", "family", "roles", "platforms", "public_key"],
            )?;
            let id = text(principal, "id")?;
            let family = text(principal, "family")?;
            if !label(id) || !label(family) {
                return Err("principal and family must be labels".into());
            }
            let roles: BTreeSet<_> = list(&principal["roles"])?.into_iter().collect();
            if roles.is_empty()
                || roles.iter().any(|role| {
                    !["issue", "control", "review", "verify", "implement"].contains(&role.as_str())
                })
            {
                return Err("unknown or empty principal role".into());
            }
            let platforms: BTreeSet<_> = list(&principal["platforms"])?.into_iter().collect();
            if roles.contains("verify") && platforms.is_empty() {
                return Err("verification node needs platforms".into());
            }
            if roles.contains("verify") && roles.len() != 1 {
                return Err(
                    "verification key cannot hold review, issue, control or implement authority"
                        .into(),
                );
            }
            let key = text(principal, "public_key")?;
            let parts: Vec<_> = key.split(' ').collect();
            if parts.len() != 2
                || parts[0] != "ssh-ed25519"
                || parts[1].len() < 40
                || parts[1].len() > 128
                || !parts[1]
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b))
            {
                return Err("one bare ed25519 public key required".into());
            }
            if !keys.insert(key.to_owned()) || principals.contains_key(id) {
                return Err("duplicate principal or shared key defeats independence".into());
            }
            principals.insert(
                id.to_owned(),
                Principal {
                    family: family.into(),
                    roles,
                    platforms,
                    public_key: key.into(),
                },
            );
        }
        Ok(Self {
            digest: chain::digest(source),
            integration_ref: integration_ref.into(),
            suites,
            minimum_reviewers,
            minimum_families,
            principals,
        })
    }
    fn principal(&self, id: &str, role: &str) -> Result<&Principal, String> {
        self.principals
            .get(id)
            .filter(|p| p.roles.contains(role))
            .ok_or_else(|| format!("principal lacks {role} authority"))
    }
}

fn git(git: &Path, repo: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let mut command = Command::new(absolute(git)?);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    command
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
        .map_err(|e| e.to_string())
}

// A fresh bare repository shares only immutable objects, not the source's config,
// hooks, working tree or custom merge drivers. Trial merge writes scratch objects.
fn trial_merge(
    input: &Inputs,
    repo: &Path,
    state: &Path,
    base: &str,
    head: &str,
) -> Result<(bool, bool), String> {
    let temporary = state.join(format!(
        ".trial-{}-{}",
        std::process::id(),
        TEMP.fetch_add(1, Ordering::SeqCst)
    ));
    fs::create_dir(&temporary).map_err(|e| e.to_string())?;
    let result = (|| {
        if !git(&input.git, &temporary, &["init", "--bare", "--quiet"])?
            .status
            .success()
        {
            return Err("trial object store initialization failed".into());
        }
        let common = git_read(
            &input.git,
            repo,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?;
        let objects = PathBuf::from(common.trim()).join("objects");
        if !objects.is_absolute() || objects.to_string_lossy().contains(['\n', '\r', '"']) {
            return Err("invalid source object directory".into());
        }
        let directory = objects.to_string_lossy().replace('\\', "/");
        fs::write(
            temporary.join("objects/info/alternates"),
            format!("{directory}\n"),
        )
        .map_err(|e| e.to_string())?;
        let merge = git(
            &input.git,
            &temporary,
            &["merge-tree", "--write-tree", base, head],
        )?;
        let ancestry = git(
            &input.git,
            &temporary,
            &["merge-base", "--is-ancestor", base, head],
        )?;
        let contains_base = match ancestry.status.code() {
            Some(0) => true,
            Some(1) => false,
            _ => return Err("base ancestry unavailable".into()),
        };
        match merge.status.code() {
            Some(0) => Ok((true, contains_base)),
            Some(1) => Ok((false, contains_base)),
            _ => Err("trial merge unavailable".into()),
        }
    })();
    let checked = absolute(&temporary)?;
    if checked.parent() != Some(state) {
        return Err("trial cleanup escapes selected state directory".into());
    }
    fs::remove_dir_all(checked).map_err(|e| e.to_string())?;
    result
}
fn git_read(git_path: &Path, repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = git(git_path, repo, args)?;
    if !out.status.success() || out.stdout.len() as u64 > LIMIT {
        return Err("local git read failed or exceeded limit".into());
    }
    String::from_utf8(out.stdout).map_err(|e| e.to_string())
}
fn reference(git_path: &Path, repo: &Path, name: &str) -> Result<String, String> {
    if !name.starts_with("refs/heads/") && !name.starts_with("refs/remotes/") {
        return Err("full local branch or tracking ref required".into());
    }
    if !git(git_path, repo, &["check-ref-format", name])?
        .status
        .success()
    {
        return Err("malformed ref".into());
    }
    Ok(git_read(
        git_path,
        repo,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{name}^{{commit}}"),
        ],
    )?
    .trim()
    .into())
}

fn signature(
    ssh: &Path,
    state: &Path,
    principal: &str,
    key: &str,
    path: &Path,
    message: &[u8],
) -> Result<(), String> {
    let temporary = state.join(format!(
        ".allowed-{}-{}",
        std::process::id(),
        TEMP.fetch_add(1, Ordering::SeqCst)
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(format!("{principal} {key}\n").as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    let result = (|| {
        let sig = absolute(&path.with_extension("json.sig"))?;
        let mut child = Command::new(absolute(ssh)?)
            .args(["-Y", "verify", "-n", NAMESPACE, "-I", principal, "-f"])
            .arg(&temporary)
            .arg("-s")
            .arg(sig)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let written = child
            .stdin
            .take()
            .ok_or("verification stdin missing")?
            .write_all(message);
        let status = child.wait().map_err(|e| e.to_string())?;
        written.map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("local authority signature invalid or principal revoked".into());
        }
        Ok(())
    })();
    fs::remove_file(temporary).map_err(|e| e.to_string())?;
    result
}

/// Sign only a versioned authority document using an explicitly selected key.
pub fn sign(ssh: &Path, key: &Path, document: &Path) -> Result<PathBuf, String> {
    let source = bytes(document)?;
    let record: Value = serde_json::from_slice(&source).map_err(|e| e.to_string())?;
    if record["v"] != 1
        || !["candidate", "review", "control"].contains(&text(&record, "kind")?)
        || !label(text(&record, "principal")?)
        || !digest(text(&record, "policy_digest")?)
    {
        return Err("not a local authority v1 document".into());
    }
    let document = absolute(document)?;
    let sig = document.with_extension("json.sig");
    if sig.exists() {
        return Err("signature already exists".into());
    }
    let result = Command::new(absolute(ssh)?)
        .args(["-Y", "sign", "-n", NAMESPACE, "-f"])
        .arg(absolute(key)?)
        .arg(&document)
        .env_remove("SSH_AUTH_SOCK")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if !result.success() {
        return Err("authority signing failed".into());
    }
    absolute(&sig)
}

struct Authentication<'a> {
    policy: &'a Policy,
    ssh: &'a Path,
    state: &'a Path,
    repo: &'a str,
}

fn record(
    path: &Path,
    kind: &str,
    fields: &[&str],
    role: &str,
    authentication: &Authentication<'_>,
) -> Result<(Value, String), String> {
    let Authentication {
        policy,
        ssh,
        state,
        repo,
    } = authentication;
    let raw = bytes(path)?;
    let value: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    object(&value, fields)?;
    if value["v"] != 1
        || value["kind"] != kind
        || value["repo"] != *repo
        || value["policy_digest"] != policy.digest
    {
        return Err("authority record version, kind, repo or policy mismatch".into());
    }
    let id = text(&value, "principal")?;
    let principal = policy.principal(id, role)?;
    signature(ssh, state, id, &principal.public_key, path, &raw)?;
    let digest = chain::digest(std::str::from_utf8(&raw).map_err(|e| e.to_string())?);
    Ok((value, digest))
}

fn sequence(value: &Value) -> Result<u64, String> {
    value["sequence"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 1_000_000)
        .ok_or_else(|| "sequence must be 1..1000000".into())
}
fn remember(state: &Path, label: &str, sequence: u64, digest: &str) -> Result<(), String> {
    let prefix = format!("{label}-");
    let mut highest = 0;
    let mut previous = String::new();
    for entry in fs::read_dir(state).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err("invalid state name".into());
        };
        if let Some(number) = name
            .strip_prefix(&prefix)
            .and_then(|s| s.strip_suffix(".json"))
        {
            let n: u64 = number
                .parse()
                .map_err(|_| "corrupt authority high-water filename")?;
            let value: Value =
                serde_json::from_slice(&bytes(&entry.path())?).map_err(|e| e.to_string())?;
            object(&value, &["sequence", "digest"])?;
            if n == 0
                || n > 1_000_000
                || value["sequence"] != n
                || !crate::local_authority::digest(text(&value, "digest")?)
            {
                return Err("corrupt authority high-water record".into());
            }
            if n > highest {
                highest = n;
                previous = text(&value, "digest")?.into();
            }
        }
    }
    if highest > sequence {
        return Err("authority history rolled back below observed sequence".into());
    }
    if highest == sequence && previous != digest {
        return Err("conflicting authority record at observed sequence".into());
    }
    let path = state.join(format!("{label}-{sequence}.json"));
    let raw = serde_json::to_vec(&json!({"sequence":sequence,"digest":digest}))
        .map_err(|e| e.to_string())?;
    if path.exists() {
        if bytes(&path)? != raw {
            return Err("authority state mismatch".into());
        }
    } else {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(&raw)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn check_observed(state: &Path, label: &str, sequence: u64, digest: &str) -> Result<(), String> {
    let path = state.join(format!("{label}-{sequence}.json"));
    if path.exists()
        && bytes(&path)?
            != serde_json::to_vec(&json!({"sequence":sequence,"digest":digest}))
                .map_err(|e| e.to_string())?
    {
        return Err("previously observed authority record changed".into());
    }
    Ok(())
}

pub struct Inputs {
    pub git: PathBuf,
    pub ssh_keygen: PathBuf,
    pub repo: PathBuf,
    pub repo_name: String,
    pub base_ref: String,
    pub head_ref: String,
    pub authority: PathBuf,
    pub state: PathBuf,
    pub spool: PathBuf,
}

const CANDIDATE: &[&str] = &[
    "v",
    "kind",
    "repo",
    "head",
    "base",
    "policy_digest",
    "principal",
    "implementers",
    "revertible",
    "ready",
];
const REVIEW: &[&str] = &[
    "v",
    "kind",
    "repo",
    "head",
    "base",
    "policy_digest",
    "principal",
    "sequence",
    "candidate_digest",
    "verdict",
    "evidence",
];
const CONTROL: &[&str] = &[
    "v",
    "kind",
    "repo",
    "policy_digest",
    "principal",
    "sequence",
    "tripped",
];

/// Derive local gate facts from signed records and local refs, without trusting an input facts file.
pub fn collect(input: &Inputs) -> Result<Value, String> {
    let repo = absolute(&input.repo)?;
    let authority = absolute(&input.authority)?;
    let state = absolute(&input.state)?;
    let spool = absolute(&input.spool)?;
    if [&authority, &state, &spool]
        .iter()
        .any(|path| path.starts_with(&repo))
    {
        return Err("authority, state and spool must be outside candidate checkout".into());
    }
    let head = reference(&input.git, &repo, &input.head_ref)?;
    let base = reference(&input.git, &repo, &input.base_ref)?;
    let source = git_read(
        &input.git,
        &repo,
        &["show", &format!("{base}:ci/integration-policy.toml")],
    )?;
    let policy = Policy::parse(&source)?;
    if input.base_ref != policy.integration_ref {
        return Err("selected base ref differs from reviewed integration policy".into());
    }
    let authentication = Authentication {
        policy: &policy,
        ssh: &input.ssh_keygen,
        state: &state,
        repo: &input.repo_name,
    };
    let (candidate, candidate_digest) = record(
        &authority.join("candidate.json"),
        "candidate",
        CANDIDATE,
        "issue",
        &authentication,
    )?;
    if candidate["head"] != head || candidate["base"] != base {
        return Err("candidate does not bind current local head and base refs".into());
    }
    let implementers = list(&candidate["implementers"])?;
    if implementers.is_empty() {
        return Err("implementers must be attributed by issuer".into());
    }
    let mut implementer_families = BTreeSet::new();
    for id in &implementers {
        implementer_families.insert(policy.principal(id, "implement")?.family.clone());
    }
    let revertible = candidate["revertible"]
        .as_bool()
        .ok_or("issuer must classify revertibility")?;
    let ready = candidate["ready"]
        .as_bool()
        .ok_or("issuer must declare candidate readiness")?;
    let candidate_id = chain::digest(&format!(
        "{}\n{}\n{}\n{}",
        input.repo_name, head, base, policy.digest
    ));
    remember(&state, &format!("i-{candidate_id}"), 1, &candidate_digest)?;
    let review_prefix = format!("r-{candidate_id}-");
    let control_label = format!(
        "c-{}",
        chain::digest(&format!("{}\n{}", input.repo_name, policy.digest))
    );
    let mut reviews: BTreeMap<String, (u64, Value, String)> = BTreeMap::new();
    let mut controls: BTreeMap<u64, (Value, String)> = BTreeMap::new();
    for entry in fs::read_dir(&authority).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name = name.to_str().ok_or("non-UTF8 authority name")?;
        if name == "candidate.json" || !name.ends_with(".json") {
            continue;
        }
        if name.starts_with("review-") {
            let (value, digest) =
                record(&entry.path(), "review", REVIEW, "review", &authentication)?;
            let id = text(&value, "principal")?.to_owned();
            let seq = sequence(&value)?;
            check_observed(&state, &format!("{review_prefix}{id}"), seq, &digest)?;
            if name != format!("review-{id}-{seq}.json")
                || value["head"] != head
                || value["base"] != base
                || value["candidate_digest"] != candidate_digest
            {
                return Err("review filename or revision binding mismatch".into());
            }
            let verdict = text(&value, "verdict")?;
            if !["accept", "reject", "abstain"].contains(&verdict) {
                return Err("unknown review verdict".into());
            }
            let evidence = value["evidence"]
                .as_array()
                .filter(|a| a.len() <= 32)
                .ok_or("bounded review evidence required")?;
            if verdict == "accept" && evidence.is_empty() {
                return Err("accept verdict has no command evidence".into());
            }
            for item in evidence {
                object(item, &["argv", "exit", "log", "log_digest"])?;
                let argv = item["argv"]
                    .as_array()
                    .filter(|a| !a.is_empty() && a.len() <= 64)
                    .ok_or("evidence argv required")?;
                if argv.iter().any(|arg| {
                    arg.as_str()
                        .is_none_or(|s| s.len() > 4096 || s.chars().any(char::is_control))
                }) {
                    return Err("invalid command evidence".into());
                }
                let exit = item["exit"]
                    .as_i64()
                    .ok_or("command exit must be integer")?;
                if verdict == "accept" && exit != 0 {
                    return Err("accept verdict includes a failed command".into());
                }
                let log = text(item, "log")?;
                if !log.starts_with("logs/")
                    || log
                        .split('/')
                        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
                    || log.contains(['\\', ':'])
                {
                    return Err("review evidence path escapes logs".into());
                }
                let path = absolute(&authority.join(log))?;
                if !path.starts_with(&authority) {
                    return Err("review evidence escapes authority directory".into());
                }
                let bytes = bytes(&path)?;
                let actual = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes));
                if item["log_digest"] != actual {
                    return Err("review evidence digest mismatch".into());
                }
            }
            if let Some((old, _, old_digest)) = reviews.get(&id) {
                if *old == seq && *old_digest != digest {
                    return Err("conflicting reviews at same sequence".into());
                }
                if *old > seq {
                    continue;
                }
            }
            reviews.insert(id, (seq, value, digest));
        } else if name.starts_with("control-") {
            let (value, digest) = record(
                &entry.path(),
                "control",
                CONTROL,
                "control",
                &authentication,
            )?;
            let seq = sequence(&value)?;
            check_observed(&state, &control_label, seq, &digest)?;
            if name != format!("control-{seq}.json") || value["tripped"].as_bool().is_none() {
                return Err("control filename or tripped flag invalid".into());
            }
            if controls
                .get(&seq)
                .is_some_and(|(_, previous)| previous != &digest)
            {
                return Err("conflicting control records".into());
            }
            controls.insert(seq, (value, digest));
        } else {
            return Err("unknown authority JSON document".into());
        }
    }
    let (&control_sequence, (control, control_digest)) = controls
        .last_key_value()
        .ok_or("signed circuit-breaker control missing")?;
    remember(&state, &control_label, control_sequence, control_digest)?;
    for entry in fs::read_dir(&state).map_err(|e| e.to_string())? {
        let name = entry.map_err(|e| e.to_string())?.file_name();
        let name = name.to_str().ok_or("non-UTF8 state name")?;
        if let Some(rest) = name
            .strip_prefix(&review_prefix)
            .and_then(|s| s.strip_suffix(".json"))
        {
            let (id, _) = rest.rsplit_once('-').ok_or("corrupt review state name")?;
            if !reviews.contains_key(id) {
                return Err("previously observed reviewer history is missing".into());
            }
        }
    }
    let mut accepts = Vec::new();
    let mut families = BTreeSet::new();
    let mut reject_count = 0;
    for (id, (seq, value, digest)) in &reviews {
        remember(&state, &format!("{review_prefix}{id}"), *seq, digest)?;
        if value["verdict"] == "reject" {
            reject_count += 1;
        }
        let principal = policy.principal(id, "review")?;
        if value["verdict"] == "accept"
            && !implementers.contains(id)
            && !implementer_families.contains(&principal.family)
        {
            accepts.push(id.clone());
            families.insert(principal.family.clone());
        }
    }
    let covered =
        accepts.len() >= policy.minimum_reviewers && families.len() >= policy.minimum_families;
    let authorised = policy
        .principals
        .iter()
        .filter(|(_, p)| p.roles.contains("verify"))
        .map(|(id, p)| (id.clone(), p.platforms.clone()))
        .collect::<BTreeMap<_, _>>();
    let store = local_verification::Store::open(&spool)?;
    let mut requests = Vec::new();
    for suite in &policy.suites {
        requests.extend(local_verification::issue(
            &input.git,
            &repo,
            &input.repo_name,
            &head,
            &base,
            suite,
        )?);
    }
    // Current base policy supplies every verification key; caller-written allowed-signers cannot enroll a node.
    let allowed = state.join(format!(
        ".nodes-{}-{}",
        std::process::id(),
        TEMP.fetch_add(1, Ordering::SeqCst)
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&allowed)
        .map_err(|e| e.to_string())?;
    for (id, principal) in &policy.principals {
        if principal.roles.contains("verify") {
            writeln!(file, "{id} {}", principal.public_key).map_err(|e| e.to_string())?;
        }
    }
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    let outcomes = local_verification::ingest(
        &store,
        &input.git,
        &repo,
        &input.ssh_keygen,
        &allowed,
        &requests,
        &authorised,
    );
    fs::remove_file(allowed).map_err(|e| e.to_string())?;
    let outcomes = outcomes?;
    let (trial_ok, contains_base) = trial_merge(input, &repo, &state, &base, &head)?;
    let facts = json!({"head":head,"base_tip":base,"base_ref":input.base_ref,"default_branch":input.base_ref,"trial_merge_ok":trial_ok,
        "ci":{"state":"missing","detail":"GitHub not consulted"},"covered":covered,"accept_seats":accepts,"counted_verdicts":accepts.len(),"verdicts_missing_evidence":[],
        "standing_changes_requested":reject_count>0,"threads_open":reject_count,"revertible":revertible,"breaker_tripped":control["tripped"],"is_draft":!ready,"category":"local-signed-candidate"});
    let mut shadow = local_verification::shadow_gate(&facts, &requests, &outcomes, &authorised)?;
    if !contains_base {
        shadow["local_would_integrate"] = json!(false);
        shadow["verification_problems"]
            .as_array_mut()
            .ok_or("verification problems missing")?
            .push(json!(
                "candidate does not include the current base; verify a new combined candidate"
            ));
    }
    shadow["local_checklist"]
        .as_array_mut()
        .ok_or("local checklist missing")?
        .insert(
            0,
            json!(if contains_base {
                "[x] 0. Tested candidate includes current trusted base"
            } else {
                "[ ] 0. Tested candidate does not include current trusted base"
            }),
        );
    if reference(&input.git, &repo, &input.base_ref)? != base
        || reference(&input.git, &repo, &input.head_ref)? != head
    {
        return Err("local refs moved during collection".into());
    }
    remember(&state, &control_label, control_sequence, control_digest)?;
    for (id, (seq, _, digest)) in &reviews {
        remember(&state, &format!("{review_prefix}{id}"), *seq, digest)?;
    }
    Ok(
        json!({"mode":"local-report-only","head":head,"base":base,"policy_digest":policy.digest,"candidate_digest":candidate_digest,"candidate_contains_base":contains_base,
        "facts":facts,"local_would_integrate":shadow["local_would_integrate"],"local_checklist":shadow["local_checklist"],"verification_problems":shadow["verification_problems"],
        "required_cards":requests.iter().map(verification::Request::id).collect::<Vec<_>>(),"authenticated_outcomes":outcomes.len()}),
    )
}
