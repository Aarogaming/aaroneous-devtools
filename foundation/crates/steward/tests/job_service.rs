use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use steward::{job_service as service, local_verification as local, verification::Request, wire};
static COUNTER: AtomicU64 = AtomicU64::new(0);
const ISSUER: &str = "issuer-012345678901234567890123456789012345";
const WORKER: &str = "worker-012345678901234567890123456789012345";
struct Fixture {
    root: PathBuf,
    config: Value,
    request: Request,
    git: PathBuf,
    ssh: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../tmp")
            .join(format!(
                "service-test-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            ));
        for d in [
            "repo/ci/suites",
            "server",
            "stage",
            "worker",
            "keys",
            "http",
        ] {
            fs::create_dir_all(root.join(d)).unwrap();
        }
        let root = root.canonicalize().unwrap();
        let git = PathBuf::from(std::env::var("DEVTOOLS_TEST_GIT").expect("explicit test git"));
        let ssh = PathBuf::from(
            std::env::var("DEVTOOLS_TEST_SSH_KEYGEN").expect("explicit test ssh-keygen"),
        );
        let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
        let repo = root.join("repo");
        fs::write(repo.join("ci/suites/probe.toml"),format!("name='probe'\nplatforms=['{platform}']\n[[step]]\nname='git-version'\nargv=['git','--version']\ntimeout_s=5\n")).unwrap();
        fs::write(
            repo.join("ci/platform-policy.toml"),
            format!("default=['{platform}']\n"),
        )
        .unwrap();
        for args in [
            vec!["init", "-q"],
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
                    .env(
                        "GIT_CONFIG_GLOBAL",
                        if cfg!(windows) { "NUL" } else { "/dev/null" }
                    )
                    .env("GIT_CONFIG_NOSYSTEM", "1")
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
        let key = root.join("keys/node");
        assert!(
            Command::new(&ssh)
                .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                .arg(&key)
                .status()
                .unwrap()
                .success()
        );
        let public = fs::read_to_string(key.with_extension("pub")).unwrap();
        fs::write(
            root.join("signers"),
            format!(
                "node {}\n",
                public
                    .split_whitespace()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        )
        .unwrap();
        fs::write(root.join("issuer-token"), ISSUER).unwrap();
        fs::write(root.join("worker-token"), WORKER).unwrap();
        let request = local::issue(&git, &repo, "fixture/devtools", &head, &head, "probe")
            .unwrap()
            .remove(0);
        let config = json!({"v":1,"git":git,"repo":repo,"repo_name":"fixture/devtools","base":head,"suites":["probe"],"spool":root.join("server"),"staging":root.join("stage"),"ssh_keygen":ssh,"allowed_signers":root.join("signers"),"principals":[{"id":"issuer","role":"issuer","platforms":[],"token_file":root.join("issuer-token")},{"id":"node","role":"worker","platforms":[platform],"token_file":root.join("worker-token")}]});
        Self {
            root,
            config,
            request,
            git,
            ssh,
        }
    }
    fn service(&self) -> service::Service {
        service::Service::from_config(&self.config).unwrap()
    }
    fn issue_claim(&self) -> Value {
        let service = self.service();
        service
            .handle(
                ISSUER,
                "POST",
                "/v1/jobs",
                &json!({"v":1,"head":self.request.head,"suite":"probe"}),
            )
            .unwrap();
        service
            .handle(WORKER, "POST", "/v1/claim", &json!({"v":1}))
            .unwrap()
    }
    fn result(&self) -> Value {
        self.issue_claim();
        let store = local::Store::open(&self.root.join("worker")).unwrap();
        let claim = store.claim(&self.request, "node").unwrap();
        let runner = local::Runner {
            git: self.git.clone(),
            tools: BTreeMap::from([("git".into(), self.git.clone())]),
            environment: BTreeMap::new(),
        };
        let receipt = runner.execute(&claim, &self.root.join("repo")).unwrap();
        local::sign(&self.ssh, &self.root.join("keys/node"), &receipt).unwrap();
        service::bundle(&receipt).unwrap()
    }
    fn worker_config(&self, endpoint: &str) -> Value {
        json!({"v":1,"node":"node","curl":std::env::var("DEVTOOLS_TEST_CURL").expect("explicit test curl"),"endpoint":endpoint,"ca_file":null,"token_file":self.root.join("worker-token"),"client_environment":client_environment(),"client_scratch":self.root.join("http"),"spool":self.root.join("worker"),"checkouts":{&self.request.head:self.root.join("repo")},"execution":{"mode":"native","runner":{"git":self.git,"tools":{"git":self.git},"environment":{}}},"ssh_keygen":self.ssh,"signing_key":self.root.join("keys/node"),"trust":"owner-trusted-local"})
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn roles_platforms_and_tokens_fail_closed() {
    let f = Fixture::new();
    let s = f.service();
    assert!(s.handle("bad", "GET", "/v1/jobs", &json!({})).is_err());
    assert!(
        s.handle(
            WORKER,
            "POST",
            "/v1/jobs",
            &json!({"v":1,"head":f.request.head,"suite":"probe"})
        )
        .is_err()
    );
    assert!(
        s.handle(ISSUER, "POST", "/v1/claim", &json!({"v":1}))
            .is_err()
    );
    f.issue_claim();
    let mut config = f.config.clone();
    config["principals"][1]["platforms"] = json!(["other-platform"]);
    assert_eq!(
        service::Service::from_config(&config)
            .unwrap()
            .handle(WORKER, "POST", "/v1/claim", &json!({"v":1}))
            .unwrap()["idle"],
        true
    );
    config["principals"][1]["token_file"] = config["principals"][0]["token_file"].clone();
    assert!(service::Service::from_config(&config).is_err());
}
#[test]
fn reconnect_and_restart_resume_same_durable_claim() {
    let f = Fixture::new();
    let first = f.issue_claim();
    let again = f
        .service()
        .handle(WORKER, "POST", "/v1/claim", &json!({"v":1}))
        .unwrap();
    assert_eq!(again["attempt"], 1);
    assert_eq!(again["resumed"], true);
    assert_eq!(again["request"], first["request"]);
    let mut config = f.config.clone();
    fs::write(
        f.root.join("other-token"),
        "other-012345678901234567890123456789012345",
    )
    .unwrap();
    config["principals"].as_array_mut().unwrap().push(json!({"id":"other","role":"worker","platforms":[f.request.platform],"token_file":f.root.join("other-token")}));
    assert_eq!(
        service::Service::from_config(&config)
            .unwrap()
            .handle(
                "other-012345678901234567890123456789012345",
                "POST",
                "/v1/claim",
                &json!({"v":1})
            )
            .unwrap()["idle"],
        true
    );
}
#[test]
fn signed_receipts_import_idempotently_and_feed_existing_gate() {
    let f = Fixture::new();
    let b = f.result();
    let s = f.service();
    assert_eq!(
        s.handle(WORKER, "POST", "/v1/results", &b).unwrap()["verdict"],
        "pass"
    );
    assert_eq!(
        s.handle(WORKER, "POST", "/v1/results", &b).unwrap()["received"],
        true
    );
    assert_eq!(
        s.handle(WORKER, "POST", "/v1/claim", &json!({"v":1}))
            .unwrap()["idle"],
        true
    );
    let receipt = f
        .root
        .join("server")
        .join(f.request.id())
        .join("receipt-1.json");
    local::verify_signature(&f.ssh, &f.root.join("signers"), "node", &receipt).unwrap();
    let suite = local::pinned_suite(&f.git, &f.root.join("repo"), &f.request).unwrap();
    assert_eq!(
        local::validate_receipt(&receipt, &f.request, &suite)
            .unwrap()
            .verdict,
        steward::verification::Verdict::Pass
    );
    fs::write(f.root.join("signers"), "").unwrap();
    assert!(s.handle(WORKER, "POST", "/v1/results", &b).is_err());
}
#[test]
fn hostile_artifacts_and_stale_attempts_never_publish() {
    let f = Fixture::new();
    let b = f.result();
    let s = f.service();
    for mutation in ["path", "log", "signature", "attempt", "node"] {
        let mut bad = b.clone();
        match mutation {
            "path" => {
                bad["files"]["../escape"] = json!("00");
            }
            "log" => {
                bad["files"]["1-git-version.stdout"] = json!("00");
            }
            "signature" => {
                bad["files"]["receipt-1.json.sig"] = json!("00");
            }
            "attempt" => {
                bad["attempt"] = json!(2);
            }
            "node" => {
                let mut r: Value = serde_json::from_slice(
                    &fs::read(
                        f.root
                            .join("worker")
                            .join(f.request.id())
                            .join("receipt-1.json"),
                    )
                    .unwrap(),
                )
                .unwrap();
                r["outcome"]["node"] = json!("other");
                bad["files"]["receipt-1.json"] =
                    json!(service::hex(&serde_json::to_vec(&r).unwrap()));
            }
            _ => unreachable!(),
        }
        assert!(
            s.handle(WORKER, "POST", "/v1/results", &bad).is_err(),
            "{mutation}"
        );
        assert!(
            !f.root
                .join("server")
                .join(f.request.id())
                .join("receipt-1.json")
                .exists()
        );
    }
    assert_eq!(fs::read_dir(f.root.join("stage")).unwrap().count(), 0);
}
#[test]
fn candidate_cannot_change_the_base_suite_or_inject_commands() {
    let f = Fixture::new();
    let s = f.service();
    assert!(
        s.handle(
            ISSUER,
            "POST",
            "/v1/jobs",
            &json!({"v":1,"head":f.request.head,"suite":"probe","argv":["evil"]})
        )
        .is_err()
    );
    assert!(
        s.handle(
            ISSUER,
            "POST",
            "/v1/jobs",
            &json!({"v":1,"head":"refs/heads/main","suite":"probe"})
        )
        .is_err()
    );
    assert!(
        s.handle(
            ISSUER,
            "POST",
            "/v1/jobs",
            &json!({"v":1,"head":f.request.head,"suite":"../probe"})
        )
        .is_err()
    );
}
#[test]
fn incomplete_publication_retries_identical_bytes_without_rerunning() {
    let f = Fixture::new();
    let b = f.result();
    let directory = f.root.join("server").join(f.request.id());
    let worker = f.root.join("worker").join(f.request.id());
    fs::copy(
        worker.join("1-git-version.stdout"),
        directory.join("1-git-version.stdout"),
    )
    .unwrap();
    assert_eq!(
        f.service()
            .handle(WORKER, "POST", "/v1/results", &b)
            .unwrap()["received"],
        true
    );
    let mut changed = b;
    changed["files"]["1-git-version.stdout"] = json!("00");
    assert!(
        f.service()
            .handle(WORKER, "POST", "/v1/results", &changed)
            .is_err()
    );
}
#[test]
fn http_rejects_ambiguous_framing_and_truncated_bodies() {
    for header in [
        "Content-Length: 0\r\ncontent-length: 1\r\n",
        "Transfer-Encoding: chunked\r\n",
        "Content-Length: 33554433\r\n",
        "Content-Length: 8\r\nContent-Type: application/json\r\n",
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let thread = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            service::read_http(&mut s)
        });
        let mut stream = TcpStream::connect(address).unwrap();
        write!(stream,"POST /v1/claim HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {WORKER}\r\n{header}\r\n").unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        assert!(thread.join().unwrap().is_err());
    }
}
struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn real_http_cli_submission_worker_polling_and_signed_collection() {
    let f = Fixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    fs::write(
        f.root.join("service.json"),
        serde_json::to_vec(&f.config).unwrap(),
    )
    .unwrap();
    let mut server = Server(
        Command::new(env!("CARGO_BIN_EXE_platform-jobs"))
            .args(["serve", "--config"])
            .arg(f.root.join("service.json"))
            .args(["--listen", &address.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut ready = false;
    for _ in 0..100 {
        if TcpStream::connect(address).is_ok() {
            ready = true;
            break;
        }
        assert!(server.0.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(ready);
    let endpoint = format!("http://{address}");
    let curl = PathBuf::from(std::env::var("DEVTOOLS_TEST_CURL").unwrap());
    let issued = service::Client {
        curl: &curl,
        endpoint: &endpoint,
        ca: None,
        token_file: &f.root.join("issuer-token"),
        environment: &client_environment(),
        scratch: &f.root.join("http"),
    }
    .call(
        ("POST", "/v1/jobs"),
        &json!({"v":1,"head":f.request.head,"suite":"probe"}),
    )
    .unwrap();
    assert_eq!(
        wire::request_from_value(&issued["cards"][0]).unwrap(),
        f.request
    );
    fs::write(
        f.root.join("worker.json"),
        serde_json::to_vec(&f.worker_config(&endpoint)).unwrap(),
    )
    .unwrap();
    for expected in ["received", "idle"] {
        let output = Command::new(env!("CARGO_BIN_EXE_platform-jobs"))
            .args(["worker-once", "--config"])
            .arg(f.root.join("worker.json"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let r: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(r[expected], true);
    }
    let status = service::Client {
        curl: &curl,
        endpoint: &endpoint,
        ca: None,
        token_file: &f.root.join("issuer-token"),
        environment: &client_environment(),
        scratch: &f.root.join("http"),
    }
    .call(("GET", "/v1/jobs"), &json!({}))
    .unwrap();
    assert_eq!(status["jobs"][0]["received"], true);
}
#[test]
fn endpoints_cannot_downgrade_tls_or_follow_unapproved_origins() {
    let f = Fixture::new();
    let curl = PathBuf::from(std::env::var("DEVTOOLS_TEST_CURL").unwrap());
    for url in [
        "http://aaroneoushome:3000",
        "http://127.0.0.1:30@evil",
        "https://example.invalid/path",
        "https://user:secret@example.invalid",
        "file:///etc/passwd",
    ] {
        assert!(
            service::Client {
                curl: &curl,
                endpoint: url,
                ca: None,
                token_file: &f.root.join("issuer-token"),
                environment: &client_environment(),
                scratch: &f.root.join("http")
            }
            .call(("GET", "/v1/jobs"), &json!({}))
            .is_err()
        );
    }
}

fn client_environment() -> BTreeMap<String, String> {
    if cfg!(windows) {
        BTreeMap::from([(
            "SYSTEMROOT".into(),
            std::env::var("DEVTOOLS_TEST_SYSTEM_ROOT").expect("explicit Windows system directory"),
        )])
    } else {
        BTreeMap::new()
    }
}

#[test]
fn large_json_body_roundtrips_without_expect_or_quoting_loss() {
    let f = Fixture::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body = json!({"v":1,"payload":"0123456789\\\"\n\u{263a}".repeat(100000)});
    let expected = body.clone();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let (method, route, bearer, actual) = service::read_http(&mut stream).unwrap();
        assert_eq!((method.as_str(), route.as_str()), ("POST", "/v1/results"));
        assert_eq!(bearer, WORKER);
        assert_eq!(actual, expected);
        let response = b"{\"v\":1,\"received\":true}";
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",response.len()).unwrap();
        stream.write_all(response).unwrap();
    });
    let curl = PathBuf::from(std::env::var("DEVTOOLS_TEST_CURL").unwrap());
    let result = service::Client {
        curl: &curl,
        endpoint: &format!("http://{address}"),
        ca: None,
        token_file: &f.root.join("worker-token"),
        environment: &client_environment(),
        scratch: &f.root.join("http"),
    }
    .call(("POST", "/v1/results"), &body)
    .unwrap();
    assert_eq!(result["received"], true);
    server.join().unwrap();
}
