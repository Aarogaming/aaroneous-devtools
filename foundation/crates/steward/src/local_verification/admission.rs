//! Opt-in reservation-before-claim boundary. Not an HTTP authorization parser.
use super::*;
use crate::intelligence_broker::{self as broker, Job, ProfileKey, Provider};

/// Trusted caller inputs. The caller authenticates evidence/policy and resolves
/// provider aliases to enrolled principals before invoking this API.
pub struct Admission<'a> {
    /// Existing private directory, separate from the verification spool.
    pub ledger: &'a Path,
    pub operation: &'a str,
    pub limit: u64,
    pub policy_revision: &'a str,
    pub evidence_digest: &'a str,
    pub profile: &'a ProfileKey,
    pub job: &'a Job,
    pub providers: &'a [Provider],
}

fn digest_id(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

// No TTL stealing. A process crash leaves this marker for explicit operator
// recovery after all users of the operation have stopped.
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn ensure(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        if read(path)? != bytes {
            return Err("immutable admission artifact differs".into());
        }
        Ok(())
    } else {
        put(path, bytes)
    }
}

fn validate_record(v: &Value, operation: &str, name: &str) -> Result<u64, String> {
    strict(
        v,
        &[
            "v",
            "operation",
            "request",
            "attempt",
            "node",
            "profile",
            "policy_revision",
            "evidence_digest",
            "cost",
            "deadline",
        ],
    )?;
    let request = wire::request_from_value(&v["request"])?;
    let attempt = v["attempt"].as_u64().ok_or("invalid reservation attempt")?;
    let node = v["node"].as_str().ok_or("invalid reservation node")?;
    strict(&v["profile"], &["provider", "revision"])?;
    if v["v"] != 1
        || v["operation"] != operation
        || !(1..=1000).contains(&attempt)
        || name != format!("{}-{attempt}.json", request.id())
        || node.is_empty()
        || node.len() > 128
        || !node
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.:".contains(&b))
        || ["policy_revision", "evidence_digest"]
            .iter()
            .any(|k| !v[*k].as_str().is_some_and(digest_id))
        || ["provider", "revision"].iter().any(|k| {
            !v["profile"][*k]
                .as_str()
                .is_some_and(|s| !s.is_empty() && s.len() <= 1024)
        })
        || v["deadline"].as_u64().is_none()
    {
        return Err("invalid reservation binding".into());
    }
    v["cost"]
        .as_u64()
        .ok_or_else(|| "invalid reservation cost".into())
}

impl Store {
    /// Reserve an upper-bound cost before making a claim available. All attempts
    /// remain charged; settlement/refunds and HTTP activation are separate work.
    /// Replays require the same binding and freshly eligible broker inputs.
    pub fn claim_admitted(
        &self,
        request: &verification::Request,
        node: &str,
        admission: &Admission<'_>,
    ) -> Result<Claim, String> {
        let a = admission;
        if !digest_id(a.operation)
            || !digest_id(a.policy_revision)
            || !digest_id(a.evidence_digest)
            || request.problem().is_some()
            || a.job.id != request.id()
            || a.job.subject_revision != request.head
        {
            return Err("invalid admission subject or provenance binding".into());
        }
        let selected = a
            .providers
            .iter()
            .find(|p| &p.key == a.profile)
            .ok_or("selected profile missing")?;
        if selected.principal != node {
            return Err("selected provider is not the authenticated worker".into());
        }
        let root = absolute(a.ledger)?;
        if !root.is_dir() || root.starts_with(&self.root) || self.root.starts_with(&root) {
            return Err("separate private admission ledger required".into());
        }
        let operation = root.join(a.operation);
        fs::create_dir_all(&operation).map_err(|e| e.to_string())?;
        if absolute(&operation)?.parent() != Some(absolute(&root)?.as_path()) {
            return Err("operation escapes admission root".into());
        }
        let lock_path = operation.join("transaction.lock");
        put(
            &lock_path,
            b"admission-v1: stop all operation users before crash recovery\n",
        )?;
        let _lock = Lock(lock_path);
        let meta = serde_json::to_vec(&json!({"v":1,"operation":a.operation,"limit":a.limit,"policy_revision":a.policy_revision,"spool":self.root})).map_err(|e| e.to_string())?;
        ensure(&operation.join("operation.json"), &meta)?;
        // Admission consumes an already-issued card. A rejected admission must
        // not leave an empty job directory that poisons service enumeration.
        let directory = absolute(&self.root.join(request.id()))?;
        if directory.parent() != Some(self.root.as_path())
            || wire::request_from_value(&value(&directory.join("request.json"))?)? != *request
        {
            return Err("admission requires the exact issued card".into());
        }
        let claim_path = directory.join("claim.json");
        let latest = self.latest_attempt(request)?;
        let existing_claim = if claim_path.exists() {
            Some(value(&claim_path)?)
        } else {
            None
        };
        let attempt = if let Some(c) = &existing_claim {
            strict(c, &["request_id", "node", "attempt"])?;
            let attempt = c["attempt"].as_u64().ok_or("invalid existing claim")?;
            if c["node"] != node
                || c["request_id"] != request.id()
                || !(1..=1000).contains(&attempt)
                || !(latest == attempt as u32 || latest + 1 == attempt as u32)
            {
                return Err("existing claim binding/history mismatch".into());
            }
            attempt as u32
        } else {
            latest.checked_add(1).ok_or("attempt overflow")?
        };
        if attempt > 1000 {
            return Err("retry limit reached".into());
        }
        let record_name = format!("{}-{attempt}.json", request.id());
        let record_path = operation.join(&record_name);
        let mut total = 0u64;
        let mut previous = None;
        let mut count = 0;
        for entry in fs::read_dir(&operation).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "non-UTF8 admission path")?;
            if ["operation.json", "transaction.lock"].contains(&name.as_str()) {
                continue;
            }
            count += 1;
            if count > 10000 || !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                return Err("invalid or oversized reservation ledger".into());
            }
            let bytes = read(&entry.path())?;
            let v: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if serde_json::to_vec(&v).map_err(|e| e.to_string())? != bytes {
                return Err("noncanonical reservation record".into());
            }
            let cost = validate_record(&v, a.operation, &name)?;
            total = total.checked_add(cost).ok_or("reservation sum overflow")?;
            if name == record_name {
                previous = Some((v, cost));
            }
        }
        if total > a.limit || (previous.is_none() && count >= 10000) {
            return Err("reservation budget or capacity exhausted".into());
        }
        if existing_claim.is_some() && previous.is_none() {
            return Err("existing claim has no matching reservation".into());
        }
        if existing_claim.is_some() && latest == attempt {
            return Err(
                "job already claimed; stop the worker and explicitly recover before retry".into(),
            );
        }
        let mut job = a.job.clone();
        job.remaining_budget = a.limit - total + previous.as_ref().map_or(0, |(_, c)| *c);
        let report = broker::assess(&job, a.providers).map_err(str::to_owned)?;
        let candidate = report
            .ranked
            .iter()
            .find(|c| &c.key == a.profile)
            .ok_or("selected profile is held by current broker policy")?;
        let record = json!({"v":1,"operation":a.operation,"request":wire::request_to_value(request),
            "attempt":attempt,"node":node,"profile":{"provider":a.profile.provider,"revision":a.profile.revision},
            "policy_revision":a.policy_revision,"evidence_digest":a.evidence_digest,
            "cost":candidate.cost_bound,"deadline":job.deadline});
        if previous.as_ref().is_some_and(|(v, _)| *v != record) {
            return Err("reservation replay binding differs".into());
        }
        ensure(
            &record_path,
            &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
        )?;
        // This is the commit order: reservation is synced before any claim. A
        // claim failure retains its charge, and incomplete records fail closed.
        if let Some(c) = existing_claim {
            self.submit(request)?;
            ensure(
                &directory.join(format!("attempt-{attempt}.json")),
                &serde_json::to_vec(&c).map_err(|e| e.to_string())?,
            )?;
            return Ok(Claim {
                request: request.clone(),
                node: node.into(),
                attempt,
                directory,
                lock: claim_path,
            });
        }
        self.submit(request)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&claim_path)
            .map_err(|e| e.to_string())?;
        // Fence the attempt under the existing per-job exclusive claim. Another
        // operation or legacy caller must not advance it after our reservation.
        if self.latest_attempt(request)? + 1 != attempt {
            return Err("attempt advanced after reservation; recovery required".into());
        }
        let claim =
            serde_json::to_vec(&json!({"request_id":request.id(),"node":node,"attempt":attempt}))
                .map_err(|e| e.to_string())?;
        file.write_all(&claim)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        put(&directory.join(format!("attempt-{attempt}.json")), &claim)?;
        Ok(Claim {
            request: request.clone(),
            node: node.into(),
            attempt,
            directory,
            lock: claim_path,
        })
    }
}
