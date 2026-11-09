use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
    },
};
use steward::{
    intelligence_broker::*,
    local_verification::{Store, admission::Admission},
    verification::Request,
};

static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    spool: PathBuf,
    ledger: PathBuf,
    request: Request,
    job: Job,
    providers: Vec<Provider>,
}
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tmp")
            .join(format!(
                "broker-admission-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::SeqCst)
            ));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let spool = root.join("spool");
        let ledger = root.join("ledger");
        fs::create_dir_all(&spool).unwrap();
        fs::create_dir_all(&ledger).unwrap();
        let request = Request {
            repo: "owner/repo".into(),
            head: "a".repeat(40),
            base_sha: "b".repeat(40),
            platform: "linux-x86_64".into(),
            suite: "foundation".into(),
            suite_digest: "c".repeat(64),
        };
        Store::open(&spool).unwrap().submit(&request).unwrap();
        let provider = Provider {
            key: ProfileKey {
                provider: "worker".into(),
                revision: "v1".into(),
            },
            principal: "worker-1".into(),
            kind: ProviderKind::Tool,
            available: true,
            resources_fit: true,
            interface: "verification-v1".into(),
            cost_bound: Some(7),
            duration_bound: Some(1),
            evidence: vec![Evidence {
                capability: "verify".into(),
                suite_revision: "v1".into(),
                reference: "record".into(),
                observed_at: 100,
                passed: 10,
                samples: 10,
            }],
        };
        let job = Job {
            id: request.id(),
            subject_revision: request.head.clone(),
            now: 100,
            deadline: 110,
            max_evidence_age: 10,
            remaining_budget: u64::MAX,
            interface: "verification-v1".into(),
            requirements: vec![Requirement {
                capability: "verify".into(),
                suite_revision: "v1".into(),
                minimum_samples: 10,
                minimum_pass_bps: 9000,
            }],
            authorized: vec![provider.key.clone()],
            excluded_principals: vec![],
        };
        Self {
            root,
            spool,
            ledger,
            request,
            job,
            providers: vec![provider],
        }
    }
    fn admission(&self) -> Admission<'_> {
        Admission {
            ledger: &self.ledger,
            operation: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            limit: 10,
            policy_revision: "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            evidence_digest: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            profile: &self.providers[0].key,
            job: &self.job,
            providers: &self.providers,
        }
    }
    fn operation(&self) -> PathBuf {
        self.ledger.join(self.admission().operation)
    }
    fn claim(&self) -> Result<steward::local_verification::Claim, String> {
        Store::open(&self.spool)?.claim_admitted(&self.request, "worker-1", &self.admission())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn reopen_cannot_duplicate_a_live_claim_or_debit_again() {
    let f = Fixture::new();
    let first = f.claim().unwrap();
    assert_eq!(first.attempt, 1);
    assert!(f.claim().err().unwrap().contains("already claimed"));
    assert_eq!(fs::read_dir(f.operation()).unwrap().count(), 2);
    let mut other = f.job.clone();
    other.id = "wrong".into();
    let mut a = f.admission();
    a.job = &other;
    assert!(
        Store::open(&f.spool)
            .unwrap()
            .claim_admitted(&f.request, "worker-1", &a)
            .is_err()
    );
}

#[test]
fn unissued_card_is_not_created_and_sequential_jobs_share_budget() {
    let f = Fixture::new();
    f.claim().unwrap();
    let mut r = f.request.clone();
    r.head = "2".repeat(40);
    let mut j = f.job.clone();
    j.id = r.id();
    j.subject_revision = r.head.clone();
    let mut a = f.admission();
    a.job = &j;
    let store = Store::open(&f.spool).unwrap();
    assert!(store.claim_admitted(&r, "worker-1", &a).is_err());
    assert!(!f.spool.join(r.id()).exists());
    store.submit(&r).unwrap();
    assert!(store.claim_admitted(&r, "worker-1", &a).is_err());
    assert!(!f.spool.join(r.id()).join("claim.json").exists());
    assert_eq!(fs::read_dir(f.operation()).unwrap().count(), 2);
}

#[test]
fn concurrent_requests_cannot_overspend_operation() {
    let f = Fixture::new();
    let barrier = Arc::new(Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|i| {
                let b = barrier.clone();
                let f = &f;
                scope.spawn(move || {
                    let mut r = f.request.clone();
                    r.head = if i == 0 {
                        "a".repeat(40)
                    } else {
                        "1".repeat(40)
                    };
                    let mut j = f.job.clone();
                    j.id = r.id();
                    j.subject_revision = r.head.clone();
                    let mut a = f.admission();
                    a.job = &j;
                    Store::open(&f.spool).unwrap().submit(&r).unwrap();
                    b.wait();
                    Store::open(&f.spool)
                        .unwrap()
                        .claim_admitted(&r, "worker-1", &a)
                        .is_ok()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|x| **x).count(), 1);
}

#[test]
fn reservation_before_claim_survives_restart_prefix() {
    let mut f = Fixture::new();
    f.claim().unwrap();
    // Persisted prefix after reservation, before claim/attempt publication.
    fs::remove_file(f.spool.join(f.request.id()).join("claim.json")).unwrap();
    fs::remove_file(f.spool.join(f.request.id()).join("attempt-1.json")).unwrap();
    f.providers[0].cost_bound = Some(6);
    assert!(f.claim().err().unwrap().contains("binding differs"));
    f.providers[0].cost_bound = Some(7);
    f.job.deadline = 109;
    assert!(f.claim().err().unwrap().contains("binding differs"));
    f.job.deadline = 110;
    assert_eq!(f.claim().unwrap().attempt, 1);
    assert_eq!(fs::read_dir(f.operation()).unwrap().count(), 2);
}

#[test]
fn explicit_recovery_creates_new_charged_attempt() {
    let f = Fixture::new();
    let store = Store::open(&f.spool).unwrap();
    let mut a = f.admission();
    a.limit = 14;
    let first = store.claim_admitted(&f.request, "worker-1", &a).unwrap();
    assert!(store.claim_admitted(&f.request, "worker-1", &a).is_err());
    drop(first); // The operator has stopped the prior executor before recovery.
    store.recover(&f.request, "worker-1", 1).unwrap();
    let second = store.claim_admitted(&f.request, "worker-1", &a).unwrap();
    assert_eq!(second.attempt, 2);
    drop(second);
    store.recover(&f.request, "worker-1", 2).unwrap();
    assert!(store.claim_admitted(&f.request, "worker-1", &a).is_err());
    assert_eq!(fs::read_dir(f.operation()).unwrap().count(), 3);
}

#[test]
fn partial_attempt_publication_is_repaired_only_for_matching_reservation() {
    let f = Fixture::new();
    f.claim().unwrap();
    fs::remove_file(f.spool.join(f.request.id()).join("attempt-1.json")).unwrap();
    assert_eq!(f.claim().unwrap().attempt, 1);
    assert!(f.spool.join(f.request.id()).join("attempt-1.json").exists());
    assert!(f.claim().err().unwrap().contains("already claimed"));
    let g = Fixture::new();
    Store::open(&g.spool)
        .unwrap()
        .claim(&g.request, "worker-1")
        .unwrap();
    assert!(g.claim().err().unwrap().contains("no matching reservation"));
}

#[test]
fn crash_lock_and_partial_record_hold_without_creating_claim() {
    let f = Fixture::new();
    fs::create_dir_all(f.operation()).unwrap();
    fs::write(f.operation().join("transaction.lock"), b"crashed").unwrap();
    assert!(f.claim().is_err());
    assert!(f.operation().join("transaction.lock").exists());
    fs::remove_file(f.operation().join("transaction.lock")).unwrap();
    fs::write(
        f.operation().join(format!("{}-1.json", f.request.id())),
        b"{",
    )
    .unwrap();
    assert!(f.claim().is_err());
    assert!(!f.spool.join(f.request.id()).join("claim.json").exists());
}

#[test]
fn recovery_does_not_refund_unknown_effects_or_refresh_budget() {
    let f = Fixture::new();
    f.claim().unwrap();
    Store::open(&f.spool)
        .unwrap()
        .recover(&f.request, "worker-1", 1)
        .unwrap();
    assert!(f.claim().is_err());
    assert!(!f.spool.join(f.request.id()).join("claim.json").exists());
    let mut a = f.admission();
    a.limit = 100;
    assert!(
        Store::open(&f.spool)
            .unwrap()
            .claim_admitted(&f.request, "worker-1", &a)
            .is_err()
    );
}

#[test]
fn revocation_expiry_worker_and_binding_changes_fail_closed_on_resume() {
    let mut f = Fixture::new();
    f.claim().unwrap();
    assert!(
        Store::open(&f.spool)
            .unwrap()
            .claim_admitted(&f.request, "other", &f.admission())
            .is_err()
    );
    let mut a = f.admission();
    a.evidence_digest = a.policy_revision;
    assert!(
        Store::open(&f.spool)
            .unwrap()
            .claim_admitted(&f.request, "worker-1", &a)
            .is_err()
    );
    f.job.authorized.clear();
    assert!(f.claim().is_err());
    f.job.authorized.push(f.providers[0].key.clone());
    f.job.now = 111;
    assert!(f.claim().is_err());
    assert!(f.spool.join(f.request.id()).join("claim.json").exists());
}
