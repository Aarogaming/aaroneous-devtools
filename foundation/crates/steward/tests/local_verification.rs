use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use steward::{chain, local_verification as local, suite, verification, wire};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_git_bin() -> PathBuf {
    PathBuf::from(
        std::env::var("DEVTOOLS_TEST_GIT")
            .expect("explicit DEVTOOLS_TEST_GIT native executable required"),
    )
}

fn test_ssh_keygen_bin() -> PathBuf {
    PathBuf::from(
        std::env::var("DEVTOOLS_TEST_SSH_KEYGEN")
            .expect("explicit DEVTOOLS_TEST_SSH_KEYGEN native executable required"),
    )
}
struct Fixture {
    root: PathBuf,
    repo: PathBuf,
    spool: PathBuf,
    git: PathBuf,
    request: verification::Request,
    manifest: suite::Suite,
}
impl Fixture {
    fn new(argv: &[&str], timeout: u32) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../tmp")
            .join(format!(
                "local-gate-test-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            ));
        let root = root.canonicalize().unwrap_or_else(|_| {
            fs::create_dir_all(&root).unwrap();
            root.canonicalize().unwrap()
        });
        let repo = root.join("repo");
        let spool = root.join("spool");
        fs::create_dir_all(repo.join("ci/suites")).unwrap();
        fs::create_dir(&spool).unwrap();
        let git = test_git_bin();
        let text = format!(
            "name='test-suite'\nplatforms=['{}-{}']\n[[step]]\nname='probe'\nargv={}\ntimeout_s={}\n",
            std::env::consts::OS,
            std::env::consts::ARCH,
            serde_json::to_string(argv).unwrap(),
            timeout
        );
        let manifest = suite::parse_suite(&text).unwrap();
        fs::write(repo.join("ci/suites/test-suite.toml"), text).unwrap();
        fs::write(
            repo.join("ci/platform-policy.toml"),
            format!(
                "default=['{}-{}']\n",
                std::env::consts::OS,
                std::env::consts::ARCH
            ),
        )
        .unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "ci"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "fixture",
            ],
        ] {
            assert!(
                Command::new(&git)
                    .arg("-C")
                    .arg(&repo)
                    .args(args)
                    .env("GIT_CONFIG_NOSYSTEM", "1")
                    .env(
                        "GIT_CONFIG_GLOBAL",
                        if cfg!(windows) { "NUL" } else { "/dev/null" }
                    )
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let head = String::from_utf8(
            Command::new(&git)
                .arg("-C")
                .arg(&repo)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        let request = verification::Request {
            repo: "test/devtools".into(),
            head: head.clone(),
            base_sha: head,
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            suite: "test-suite".into(),
            suite_digest: manifest.digest.clone(),
        };
        Self {
            root,
            repo,
            spool,
            git,
            request,
            manifest,
        }
    }
    fn run(&self) -> (local::Store, PathBuf) {
        let store = local::Store::open(&self.spool).unwrap();
        let claim = store.claim(&self.request, "test-node").unwrap();
        let mut environment = BTreeMap::new();
        if let Ok(value) = std::env::var("SystemRoot") {
            environment.insert("SystemRoot".into(), value);
        }
        let runner = local::Runner {
            git: self.git.clone(),
            tools: BTreeMap::from([
                ("git".into(), self.git.clone()),
                ("probe".into(), std::env::current_exe().unwrap()),
            ]),
            environment,
        };
        let receipt = runner.execute(&claim, &self.repo).unwrap();
        (store, receipt)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn replacement_objects_cannot_change_pinned_suite() {
    use std::io::Write;
    use std::process::Stdio;
    let fixture = Fixture::new(&["git", "--version"], 5);
    let output = Command::new(&fixture.git)
        .arg("-C")
        .arg(&fixture.repo)
        .args([
            "rev-parse",
            &format!("{}:ci/suites/test-suite.toml", fixture.request.base_sha),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let original = String::from_utf8(output.stdout).unwrap().trim().to_owned();
    let mut child = Command::new(&fixture.git)
        .arg("-C")
        .arg(&fixture.repo)
        .args(["hash-object", "-w", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"not a suite\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let replacement = String::from_utf8(output.stdout).unwrap().trim().to_owned();
    assert!(
        Command::new(&fixture.git)
            .arg("-C")
            .arg(&fixture.repo)
            .args(["replace", &original, &replacement])
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(
        local::pinned_suite(&fixture.git, &fixture.repo, &fixture.request).unwrap(),
        fixture.manifest
    );
    assert_eq!(
        local::issue(
            &fixture.git,
            &fixture.repo,
            "test/devtools",
            &fixture.request.head,
            &fixture.request.base_sha,
            "test-suite"
        )
        .unwrap(),
        vec![fixture.request.clone()]
    );
}

#[test]
fn real_execution_pins_base_and_hashes_logs() {
    let fixture = Fixture::new(&["git", "--version"], 5);
    let requests = local::issue(
        &fixture.git,
        &fixture.repo,
        "test/devtools",
        &fixture.request.head,
        &fixture.request.base_sha,
        "test-suite",
    )
    .unwrap();
    assert_eq!(requests, vec![fixture.request.clone()]);
    let (_, receipt) = fixture.run();
    let outcome = local::validate_receipt(&receipt, &fixture.request, &fixture.manifest).unwrap();
    assert_eq!(outcome.verdict, verification::Verdict::Pass);
    assert!(!outcome.signature_verified);
    let log = receipt.parent().unwrap().join("1-probe.stdout");
    fs::write(&log, "forged").unwrap();
    assert!(local::validate_receipt(&receipt, &fixture.request, &fixture.manifest).is_err());
}

#[test]
fn failed_exit_and_forged_verdict_cannot_pass() {
    let fixture = Fixture::new(&["git", "not-a-real-command"], 5);
    let (_, receipt) = fixture.run();
    let outcome = local::validate_receipt(&receipt, &fixture.request, &fixture.manifest).unwrap();
    assert_eq!(outcome.verdict, verification::Verdict::Fail);
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
    value["outcome"]["verdict"] = json!("pass");
    fs::write(&receipt, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(local::validate_receipt(&receipt, &fixture.request, &fixture.manifest).is_err());
}

#[test]
fn exclusive_claim_restart_retry_and_wrong_recovery() {
    let fixture = Fixture::new(&["git", "--version"], 5);
    let store = local::Store::open(&fixture.spool).unwrap();
    let first = store.claim(&fixture.request, "test-node").unwrap();
    let restarted = local::Store::open(&fixture.spool).unwrap();
    assert!(restarted.claim(&fixture.request, "other-node").is_err());
    assert!(
        restarted
            .recover(&fixture.request, "other-node", first.attempt)
            .is_err()
    );
    let dir = fixture.spool.join(fixture.request.id());
    let abandoned = dir.join(format!("abandoned-{}.json", first.attempt));
    let claim_json = serde_json::json!({
        "request_id": fixture.request.id(),
        "node": "test-node",
        "attempt": first.attempt,
    });
    fs::write(&abandoned, serde_json::to_vec(&claim_json).unwrap()).unwrap();
    restarted
        .recover(&fixture.request, "test-node", first.attempt)
        .unwrap();
    assert!(!dir.join("claim.json").exists());
    let second = restarted.claim(&fixture.request, "other-node").unwrap();
    assert_eq!(second.attempt, 2);
    assert_eq!(restarted.latest_attempt(&fixture.request).unwrap(), 2);
}

#[test]
fn dirty_checkout_wrong_host_and_stale_base_refused() {
    let fixture = Fixture::new(&["git", "--version"], 5);
    let store = local::Store::open(&fixture.spool).unwrap();
    let claim = store.claim(&fixture.request, "test-node").unwrap();
    let runner = local::Runner {
        git: fixture.git.clone(),
        tools: BTreeMap::from([("git".into(), fixture.git.clone())]),
        environment: BTreeMap::new(),
    };
    fs::write(fixture.repo.join("untracked"), "candidate changed").unwrap();
    assert!(runner.execute(&claim, &fixture.repo).is_err());
    let mut wrong = fixture.request.clone();
    wrong.suite_digest = "0".repeat(64);
    assert!(local::pinned_suite(&fixture.git, &fixture.repo, &wrong).is_err());
    wrong = fixture.request.clone();
    wrong.platform = "another-platform".into();
    assert!(local::pinned_suite(&fixture.git, &fixture.repo, &wrong).is_err());
}

fn facts(request: &verification::Request) -> serde_json::Value {
    json!({"head":request.head,"base_tip":request.base_sha,"base_ref":"main","default_branch":"main","trial_merge_ok":true,
        "ci":{"state":"failure","detail":"billing-blocked"},"covered":true,"accept_seats":["reviewer-a","reviewer-b"],"accept_families":["family-a","family-b"],"counted_verdicts":2,"verdicts_missing_evidence":[],
        "standing_changes_requested":false,"category":"mixed","is_draft":false,"breaker_tripped":false,"threads_open":0,"revertible":true})
}

#[test]
fn local_shadow_requires_exact_request_and_other_policy_boxes() {
    let fixture = Fixture::new(&["git", "--version"], 5);
    let (_, receipt) = fixture.run();
    let mut outcome =
        local::validate_receipt(&receipt, &fixture.request, &fixture.manifest).unwrap();
    outcome.signature_verified = true;
    let registry = BTreeMap::from([(
        "test-node".into(),
        BTreeSet::from([fixture.request.platform.clone()]),
    )]);
    let report = local::shadow_gate(
        &facts(&fixture.request),
        std::slice::from_ref(&fixture.request),
        std::slice::from_ref(&outcome),
        &registry,
    )
    .unwrap();
    assert!(
        report["verification_problems"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["github_would_integrate"], false);
    assert_eq!(report["local_would_integrate"], true);
    let mut stale = facts(&fixture.request);
    stale["base_tip"] = json!("0".repeat(40));
    assert_eq!(
        local::shadow_gate(
            &stale,
            std::slice::from_ref(&fixture.request),
            std::slice::from_ref(&outcome),
            &registry
        )
        .unwrap()["local_would_integrate"],
        false
    );
    let mut no_review = facts(&fixture.request);
    no_review["accept_seats"] = json!([]);
    assert_eq!(
        local::shadow_gate(
            &no_review,
            std::slice::from_ref(&fixture.request),
            std::slice::from_ref(&outcome),
            &registry
        )
        .unwrap()["local_would_integrate"],
        false
    );
    outcome.request_id = chain::digest("different request same head");
    assert!(
        !local::shadow_gate(
            &facts(&fixture.request),
            std::slice::from_ref(&fixture.request),
            &[outcome],
            &registry
        )
        .unwrap()["verification_problems"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn actual_ssh_signatures_tamper_revocation_and_attempt_fencing() {
    let fixture = Fixture::new(&["git", "--version"], 5);
    let ssh = test_ssh_keygen_bin();
    let key = fixture.root.join("fixture-key");
    assert!(
        Command::new(&ssh)
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&key)
            .status()
            .unwrap()
            .success()
    );
    let public = fs::read_to_string(key.with_extension("pub")).unwrap();
    let allowed = fixture.root.join("allowed-signers");
    fs::write(&allowed, format!("test-node {public}")).unwrap();
    let (store, receipt) = fixture.run();
    local::sign(&ssh, &key, &receipt).unwrap();
    local::verify_signature(&ssh, &allowed, "test-node", &receipt).unwrap();
    let registry = BTreeMap::from([(
        "test-node".into(),
        BTreeSet::from([fixture.request.platform.clone()]),
    )]);
    let outcomes = local::ingest(
        &store,
        &fixture.git,
        &fixture.repo,
        &ssh,
        &allowed,
        std::slice::from_ref(&fixture.request),
        &registry,
    )
    .unwrap();
    assert!(outcomes[0].signature_verified);
    let second = store.claim(&fixture.request, "test-node").unwrap();
    assert_eq!(second.attempt, 2);
    assert!(
        local::ingest(
            &store,
            &fixture.git,
            &fixture.repo,
            &ssh,
            &allowed,
            std::slice::from_ref(&fixture.request),
            &registry
        )
        .unwrap()
        .is_empty()
    );
    fs::write(&allowed, "").unwrap();
    assert!(local::verify_signature(&ssh, &allowed, "test-node", &receipt).is_err());
    fs::write(&allowed, format!("test-node {public}")).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
    value["outcome"]["node"] = json!("other-node");
    fs::write(&receipt, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(local::verify_signature(&ssh, &allowed, "test-node", &receipt).is_err());
    assert!(wire::outcome_from_value(&value["outcome"], false).is_ok());
}

#[test]
#[ignore = "subprocess fixture, explicitly launched by timeout test"]
fn sleep_helper() {
    std::thread::sleep(std::time::Duration::from_secs(5));
}

#[test]
fn actual_process_timeout_is_a_failure_not_a_pass() {
    let fixture = Fixture::new(&["probe", "--ignored", "--exact", "sleep_helper"], 1);
    let (store, receipt) = fixture.run();
    let outcome = local::validate_receipt(&receipt, &fixture.request, &fixture.manifest).unwrap();
    assert_eq!(outcome.verdict, verification::Verdict::Fail);
    let evidence: serde_json::Value = serde_json::from_slice(&fs::read(receipt).unwrap()).unwrap();
    assert_eq!(evidence["steps"][0]["timed_out"], true);
    assert!(store.claim(&fixture.request, "test-node").is_err());
}

#[test]
fn cli_issues_runs_signs_and_shadows_without_github() {
    let fixture = Fixture::new(&["git", "--version"], 5);
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_local-verification"));
    let call = |args: Vec<String>| -> serde_json::Value {
        let output = Command::new(&binary).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    };
    let common = vec![
        "--git".to_owned(),
        fixture.git.to_str().unwrap().to_owned(),
        "--repo".into(),
        fixture.repo.to_str().unwrap().into(),
        "--repo-name".into(),
        "test/devtools".into(),
        "--head".into(),
        fixture.request.head.clone(),
        "--base".into(),
        fixture.request.base_sha.clone(),
        "--suite".into(),
        "test-suite".into(),
        "--spool".into(),
        fixture.spool.to_str().unwrap().into(),
    ];
    let mut issue_args = vec!["issue".into()];
    issue_args.extend(common.clone());
    let cards = call(issue_args);
    let card = cards["cards"][0]["card"].as_str().unwrap();
    let config_path = fixture.root.join("runner.json");
    let mut environment = BTreeMap::new();
    if let Ok(root) = std::env::var("SystemRoot") {
        environment.insert("SystemRoot", root);
    }
    fs::write(
        &config_path,
        serde_json::to_vec(
            &json!({"git":fixture.git,"tools":{"git":fixture.git},"environment":environment}),
        )
        .unwrap(),
    )
    .unwrap();
    let executed = call(vec![
        "run".into(),
        "--repo".into(),
        fixture.repo.to_str().unwrap().into(),
        "--card".into(),
        card.into(),
        "--spool".into(),
        fixture.spool.to_str().unwrap().into(),
        "--node".into(),
        "test-node".into(),
        "--config".into(),
        config_path.to_str().unwrap().into(),
        "--trust".into(),
        "owner-trusted-local".into(),
    ]);
    let receipt = executed["receipt"].as_str().unwrap();
    let ssh = test_ssh_keygen_bin();
    let key = fixture.root.join("cli-key");
    assert!(
        Command::new(&ssh)
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&key)
            .status()
            .unwrap()
            .success()
    );
    call(vec![
        "sign".into(),
        "--ssh-keygen".into(),
        ssh.to_str().unwrap().into(),
        "--key".into(),
        key.to_str().unwrap().into(),
        "--receipt".into(),
        receipt.into(),
    ]);
    let allowed = fixture.root.join("allowed");
    fs::write(
        &allowed,
        format!(
            "test-node {}",
            fs::read_to_string(key.with_extension("pub")).unwrap()
        ),
    )
    .unwrap();
    let registry = fixture.root.join("registry.json");
    fs::write(
        &registry,
        serde_json::to_vec(&json!({"test-node":[fixture.request.platform]})).unwrap(),
    )
    .unwrap();
    let facts_path = fixture.root.join("facts.json");
    fs::write(
        &facts_path,
        serde_json::to_vec(&facts(&fixture.request)).unwrap(),
    )
    .unwrap();
    let mut shadow_args = vec!["shadow".into()];
    shadow_args.extend(common);
    shadow_args.extend(vec![
        "--ssh-keygen".into(),
        ssh.to_str().unwrap().into(),
        "--allowed-signers".into(),
        allowed.to_str().unwrap().into(),
        "--registry".into(),
        registry.to_str().unwrap().into(),
        "--facts".into(),
        facts_path.to_str().unwrap().into(),
        "--trust".into(),
        "owner-trusted-local".into(),
    ]);
    let shadow = call(shadow_args);
    assert_eq!(shadow["local_would_integrate"], true);
    assert_eq!(shadow["github_would_integrate"], false);
    assert_eq!(shadow["mode"], "shadow");
}
