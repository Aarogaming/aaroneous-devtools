use crate::{WorkerConfig, read_text};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    Plan,
    Build,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Depth {
    #[default]
    Brief,
    Thorough,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Blueprint {
    #[serde(default)]
    pub depth: Depth,
    pub schema_version: u32,
    pub purpose: String,
    pub checkout: PathBuf,
    pub revision: String,
    pub source_digest: String,
    pub evidence: Vec<String>,
    pub scope: Vec<String>,
    pub steps: Vec<Step>,
    pub success_criteria: Vec<String>,
    pub rollback: String,
    pub unresolved_questions: Vec<String>,
    pub created_at_secs: u64,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub title: String,
    pub action: String,
    pub depends_on: Vec<usize>,
    pub expected_evidence: String,
}
#[derive(Serialize)]
pub struct Criterion {
    pub label: String,
    pub satisfied: bool,
    pub detail: String,
}
#[derive(Serialize)]
pub struct Packet {
    pub title: String,
    pub summary: String,
    pub facts: Vec<String>,
    pub next_steps: Vec<String>,
}
#[derive(Serialize)]
pub struct PlanStatus {
    pub mode: Mode,
    pub build_ready: bool,
    pub criteria: Vec<Criterion>,
    pub blueprint: Option<Blueprint>,
    pub packet: Packet,
}

pub fn revision(config: &WorkerConfig) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&config.checkout)
        .output()?;
    let value = String::from_utf8(output.stdout)?.trim().to_owned();
    if !output.status.success() || value.len() < 40 || !value.bytes().all(|b| b.is_ascii_hexdigit())
    {
        bail!("Repository revision unavailable");
    }
    Ok(value)
}
pub fn clean(config: &WorkerConfig) -> Result<bool> {
    let output = Command::new("git")
        .args([
            "status",
            "--porcelain",
            "--untracked-files=normal",
            "--",
            "AGENTS.md",
            "crates/wire/src/lib.rs",
        ])
        .current_dir(&config.checkout)
        .output()?;
    if !output.status.success() {
        bail!("Repository state unavailable");
    }
    Ok(output.stdout.is_empty())
}
fn lowercase_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn source_digest(config: &WorkerConfig, evidence: &[String]) -> Result<String> {
    let mut hasher = Sha256::new();
    for file in evidence {
        let path = config.checkout.join(file);
        super::check_path(&path)?;
        let text = read_text(&path, 65536)?;
        hasher.update((file.len() as u64).to_le_bytes());
        hasher.update(file.as_bytes());
        hasher.update((text.len() as u64).to_le_bytes());
        hasher.update(text.as_bytes());
    }
    Ok(lowercase_hex(&hasher.finalize()))
}
fn blueprint_path(runtime: &Path) -> PathBuf {
    runtime.join("blueprint.json")
}
pub fn load(runtime: &Path) -> Result<Option<Blueprint>> {
    let path = blueprint_path(runtime);
    if !path.exists() {
        return Ok(None);
    }
    super::check_path(&path)?;
    Ok(Some(serde_json::from_str(&read_text(&path, 65536)?)?))
}
pub fn prepare(config: &WorkerConfig, runtime: &Path, now: u64) -> Result<Blueprint> {
    let evidence = vec!["AGENTS.md".into(), "crates/wire/src/lib.rs".into()];
    let blueprint = Blueprint {
        depth: Depth::Brief,
        schema_version:1,
        purpose:"Create a bounded, read-only wire research job for the local maintenance pipeline".into(),
        checkout:config.checkout.clone(), revision:revision(config)?, source_digest:source_digest(config,&evidence)?, evidence,
        scope:vec!["Read the declared framework rules and communication component".into(), "Publish a validated local support job definition".into(), "Keep job definitions, run evidence and research outside Aaroneous".into()],
        steps:vec![Step {title:"Inspect readiness and blueprint evidence".into(),action:"plan-inspection".into(),depends_on:vec![],expected_evidence:"Repository revision, source digests and mechanical entry checks; optional research remains unverified".into()},Step {title:"Publish the prepared job definition".into(),action:"publish-local-job".into(),depends_on:vec![0],expected_evidence:"An immutable schema-valid manifest with approved inputs, bounded context and owner-reviewed results".into()}],
        success_criteria:vec!["The job manifest validates before publication".into(),"The authoring evidence is current at publication".into(),"Job execution failures remain explicit run outcomes for orchestration to handle".into(),"No generated patch is applied, committed or published".into()],
        rollback:"Retire or supersede a published job without deleting run or retry history. Keep prior definitions and reports. Job authoring never rolls back shared product source.".into(),
        unresolved_questions:vec![], created_at_secs:now,
    };
    fs::write(
        blueprint_path(runtime),
        serde_json::to_vec_pretty(&blueprint)?,
    )?;
    Ok(blueprint)
}
fn complete(blueprint: &Blueprint) -> bool {
    blueprint.schema_version == 1
        && !blueprint.purpose.is_empty()
        && !blueprint.scope.is_empty()
        && !blueprint.evidence.is_empty()
        && !blueprint.success_criteria.is_empty()
        && !blueprint.rollback.is_empty()
        && blueprint.unresolved_questions.is_empty()
        && blueprint.steps.len() == 2
        && blueprint.steps[0].action == "plan-inspection"
        && blueprint.steps[0].depends_on.is_empty()
        && blueprint.steps[1].action == "publish-local-job"
        && blueprint.steps[1].depends_on == [0]
        && blueprint
            .steps
            .iter()
            .all(|step| !step.title.is_empty() && !step.expected_evidence.is_empty())
}
fn reserved(config: &WorkerConfig, paths: &[String]) -> Result<bool> {
    let queue = read_text(&config.queue, 1048576)?;
    Ok(queue.lines().filter(|row| row.starts_with('|')).any(|row| {
        let columns: Vec<_> = row.split('|').map(str::trim).collect();
        if columns
            .get(5)
            .is_some_and(|branch| branch.contains("(devtools)"))
        {
            return false;
        }
        columns
            .get(3)
            .is_some_and(|status| matches!(*status, "Claimed" | "Review"))
            && paths.iter().any(|path| {
                row.contains(path)
                    || row.split('`').any(|scope| {
                        scope
                            .strip_suffix("/**")
                            .is_some_and(|prefix| path.starts_with(&format!("{prefix}/")))
                    })
            })
    }))
}
pub fn inspect(
    config: &WorkerConfig,
    runtime: &Path,
    mode: Mode,
    allow_job_build: bool,
    _active: bool,
) -> Result<PlanStatus> {
    let blueprint = load(runtime)?;
    let blueprint_complete = blueprint.as_ref().is_some_and(complete);
    let current = blueprint.as_ref().is_some_and(|b| {
        b.checkout == config.checkout
            && revision(config).is_ok_and(|revision| revision == b.revision)
            && source_digest(config, &b.evidence).is_ok_and(|digest| digest == b.source_digest)
    });
    let tree_clean = clean(config).unwrap_or(false);
    let scope_free = blueprint
        .as_ref()
        .is_some_and(|b| reserved(config, &b.evidence).is_ok_and(|reserved| !reserved));
    let criteria = vec![
        Criterion {label:"Blueprint is complete".into(),satisfied:blueprint_complete,detail:"A bounded scope, ordered steps, explicit success evidence, rollback and no open questions are required.".into()},
        Criterion {label:"Evidence is current".into(),satisfied:current,detail:"The checkout revision and evidence digests must match the prepared blueprint.".into()},
        Criterion {label:"Shared work is safe".into(),satisfied:tree_clean,detail:"Modified evidence files block publication. Unrelated product work and existing orchestration continue independently.".into()},
        Criterion {label:"Ownership is clear".into(),satisfied:scope_free,detail:"A Claimed or Review scope cannot overlap blueprint evidence.".into()},
        Criterion {label:"Publishing is configured".into(),satisfied:allow_job_build,detail:"The launcher must explicitly enable publishing validated job definitions.".into()},
        Criterion {label:"Native job boundaries are valid".into(),satisfied:config.jobs_directory.is_dir() && config.results_directory.is_dir(),detail:"Published jobs use the existing bounded native worker. Running jobs do not block authoring.".into()},
    ];
    let build_ready = criteria.iter().all(|criterion| criterion.satisfied);
    Ok(PlanStatus {mode,build_ready,criteria,blueprint,packet:Packet {title:"Maintenance readiness".into(),summary:if build_ready {"The prepared job definition meets mechanical publication checks. Build may publish the prepared job; orchestration controls subsequent dispatch."}else{"Stay in Plan. Resolve the listed blockers before publishing the prepared job."}.into(),facts:vec!["Research is read-only brainstorming, not an approved implementation.".into(),"This template defines read-only research; it does not implement arbitrary product features.".into()],next_steps:vec!["Prepare the blueprint, gather research, and inspect unresolved risks.".into(),"Resolve shared work and ownership blockers without overwriting contributors.".into(),"Switch job authoring to Build only after every publication check passes.".into()]}})
}
pub fn switch(
    config: &WorkerConfig,
    runtime: &Path,
    requested: Mode,
    allow_job_build: bool,
    _active: bool,
) -> Result<Mode> {
    if requested == Mode::Build
        && !inspect(config, runtime, Mode::Plan, allow_job_build, _active)?.build_ready
    {
        bail!("Build entry criteria are not satisfied; the utility remains in Plan");
    }
    Ok(requested)
}

/// Build creates a job definition; it does not execute the job or control global orchestration.
pub fn publish(
    config: &WorkerConfig,
    runtime: &Path,
    mode: Mode,
    allow_job_build: bool,
) -> Result<String> {
    if mode != Mode::Build {
        bail!("Publishing a job requires job authoring to be switched to Build");
    }
    switch(config, runtime, Mode::Build, allow_job_build, false)?;
    let blueprint = load(runtime)?.context("Prepare the job blueprint first")?;
    let spec = job_spec(&blueprint);
    let destination = config.jobs_directory.join(format!("{}.toml", spec.id));
    if destination.exists() {
        return Ok("This job definition is already published. Orchestration owns its execution and history.".into());
    }
    crate::approve_job(config, spec)?;
    Ok("Job definition published. The independent local scheduler will process eligible work; no product change was made.".into())
}

fn job_spec(blueprint: &Blueprint) -> crate::NewJob {
    let instruction=match blueprint.depth {
        Depth::Brief=>"Develop a bounded information packet for wire boundary hardening using only the declared source. Return up to three concrete observations, risks or unresolved assumptions, and testable acceptance criteria. Compare existing approaches where supported by evidence. No implementation, command execution or verification claims. rust_draft must be null.",
        Depth::Thorough=>"Develop a thorough blueprint research packet for wire boundary hardening using only the declared source. Prioritize up to three source-grounded findings. For each, distinguish observations from assumptions, explain existing standard approaches, outline dependencies and propose acceptance tests that can disprove the claim. Expose unresolved questions and evidence needed to resolve them. Do not implement anything, execute commands or claim verification. rust_draft must be null.",
    }.to_owned();
    let digest = lowercase_hex(&Sha256::digest(
        format!(
            "wire-research-v1\n{}\n{}",
            blueprint.source_digest, instruction
        )
        .as_bytes(),
    ));
    crate::NewJob {
        id: format!("JOB-WIRE-{}", &digest[..12]),
        mode: "analysis".into(),
        evidence: vec!["crates/wire/src/lib.rs".into()],
        instruction,
    }
}
pub fn published_id(blueprint: &Blueprint) -> String {
    job_spec(blueprint).id
}
pub fn refine(config: &WorkerConfig, runtime: &Path, now: u64) -> Result<Blueprint> {
    let mut blueprint = prepare(config, runtime, now)?;
    blueprint.depth = Depth::Thorough;
    blueprint.steps[1].expected_evidence="A validated, versioned research definition requiring alternatives, dependencies, falsifiable acceptance tests and explicit unresolved questions".into();
    blueprint.success_criteria.push("Research distinguishes observed facts from assumptions and identifies evidence needed to settle open questions".into());
    fs::write(
        blueprint_path(runtime),
        serde_json::to_vec_pretty(&blueprint)?,
    )?;
    Ok(blueprint)
}
#[cfg(test)]
mod tests {
    #[test]
    fn digest_encoding_preserves_lowercase_width_and_sha256_values() {
        assert_eq!(super::lowercase_hex(&[0, 1, 15, 16, 255]), "00010f10ff");
        assert_eq!(
            super::lowercase_hex(&<sha2::Sha256 as sha2::Digest>::digest(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    use super::*;
    fn fixture() -> (tempfile::TempDir, WorkerConfig, PathBuf) {
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path();
        for dir in [
            "checkout/crates/wire/src",
            "jobs",
            "results",
            "archive",
            "reservation",
            "runtime",
        ] {
            fs::create_dir_all(root.join(dir)).expect("dir");
        }
        for file in ["AGENTS.md", "crates/wire/src/lib.rs"] {
            fs::write(root.join("checkout").join(file), "explicit contract\n").expect("source");
        }
        fs::write(root.join("queue.md"), "empty\n").expect("queue");
        for args in [
            vec!["init"],
            vec!["config", "user.name", "Fixture"],
            vec!["config", "user.email", "fixture@example.invalid"],
            vec!["add", "."],
            vec!["commit", "-m", "fixture"],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(root.join("checkout"))
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .expect("git")
                    .success()
            );
        }
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
        let runtime = root.join("runtime");
        (temp, config, runtime)
    }
    #[test]
    fn build_requires_blueprint_and_explicit_recipe_permission() {
        let (_temp, config, runtime) = fixture();
        assert!(switch(&config, &runtime, Mode::Build, true, false).is_err());
        prepare(&config, &runtime, 1).expect("blueprint");
        assert!(switch(&config, &runtime, Mode::Build, false, false).is_err());
        assert!(switch(&config, &runtime, Mode::Build, true, true).is_ok());
        assert!(switch(&config, &runtime, Mode::Build, true, false).is_ok());
    }
    #[test]
    fn dirty_or_changed_evidence_and_claims_keep_plan_locked() {
        let (_temp, config, runtime) = fixture();
        prepare(&config, &runtime, 1).expect("blueprint");
        fs::write(config.checkout.join("AGENTS.md"), "changed").expect("change");
        let state = inspect(&config, &runtime, Mode::Plan, true, false).expect("state");
        assert!(!state.build_ready);
        assert!(!state.criteria[1].satisfied);
        assert!(!state.criteria[2].satisfied);
        assert!(switch(&config, &runtime, Mode::Plan, true, false).is_ok());
        fs::write(
            &config.queue,
            "| X | `crates/wire/src/lib.rs` | Claimed | owner |\n",
        )
        .expect("claim");
        assert!(
            !inspect(&config, &runtime, Mode::Plan, true, false)
                .expect("state")
                .criteria[3]
                .satisfied
        );
    }
    #[test]
    fn unresolved_questions_or_modified_recipe_fail_closed() {
        let (_temp, config, runtime) = fixture();
        let mut blueprint = prepare(&config, &runtime, 1).expect("blueprint");
        blueprint
            .unresolved_questions
            .push("Unresolved dependency".into());
        assert!(!complete(&blueprint));
        blueprint.unresolved_questions.clear();
        blueprint.steps[1].action = "arbitrary-command".into();
        assert!(!complete(&blueprint));
    }

    #[test]
    fn building_publishes_jobs_without_running_commands_or_touching_product() {
        let (_temp, config, runtime) = fixture();
        assert!(publish(&config, &runtime, Mode::Plan, true).is_err());
        assert!(publish(&config, &runtime, Mode::Build, true).is_err());
        prepare(&config, &runtime, 1).expect("blueprint");
        publish(&config, &runtime, Mode::Build, true).expect("publish");
        assert_eq!(
            crate::list_files(&config.jobs_directory, "toml")
                .expect("jobs")
                .len(),
            1
        );
        assert!(
            crate::list_files(&config.results_directory, "json")
                .expect("results")
                .is_empty()
        );
        assert!(clean(&config).expect("clean"));
        publish(&config, &runtime, Mode::Build, true).expect("idempotent publish");
        assert_eq!(
            crate::list_files(&config.jobs_directory, "toml")
                .expect("jobs")
                .len(),
            1
        );
    }
    #[test]
    fn unrelated_changes_and_private_claims_do_not_freeze_job_authoring() {
        let (_temp, config, runtime) = fixture();
        prepare(&config, &runtime, 1).expect("blueprint");
        fs::write(config.checkout.join("unrelated.rs"), "other owner's work").expect("unrelated");
        fs::write(
            &config.queue,
            "| C12 | AGENTS.md | Review | other | main (devtools) | now |\n",
        )
        .expect("queue");
        assert!(
            inspect(&config, &runtime, Mode::Plan, true, true)
                .expect("status")
                .build_ready
        );
    }

    #[test]
    fn refinement_creates_a_new_job_version_and_preserves_previous_definition() {
        let (_temp, config, runtime) = fixture();
        let original = prepare(&config, &runtime, 1).expect("prepare");
        publish(&config, &runtime, Mode::Build, true).expect("publish");
        let updated = refine(&config, &runtime, 2).expect("refine");
        assert_ne!(published_id(&original), published_id(&updated));
        publish(&config, &runtime, Mode::Build, true).expect("publish refined");
        assert_eq!(
            crate::list_files(&config.jobs_directory, "toml")
                .expect("jobs")
                .len(),
            2
        );
        assert!(clean(&config).expect("clean"));
    }
}
