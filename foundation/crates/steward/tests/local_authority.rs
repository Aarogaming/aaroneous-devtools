use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use steward::{
    chain,
    local_authority::{self, Inputs, Policy},
    local_verification,
};
static COUNTER: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    input: Inputs,
    keys: BTreeMap<String, PathBuf>,
    head: String,
    base: String,
    digest: String,
}
impl Fixture {
    fn new(family_b: &str, change_head_policy: bool) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../tmp")
            .join(format!(
                "local-authority-test-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            ));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let git =
            PathBuf::from(std::env::var("DEVTOOLS_TEST_GIT").expect("explicit native test git"));
        let ssh = PathBuf::from(
            std::env::var("DEVTOOLS_TEST_SSH_KEYGEN").expect("explicit native test ssh-keygen"),
        );
        let repo = root.join("repo");
        fs::create_dir_all(repo.join("ci/suites")).unwrap();
        for directory in ["authority/logs", "state", "spool", "keys"] {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        let mut keys = BTreeMap::new();
        let mut public = BTreeMap::new();
        for id in [
            "issuer",
            "node",
            "builder",
            "reviewer-a",
            "reviewer-a-2",
            "reviewer-b",
        ] {
            let key = root.join("keys").join(id);
            assert!(
                Command::new(&ssh)
                    .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                    .arg(&key)
                    .status()
                    .unwrap()
                    .success()
            );
            let raw = fs::read_to_string(key.with_extension("pub")).unwrap();
            public.insert(
                id,
                raw.split_whitespace().take(2).collect::<Vec<_>>().join(" "),
            );
            keys.insert(id.into(), key);
        }
        let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
        let mut policy="version=1\nintegration_ref='refs/heads/main'\nrequired_suites=['test-suite']\nminimum_reviewers=2\nminimum_families=2\n".to_owned();
        for (id, family, roles, platforms) in [
            ("issuer", "owner", vec!["issue", "control"], vec![]),
            ("node", "worker", vec!["verify"], vec![platform.as_str()]),
            ("builder", "codex", vec!["implement"], vec![]),
            ("reviewer-a", "claude", vec!["review"], vec![]),
            ("reviewer-a-2", "claude", vec!["review"], vec![]),
            ("reviewer-b", family_b, vec!["review"], vec![]),
        ] {
            policy.push_str(&format!("\n[[principals]]\nid='{id}'\nfamily='{family}'\nroles={}\nplatforms={}\npublic_key='{}'\n",serde_json::to_string(&roles).unwrap(),serde_json::to_string(&platforms).unwrap(),public[id]));
        }
        Policy::parse(&policy).unwrap();
        let digest = chain::digest(&policy);
        fs::write(repo.join("ci/integration-policy.toml"), &policy).unwrap();
        fs::write(
            repo.join("ci/platform-policy.toml"),
            format!("default=['{platform}']\n"),
        )
        .unwrap();
        fs::write(repo.join("ci/suites/test-suite.toml"),format!("name='test-suite'\nplatforms=['{platform}']\n[[step]]\nname='native-probe'\nargv=['git','--version']\ntimeout_s=10\n")).unwrap();
        let command = |args: &[&str]| {
            let out = Command::new(&git)
                .arg("-C")
                .arg(&repo)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env(
                    "GIT_CONFIG_GLOBAL",
                    if cfg!(windows) { "NUL" } else { "/dev/null" },
                )
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8(out.stdout).unwrap()
        };
        command(&["init", "-q", "-b", "main"]);
        fs::write(
            repo.join("README.md"),
            (0..10)
                .map(|line| format!("line-{line}\n"))
                .collect::<String>(),
        )
        .unwrap();
        command(&["add", "."]);
        command(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "base",
        ]);
        let base = command(&["rev-parse", "HEAD"]).trim().to_owned();
        command(&["checkout", "-qb", "candidate"]);
        fs::write(repo.join("README.md"), "candidate\n").unwrap();
        if change_head_policy {
            fs::write(
                repo.join("ci/integration-policy.toml"),
                policy
                    .replace("minimum_reviewers=2", "minimum_reviewers=1")
                    .replace("minimum_families=2", "minimum_families=1"),
            )
            .unwrap();
        }
        command(&["add", "."]);
        command(&[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "candidate",
        ]);
        let head = command(&["rev-parse", "HEAD"]).trim().to_owned();
        let input = Inputs {
            git,
            ssh_keygen: ssh,
            repo,
            repo_name: "fixture/devtools".into(),
            base_ref: "refs/heads/main".into(),
            head_ref: "refs/heads/candidate".into(),
            authority: root.join("authority"),
            state: root.join("state"),
            spool: root.join("spool"),
        };
        let fixture = Self {
            root,
            input,
            keys,
            head,
            base,
            digest,
        };
        fixture.candidate(true);
        fixture.control(1, false);
        fixture.review("reviewer-a", 1, "accept");
        fixture.review("reviewer-b", 1, "accept");
        fixture.verify();
        fixture
    }
    fn write(&self, name: &str, principal: &str, value: Value) {
        let path = self.input.authority.join(name);
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let signature = path.with_extension("json.sig");
        if signature.exists() {
            fs::remove_file(signature).unwrap();
        }
        local_authority::sign(&self.input.ssh_keygen, &self.keys[principal], &path).unwrap();
    }
    fn candidate(&self, ready: bool) {
        self.write("candidate.json","issuer",json!({"v":1,"kind":"candidate","repo":self.input.repo_name,"head":self.head,"base":self.base,"policy_digest":self.digest,"principal":"issuer","implementers":["builder"],"revertible":true,"ready":ready}));
    }
    fn control(&self, sequence: u64, tripped: bool) {
        self.write(&format!("control-{sequence}.json"),"issuer",json!({"v":1,"kind":"control","repo":self.input.repo_name,"policy_digest":self.digest,"principal":"issuer","sequence":sequence,"tripped":tripped}));
    }
    fn review(&self, id: &str, sequence: u64, verdict: &str) {
        let log = format!("logs/{id}-{sequence}.txt");
        let raw = "cargo test: exit 0\n";
        fs::write(self.input.authority.join(&log), raw).unwrap();
        self.write(&format!("review-{id}-{sequence}.json"),id,json!({"v":1,"kind":"review","repo":self.input.repo_name,"head":self.head,"base":self.base,"policy_digest":self.digest,"principal":id,"sequence":sequence,"candidate_digest":chain::digest(&fs::read_to_string(self.input.authority.join("candidate.json")).unwrap()),"verdict":verdict,"evidence":[{"argv":["cargo","test"],"exit":0,"log":log,"log_digest":chain::digest(raw)}]}));
    }
    fn verify(&self) {
        let request = local_verification::issue(
            &self.input.git,
            &self.input.repo,
            &self.input.repo_name,
            &self.head,
            &self.base,
            "test-suite",
        )
        .unwrap()
        .remove(0);
        let store = local_verification::Store::open(&self.input.spool).unwrap();
        let claim = store.claim(&request, "node").unwrap();
        let mut environment = BTreeMap::new();
        if let Ok(value) = std::env::var("SystemRoot") {
            environment.insert("SystemRoot".into(), value);
        }
        let runner = local_verification::Runner {
            git: self.input.git.clone(),
            tools: BTreeMap::from([("git".into(), self.input.git.clone())]),
            environment,
        };
        let receipt = runner.execute(&claim, &self.input.repo).unwrap();
        local_verification::sign(&self.input.ssh_keygen, &self.keys["node"], &receipt).unwrap();
    }
    fn collect(&self) -> Value {
        local_authority::collect(&self.input).unwrap()
    }
    fn remove(&self, name: &str) {
        fs::remove_file(self.input.authority.join(name)).unwrap();
        fs::remove_file(self.input.authority.join(name).with_extension("json.sig")).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn authentic_local_records_allow_report_without_github_or_handwritten_facts() {
    let fixture = Fixture::new("gemini", false);
    let report = fixture.collect();
    assert_eq!(report["local_would_integrate"], true);
    assert_eq!(report["authenticated_outcomes"], 1);
    assert_eq!(report["facts"]["counted_verdicts"], 2);
    assert_eq!(report["facts"]["ci"]["state"], "missing");
    assert_eq!(fixture.collect()["local_would_integrate"], true);
    let cli = Command::new(env!("CARGO_BIN_EXE_local-integration"))
        .arg("collect")
        .args([
            "--git",
            fixture.input.git.to_str().unwrap(),
            "--ssh-keygen",
            fixture.input.ssh_keygen.to_str().unwrap(),
            "--repo",
            fixture.input.repo.to_str().unwrap(),
            "--repo-name",
            &fixture.input.repo_name,
            "--base-ref",
            &fixture.input.base_ref,
            "--head-ref",
            &fixture.input.head_ref,
            "--authority",
            fixture.input.authority.to_str().unwrap(),
            "--state",
            fixture.input.state.to_str().unwrap(),
            "--spool",
            fixture.input.spool.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let output: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(output["local_would_integrate"], true);
}

#[test]
fn standing_rejection_and_control_trip_cannot_be_replayed_away() {
    let fixture = Fixture::new("gemini", false);
    assert_eq!(fixture.collect()["local_would_integrate"], true);
    fixture.review("reviewer-a", 2, "reject");
    assert_eq!(fixture.collect()["local_would_integrate"], false);
    fixture.remove("review-reviewer-a-2.json");
    assert!(
        local_authority::collect(&fixture.input)
            .unwrap_err()
            .contains("rolled back")
    );
    fixture.review("reviewer-a", 3, "accept");
    assert_eq!(fixture.collect()["local_would_integrate"], true);
    fixture.control(2, true);
    assert_eq!(fixture.collect()["local_would_integrate"], false);
    fixture.remove("control-2.json");
    assert!(
        local_authority::collect(&fixture.input)
            .unwrap_err()
            .contains("rolled back")
    );
    fixture.control(3, false);
    assert_eq!(fixture.collect()["local_would_integrate"], true);
    fixture.control(1, true);
    assert!(
        local_authority::collect(&fixture.input)
            .unwrap_err()
            .contains("previously observed authority record changed")
    );
}

#[test]
fn reviewer_removal_tampering_and_failed_evidence_block() {
    let fixture = Fixture::new("gemini", false);
    fixture.collect();
    fixture.remove("review-reviewer-a-1.json");
    assert!(
        local_authority::collect(&fixture.input)
            .unwrap_err()
            .contains("history is missing")
    );
    fixture.review("reviewer-a", 2, "accept");
    fixture.collect();
    fs::write(
        fixture.input.authority.join("logs/reviewer-a-2.txt"),
        "forged\n",
    )
    .unwrap();
    assert!(
        local_authority::collect(&fixture.input)
            .unwrap_err()
            .contains("digest mismatch")
    );
    fs::write(
        fixture.input.authority.join("logs/reviewer-a-2.txt"),
        "cargo test: exit 0\n",
    )
    .unwrap();
    fixture.review("reviewer-a", 3, "accept");
    let path = fixture.input.authority.join("review-reviewer-a-3.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["evidence"][0]["exit"] = json!(1);
    fixture.write("review-reviewer-a-3.json", "reviewer-a", value);
    assert!(
        local_authority::collect(&fixture.input)
            .unwrap_err()
            .contains("failed command")
    );
}

#[test]
fn model_family_independence_and_head_policy_are_not_self_declared() {
    let same = Fixture::new("claude", false);
    assert_eq!(same.collect()["local_would_integrate"], false);
    let implementer_family = Fixture::new("codex", false);
    assert_eq!(implementer_family.collect()["facts"]["counted_verdicts"], 1);
    let candidate_policy = Fixture::new("gemini", true);
    candidate_policy.remove("review-reviewer-b-1.json");
    assert_eq!(candidate_policy.collect()["local_would_integrate"], false);
}

#[test]
fn exact_refs_candidate_readiness_and_signatures_are_required() {
    let fixture = Fixture::new("gemini", false);
    fixture.candidate(false);
    fixture.review("reviewer-a", 1, "accept");
    fixture.review("reviewer-b", 1, "accept");
    assert_eq!(fixture.collect()["local_would_integrate"], false);
    let mut fixture = Fixture::new("gemini", false);
    let path = fixture.input.authority.join("candidate.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["ready"] = json!(false);
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        local_authority::collect(&fixture.input)
            .unwrap_err()
            .contains("signature invalid")
    );
    fixture.candidate(true);
    fixture.input.head_ref = "refs/heads/main".into();
    assert!(
        local_authority::collect(&fixture.input)
            .unwrap_err()
            .contains("current local head")
    );
}

#[test]
fn local_job_receipt_missing_or_unauthenticated_never_passes() {
    let fixture = Fixture::new("gemini", false);
    let request = local_verification::issue(
        &fixture.input.git,
        &fixture.input.repo,
        &fixture.input.repo_name,
        &fixture.head,
        &fixture.base,
        "test-suite",
    )
    .unwrap()
    .remove(0);
    let store = local_verification::Store::open(&fixture.input.spool).unwrap();
    let receipt = store.receipt_path(&request, 1).unwrap();
    let signature = receipt.with_extension("json.sig");
    fs::remove_file(signature).unwrap();
    assert!(local_authority::collect(&fixture.input).is_err());
    fs::remove_file(receipt).unwrap();
    assert_eq!(fixture.collect()["local_would_integrate"], false);
}

#[test]
fn policy_enrollment_is_explicit_and_duplicate_keys_are_refused() {
    let source = "version = 1\nintegration_ref = 'refs/heads/main'\nrequired_suites = ['foundation-ci']\nminimum_reviewers = 2\nminimum_families = 2\nprincipals = []\n".to_owned();
    let policy = Policy::parse(&source).unwrap();
    assert!(policy.principals.is_empty());
    assert!(
        Policy::parse(&source.replace("minimum_reviewers = 2", "minimum_reviewers = 0")).is_err()
    );
    let fixture = Fixture::new("gemini", false);
    let policy = fs::read_to_string(fixture.input.repo.join("ci/integration-policy.toml")).unwrap();
    let a = fs::read_to_string(fixture.keys["reviewer-a"].with_extension("pub"))
        .unwrap()
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    let b = fs::read_to_string(fixture.keys["reviewer-b"].with_extension("pub"))
        .unwrap()
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        Policy::parse(&policy.replace(&b, &a))
            .unwrap_err()
            .contains("shared key")
    );
    assert!(Policy::parse(&policy.replace("[\"verify\"]", "[\"verify\",\"issue\"]")).is_err());
    Policy::parse(
        &fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../ci/integration-policy.toml"),
        )
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn signed_candidate_cannot_be_rewritten_after_its_context_was_observed() {
    let fixture = Fixture::new("gemini", false);
    assert_eq!(fixture.collect()["local_would_integrate"], true);
    fixture.candidate(false);
    assert!(
        local_authority::collect(&fixture.input)
            .unwrap_err()
            .contains("conflicting authority")
    );
}

#[test]
fn real_trial_merge_conflict_blocks_even_when_review_and_tests_pass() {
    let mut fixture = Fixture::new("gemini", false);
    let git = |args: &[&str]| {
        let output = Command::new(&fixture.input.git)
            .arg("-C")
            .arg(&fixture.input.repo)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    git(&["checkout", "-q", "main"]);
    fs::write(
        fixture.input.repo.join("README.md"),
        "different main content\n",
    )
    .unwrap();
    git(&["add", "README.md"]);
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "advance main",
    ]);
    let base = git(&["rev-parse", "HEAD"]).trim().to_owned();
    git(&["checkout", "-q", "candidate"]);
    fixture.base = base;
    fixture.candidate(true);
    fixture.review("reviewer-a", 1, "accept");
    fixture.review("reviewer-b", 1, "accept");
    fixture.verify();
    let report = fixture.collect();
    assert_eq!(report["facts"]["trial_merge_ok"], false);
    assert_eq!(report["local_would_integrate"], false);
    assert_eq!(report["authenticated_outcomes"], 1);
}

#[test]
fn replace_refs_cannot_change_pinned_suite_bytes() {
    use std::io::Write;
    use std::process::Stdio;
    let fixture = Fixture::new("gemini", false);
    let output = Command::new(&fixture.input.git)
        .arg("-C")
        .arg(&fixture.input.repo)
        .args([
            "rev-parse",
            &format!("{}:ci/suites/test-suite.toml", fixture.base),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let original = String::from_utf8(output.stdout).unwrap().trim().to_owned();
    let mut child = Command::new(&fixture.input.git)
        .arg("-C")
        .arg(&fixture.input.repo)
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
        Command::new(&fixture.input.git)
            .arg("-C")
            .arg(&fixture.input.repo)
            .args(["replace", &original, &replacement])
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(fixture.collect()["local_would_integrate"], true);
}

#[test]
fn source_repo_merge_driver_is_not_executed_by_trusted_collector() {
    let mut fixture = Fixture::new("gemini", false);
    let git = |args: &[&str]| {
        let output = Command::new(&fixture.input.git)
            .arg("-C")
            .arg(&fixture.input.repo)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    let candidate = (0..10)
        .map(|line| {
            if line == 2 {
                "candidate-edit\n".to_owned()
            } else {
                format!("line-{line}\n")
            }
        })
        .collect::<String>();
    fs::write(fixture.input.repo.join("README.md"), candidate).unwrap();
    git(&["add", "README.md"]);
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "candidate separate hunk",
    ]);
    let head = git(&["rev-parse", "HEAD"]).trim().to_owned();
    git(&["checkout", "-q", "main"]);
    let main = (0..10)
        .map(|line| {
            if line == 8 {
                "base-edit\n".to_owned()
            } else {
                format!("line-{line}\n")
            }
        })
        .collect::<String>();
    fs::write(fixture.input.repo.join("README.md"), main).unwrap();
    git(&["add", "README.md"]);
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "-qm",
        "main separate hunk",
    ]);
    let base = git(&["rev-parse", "HEAD"]).trim().to_owned();
    git(&["checkout", "-q", "candidate"]);
    git(&["config", "merge.default", "poison"]);
    git(&["config", "merge.poison.driver", "false"]);
    fixture.head = head;
    fixture.base = base;
    fixture.candidate(true);
    fixture.review("reviewer-a", 1, "accept");
    fixture.review("reviewer-b", 1, "accept");
    fixture.verify();
    let report = fixture.collect();
    assert_eq!(report["facts"]["trial_merge_ok"], true);
    assert_eq!(report["candidate_contains_base"], false);
    assert_eq!(report["local_would_integrate"], false);
}

#[test]
fn extending_reviewer_names_preserve_independent_replay_histories() {
    let fixture = Fixture::new("gemini", false);
    fixture.review("reviewer-a-2", 1, "accept");
    assert_eq!(fixture.collect()["local_would_integrate"], true);
    assert_eq!(fixture.collect()["local_would_integrate"], true);
    fixture.review("reviewer-a-2", 2, "reject");
    assert_eq!(fixture.collect()["local_would_integrate"], false);
    fixture.remove("review-reviewer-a-2-2.json");
    assert!(
        local_authority::collect(&fixture.input)
            .unwrap_err()
            .contains("rolled back")
    );
}

#[test]
fn incomplete_staging_is_ignored_but_corrupt_final_state_is_preserved() {
    let fixture = Fixture::new("gemini", false);
    fs::write(fixture.input.state.join(".pending-interrupted"), b"{").unwrap();
    assert_eq!(fixture.collect()["local_would_integrate"], true);
    let record = fs::read_dir(&fixture.input.state)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|ext| ext == "json"))
        .unwrap();
    fs::write(&record, b"").unwrap();
    assert!(local_authority::collect(&fixture.input).is_err());
    assert_eq!(fs::read(&record).unwrap(), b"");
    assert_eq!(
        fs::read(fixture.input.state.join(".pending-interrupted")).unwrap(),
        b"{"
    );
}

#[test]
fn blocked_cli_returns_two_and_retains_json_decision() {
    let fixture = Fixture::new("gemini", false);
    fixture.control(2, true);
    let cli = Command::new(env!("CARGO_BIN_EXE_local-integration"))
        .arg("collect")
        .args([
            "--git",
            fixture.input.git.to_str().unwrap(),
            "--ssh-keygen",
            fixture.input.ssh_keygen.to_str().unwrap(),
            "--repo",
            fixture.input.repo.to_str().unwrap(),
            "--repo-name",
            &fixture.input.repo_name,
            "--base-ref",
            &fixture.input.base_ref,
            "--head-ref",
            &fixture.input.head_ref,
            "--authority",
            fixture.input.authority.to_str().unwrap(),
            "--state",
            fixture.input.state.to_str().unwrap(),
            "--spool",
            fixture.input.spool.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        cli.status.code() == Some(2),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let output: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(output["local_would_integrate"], false);
}
