mod agentic_registrar;
mod component_inventory;
mod dependency_review;
mod dogfood;
mod intake;
mod intake_generator;
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
