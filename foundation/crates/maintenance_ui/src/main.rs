#![deny(unsafe_code)]

use anyhow::{Context, Result, bail};
use clap::Parser;
use maintenance_ui::{
    WorkerConfig,
    packets::{self, Mode},
};
use serde_json::json;
use std::{
    fs,
    io::Read,
    net::Ipv4Addr,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tiny_http::{Header, Method, Request, Response, Server};

#[derive(Parser)]
#[command(
    name = "devtools",
    about = "Packet-based local maintenance utility; Plan is the default and Build is gated"
)]
struct Args {
    #[arg(long)]
    worker_config: PathBuf,
    #[arg(long)]
    runtime: PathBuf,
    #[arg(long)]
    foundation: PathBuf,
    #[arg(long, default_value_t = 8787)]
    port: u16,
    #[arg(long, alias = "allow-verify")]
    allow_job_build: bool,
    #[arg(long)]
    open: bool,
}
struct Active {
    child: Child,
    kind: &'static str,
    log: PathBuf,
    started_at_secs: u64,
    revision: Option<String>,
    clean: bool,
}
struct App {
    config: WorkerConfig,
    config_path: PathBuf,
    foundation: PathBuf,
    runtime: PathBuf,
    allow_job_build: bool,
    active: Option<Active>,
    last_run: Option<serde_json::Value>,
    mode: Mode,
    running: bool,
    research_job: Option<String>,
    research_requested: bool,
    next_poll: Instant,
}
fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
impl App {
    fn finish(&mut self) -> Result<()> {
        let status = match self.active.as_mut() {
            Some(active) => active.child.try_wait()?,
            None => None,
        };
        if let Some(status) = status {
            let active = self.active.take().context("Active process missing")?;
            let mut output = String::new();
            fs::File::open(&active.log)?
                .take(32768)
                .read_to_string(&mut output)?;
            let current_revision = packets::revision(&self.config).ok();
            let unchanged = active.revision == current_revision
                && active.clean
                && packets::clean(&self.config).unwrap_or(false);

            // Run failures belong to orchestration; they do not change unrelated authoring state.
            self.last_run = Some(
                json!({"kind":active.kind,"started_at_secs":active.started_at_secs,"finished_at_secs":now()?,"exit_code":status.code(),"command_completed":status.success(),"baseline_verified":false,"start_revision":active.revision,"end_revision":current_revision,"source_unchanged":unchanged,"log":active.log,"output_prefix":output,"meaning":"Read-only worker tick finished. It may be quiet, deferred or completed; inspect the retained result. This is not verified code or approval to build."}),
            );
            fs::write(
                self.runtime.join("last-run.json"),
                serde_json::to_vec_pretty(&self.last_run)?,
            )?;
        }
        Ok(())
    }
    fn start(&mut self, kind: &'static str) -> Result<()> {
        self.finish()?;
        if self.active.is_some() {
            bail!("Another operation is active");
        }
        if kind != "research" {
            bail!("Only the published read-only research recipe is executable in this utility");
        }
        let mut count = 0usize;
        let mut bytes = 0u64;
        for entry in fs::read_dir(&self.runtime)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with("run-") {
                count += 1;
                bytes = bytes.saturating_add(entry.metadata()?.len());
            }
        }
        if count >= 512 || bytes > 64 * 1024 * 1024 {
            bail!("Retention requires maintainer review before more operations");
        }
        let log = self
            .runtime
            .join(format!("run-{}-{}.log", now()?, unique_token()?));
        let output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&log)?;
        let error = output.try_clone()?;
        let revision = packets::revision(&self.config).ok();
        let clean = packets::clean(&self.config).unwrap_or(false);
        let mut command = Command::new(&self.foundation);
        command
            .arg("local-worker-tick")
            .arg("--config")
            .arg(&self.config_path);
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::from(output))
            .stderr(Stdio::from(error))
            .spawn()
            .context("Cannot launch prepared operation")?;
        self.active = Some(Active {
            child,
            kind,
            log,
            started_at_secs: now()?,
            revision,
            clean,
        });
        Ok(())
    }
    fn research(&mut self) -> Result<()> {
        if self.active.is_some() {
            bail!("Research is already running");
        }
        let revision = packets::revision(&self.config)?;
        let id = format!("PKT-WIRE-{}", &revision[..12]);
        let path = self.config.jobs_directory.join(format!("{id}.toml"));
        if !path.exists() {
            maintenance_ui::approve_job(&self.config,maintenance_ui::NewJob {id:id.clone(),mode:"analysis".into(),evidence:vec!["crates/wire/src/lib.rs".into()],instruction:"Develop a read-only brainstorming packet for wire boundary hardening. Identify up to three source-grounded observations, assumptions and unresolved questions. Suggest bounded acceptance tests and existing standard approaches. Do not implement anything, claim verification, assume unseen source behavior or approve a plan. rust_draft must be null.".into()})?;
        }
        self.research_job = Some(id);
        self.research_requested = true;
        self.next_poll = Instant::now() + Duration::from_secs(120);
        self.start("research")
    }
    fn research_packet(&mut self) -> Result<serde_json::Value> {
        let Some(id) = &self.research_job else {
            return Ok(
                json!({"title":"Research packet","summary":"Not requested yet","observations":[],"questions":[],"verified":false}),
            );
        };
        for path in maintenance_ui::list_files(&self.config.results_directory, "json")?
            .into_iter()
            .chain(maintenance_ui::list_files(
                &self.config.archive_directory,
                "json",
            )?)
        {
            let record: serde_json::Value =
                serde_json::from_str(&maintenance_ui::read_text(&path, 1048576)?)?;
            if record["job_id"] == *id {
                self.research_requested = false;
                return Ok(
                    json!({"title":"Wire boundary brainstorming","summary":if record["outcome"] == "failed" {"Research failed. A maintainer must inspect the retained evidence; no automatic retry."} else {"Unverified ideas for the blueprint. These cannot authorize commands or change Build criteria."},"observations":record["proposal"]["observations"],"questions":record["proposal"]["risks"],"suggested_tests":record["proposal"]["test_plan"],"error":record["error"],"verified":false}),
                );
            }
        }
        Ok(
            json!({"title":"Research packet","summary":"Queued or running; waits for a safe local dispatch slot","observations":[],"questions":[],"verified":false}),
        )
    }
    fn snapshot(&mut self) -> Result<serde_json::Value> {
        self.finish()?;
        let mut plan = packets::inspect(
            &self.config,
            &self.runtime,
            self.mode,
            self.allow_job_build,
            false,
        )?;
        if self.mode == Mode::Build && self.active.is_none() && !plan.build_ready {
            self.mode = Mode::Plan;
            plan.mode = Mode::Plan;
        }
        Ok(
            json!({"plan":plan,"research":self.research_packet()?,"status":maintenance_ui::status(&self.config,now()?)?,"active":self.active.as_ref().map(|a| json!({"kind":a.kind,"started_at_secs":a.started_at_secs})),"last_run":self.last_run}),
        )
    }
}
fn unique_token() -> Result<String> {
    let mut bytes = [0u8; 24];
    getrandom::getrandom(&mut bytes)
        .map_err(|error| anyhow::anyhow!("OS randomness unavailable: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
fn header<'a>(request: &'a Request, name: &'static str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str())
}
fn authorize(request: &Request, host: &str, origin: &str, token: &str) -> bool {
    header(request, "Host") == Some(host)
        && header(request, "X-Devtools-Token") == Some(token)
        && header(request, "Origin").is_none_or(|value| value == origin)
}
fn body(request: &mut Request) -> Result<String> {
    if header(request, "Content-Type") != Some("application/json")
        || request.body_length().is_none_or(|size| size > 1024)
    {
        bail!("A small JSON packet selection is required");
    }
    let mut body = String::new();
    request.as_reader().take(1025).read_to_string(&mut body)?;
    if body.len() > 1024 {
        bail!("Packet selection too large");
    }
    Ok(body)
}
fn respond(request: Request, status: u16, content_type: &str, body: String) -> Result<()> {
    let headers = [
        ("Content-Type", content_type),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "no-referrer"),
        (
            "Content-Security-Policy",
            "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
        ),
    ];
    let mut response = Response::from_string(body).with_status_code(status);
    for (name, value) in headers {
        response.add_header(
            Header::from_bytes(name, value).map_err(|_| anyhow::anyhow!("Invalid header"))?,
        );
    }
    request.respond(response)?;
    Ok(())
}
fn route(app: &mut App, request: &mut Request) -> Result<String> {
    match (request.method(), request.url()) {
        (&Method::Get, "/api/status") => Ok(serde_json::to_string(&app.snapshot()?)?),
        (&Method::Post, "/api/packet") => {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Selection {
                packet: String,
            }
            let selection: Selection = serde_json::from_str(&body(request)?)?;
            let message = match selection.packet.as_str() {
                "readiness" => {
                    app.snapshot()?;
                    "Readiness inspected. The packet shows which job publication checks are met."
                }
                "research" => {
                    app.research()?;
                    "Read-only brainstorming requested. It cannot change the plan or run implementation."
                }
                "blueprint" => {
                    if app.mode != Mode::Plan {
                        bail!("Prepare a job blueprint in Plan authoring mode");
                    }
                    packets::prepare(&app.config, &app.runtime, now()?)?;
                    "Job blueprint prepared. Inspect its scope and publication criteria before switching authoring to Build."
                }
                "refine-blueprint" => {
                    if app.mode != Mode::Plan {
                        bail!("Return job authoring to Plan before changing a definition");
                    }
                    packets::refine(&app.config, &app.runtime, now()?)?;
                    "Blueprint refined with alternatives, dependencies and falsifiable acceptance tests. Publication still requires Build criteria."
                }
                "build-job" => {
                    let message =
                        packets::publish(&app.config, &app.runtime, app.mode, app.allow_job_build)?;
                    if let Some(blueprint) = packets::load(&app.runtime)? {
                        app.research_job = Some(packets::published_id(&blueprint));
                    }
                    return Ok(json!({"message":message}).to_string());
                }
                _ => bail!(
                    "Unknown packet; arbitrary commands and custom job inputs are not supported"
                ),
            };
            Ok(json!({"message":message}).to_string())
        }
        (&Method::Post, "/api/mode") => {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Selection {
                mode: Mode,
            }
            let selection: Selection = serde_json::from_str(&body(request)?)?;

            app.mode = packets::switch(
                &app.config,
                &app.runtime,
                selection.mode,
                app.allow_job_build,
                false,
            )?;

            Ok(json!({"message":if app.mode==Mode::Build {"Job authoring entered Build after every publication check passed. Existing orchestration continues independently."}else{"Job authoring returned to Plan. Published jobs and their orchestration are unchanged."}}).to_string())
        }
        (&Method::Post, "/api/shutdown") => {
            let _: serde_json::Value = serde_json::from_str(&body(request)?)?;
            if app.active.is_some() {
                bail!("Wait for the active operation before closing");
            }
            app.running = false;
            Ok(json!({"message":"Utility closed. Native OS schedules are unchanged."}).to_string())
        }
        _ => bail!(
            "Unsupported action; direct job, review, command and cloud dispatch endpoints are not exposed"
        ),
    }
}
fn main() -> Result<()> {
    let args = Args::parse();
    if args.port == 0 || !args.foundation.is_absolute() || !args.foundation.is_file() {
        bail!("An explicit existing Foundation binary and nonzero loopback port are required");
    }
    let config_path = maintenance_ui::session_config(&args.worker_config, &args.runtime)?;
    let mut config = maintenance_ui::load_config(&config_path)?;
    config.jobs_directory = args.runtime.join("packet-jobs");
    fs::create_dir_all(&config.jobs_directory)?;
    fs::write(&config_path, toml::to_string(&config)?)?;
    let config = maintenance_ui::load_config(&config_path)?;
    let mut app = App {
        config,
        config_path,
        foundation: args.foundation,
        runtime: args.runtime,
        allow_job_build: args.allow_job_build,
        active: None,
        last_run: None,
        mode: Mode::Plan,
        running: true,
        research_job: None,
        research_requested: false,
        next_poll: Instant::now(),
    };
    if let Ok(text) = maintenance_ui::read_text(&app.runtime.join("last-run.json"), 65536) {
        app.last_run = serde_json::from_str(&text).ok();
    }
    let token = unique_token()?;
    let host = format!("127.0.0.1:{}", args.port);
    let origin = format!("http://{host}");
    let server = Server::http((Ipv4Addr::LOCALHOST, args.port))
        .map_err(|error| anyhow::anyhow!("Cannot start utility: {error}"))?;
    println!(
        "Open {origin}/#{token}\nJob authoring starts in Plan. Build publishes validated jobs; orchestration is independent.\nResearch stays read-only. No direct edits, rollback, commits, pushes or paid dispatch are exposed."
    );
    if args.open {
        let url = format!("{origin}/#{token}");
        #[cfg(windows)]
        let opener = Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(&url)
            .spawn();
        #[cfg(target_os = "macos")]
        let opener = Command::new("open").arg(&url).spawn();
        #[cfg(all(not(windows), not(target_os = "macos")))]
        let opener = Command::new("xdg-open").arg(&url).spawn();
        if let Err(error) = opener {
            eprintln!("Open the printed link manually; browser launch unavailable: {error}");
        }
    }
    while app.running {
        app.finish()?;
        if app.active.is_none()
            && Instant::now() >= app.next_poll
            && !maintenance_ui::list_files(&app.config.jobs_directory, "toml")?.is_empty()
        {
            app.next_poll = Instant::now() + Duration::from_secs(120);
            let _ = app.research_packet()?;
            if let Err(error) = app.start("research") {
                eprintln!("Published work deferred: {error:#}");
            }
        }
        let Some(mut request) = server.recv_timeout(Duration::from_millis(250))? else {
            continue;
        };
        if header(&request, "Host") != Some(host.as_str()) {
            respond(
                request,
                403,
                "application/json",
                json!({"error":"Use the exact startup loopback address"}).to_string(),
            )?;
            continue;
        }
        if request.method() == &Method::Get {
            let public = match request.url() {
                "/" => Some(("text/html; charset=utf-8", include_str!("index.html"))),
                "/app.js" => Some(("application/javascript", include_str!("app.js"))),
                "/style.css" => Some(("text/css", include_str!("style.css"))),
                _ => None,
            };
            if let Some((content_type, content)) = public {
                respond(request, 200, content_type, content.into())?;
                continue;
            }
        }
        if !authorize(&request, &host, &origin, &token) {
            respond(
                request,
                403,
                "application/json",
                json!({"error":"Open the complete startup link for this local session"})
                    .to_string(),
            )?;
            continue;
        }
        let (status, text) = match route(&mut app, &mut request) {
            Ok(text) => (200, text),
            Err(error) => (400, json!({"error":format!("{error:#}")}).to_string()),
        };
        respond(request, status, "application/json", text)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_token_origin_and_exact_host() {
        use tiny_http::TestRequest;
        fn request(host: &str, origin: &str, token: &str) -> Request {
            TestRequest::new()
                .with_header(Header::from_bytes("Host", host).expect("header"))
                .with_header(Header::from_bytes("Origin", origin).expect("header"))
                .with_header(Header::from_bytes("X-Devtools-Token", token).expect("header"))
                .into()
        }
        let host = "127.0.0.1:8787";
        let origin = "http://127.0.0.1:8787";
        assert!(authorize(
            &request(host, origin, "secret"),
            host,
            origin,
            "secret"
        ));
        assert!(!authorize(
            &request(host, "https://attacker.example", "secret"),
            host,
            origin,
            "secret"
        ));
        assert!(!authorize(
            &request("attacker.example", origin, "secret"),
            host,
            origin,
            "secret"
        ));
        assert!(!authorize(
            &request(host, origin, "wrong"),
            host,
            origin,
            "secret"
        ));
    }

    #[test]
    fn removed_direct_endpoints_and_plan_verification_are_rejected() {
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path();
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
        };
        let mut app = App {
            config,
            config_path: root.join("worker.toml"),
            foundation: root.join("foundation"),
            runtime: root.join("runtime"),
            allow_job_build: true,
            active: None,
            last_run: None,
            mode: Mode::Plan,
            running: true,
            research_job: None,
            research_requested: false,
            next_poll: Instant::now(),
        };
        for path in [
            "/api/jobs",
            "/api/review",
            "/api/tick",
            "/api/verify",
            "/api/cloud/send",
            "/api/schedule",
        ] {
            let mut request: Request = tiny_http::TestRequest::new()
                .with_method(Method::Post)
                .with_path(path)
                .into();
            assert!(route(&mut app, &mut request).is_err(), "{path}");
        }
        assert!(app.start("verification").is_err());
        assert!(app.active.is_none());
        assert!(app.mode == Mode::Plan);
    }
}
