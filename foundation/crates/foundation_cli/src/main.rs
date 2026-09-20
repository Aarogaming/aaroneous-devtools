mod agentic_registrar;
mod capability_composition;
mod cloud_providers;
mod component_inventory;
mod consensus_engine;
mod cross_verifier;
mod dependency_review;
mod dogfood;
mod frontier_discovery;
mod hypervisor_mcp;
mod intake;
mod intake_generator;
mod job_converter;
mod local_proposal;
mod local_worker;
mod multithreader;
mod scheduler;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "foundation",
    about = "Explicit-input development foundation tools"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Agentic Queue Registrar & Dispatch Control Plane
    AgenticRegistrar {
        #[command(subcommand)]
        action: RegistrarAction,
    },
    /// Perform Rust Git discovery on a repository checkout
    DiscoverGit {
        #[arg(long)]
        checkout: PathBuf,
    },
    /// Native in-process Git repository discovery using gix
    DiscoverGitNative {
        #[arg(long)]
        checkout: PathBuf,
    },
    /// Query crates.io and GitHub for open-source candidate crates and generate frontier audit jobs
    FrontierDiscover {
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 5)]
        limit: usize,
        #[arg(long, default_value = "d:\\aaroneous-devtools\\worker-jobs")]
        output_dir: PathBuf,
    },
    /// Generate bounded cross-category capability composition candidate pairs from an
    /// Aaroneous capability registry JSON export and emit worker-jobs analysis packets
    /// asking a reviewer to score each pair against BUILD_WORTHINESS_STANDARD.md
    CapabilityCompose {
        #[arg(long)]
        capabilities: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Scrape and convert an external architecture document or whitepaper using Firecrawl
    FirecrawlScrape {
        #[arg(long)]
        url: String,
        #[arg(long)]
        endpoint: Option<String>,
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Evaluate a candidate crate or repository against the Filtration Standard and Quadratic Review Criteria
    EvaluateFrontier {
        #[arg(long)]
        target_crate: String,
        #[arg(long, default_value = "0.1.0")]
        version: String,
        #[arg(long)]
        repository_url: Option<String>,
        #[arg(long)]
        license: Option<String>,
        #[arg(long)]
        description: String,
        #[arg(long)]
        active_need: String,
        #[arg(long)]
        q1_needle: Option<f64>,
        #[arg(long)]
        q2_feasibility: Option<f64>,
        #[arg(long)]
        q3_tests: Option<f64>,
        #[arg(long)]
        q4_leverage: Option<f64>,
        #[arg(long)]
        rationale: Option<String>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Launch the native Aaroneous Hypervisor MCP Server daemon process out-of-tree.
    HypervisorMcp {
        #[arg(long, default_value = "d:\\Aaroneous")]
        checkout: PathBuf,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 8766)]
        port: u16,
    },
    /// Closed-loop end-to-end autonomous development pipeline
    Pipeline {
        #[arg(long)]
        config: PathBuf,
    },
    /// Register Windows Scheduled Task for foundation passive engine
    RegisterScheduledTask {
        #[arg(long)]
        binary_path: PathBuf,
        #[arg(long, default_value = "Aaroneous-Devtools-Passive-Engine")]
        task_name: String,
        #[arg(long, default_value_t = 15)]
        interval_minutes: u32,
    },
    /// Poll approved scratch jobs once; the operating system schedules subsequent ticks.
    LocalWorkerTick {
        #[arg(long)]
        config: PathBuf,
    },
    /// Run local worker continuous event monitor loop until stopped (Ctrl+C).
    /// Immediately dispatches the next job/prep item as soon as the prior one completes.
    LocalWorkerServe {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value_t = 3)]
        poll_interval_secs: u64,
    },
    /// View local agent probation ledger status and quality pass rate.
    LocalWorkerProbation {
        #[arg(long)]
        config: PathBuf,
    },
    /// Perform adversarial Red-Team cross-verification of an AI proposal.
    CrossVerify {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        proposal: PathBuf,
        #[arg(long, default_value = "anthropic-claude")]
        auditor_provider: String,
        #[arg(long, default_value = "claude-3-5-sonnet")]
        auditor_model: String,
    },
    /// View multi-agent cross-verification consensus ledger and defect metrics.
    ConsensusStatus {
        #[arg(long)]
        config: PathBuf,
    },
    /// Review declared evidence and populate an owner-reviewed inbox.
    MultithreaderReview {
        #[arg(long)]
        config: PathBuf,
    },
    /// Run multithreader review cycles on an interval until stopped (Ctrl+C).
    /// Persistence lives in this process, not an installed OS scheduled task:
    /// start it when you want reviews running, stop it when you don't.
    MultithreaderServe {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value_t = 30)]
        interval_minutes: u32,
    },
    /// Validate declared paths and write a versioned input report.
    ValidateStack {
        /// Absolute path to a stack manifest.
        #[arg(long)]
        manifest: PathBuf,
    },
    ValidateInputs {
        /// Absolute path to an Aaroneous checkout.
        #[arg(long)]
        checkout: PathBuf,
        /// Absolute path for the report file; its parent must already exist.
        #[arg(long)]
        output: PathBuf,
        /// Additional absolute support directories, such as models, datasets, or toolchains.
        #[arg(long = "support-dir")]
        support_dirs: Vec<PathBuf>,
    },
    /// Run cargo-audit against an explicit checkout and write a versioned,
    /// schema'd dependency-vulnerability report. Read-only: never touches
    /// Cargo.lock or the checkout in any way. See S2 in governance/WORKLIST.md
    /// and the schema doc at contracts/dependency-review/v1/SCHEMA.md.
    DependencyReview {
        /// Absolute path to an Aaroneous (or other Cargo workspace) checkout.
        #[arg(long)]
        checkout: PathBuf,
        /// Absolute path for the report file; its parent must already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// Evaluate a coordination queue and write a read-only scheduling report.
    ScheduleReview {
        #[arg(long)]
        queue: PathBuf,
        #[arg(long)]
        program: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 3)]
        primary_capacity: usize,
    },
    /// Inventory one declared Cargo component without modifying its checkout.
    ComponentInventory {
        /// Absolute path to a Cargo checkout.
        #[arg(long)]
        checkout: PathBuf,
        /// Relative path from checkout root to the component crate.
        #[arg(long)]
        crate_path: PathBuf,
        /// Absolute path for the versioned report file.
        #[arg(long)]
        output: PathBuf,
    },
    LocalProposal {
        #[arg(long)]
        endpoint: String,
        #[arg(long, default_value = "qwen3.5:9b-q6")]
        model: String,
        #[arg(long)]
        evidence: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 4096)]
        num_predict: u32,
    },
    DogfoodMetadata {
        #[arg(long)]
        checkout: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    ValidateIntake {
        #[arg(long)]
        proposal: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    GenerateIntake {
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        evidence: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Convert a coordination queue or roadmap row into an approved worker job manifest TOML.
    QueueToJob {
        #[arg(long)]
        queue: PathBuf,
        #[arg(long)]
        task_id: String,
        #[arg(long, default_value = "analysis")]
        mode: String,
        #[arg(long = "evidence")]
        evidence: Vec<String>,
        #[arg(long)]
        output_dir: PathBuf,
    },
    /// Automatically sync all Ready or In Progress queue rows into approved worker job manifests.
    SyncWorkerJobs {
        #[arg(long)]
        queue: PathBuf,
        #[arg(long, default_value = "analysis")]
        mode: String,
        #[arg(long)]
        output_dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum RegistrarAction {
    Status {
        #[arg(long)]
        config: PathBuf,
    },
    Dashboard {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Register {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        crate_name: String,
        #[arg(long)]
        task: String,
        #[arg(long)]
        target_file: String,
        #[arg(long)]
        instruction: String,
    },
    Dispatch {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        task_id: Option<String>,
    },
    Daemon {
        #[arg(long)]
        config: PathBuf,
    },
    /// Explicitly accept a pending-owner-review draft: writes it into the
    /// checkout, re-verifies (cargo check + clippy), and only then commits.
    /// The only action that ever touches the real checkout or git.
    Promote {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        task_id: String,
    },
    Serve {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    SetupMcp {
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}

const INPUT_REPORT_KIND: &str = "foundation.input-report";

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct InputReport {
    schema_version: u32,
    artifact_kind: String,
    checkout: String,
    output: String,
    support_directories: Vec<String>,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::AgenticRegistrar { action } => match action {
            RegistrarAction::Status { config } => agentic_registrar::print_status(&config),
            RegistrarAction::Dashboard { config, output } => {
                agentic_registrar::generate_dashboard_html(&config, &output)
            }
            RegistrarAction::Register {
                config,
                id,
                crate_name,
                task,
                target_file,
                instruction,
            } => agentic_registrar::register_task(
                &config,
                &id,
                &crate_name,
                &task,
                &target_file,
                &instruction,
            ),
            RegistrarAction::Dispatch { config, task_id } => {
                agentic_registrar::dispatch(&config, task_id.as_deref())
            }
            RegistrarAction::Daemon { config } => agentic_registrar::daemon(&config),
            RegistrarAction::Promote { config, task_id } => {
                agentic_registrar::promote(&config, &task_id)
            }
            RegistrarAction::Serve { config, port } => agentic_registrar::serve(&config, port),
            RegistrarAction::SetupMcp { port } => {
                let paths = agentic_registrar::auto_register_ide_mcp_config(port)?;
                for p in paths {
                    println!("Auto-configured IDE MCP setting: {}", p.display());
                }
                println!("MCP server auto-configuration complete for port {port}.");
                Ok(())
            }
        },
        Command::DiscoverGit { checkout } => {
            agentic_registrar::git_discover_repository(&checkout).map(|_| ())
        }
        Command::DiscoverGitNative { checkout } => {
            let summary = frontier_discovery::discover_native(&checkout)?;
            println!("=== NATIVE GIX GIT DISCOVERY ===");
            println!("Checkout:         {}", summary.checkout);
            println!("Branch:           {}", summary.branch);
            println!("Tracked Entries:  {}", summary.tracked_files_count);
            println!("Submodules:       {}", summary.submodules_count);
            println!("Remotes:          {}", summary.remotes.len());
            for r in &summary.remotes {
                println!("  - {r}");
            }
            println!("================================");
            Ok(())
        }
        Command::FrontierDiscover {
            query,
            limit,
            output_dir,
        } => {
            println!(
                "Searching crates.io for open-source frontier candidates: '{query}' (limit: {limit})..."
            );
            let candidates = frontier_discovery::search_crates_io(&query, limit)?;
            println!("Found {} candidates:", candidates.len());
            for (idx, c) in candidates.iter().enumerate() {
                println!(
                    "  [{}] {} (v{}) - {}",
                    idx + 1,
                    c.name,
                    c.version,
                    c.description
                );
                let job_path = frontier_discovery::create_frontier_job(
                    c,
                    &output_dir,
                    "docs/ARCHITECTURE.md",
                )?;
                println!(
                    "      -> Created frontier audit job: {}",
                    job_path.display()
                );
            }
            Ok(())
        }
        Command::CapabilityCompose {
            capabilities,
            output_dir,
            limit,
        } => {
            println!(
                "Generating bounded cross-category capability composition pairs from {} (limit: {limit})...",
                capabilities.display()
            );
            let written = capability_composition::run(&capabilities, &output_dir, limit)?;
            println!("Emitted {} composition analysis packet(s):", written.len());
            for path in &written {
                println!("  -> {}", path.display());
            }
            Ok(())
        }
        Command::FirecrawlScrape {
            url,
            endpoint,
            api_key,
        } => {
            println!("Scraping architectural docs via Firecrawl for: {url}...");
            let markdown = frontier_discovery::firecrawl_scrape(
                &url,
                endpoint.as_deref(),
                api_key.as_deref(),
            )?;
            println!("=== SCRAPED MARKDOWN ({} bytes) ===", markdown.len());
            println!("{}", markdown.chars().take(500).collect::<String>());
            println!("... [truncated]");
            Ok(())
        }
        Command::EvaluateFrontier {
            target_crate,
            version,
            repository_url,
            license,
            description,
            active_need,
            q1_needle,
            q2_feasibility,
            q3_tests,
            q4_leverage,
            rationale,
            output,
        } => {
            println!("Evaluating candidate '{target_crate}' against Filtration Standard...");
            let gates = frontier_discovery::validate_standard_filtration_gates(
                license.as_deref(),
                &description,
            );

            let raw_scores = match (q1_needle, q2_feasibility, q3_tests, q4_leverage) {
                (Some(q1), Some(q2), Some(q3), Some(q4)) => Some((q1, q2, q3, q4)),
                _ => None,
            };

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();

            let rat = rationale.unwrap_or_else(|| {
                format!("Evaluated against active workspace need: '{active_need}'")
            });

            let report =
                frontier_discovery::compute_quadratic_review(frontier_discovery::ReviewInput {
                    target_crate: &target_crate,
                    version: &version,
                    repository_url,
                    active_need: &active_need,
                    now,
                    gates,
                    raw_scores,
                    rationale: &rat,
                });

            println!("=== FRONTIER EVALUATION REPORT ===");
            println!(
                "Target Crate:    {} (v{})",
                report.target_crate, report.version
            );
            println!("Active Need:     {}", report.active_need);
            println!(
                "Filtration:      {}",
                if report.filtration_passed {
                    "PASSED"
                } else {
                    "FAILED"
                }
            );
            for g in &report.gates {
                let status_str = if g.passed { "PASS" } else { "FAIL" };
                println!("  [{status_str}] {}: {}", g.gate_id, g.name);
                if let Some(ref r) = g.violation_reason {
                    println!("         Violation: {r}");
                }
            }
            if let Some(ref s) = report.scores {
                println!("---------------------------------");
                println!(
                    "Q1 (Needle 30%):        {:.1} / 10.0",
                    s.q1_algorithmic_needle
                );
                println!(
                    "Q2 (Feasibility 30%):   {:.1} / 10.0",
                    s.q2_refactoring_feasibility
                );
                println!(
                    "Q3 (Tests 20%):         {:.1} / 10.0",
                    s.q3_test_portability
                );
                println!(
                    "Q4 (Leverage 20%):      {:.1} / 10.0",
                    s.q4_milestone_leverage
                );
                println!("Linear Weighted Score:  {:.2}", s.linear_weighted_score);
                println!("Quadratic Penalty:      -{:.2}", s.quadratic_defect_penalty);
                println!("Composite Score:        {:.2} / 100.0", s.composite_score);
            }
            println!("Recommendation:  {}", report.recommendation);
            println!("Rationale:       {}", report.rationale);
            println!("=================================");

            if let Some(out_path) = output {
                let json_text = serde_json::to_string_pretty(&report)?;
                std::fs::write(&out_path, json_text)?;
                println!("Report written to: {}", out_path.display());
            }

            Ok(())
        }
        Command::HypervisorMcp {
            checkout,
            host,
            port,
        } => hypervisor_mcp::launch(&checkout, &host, port),
        Command::Pipeline { config } => agentic_registrar::run_pipeline(&config),
        Command::RegisterScheduledTask {
            binary_path,
            task_name,
            interval_minutes,
        } => agentic_registrar::register_task_scheduler(&binary_path, &task_name, interval_minutes),
        Command::LocalWorkerTick { config } => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            local_worker::tick(&config, ((now / 60) % 60) as u32, now)
        }
        Command::LocalWorkerServe {
            config,
            poll_interval_secs,
        } => local_worker::serve(&config, poll_interval_secs),
        Command::LocalWorkerProbation { config } => local_worker::print_probation_status(&config),
        Command::CrossVerify {
            config,
            proposal,
            auditor_provider,
            auditor_model,
        } => run_cross_verification(&config, &proposal, &auditor_provider, &auditor_model),
        Command::ConsensusStatus { config } => run_consensus_status(&config),
        Command::MultithreaderReview { config } => multithreader::run(&config),
        Command::MultithreaderServe {
            config,
            interval_minutes,
        } => multithreader::serve(&config, interval_minutes),
        Command::ValidateStack { manifest } => validate_stack(&manifest),
        Command::ValidateInputs {
            checkout,
            output,
            support_dirs,
        } => validate_inputs(&checkout, &output, &support_dirs),
        Command::DependencyReview { checkout, output } => {
            dependency_review::run(&checkout, &output)
        }
        Command::ScheduleReview {
            queue,
            program,
            output,
            primary_capacity,
        } => scheduler::run(&queue, &program, &output, primary_capacity),
        Command::ComponentInventory {
            checkout,
            crate_path,
            output,
        } => component_inventory::run(&checkout, &crate_path, &output),
        Command::LocalProposal {
            endpoint,
            model,
            evidence,
            output,
            num_predict,
        } => local_proposal::run(&endpoint, &model, &evidence, &output, num_predict),
        Command::DogfoodMetadata { checkout, output } => dogfood::metadata(&checkout, &output),
        Command::ValidateIntake { proposal, output } => intake::validate(&proposal, &output),
        Command::GenerateIntake {
            endpoint,
            evidence,
            output,
        } => intake_generator::run(&endpoint, &evidence, &output),
        Command::QueueToJob {
            queue,
            task_id,
            mode,
            evidence,
            output_dir,
        } => job_converter::convert(&queue, &task_id, &mode, &evidence, &output_dir).map(|_| ()),
        Command::SyncWorkerJobs {
            queue,
            mode,
            output_dir,
        } => job_converter::sync_worker_jobs(&queue, &mode, &output_dir).map(|_| ()),
    }
}

#[derive(Debug, Deserialize)]
struct StackManifest {
    version: u32,
    name: String,
    commands: Vec<StackCommand>,
}

#[derive(Debug, Deserialize)]
struct StackCommand {
    name: String,
    program: String,
    #[serde(default)]
    args: Vec<String>,
    read_only: bool,
}

fn validate_stack(manifest_path: &Path) -> Result<()> {
    if !manifest_path.is_absolute() {
        bail!(
            "manifest must be an absolute path: {}",
            manifest_path.display()
        );
    }
    let content = fs::read_to_string(manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest: StackManifest = toml::from_str(&content).context("invalid stack manifest")?;
    if manifest.version != 1 || manifest.name.trim().is_empty() || manifest.commands.is_empty() {
        bail!("stack manifest requires version 1, a name, and at least one command");
    }
    for command in &manifest.commands {
        if command.name.trim().is_empty() || command.program.trim().is_empty() || !command.read_only
        {
            bail!("each command requires a name, a program, and read_only = true");
        }
        if contains_absolute_or_parent_path(&command.program)
            || command
                .args
                .iter()
                .any(|argument| contains_absolute_or_parent_path(argument))
        {
            bail!("stack manifests may not embed absolute or parent-directory paths");
        }
    }
    println!("validated stack manifest: {}", manifest.name);
    Ok(())
}

fn contains_absolute_or_parent_path(value: &str) -> bool {
    Path::new(value).is_absolute() || value.split(['/', '\\']).any(|segment| segment == "..")
}
fn validate_inputs(checkout: &Path, output: &Path, support_dirs: &[PathBuf]) -> Result<()> {
    require_absolute_directory("checkout", checkout)?;
    if !checkout.join(".git").exists() {
        bail!(
            "checkout must contain a .git file or directory: {}",
            checkout.display()
        );
    }
    if !output.is_absolute() {
        bail!("output must be an absolute path: {}", output.display());
    }
    let parent = output
        .parent()
        .context("output path must have an existing parent directory")?;
    require_absolute_directory("output parent", parent)?;
    for support_dir in support_dirs {
        require_absolute_directory("support directory", support_dir)?;
    }

    let report = InputReport {
        schema_version: 1,
        artifact_kind: INPUT_REPORT_KIND.to_owned(),
        checkout: checkout.display().to_string(),
        output: output.display().to_string(),
        support_directories: support_dirs
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
    };
    let json = serde_json::to_vec_pretty(&report).context("failed to serialize input report")?;
    fs::write(output, json).with_context(|| format!("failed to write {}", output.display()))?;
    println!("validated explicit foundation inputs: {}", output.display());
    Ok(())
}

pub(crate) fn require_absolute_directory(label: &str, path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("{label} must be an absolute path: {}", path.display());
    }
    let metadata = fs::metadata(path)
        .with_context(|| format!("{label} is unavailable: {}", path.display()))?;
    if !metadata.is_dir() {
        bail!("{label} must be a directory: {}", path.display());
    }
    Ok(())
}

fn run_cross_verification(
    _config_path: &Path,
    proposal_path: &Path,
    auditor_provider: &str,
    auditor_model: &str,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let content = fs::read_to_string(proposal_path)
        .with_context(|| format!("failed to read proposal file: {}", proposal_path.display()))?;
    let record: serde_json::Value = serde_json::from_str(&content)?;

    let author_provider = record["artifact_kind"]
        .as_str()
        .unwrap_or("foundation.local-worker");
    let author_model = "qwen3.5:9b-q6";
    let task_id = record["job_id"].as_str().unwrap_or("UNKNOWN_TASK");
    let input = record["input"].as_str().unwrap_or("");
    let proposal_val = &record["proposal"];

    if proposal_val.is_null() {
        bail!("proposal file has no valid proposal object");
    }

    let draft: local_worker::Draft = serde_json::from_value(proposal_val.clone())?;

    let provider: Box<dyn cloud_providers::AgentProvider> = match auditor_provider {
        "anthropic-claude" | "claude" => Box::new(
            cloud_providers::AnthropicClaudeProvider::from_env(auditor_model)?,
        ),
        "google-gemini" | "gemini" => Box::new(cloud_providers::GoogleGeminiProvider::from_env(
            auditor_model,
        )?),
        "openai-codex" | "codex" => Box::new(cloud_providers::OpenAiCodexProvider::from_env(
            auditor_model,
        )?),
        "local-ollama" | "ollama" => Box::new(cloud_providers::LocalOllamaProvider::new(
            "http://localhost:11434".into(),
            auditor_model.into(),
        )),
        other => bail!("unsupported auditor provider: {other}"),
    };

    println!("Executing Red-Team cross-audit via {auditor_provider} ({auditor_model})...");
    let verdict = cross_verifier::cross_verify_proposal(
        author_provider,
        author_model,
        &draft,
        input,
        provider.as_ref(),
        now,
    )?;

    println!("=== CROSS-AUDIT VERDICT ===");
    println!("Task:                {}", verdict.proposal_task);
    println!(
        "Author:              {} ({})",
        verdict.author_provider, verdict.author_model
    );
    println!(
        "Auditor:             {} ({})",
        verdict.auditor_provider, verdict.auditor_model
    );
    println!("Decision:            {:?}", verdict.decision);
    println!(
        "Flaws Detected:      {}",
        verdict.security_flaws_found.len()
    );
    for flaw in &verdict.security_flaws_found {
        println!("  - Flaw: {flaw}");
    }
    println!("Recommended Action:  {}", verdict.recommended_action);

    let results_dir = proposal_path
        .parent()
        .context("proposal file parent directory missing")?;
    let consensus_rec = consensus_engine::evaluate_consensus(
        task_id,
        author_provider,
        author_model,
        &[verdict],
        true,
        1,
        now,
    );
    consensus_engine::update_consensus_ledger(results_dir, &consensus_rec)?;
    Ok(())
}

fn run_consensus_status(config_path: &Path) -> Result<()> {
    let content = fs::read_to_string(config_path)?;
    let val: serde_json::Value = toml::from_str(&content)?;
    let results_dir = PathBuf::from(
        val["results_directory"]
            .as_str()
            .context("missing results_directory")?,
    );
    consensus_engine::print_consensus_status(&results_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_checkout() {
        let temp = tempfile::tempdir().expect("test directory");
        let error = validate_inputs(Path::new("relative"), &temp.path().join("report.json"), &[])
            .expect_err("relative checkout must be rejected");
        assert!(error.to_string().contains("absolute"));
    }

    #[test]
    fn fixture_matches_input_report_contract() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../contracts/fixtures/input-report-v1.json"
        ));
        let report: InputReport = serde_json::from_str(fixture).expect("valid contract fixture");
        assert_eq!(report.schema_version, 1);
        assert_eq!(report.artifact_kind, INPUT_REPORT_KIND);
        assert_eq!(report.support_directories.len(), 2);
    }
    #[test]
    fn records_declared_support_directories() {
        let temp = tempfile::tempdir().expect("test directory");
        let checkout = temp.path().join("checkout");
        let support = temp.path().join("models");
        fs::create_dir_all(checkout.join(".git")).expect("checkout metadata");
        fs::create_dir_all(&support).expect("support directory");
        let output = temp.path().join("report.json");

        validate_inputs(&checkout, &output, std::slice::from_ref(&support)).expect("valid inputs");
        let report: InputReport =
            serde_json::from_slice(&fs::read(output).expect("report")).expect("valid report");
        assert_eq!(report.schema_version, 1);
        assert_eq!(report.artifact_kind, INPUT_REPORT_KIND);
        assert_eq!(
            report.support_directories,
            vec![support.display().to_string()]
        );
    }
}
