//! Service and polling client for revision-bound local verification.
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use steward::{job_service as service, local_verification as local, wire};

fn fields(value: &Value, names: &[&str]) -> Result<(), String> {
    let map = value.as_object().ok_or("object required")?;
    if map.len() != names.len() || map.keys().any(|k| !names.contains(&k.as_str())) {
        return Err("unknown or missing config fields".into());
    }
    Ok(())
}
fn path(v: &Value, key: &str) -> Result<PathBuf, String> {
    service::absolute(Path::new(service::text(v, key)?))
}
fn ca(config: &Value) -> Result<Option<PathBuf>, String> {
    if config["ca_file"].is_null() {
        Ok(None)
    } else {
        Ok(Some(path(config, "ca_file")?))
    }
}
fn request(config: &Value, method: &str, route: &str, body: &Value) -> Result<Value, String> {
    let environment = config["client_environment"]
        .as_object()
        .ok_or("client_environment map required")?
        .iter()
        .map(|(key, value)| {
            Ok((
                key.clone(),
                value
                    .as_str()
                    .ok_or("client environment string required")?
                    .to_owned(),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    service::Client {
        curl: &path(config, "curl")?,
        endpoint: service::text(config, "endpoint")?,
        ca: ca(config)?.as_deref(),
        token_file: &path(config, "token_file")?,
        environment: &environment,
        scratch: &path(config, "client_scratch")?,
    }
    .call((method, route), body)
}
fn runner(config: &Value) -> Result<local::Runner, String> {
    fields(config, &["git", "tools", "environment"])?;
    let tools = config["tools"]
        .as_object()
        .ok_or("tools map required")?
        .iter()
        .map(|(k, v)| {
            Ok((
                k.clone(),
                PathBuf::from(v.as_str().ok_or("tool path required")?),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let environment = config["environment"]
        .as_object()
        .ok_or("environment map required")?
        .iter()
        .map(|(k, v)| {
            Ok((
                k.clone(),
                v.as_str().ok_or("environment string required")?.to_owned(),
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    Ok(local::Runner {
        git: path(config, "git")?,
        tools,
        environment,
    })
}
fn docker_execute(
    config: &Value,
    repo: &Path,
    spool: &Path,
    card: &Path,
    node: &str,
) -> Result<(), String> {
    fields(
        config,
        &[
            "mode",
            "docker",
            "docker_config",
            "daemon",
            "image",
            "runner_config",
            "host_git",
            "uid",
            "gid",
            "timeout_s",
        ],
    )?;
    let image = service::text(config, "image")?;
    let pinned = image.rsplit_once("@sha256:").is_some_and(|(_, d)| {
        d.len() == 64
            && d.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    });
    let daemon = service::text(config, "daemon")?;
    if !pinned
        || !daemon.starts_with("unix:///run/user/")
        || !daemon.ends_with("/docker.sock")
        || daemon.contains([',', '\n', '\r'])
    {
        return Err("pinned image and explicit rootless Docker socket required".into());
    }
    let uid = config["uid"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= u32::MAX as u64)
        .ok_or("nonroot uid required")?;
    let gid = config["gid"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= u32::MAX as u64)
        .ok_or("nonroot gid required")?;
    let timeout = config["timeout_s"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 86400)
        .ok_or("bounded timeout required")?;
    let runner = path(config, "runner_config")?;
    let docker_config = path(config, "docker_config")?;
    if !docker_config.is_dir()
        || std::fs::read_dir(&docker_config)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
    {
        return Err("explicit empty Docker client configuration directory required".into());
    }
    // Only this job's spool is mounted. Tokens and signing keys stay in the host controller.
    let job = service::absolute(card.parent().ok_or("card parent required")?)?;
    if job.parent() != Some(spool) {
        return Err("card outside worker spool".into());
    }
    let card_id = job
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("card id required")?;
    let mount = |source: &Path, target: &str, readonly: bool| -> Result<String, String> {
        let source = source.to_str().ok_or("UTF8 mount path required")?;
        if source.contains([',', '\n', '\r']) {
            return Err("invalid mount source".into());
        }
        Ok(format!(
            "type=bind,source={source},target={target}{}",
            if readonly { ",readonly" } else { "" }
        ))
    };
    let name = format!("aaroneous-job-{}-{}", std::process::id(), &card_id[..16]);
    let docker = path(config, "docker")?;
    if !cfg!(target_os = "linux") {
        return Err("Docker execution profile requires a Linux controller".into());
    }
    let info = Command::new(&docker)
        .args([
            "--config",
            docker_config
                .to_str()
                .ok_or("UTF8 Docker config path required")?,
            "--host",
            daemon,
            "info",
            "--format",
            "{{json .SecurityOptions}} {{.CgroupVersion}}",
        ])
        .env_clear()
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    let info_text = String::from_utf8(info.stdout).map_err(|e| e.to_string())?;
    if !info.status.success()
        || !info_text.contains("name=rootless")
        || !info_text.trim().ends_with(" 2")
    {
        return Err("rootless daemon with cgroup v2 required".into());
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if std::fs::metadata(spool).map_err(|e| e.to_string())?.mode() & 0o077 != 0 {
            return Err("worker spool must be private (mode 0700)".into());
        }
        // The private parent remains inaccessible to other host users. The mapped
        // nonroot container uid needs write access only to this bind-mounted job.
        std::fs::set_permissions(&job, std::fs::Permissions::from_mode(0o777))
            .map_err(|e| e.to_string())?;
    }
    let mut child = Command::new(&docker)
        .args([
            "--config",
            docker_config
                .to_str()
                .ok_or("UTF8 Docker config path required")?,
            "--host",
            daemon,
            "run",
            "--rm",
            "--name",
            &name,
            "--pull",
            "never",
            "--network",
            "none",
            "--read-only",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges:true",
            "--pids-limit",
            "256",
            "--cpus",
            "2",
            "--memory",
            "4g",
            "--memory-swap",
            "4g",
            "--user",
            &format!("{uid}:{gid}"),
            "--tmpfs",
            "/tmp:rw,exec,nosuid,nodev,size=256m,mode=1777",
            "--tmpfs",
            "/work:rw,exec,nosuid,nodev,size=2g,mode=1777",
            "--mount",
            &mount(repo, "/candidate", true)?,
            "--mount",
            &mount(&job, &format!("/spool/{card_id}"), false)?,
            "--mount",
            &mount(&runner, "/runner.json", true)?,
            "--entrypoint",
            "/usr/local/bin/local-verification",
            image,
            "run",
            "--repo",
            "/candidate",
            "--card",
            &format!("/spool/{card_id}/request.json"),
            "--spool",
            "/spool",
            "--node",
            node,
            "--config",
            "/runner.json",
            "--trust",
            "owner-trusted-local",
        ])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(timeout);
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err("Docker executor failed; claim retained".into())
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let cleanup = Command::new(&docker)
                .args([
                    "--config",
                    docker_config
                        .to_str()
                        .ok_or("UTF8 Docker config path required")?,
                    "--host",
                    daemon,
                    "rm",
                    "--force",
                    &name,
                ])
                .env_clear()
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|e| e.to_string())?;
            return Err(format!(
                "executor deadline exceeded; container cleanup success={}; inspect before recovery",
                cleanup.success()
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
fn worker(config: &Value) -> Result<Value, String> {
    fields(
        config,
        &[
            "v",
            "node",
            "curl",
            "endpoint",
            "ca_file",
            "token_file",
            "client_environment",
            "client_scratch",
            "spool",
            "checkouts",
            "execution",
            "ssh_keygen",
            "signing_key",
            "trust",
        ],
    )?;
    if config["v"] != 1 || service::text(config, "trust")? != "owner-trusted-local" {
        return Err(
            "explicit owner-trusted-local profile required; public forks unsupported".into(),
        );
    }
    let spool = path(config, "spool")?;
    let key = path(config, "signing_key")?;
    let token = path(config, "token_file")?;
    if key.starts_with(&spool) || token.starts_with(&spool) {
        return Err("credentials must be outside job spool".into());
    }
    let response = request(config, "POST", "/v1/claim", &json!({"v":1}))?;
    if response["v"] != 1 {
        return Err("unsupported service response".into());
    }
    if response["idle"] == true {
        return Ok(response);
    }
    fields(&response, &["v", "request", "attempt", "node", "resumed"])?;
    let r = wire::request_from_value(&response["request"])?;
    let node = service::text(config, "node")?;
    if response["node"] != node
        || r.platform != format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
    {
        return Err("worker identity/platform mismatch".into());
    }
    let attempt = u32::try_from(response["attempt"].as_u64().ok_or("attempt missing")?)
        .map_err(|e| e.to_string())?;
    let checkouts = config["checkouts"]
        .as_object()
        .ok_or("explicit head-to-checkout map required")?;
    let repo = service::absolute(Path::new(
        checkouts
            .get(&r.head)
            .and_then(Value::as_str)
            .ok_or("head has no owner-approved prepared checkout")?,
    ))?;
    if path(config, "client_scratch")?.starts_with(&repo) {
        return Err("client scratch must be outside candidate checkout".into());
    }
    if key.starts_with(&repo) || token.starts_with(&repo) || spool.starts_with(&repo) {
        return Err("credential/spool inside candidate checkout refused".into());
    }
    let store = local::Store::open(&spool)?;
    let card = store.submit(&r)?;
    let receipt = store.receipt_path(&r, attempt)?;
    if !receipt.exists() {
        if store.latest_attempt(&r)? + 1 != attempt {
            return Err(
                "incomplete prior execution; stop worker and explicitly recover both claims".into(),
            );
        }
        match service::text(&config["execution"], "mode")? {
            "native" => {
                fields(&config["execution"], &["mode", "runner"])?;
                let claim = store.claim(&r, node)?;
                if claim.attempt != attempt {
                    return Err("attempt mismatch".into());
                }
                runner(&config["execution"]["runner"])?.execute(&claim, &repo)?;
            }
            "docker" => docker_execute(&config["execution"], &repo, &spool, &card, node)?,
            _ => return Err("unknown execution profile".into()),
        }
    }
    let git = match service::text(&config["execution"], "mode")? {
        "native" => path(&config["execution"]["runner"], "git")?,
        "docker" => path(&config["execution"], "host_git")?,
        _ => return Err("execution mode refused".into()),
    };
    let manifest = local::pinned_suite(&git, &repo, &r)?;
    let outcome = local::validate_receipt(&receipt, &r, &manifest)?;
    if outcome.node != node || outcome.attempt != attempt {
        return Err("receipt identity differs".into());
    }
    let signature = receipt.with_extension("json.sig");
    if !signature.exists() {
        local::sign(&path(config, "ssh_keygen")?, &key, &receipt)?;
    }
    request(config, "POST", "/v1/results", &service::bundle(&receipt)?)
}
fn run() -> Result<Value, String> {
    let mut args = std::env::args().skip(1);
    let command=args.next().ok_or("usage: platform-jobs serve|submit|status|worker-once|worker-loop --config <absolute> [--listen <loopback:port>|--head <sha> --suite <name>|--interval-s <N>]")?;
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        if !key.starts_with("--") || options.contains_key(&key) {
            return Err("duplicate or malformed option".into());
        }
        options.insert(key, args.next().ok_or("missing value")?);
    }
    let allowed = match command.as_str() {
        "serve" => vec!["--config", "--listen"],
        "submit" => vec!["--config", "--head", "--suite"],
        "status" | "worker-once" => vec!["--config"],
        "worker-loop" => vec!["--config", "--interval-s"],
        _ => return Err("unknown command".into()),
    };
    if options.len() != allowed.len() || options.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err("unknown or missing options".into());
    }
    let config = service::read_json(Path::new(&options["--config"]))?;
    match command.as_str() {
        "serve" => {
            service::serve(
                service::Service::from_config(&config)?,
                options["--listen"]
                    .parse()
                    .map_err(|_| "explicit loopback socket required")?,
            )?;
            Ok(json!({"stopped":true}))
        }
        "submit" | "status" => {
            fields(
                &config,
                &[
                    "v",
                    "curl",
                    "endpoint",
                    "ca_file",
                    "token_file",
                    "client_environment",
                    "client_scratch",
                ],
            )?;
            if config["v"] != 1 {
                return Err("unsupported version".into());
            }
            if command == "submit" {
                request(
                    &config,
                    "POST",
                    "/v1/jobs",
                    &json!({"v":1,"head":options["--head"],"suite":options["--suite"]}),
                )
            } else {
                request(&config, "GET", "/v1/jobs", &json!({}))
            }
        }
        "worker-once" => worker(&config),
        "worker-loop" => {
            let interval = options["--interval-s"]
                .parse::<u64>()
                .ok()
                .filter(|n| (5..=3600).contains(n))
                .ok_or("poll interval must be 5-3600 seconds")?;
            let mut failures = 0;
            loop {
                match worker(&config) {
                    Ok(result) => {
                        failures = 0;
                        println!("{result}");
                    }
                    Err(_) => {
                        failures += 1;
                        eprintln!("worker tick failed; claims retained ({failures}/3)");
                        if failures == 3 {
                            return Err(
                                "worker stopped after three failed ticks; inspect retained claims"
                                    .into(),
                            );
                        }
                    }
                }
                std::thread::sleep(Duration::from_secs(interval));
            }
        }
        _ => Err("unknown command".into()),
    }
}
fn main() {
    match run() {
        Ok(v) => println!("{v}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
