//! Authenticated transport around the existing verification spool. No merge authority.
use crate::{local_verification as local, verification::Request, wire};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const BODY_LIMIT: usize = 32 * 1024 * 1024;
const SMALL_BODY_LIMIT: usize = 4 * 1024;
const FILE_LIMIT: usize = 8 * 1024 * 1024;
static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn absolute(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err("explicit absolute non-symlink path required".into());
    }
    path.canonicalize().map_err(|e| e.to_string())
}
pub fn read(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let path = absolute(path)?;
    if !path.is_file() {
        return Err("regular file required".into());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("file exceeds size limit".into());
    }
    Ok(bytes)
}
pub fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&read(path, FILE_LIMIT)?).map_err(|e| e.to_string())
}
fn strict(value: &Value, fields: &[&str]) -> Result<(), String> {
    let map = value.as_object().ok_or("object required")?;
    if map.len() != fields.len() || map.keys().any(|k| !fields.contains(&k.as_str())) {
        return Err("unknown or missing fields".into());
    }
    Ok(())
}
pub fn text<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
    value[name]
        .as_str()
        .ok_or_else(|| format!("{name} must be a string"))
}
fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut hex = String::with_capacity(64);
    for byte in hasher.finalize() {
        let _ = std::fmt::Write::write_fmt(&mut hex, format_args!("{byte:02x}"));
    }
    hex
}
fn immutable(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        return if read(path, FILE_LIMIT)? == bytes {
            Ok(())
        } else {
            Err("immutable artifact differs".into())
        };
    }
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| e.to_string())
}
fn labels(value: &Value) -> Result<BTreeSet<String>, String> {
    value
        .as_array()
        .ok_or("list required")?
        .iter()
        .map(|s| {
            let s = s.as_str().ok_or("label string required")?;
            if s.is_empty()
                || s.len() > 48
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.".contains(&b))
            {
                return Err("invalid label".into());
            }
            Ok(s.to_owned())
        })
        .collect()
}
pub fn token(path: &Path) -> Result<String, String> {
    let bytes = read(path, 256)?;
    let value = std::str::from_utf8(&bytes)
        .map_err(|e| e.to_string())?
        .trim();
    if !(32..=128).contains(&value.len())
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
    {
        return Err("token requires 32-128 URL-safe characters".into());
    }
    Ok(value.into())
}
fn equal_secret(a: &[u8], b: &[u8]) -> bool {
    let mut difference = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        difference |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    difference == 0
}
fn id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

struct Principal {
    id: String,
    role: String,
    platforms: BTreeSet<String>,
    token: String,
}
pub struct Service {
    git: PathBuf,
    repo: PathBuf,
    name: String,
    base: String,
    suites: BTreeSet<String>,
    pub spool: PathBuf,
    staging: PathBuf,
    ssh: PathBuf,
    signers: PathBuf,
    principals: Vec<Principal>,
}
impl Service {
    pub fn from_config(config: &Value) -> Result<Self, String> {
        strict(
            config,
            &[
                "v",
                "git",
                "repo",
                "repo_name",
                "base",
                "suites",
                "spool",
                "staging",
                "ssh_keygen",
                "allowed_signers",
                "principals",
            ],
        )?;
        if config["v"] != 1 {
            return Err("unsupported config version".into());
        }
        let path = |name| absolute(Path::new(text(config, name)?));
        let repo = path("repo")?;
        let spool = path("spool")?;
        let staging = path("staging")?;
        if !spool.is_dir()
            || !staging.is_dir()
            || spool.starts_with(&repo)
            || staging.starts_with(&repo)
            || staging.starts_with(&spool)
            || spool.starts_with(&staging)
        {
            return Err(
                "separate private spool and staging directories outside checkout required".into(),
            );
        }
        let mut principals = Vec::<Principal>::new();
        for p in config["principals"]
            .as_array()
            .ok_or("principals list required")?
        {
            strict(p, &["id", "role", "platforms", "token_file"])?;
            let name = text(p, "id")?.to_owned();
            let probe = Request {
                repo: text(config, "repo_name")?.into(),
                head: text(config, "base")?.into(),
                base_sha: text(config, "base")?.into(),
                platform: name.clone(),
                suite: "probe".into(),
                suite_digest: "0".repeat(64),
            };
            if probe.problem().is_some() {
                return Err("invalid principal or repository/base".into());
            }
            let role = text(p, "role")?.to_owned();
            let platforms = labels(&p["platforms"])?;
            if !["issuer", "worker"].contains(&role.as_str())
                || (role == "worker" && platforms.is_empty())
                || (role == "issuer" && !platforms.is_empty())
            {
                return Err(
                    "principal requires exactly issuer or platform-scoped worker role".into(),
                );
            }
            let secret = token(Path::new(text(p, "token_file")?))?;
            if principals.iter().any(|other| {
                other.id == name || equal_secret(other.token.as_bytes(), secret.as_bytes())
            }) {
                return Err("duplicate principal or token".into());
            }
            principals.push(Principal {
                id: name,
                role,
                platforms,
                token: secret,
            });
        }
        if principals.is_empty() {
            return Err("no principals enrolled".into());
        }
        let suites = labels(&config["suites"])?;
        if suites.is_empty() {
            return Err("no suites enabled".into());
        }
        Ok(Self {
            git: path("git")?,
            repo,
            name: text(config, "repo_name")?.into(),
            base: text(config, "base")?.into(),
            suites,
            spool,
            staging,
            ssh: path("ssh_keygen")?,
            signers: path("allowed_signers")?,
            principals,
        })
    }
    fn cards(&self) -> Result<Vec<Request>, String> {
        let mut requests = Vec::new();
        for entry in fs::read_dir(&self.spool).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            if !id(name.to_str().ok_or("non-UTF8 spool entry")?) {
                return Err("unexpected spool entry".into());
            }
            let directory = absolute(&entry.path())?;
            if directory.parent() != Some(self.spool.as_path()) {
                return Err("spool escape".into());
            }
            let request = local::load_card(&directory.join("request.json"))?;
            if request.id() != name.to_string_lossy() {
                return Err("card directory mismatch".into());
            }
            requests.push(request);
            if requests.len() > 10000 {
                return Err("spool capacity exceeded; operator archival required".into());
            }
        }
        requests.sort_by_key(Request::id);
        Ok(requests)
    }
    fn permitted(&self, request: &Request) -> Result<(), String> {
        if request.repo != self.name
            || request.base_sha != self.base
            || !self.suites.contains(&request.suite)
        {
            return Err("request outside configured repository/base/suites".into());
        }
        if !local::issue(
            &self.git,
            &self.repo,
            &self.name,
            &request.head,
            &self.base,
            &request.suite,
        )?
        .contains(request)
        {
            return Err("request differs from trusted base policy".into());
        }
        Ok(())
    }
    fn principal(&self, bearer: &str) -> Result<&Principal, String> {
        self.principals
            .iter()
            .find(|p| equal_secret(p.token.as_bytes(), bearer.as_bytes()))
            .ok_or_else(|| "authentication refused".into())
    }
    fn request_body_limit(&self, bearer: &str, method: &str, route: &str) -> Result<usize, String> {
        let principal = self.principal(bearer)?;
        match (method, route, principal.role.as_str()) {
            ("GET", "/v1/jobs", "issuer" | "worker") => Ok(0),
            ("POST", "/v1/jobs", "issuer") => Ok(SMALL_BODY_LIMIT),
            ("POST", "/v1/claim", "worker") => Ok(SMALL_BODY_LIMIT),
            ("POST", "/v1/results", "worker") => Ok(BODY_LIMIT),
            _ => Err("route or role refused".into()),
        }
    }
    pub fn handle(
        &self,
        bearer: &str,
        method: &str,
        route: &str,
        body: &Value,
    ) -> Result<Value, String> {
        let principal = self.principal(bearer)?;
        let store = local::Store::open(&self.spool)?;
        match (method, route) {
            ("POST", "/v1/jobs") if principal.role == "issuer" => {
                strict(body, &["v", "head", "suite"])?;
                if body["v"] != 1 || !self.suites.contains(text(body, "suite")?) {
                    return Err("unsupported request version or suite".into());
                }
                if self.cards()?.len() >= 10000 {
                    return Err("spool capacity reached".into());
                }
                let cards = local::issue(
                    &self.git,
                    &self.repo,
                    &self.name,
                    text(body, "head")?,
                    &self.base,
                    text(body, "suite")?,
                )?;
                for card in &cards {
                    store.submit(card)?;
                }
                Ok(
                    json!({"v":1,"cards":cards.iter().map(wire::request_to_value).collect::<Vec<_>>()}),
                )
            }
            ("GET", "/v1/jobs") => {
                let mut jobs = Vec::new();
                for r in self.cards()? {
                    if principal.role == "worker" && !principal.platforms.contains(&r.platform) {
                        continue;
                    }
                    let directory = self.spool.join(r.id());
                    let attempt = store.latest_attempt(&r)?;
                    jobs.push(json!({"request":wire::request_to_value(&r),"attempt":attempt,"claimed":directory.join("claim.json").exists(),"received":directory.join(format!("received-{attempt}.json")).exists()}));
                }
                Ok(json!({"v":1,"jobs":jobs,"authority":"signed-local-receipts-shadow-only"}))
            }
            ("POST", "/v1/claim") if principal.role == "worker" => {
                strict(body, &["v"])?;
                if body["v"] != 1 {
                    return Err("unsupported version".into());
                }
                for r in self.cards()? {
                    if !principal.platforms.contains(&r.platform) {
                        continue;
                    }
                    let directory = self.spool.join(r.id());
                    let attempt = store.latest_attempt(&r)?;
                    if directory.join(format!("received-{attempt}.json")).exists() {
                        continue;
                    }
                    self.permitted(&r)?;
                    let lock = directory.join("claim.json");
                    if lock.exists() {
                        let claim = read_json(&lock)?;
                        if claim
                            != json!({"request_id":r.id(),"node":principal.id,"attempt":attempt})
                        {
                            continue;
                        }
                        return Ok(
                            json!({"v":1,"request":wire::request_to_value(&r),"attempt":attempt,"node":principal.id,"resumed":true}),
                        );
                    }
                    let claim = store.claim(&r, &principal.id)?;
                    return Ok(
                        json!({"v":1,"request":wire::request_to_value(&r),"attempt":claim.attempt,"node":principal.id,"resumed":false}),
                    );
                }
                Ok(json!({"v":1,"idle":true}))
            }
            ("POST", "/v1/results") if principal.role == "worker" => self.receive(principal, body),
            _ => Err("route or role refused".into()),
        }
    }
    fn receive(&self, p: &Principal, body: &Value) -> Result<Value, String> {
        strict(body, &["v", "request_id", "attempt", "files"])?;
        let request_id = text(body, "request_id")?;
        if body["v"] != 1 || !id(request_id) {
            return Err("invalid bundle identity".into());
        }
        let directory = self.spool.join(request_id);
        if absolute(&directory)?.parent() != Some(self.spool.as_path()) {
            return Err("spool escape".into());
        }
        let request = local::load_card(&directory.join("request.json"))?;
        self.permitted(&request)?;
        if !p.platforms.contains(&request.platform) {
            return Err("node platform refused".into());
        }
        let attempt = u32::try_from(body["attempt"].as_u64().ok_or("attempt required")?)
            .map_err(|e| e.to_string())?;
        let store = local::Store::open(&self.spool)?;
        if attempt == 0 || store.latest_attempt(&request)? != attempt {
            return Err("stale or unknown attempt".into());
        }
        let completion = directory.join(format!("received-{attempt}.json"));
        let claim = json!({"request_id":request_id,"node":p.id,"attempt":attempt});
        let bundle_digest = digest(&serde_json::to_vec(body).map_err(|e| e.to_string())?);
        let completed = json!({"v":1,"claim":claim,"bundle_digest":bundle_digest});
        if completion.exists() {
            if read_json(&completion)? != completed {
                return Err("completed result differs".into());
            }
        } else if read_json(&directory.join("claim.json"))? != claim {
            return Err("claim owner mismatch".into());
        }
        let manifest = local::pinned_suite(&self.git, &self.repo, &request)?;
        let stage = Stage::new(&self.staging)?;
        let files = body["files"].as_object().ok_or("files object required")?;
        let receipt_name = format!("receipt-{attempt}.json");
        let receipt = stage.0.join(&receipt_name);
        let bytes = unhex(
            files
                .get(&receipt_name)
                .and_then(Value::as_str)
                .ok_or("receipt missing")?,
        )?;
        let parsed: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let mut required = BTreeSet::from([receipt_name.clone(), format!("{receipt_name}.sig")]);
        for step in parsed["steps"].as_array().ok_or("receipt steps required")? {
            let name = text(step, "name")?;
            if !manifest.steps.iter().any(|s| s.name == name) {
                return Err("unknown step".into());
            }
            for stream in ["stdout", "stderr"] {
                required.insert(format!("{attempt}-{name}.{stream}"));
            }
        }
        if required != files.keys().cloned().collect() {
            return Err("unexpected or missing artifact names".into());
        }
        for (name, value) in files {
            let bytes = unhex(value.as_str().ok_or("hex artifact required")?)?;
            immutable(&stage.0.join(name), &bytes)?;
        }
        let outcome = local::validate_receipt(&receipt, &request, &manifest)?;
        if outcome.node != p.id || outcome.attempt != attempt {
            return Err("receipt node/attempt mismatch".into());
        }
        local::verify_signature(&self.ssh, &self.signers, &p.id, &receipt)?;
        // Publish logs and signature before the receipt. Partial publication retains the claim.
        for name in required.iter().filter(|s| **s != receipt_name) {
            immutable(
                &directory.join(name),
                &read(&stage.0.join(name), FILE_LIMIT)?,
            )?;
        }
        immutable(&directory.join(&receipt_name), &bytes)?;
        immutable(
            &completion,
            &serde_json::to_vec(&completed).map_err(|e| e.to_string())?,
        )?;
        let timeout = parsed["steps"]
            .as_array()
            .ok_or("steps required")?
            .iter()
            .any(|s| s["timed_out"] == true);
        let lock = directory.join("claim.json");
        if !timeout && lock.exists() {
            if read_json(&lock)? != claim {
                return Err("claim changed during import".into());
            }
            fs::remove_file(lock).map_err(|e| e.to_string())?;
        }
        Ok(
            json!({"v":1,"request_id":request_id,"attempt":attempt,"received":true,"verdict":wire::outcome_to_value(&outcome)["verdict"],"recovery_required":timeout}),
        )
    }
}
struct Stage(PathBuf);
impl Stage {
    fn new(root: &Path) -> Result<Self, String> {
        let path = root.join(format!(
            "upload-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).map_err(|e| e.to_string())?;
        Ok(Self(path))
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(text: &str) -> Result<Vec<u8>, String> {
    if text.len() > FILE_LIMIT * 2
        || text.len() % 2 != 0
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("invalid or oversized hex artifact".into());
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|b| {
            u8::from_str_radix(std::str::from_utf8(b).map_err(|e| e.to_string())?, 16)
                .map_err(|e| e.to_string())
        })
        .collect()
}
pub fn bundle(receipt: &Path) -> Result<Value, String> {
    let bytes = read(receipt, FILE_LIMIT)?;
    let r: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let outcome = wire::outcome_from_value(&r["outcome"], false)?;
    let directory = receipt.parent().ok_or("receipt parent required")?;
    let name = format!("receipt-{}.json", outcome.attempt);
    if receipt.file_name().and_then(|s| s.to_str()) != Some(&name) {
        return Err("receipt filename mismatch".into());
    }
    let mut files = BTreeMap::from([
        (name.clone(), hex(&bytes)),
        (
            format!("{name}.sig"),
            hex(&read(&receipt.with_extension("json.sig"), FILE_LIMIT)?),
        ),
    ]);
    for step in r["steps"].as_array().ok_or("steps required")? {
        let name = text(step, "name")?;
        if labels(&json!([name]))?.len() != 1 {
            return Err("invalid step label".into());
        }
        for stream in ["stdout", "stderr"] {
            let name = format!("{}-{name}.{stream}", outcome.attempt);
            files.insert(name.clone(), hex(&read(&directory.join(name), FILE_LIMIT)?));
        }
    }
    let b = json!({"v":1,"request_id":outcome.request_id,"attempt":outcome.attempt,"files":files});
    if serde_json::to_vec(&b).map_err(|e| e.to_string())?.len() > BODY_LIMIT {
        return Err("bundle exceeds transport limit".into());
    }
    Ok(b)
}

/// Loopback only. Network-facing TLS and connection limits belong to the reverse proxy.
pub fn serve(service: Service, address: SocketAddr) -> Result<(), String> {
    if !address.ip().is_loopback() {
        return Err("backend must bind loopback; use the HTTPS proxy".into());
    }
    let listener = TcpListener::bind(address).map_err(|e| e.to_string())?;
    eprintln!(
        "platform job backend listening on {}",
        listener.local_addr().map_err(|e| e.to_string())?
    );
    for stream in listener.incoming() {
        let mut stream = stream.map_err(|e| e.to_string())?;
        let answer = read_http(&mut stream, &service).and_then(|(method, route, bearer, body)| {
            service.handle(&bearer, &method, &route, &body)
        });
        let (status, body) = match answer {
            Ok(body) => ("200 OK", body),
            Err(_) => (
                "400 Bad Request",
                json!({"v":1,"error":"request refused; inspect configuration and retained job state"}),
            ),
        };
        let bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        );
        let _ = stream.write_all(&bytes);
    }
    Ok(())
}
type HttpInput = (String, String, String, Value);
pub fn read_http(stream: &mut TcpStream, service: &Service) -> Result<HttpInput, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= 8192 || Instant::now() > deadline {
            return Err("header deadline/size exceeded".into());
        }
        let mut b = [0];
        stream.read_exact(&mut b).map_err(|e| e.to_string())?;
        header.push(b[0]);
    }
    let header = std::str::from_utf8(&header).map_err(|e| e.to_string())?;
    let mut lines = header.split("\r\n");
    let parts = lines
        .next()
        .ok_or("request line required")?
        .split(' ')
        .collect::<Vec<_>>();
    if parts.len() != 3 || parts[2] != "HTTP/1.1" || !["GET", "POST"].contains(&parts[0]) {
        return Err("invalid request line".into());
    }
    let mut headers = BTreeMap::new();
    for line in lines.filter(|s| !s.is_empty()) {
        let (name, value) = line.split_once(':').ok_or("invalid header")?;
        if name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || headers
                .insert(name.to_ascii_lowercase(), value.trim())
                .is_some()
        {
            return Err("invalid or duplicate header".into());
        }
    }
    if headers.contains_key("transfer-encoding")
        || headers.contains_key("expect")
        || !headers.contains_key("host")
    {
        return Err("unsupported HTTP framing".into());
    }
    let bearer = headers
        .get("authorization")
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or("authorization required")?;
    if bearer.len() > 128 {
        return Err("invalid bearer".into());
    }
    let length = headers
        .get("content-length")
        .unwrap_or(&"0")
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    let route_limit = service.request_body_limit(bearer, parts[0], parts[1])?;
    if length > route_limit
        || (parts[0] == "GET" && length != 0)
        || (parts[0] == "POST" && headers.get("content-type") != Some(&"application/json"))
    {
        return Err("invalid body".into());
    }
    let mut bytes = vec![0; length];
    let mut offset = 0;
    while offset < length {
        if Instant::now() > deadline {
            return Err("body deadline exceeded".into());
        }
        let n = stream
            .read(&mut bytes[offset..])
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("truncated body".into());
        }
        offset += n;
    }
    let body = if bytes.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?
    };
    Ok((parts[0].into(), parts[1].into(), bearer.into(), body))
}

/// Explicit native curl supplies TLS, custom CA and connection timeouts. No shell, config, proxy or redirect discovery.
pub struct Client<'a> {
    pub curl: &'a Path,
    pub endpoint: &'a str,
    pub ca: Option<&'a Path>,
    pub token_file: &'a Path,
    pub environment: &'a BTreeMap<String, String>,
    pub scratch: &'a Path,
}
impl Client<'_> {
    pub fn call(&self, request: (&str, &str), body: &Value) -> Result<Value, String> {
        let curl = self.curl;
        let endpoint = self.endpoint;
        let ca = self.ca;
        let token_file = self.token_file;
        let environment = self.environment;
        let (method, route) = request;
        if environment
            .keys()
            .any(|k| !["SYSTEMROOT", "WINDIR", "SYSTEMDRIVE"].contains(&k.as_str()))
        {
            return Err(
                "HTTP client environment permits only explicit Windows system variables".into(),
            );
        }
        let url = endpoint.strip_suffix('/').unwrap_or(endpoint);
        let loopback = ["http://127.0.0.1:", "http://[::1]:"].iter().any(|p| {
            url.strip_prefix(p)
                .is_some_and(|s| s.parse::<u16>().is_ok())
        });
        let https = url.strip_prefix("https://").is_some_and(|s| {
            !s.is_empty()
                && !s.contains(['/', '@', '?', '#', '\\'])
                && !s.chars().any(char::is_whitespace)
        });
        if !loopback && !https {
            return Err("HTTPS origin or explicit loopback HTTP endpoint required".into());
        }
        if !["/v1/jobs", "/v1/claim", "/v1/results"].contains(&route)
            || !["GET", "POST"].contains(&method)
        {
            return Err("invalid client route/method".into());
        }
        let secret = token(token_file)?;
        let bytes = if method == "GET" {
            Vec::new()
        } else {
            serde_json::to_vec(body).map_err(|e| e.to_string())?
        };
        if bytes.len() > BODY_LIMIT {
            return Err("body exceeds transport limit".into());
        }
        let stage = Stage::new(&absolute(self.scratch)?)?;
        let body_path = stage.0.join("body.json");
        if method == "POST" {
            immutable(&body_path, &bytes)?;
        }
        let mut command = Command::new(absolute(curl)?);
        command.args([
            "--disable",
            "--silent",
            "--show-error",
            "--fail",
            "--noproxy",
            "*",
            "--connect-timeout",
            "5",
            "--max-time",
            "30",
            "--max-filesize",
            "33554432",
            "--proto",
            "=http,https",
            "--request",
            method,
        ]);
        if let Some(ca) = ca {
            command.arg("--cacert").arg(absolute(ca)?);
        }
        if method == "POST" {
            command.arg("--data-binary").arg(format!(
                "@{}",
                body_path.to_str().ok_or("UTF8 scratch path required")?
            ));
        }
        // Bearer travels via stdin config, never argv or inherited environment.
        command
            .args(["--config", "-", "--url", &format!("{url}{route}")])
            .env_clear()
            .envs(environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(|e| e.to_string())?;
        let config = format!(
            "header = \"Authorization: Bearer {secret}\"\nheader = \"Content-Type: application/json\"\nheader = \"Expect:\"\n"
        );
        child
            .stdin
            .take()
            .ok_or("curl input missing")?
            .write_all(config.as_bytes())
            .map_err(|e| e.to_string())?;
        let mut output = Vec::new();
        child
            .stdout
            .take()
            .ok_or("curl output missing")?
            .take(BODY_LIMIT as u64 + 1)
            .read_to_end(&mut output)
            .map_err(|e| e.to_string())?;
        if output.len() > BODY_LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            return Err("response too large".into());
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!(
                "service connection or request failed (curl exit {:?})",
                status.code()
            ));
        }
        serde_json::from_slice(&output).map_err(|e| e.to_string())
    }
}
