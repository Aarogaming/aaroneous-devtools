mod agentic_registrar;
mod audit_rollup;
mod batch_overlap;
mod candidate_metadata;
mod candidate_retrieval;
mod capability_composition;
mod cloud_providers;
mod component_inventory;
mod consensus_engine;
mod consensus_readiness;
mod cratification_intake;
mod cross_verifier;
mod decision_trace;
mod dependency_review;
mod discovery_digest;
mod dogfood;
mod exploration_seed;
mod fact_checker;
mod frontier_discovery;
mod hypervisor_mcp;
mod intake;
mod intake_generator;
mod job_converter;
mod local_proposal;
mod local_staging;
mod local_worker;
mod multithreader;
mod outbox;
mod portfolio_digest;
mod portfolio_jobs;
mod reservation;
mod scheduler;
mod state_cache;
mod target_decomposition;

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

/// CLI-selectable mirror of `dogfood::ComplianceProfile`'s variants (M53).
/// `clap::ValueEnum` can't derive directly on `ComplianceProfile` itself
/// since `AaroneousStrict`/`Custom` carry data a flag value can't hold; this
/// plain enum picks which named preset applies, and the surrounding
/// `Option<PathBuf>`/`Vec<String>` sibling args (`cratify_target`,
/// `verify_command`) supply that variant's data, matching how other
/// commands in this file already spread a compound choice across several
/// `#[arg(long)]` fields rather than one complex enum arg.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ProfileKind {
    AaroneousStrict,
    DevtoolsOperatingModel,
    Custom,
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
        /// Optional absolute path to the M57 accumulating per-candidate
        /// metadata directory. When omitted, behavior is exactly as before
        /// this milestone -- purely additive.
        #[arg(long)]
        metadata_dir: Option<PathBuf>,
    },
    /// Run frontier discovery cycles on an interval until stopped (Ctrl+C).
    /// Reads an explicit, absolute queries file (one search term per line;
    /// blank lines and `#` comments are skipped) instead of inferring what
    /// to search for. Persistence lives in this process, not an installed
    /// OS scheduled task: start it when you want discovery running, stop it
    /// when you don't.
    FrontierDiscoverServe {
        /// Absolute path to a newline-separated list of crates.io search queries.
        #[arg(long)]
        queries: PathBuf,
        #[arg(long, default_value = "d:\\aaroneous-devtools\\worker-jobs")]
        output_dir: PathBuf,
        #[arg(long, default_value_t = 5)]
        limit: usize,
        #[arg(long, default_value_t = 30)]
        interval_minutes: u32,
        /// Optional absolute path to the M57 accumulating per-candidate
        /// metadata directory. When omitted, behavior is exactly as before
        /// this milestone -- purely additive.
        #[arg(long)]
        metadata_dir: Option<PathBuf>,
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
    /// Turn a `cratify harvest --output <path>` JSON export into one
    /// code-draft worker-jobs packet per harvested crate, asking for a
    /// compliant Rust port rather than auto-generating one (M49)
    CratificationIntake {
        #[arg(long)]
        crate_specs: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
    },
    /// Seed a multi-perspective draft-exploration round (M48): given one
    /// base `code-draft` goal, emit one `worker-jobs/*.toml` packet per
    /// perspective, all sharing a freshly generated `exploration_group`, so
    /// M46's comparison-summary and M44's pre-proofing have several real
    /// drafts to compare once results come back. `--perspectives` may be
    /// repeated; omit it entirely to use the fixed starter vocabulary
    /// (`correctness-first`, `performance-first`, `minimal-diff`).
    SeedExploration {
        /// Absolute path to a text file containing the base drafting
        /// goal/instruction.
        #[arg(long)]
        instruction_file: PathBuf,
        /// Relative evidence file path(s), same shape the emitted job's
        /// `evidence` field takes. Repeat the flag for multiple paths.
        #[arg(long)]
        evidence: Vec<PathBuf>,
        /// Short label for this goal, not the full instruction.
        #[arg(long)]
        goal: String,
        /// Perspective label(s) to seed. Repeat the flag for multiple
        /// perspectives; omit entirely to use the default three.
        #[arg(long)]
        perspectives: Vec<String>,
        #[arg(long)]
        output_dir: PathBuf,
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
    /// Adaptive persistent local-worker loop: tick, look at what actually
    /// happened, and decide the next tick's delay from that -- backing off
    /// when there's nothing to do rather than re-firing on a blind fixed
    /// interval. Opt-in like multithreader-serve: "on" is running this
    /// command, "off" is not running it or stopping it.
    LocalWorkerServe {
        #[arg(long)]
        config: PathBuf,
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
    /// Read a directory of freshly-generated candidate `.toml` packets
    /// (schema: schema_version, id, mode, evidence, instruction) and report
    /// which pairs declare overlapping evidence, before any of them are
    /// written to worker-jobs/. Read-only: never modifies, moves, or
    /// deletes candidate files.
    BatchOverlapCheck {
        /// Absolute path to a directory of candidate `.toml` packets.
        #[arg(long)]
        candidates_dir: PathBuf,
        /// Absolute path for the JSON overlap report; its parent must already exist.
        #[arg(long)]
        output: PathBuf,
    },
    /// List every pending result under `artifacts/local-worker-service/results/`
    /// as a compact, machine-readable summary (job id, outcome, one-line gist,
    /// risk count, whether it's already been reviewed) -- so a reviewer, human
    /// or another agent with file/shell access, doesn't have to open dozens of
    /// individual JSON files one at a time. Read-only.
    OutboxList {
        #[arg(long)]
        results: PathBuf,
        /// Optional: marks entries `reviewed: true` if they already have a
        /// recorded verdict in this file.
        #[arg(long)]
        verdicts: Option<PathBuf>,
        #[arg(long)]
        output: PathBuf,
    },
    /// Write the full `Record` (including the complete proposal) for one
    /// `job_id` from the outbox. Read-only.
    OutboxShow {
        #[arg(long)]
        results: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// Append one reviewer's structured verdict (accept/reject/needs-changes/
    /// uncertain) on one outbox result to `verdicts`. Never promotes, claims,
    /// or edits the result itself -- this only records evidence a reviewer
    /// (human, or another agent) can act on; actual promotion still goes
    /// through `agentic-registrar promote`. Refuses an unknown `job_id` or an
    /// unrecognized verdict string before writing anything.
    OutboxVerdict {
        #[arg(long)]
        results: PathBuf,
        #[arg(long)]
        verdicts: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        reviewer: String,
        /// One of: accept, reject, needs-changes, uncertain.
        #[arg(long)]
        verdict: String,
        #[arg(long, default_value = "")]
        notes: String,
    },
    /// Write every recorded verdict from `verdicts` as a JSON array to
    /// `output`. Read-only; an empty (not missing-file-error) result if no
    /// verdicts have been recorded yet.
    OutboxListVerdicts {
        #[arg(long)]
        verdicts: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Record an `accept` verdict on one outbox result and, if that
    /// succeeds, push its draft as an isolated `local-agent/{id}` branch
    /// (M61). One discrete, named reviewer action about one specific item
    /// -- never an automatic trigger. See
    /// `agentic_registrar::promote_verified_draft`'s doc comment for the
    /// full safety-rail list (high-risk allowlist, circuit breaker,
    /// re-verification, self-approval denylist, mandatory justification).
    /// Never opens a pull request and never merges.
    OutboxPromote {
        #[arg(long)]
        results: PathBuf,
        #[arg(long)]
        verdicts: PathBuf,
        /// Absolute path to the agentic-registrar config (its own
        /// `checkout`/reservation/circuit-breaker settings apply unchanged).
        #[arg(long)]
        registrar_config: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        crate_name: String,
        #[arg(long)]
        target_file: String,
        #[arg(long)]
        reviewer: String,
        /// Required, non-empty: a real reason, not a blank accept.
        #[arg(long)]
        justification: String,
        /// Optional: append a `notice` row here on a successful push.
        #[arg(long)]
        ping: Option<PathBuf>,
    },
    /// Stage a draft as a real local git commit inside a dedicated,
    /// never-pushed `local-staging/{id}` worktree (M62). Contains no path
    /// to a shared remote anywhere -- reuses the same worktree across
    /// repeated calls for the same `id` so iterations accumulate as real
    /// local history. Refuses to force-commit an unchanged redraft.
    StageDraft {
        #[arg(long)]
        checkout: PathBuf,
        #[arg(long)]
        worktrees_root: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        target_file: String,
        /// Absolute path to a file containing the draft source; read as-is.
        #[arg(long)]
        draft_file: PathBuf,
        #[arg(long)]
        message: String,
        #[arg(long)]
        output: PathBuf,
    },
    /// List every existing staging worktree under `worktrees_root`. Read-only.
    ListStaged {
        #[arg(long)]
        worktrees_root: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Remove a staging worktree and its local-only branch.
    PruneStaged {
        #[arg(long)]
        checkout: PathBuf,
        #[arg(long)]
        worktrees_root: PathBuf,
        #[arg(long)]
        id: String,
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
        /// Optional advisory concurrence report from an Aaroneous hypervisor run.
        #[arg(long)]
        concurrence_report: Option<PathBuf>,
    },
    /// Convert unclaimed `governance/MULTITHREAD_PROGRAM.md` portfolio rows
    /// into local-worker `worker-jobs/*.toml` analysis packets requesting a
    /// speculative proposed approach -- never a claim. Skips any row whose
    /// ID already appears in a `Claimed`/`Review` queue row's Scope text.
    PortfolioTriage {
        #[arg(long)]
        program: PathBuf,
        #[arg(long)]
        queue: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
    },
    /// Aggregate completed local-worker portfolio triage results into
    /// a unified human- and cloud-agent-facing digest markdown file.
    PortfolioDigest {
        /// Absolute path to the local worker's results directory.
        #[arg(long)]
        results_dir: PathBuf,
        /// Optional absolute path to the local worker's archive directory.
        #[arg(long)]
        archive_dir: Option<PathBuf>,
        /// Absolute path for the generated markdown digest file.
        #[arg(long)]
        output: PathBuf,
    },
    /// Aggregate internal capability composition and external frontier discovery results into
    /// a unified executive discovery digest markdown file.
    DiscoveryDigest {
        /// Absolute path to the local worker's results directory.
        #[arg(long)]
        results_dir: PathBuf,
        /// Optional absolute path to the local worker's archive directory.
        #[arg(long)]
        archive_dir: Option<PathBuf>,
        /// Absolute path for the generated markdown digest file.
        #[arg(long)]
        output: PathBuf,
    },
    /// Query accumulated M57 candidate metadata records using explainable
    /// weighted Jaccard similarity (M59).
    QueryCandidates {
        /// Absolute path to the metadata directory.
        #[arg(long)]
        metadata_dir: PathBuf,
        /// The query string or candidate ID to find matches for.
        #[arg(long)]
        like: String,
        /// Optional kind filter: "crate" or "cluster".
        #[arg(long)]
        kind: Option<String>,
        /// Minimum similarity threshold (0.0 to 1.0).
        #[arg(long, default_value_t = 0.1)]
        min_score: f64,
        /// Maximum number of matches to return.
        #[arg(long, default_value_t = 5)]
        limit: usize,
        /// Emit results as JSON instead of human-readable text.
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Verify ground-truth factual claims and citations against a repository checkout (M56).
    VerifyClaims {
        /// Absolute path to the repository checkout.
        #[arg(long)]
        checkout: PathBuf,
        /// Absolute path to the file containing claims, or text.
        #[arg(long)]
        input: PathBuf,
        /// Emit results as JSON instead of human-readable text.
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Compile continuous invariant and architectural audits into an executive status dashboard.
    AuditRollup {
        /// Absolute path to the local worker's results directory.
        #[arg(long)]
        results_dir: PathBuf,
        /// Absolute path to the local worker's archive directory.
        #[arg(long)]
        archive_dir: PathBuf,
        /// Absolute path for the generated markdown status file.
        #[arg(long)]
        output: PathBuf,
    },
    /// Cross-reference a coordination queue against a consensus ledger and
    /// write a read-only readiness report. Never modifies the queue or the
    /// ledger; a human reviewer still performs the actual Done edit.
    ConsensusReadiness {
        #[arg(long)]
        queue: PathBuf,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        output: PathBuf,
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
    /// Write a code draft to an explicit, already-existing checkout and run a
    /// caller-scoped `cargo` verification command against it, capturing
    /// pass/fail as a versioned report. Never creates or initializes the
    /// checkout itself; the checkout must already exist.
    VerifyDraft {
        /// Absolute path to an already-existing Git checkout.
        #[arg(long)]
        checkout: PathBuf,
        /// Relative path (from checkout root) the draft is written to.
        #[arg(long)]
        target_file: PathBuf,
        /// Absolute path to a file containing the draft source to verify.
        #[arg(long)]
        draft_file: PathBuf,
        /// Verification command args passed to `cargo`, e.g. `check` or
        /// `test -p some_crate`.
        #[arg(long = "verify-command", num_args = 1..)]
        verify_command: Vec<String>,
        /// Absolute path for the versioned verification report file.
        #[arg(long)]
        output: PathBuf,
    },
    /// Write a code draft to an already-existing Aaroneous checkout and run
    /// that checkout's own `cratify audit` compliance pipeline against it
    /// (`cargo run -p cratify -- audit <cratify_target>`), capturing
    /// pass/fail as a versioned report. Thin convenience over `VerifyDraft`
    /// that saves callers from hand-constructing the cratify verify-command;
    /// never creates or initializes the checkout itself.
    CratifyAuditDraft {
        /// Absolute path to an already-existing Aaroneous Git checkout.
        #[arg(long)]
        checkout: PathBuf,
        /// Relative path (from checkout root) the draft is written to.
        #[arg(long)]
        target_file: PathBuf,
        /// Absolute path to a file containing the draft source to audit.
        #[arg(long)]
        draft_file: PathBuf,
        /// Relative path (from checkout root) passed to `cratify audit`,
        /// e.g. the crate/directory containing `target_file`.
        #[arg(long)]
        cratify_target: PathBuf,
        /// Absolute path for the versioned verification report file.
        #[arg(long)]
        output: PathBuf,
    },
    /// Write a code draft to an already-existing checkout and verify it
    /// against a named, selectable `ComplianceProfile` (M53) instead of a
    /// hand-constructed `verify_command`. `--profile aaroneous-strict` and
    /// `--profile devtools-operating-model` are deliberately not
    /// interchangeable — see `governance/REVIEW_CONDITIONS.md`'s devtools
    /// conditions table for which applies to which target. Never creates or
    /// initializes the checkout itself.
    VerifyDraftWithProfile {
        /// Absolute path to an already-existing Git checkout.
        #[arg(long)]
        checkout: PathBuf,
        /// Relative path (from checkout root) the draft is written to.
        #[arg(long)]
        target_file: PathBuf,
        /// Absolute path to a file containing the draft source to verify.
        #[arg(long)]
        draft_file: PathBuf,
        /// Which compliance ruleset to verify against.
        #[arg(long, value_enum)]
        profile: ProfileKind,
        /// Relative path (from checkout root) passed to `cratify audit`.
        /// Required (and only meaningful) when `--profile aaroneous-strict`.
        #[arg(long)]
        cratify_target: Option<PathBuf>,
        /// Verification command args passed to `cargo`. Required (and only
        /// meaningful) when `--profile custom`.
        #[arg(long = "verify-command", num_args = 0..)]
        verify_command: Vec<String>,
        /// Absolute path for the versioned verification report file.
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
    /// Record a human reviewer's accept/reject outcome for one
    /// `exploration_group` sibling job (M46). Never inferred or guessed by
    /// this tool: the reviewer supplies the outcome after comparing an
    /// `exploration_group`'s `*-exploration-summary.json` artifact.
    RecordExplorationOutcome {
        /// Absolute path to the local worker's results directory.
        #[arg(long)]
        results_dir: PathBuf,
        /// The `id` of the job being marked.
        #[arg(long)]
        job_id: String,
        /// `accepted` or `rejected`.
        #[arg(long)]
        outcome: String,
        /// Required when `outcome` is `rejected`: why this sibling lost.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Record an architectural branch selection from a Solution Tree, updating
    /// the result record and generating structured SFT/DPO decision traces.
    SelectBranch {
        /// Absolute path to the local worker's results directory.
        #[arg(long)]
        results_dir: PathBuf,
        /// The `id` of the job whose branch is being selected.
        #[arg(long)]
        job_id: String,
        /// The name of the chosen branch from the solution tree.
        #[arg(long)]
        branch: String,
        /// Detailed rationale explaining why this branch was chosen over alternatives.
        #[arg(long)]
        rationale: String,
        /// Directory where decision traces (SFT/DPO datasets) are stored.
        #[arg(
            long,
            default_value = "d:\\aaroneous-devtools\\artifacts\\decision-traces"
        )]
        traces_dir: PathBuf,
    },
    /// Cluster a harvested `cratify harvest --output <path>` `CrateSpec`
    /// export by module/directory boundary and emit one `analysis`
    /// worker-jobs packet per cluster that clears the G5 5,000-LOC
    /// refactoring-surface bound (M54). Clusters over the bound are not
    /// dropped -- they're recorded in the written report's `oversized` list
    /// so an unsplittable module is a visible finding, not a silent gap.
    DecomposeTarget {
        /// Absolute path to a `cratify harvest --output <path>` JSON export.
        #[arg(long)]
        crate_specs: PathBuf,
        /// Short label for the overall target being decomposed (e.g.
        /// "bevy_ecs"); becomes every emitted packet's `goal`, slugged into
        /// their shared `exploration_group`.
        #[arg(long)]
        target_name: String,
        /// Size threshold in total lines per cluster; a cluster over this
        /// does not get a packet and is recorded in the oversized report
        /// section instead. Defaults to 5,000 to match
        /// contracts/FRONTIER_FILTRATION_STANDARD_V1.md's G5 gate exactly.
        #[arg(long, default_value_t = 5000)]
        max_lines: usize,
        #[arg(long)]
        output_dir: PathBuf,
        /// Absolute path the decomposition summary report JSON is written
        /// to.
        #[arg(long)]
        report_output: PathBuf,
        /// Optional absolute path to the M57 accumulating per-candidate
        /// metadata directory. When omitted, behavior is exactly as before
        /// this milestone -- purely additive.
        #[arg(long)]
        metadata_dir: Option<PathBuf>,
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
    /// Explicitly accept a pending-owner-review draft: refuses a high-risk
    /// change outright, checks the circuit breaker, writes it on an
    /// isolated `local-agent/{task_id}` branch, re-verifies (cargo check +
    /// clippy), commits, and pushes that branch to origin. The only action
    /// that ever touches the real checkout or git. Never opens a pull
    /// request itself — see governance/LOCAL_MODEL_AUTONOMY_POLICY.md.
    Promote {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        task_id: String,
    },
    /// Clear a tripped promotion circuit breaker after investigating why it
    /// tripped. `reason` is recorded only in the printed confirmation.
    ResetCircuitBreaker {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        reason: String,
    },
    /// Cleanly revert an already-promoted commit (M50): validates `commit`
    /// is real and reachable from `main`, reverts it on a new isolated
    /// `local-agent/revert-{sha}` branch (never directly on `main`), pushes
    /// that branch, and records the revert (commit, reason, branch,
    /// timestamp) as a row in `revert_log`. Never opens a pull request
    /// itself — see governance/LOCAL_MODEL_AUTONOMY_POLICY.md. Deciding
    /// *when* a revert is warranted stays a human/cloud-agent call; this
    /// only executes one already decided.
    RevertPromotion {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        commit: String,
        #[arg(long)]
        reason: String,
        /// Absolute path to the revert evidence log, e.g.
        /// governance/REVERT_LOG.md. Created with an explanatory header on
        /// first use.
        #[arg(long)]
        revert_log: PathBuf,
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
            RegistrarAction::ResetCircuitBreaker { config, reason } => {
                agentic_registrar::reset_circuit_breaker(&config, &reason)
            }
            RegistrarAction::RevertPromotion {
                config,
                commit,
                reason,
                revert_log,
            } => agentic_registrar::revert_promotion(&config, &commit, &reason, &revert_log),
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
            metadata_dir,
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

                let prefilter = frontier_discovery::mechanical_prefilter_with_metadata(
                    metadata_dir.as_deref(),
                    &c.name,
                    &c.version,
                )?;
                if !prefilter.license_gate {
                    println!(
                        "      -> Skipped: failed G1 license gate (license: '{}')",
                        prefilter.license
                    );
                    continue;
                }

                let ecosystem_notes = match &prefilter.ecosystem_signal {
                    frontier_discovery::EcosystemSignal::Clean => None,
                    frontier_discovery::EcosystemSignal::Flagged(reasons) => {
                        println!("      -> G4 signal (advisory): {}", reasons.join("; "));
                        Some(reasons.join("; "))
                    }
                };

                let job_path = frontier_discovery::create_frontier_job_with_notes(
                    c,
                    &output_dir,
                    "docs/ARCHITECTURE.md",
                    &query,
                    ecosystem_notes.as_deref(),
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
        Command::CratificationIntake {
            crate_specs,
            output_dir,
        } => {
            println!(
                "Generating code-draft cratification packets from {}...",
                crate_specs.display()
            );
            let written = cratification_intake::run(&crate_specs, &output_dir)?;
            println!("Emitted {} cratification packet(s):", written.len());
            for path in &written {
                println!("  -> {}", path.display());
            }
            Ok(())
        }
        Command::DecomposeTarget {
            crate_specs,
            target_name,
            max_lines,
            output_dir,
            report_output,
            metadata_dir,
        } => {
            println!(
                "Decomposing target '{target_name}' from {} (max {max_lines} lines/cluster)...",
                crate_specs.display()
            );
            let report = target_decomposition::run_with_metadata(
                &crate_specs,
                &target_name,
                max_lines,
                &output_dir,
                &report_output,
                metadata_dir.as_deref(),
            )?;
            println!(
                "Found {} cluster(s): {} passed (packets emitted), {} oversized (see {})",
                report.total_clusters_found,
                report.passed_count,
                report.oversized_count,
                report_output.display()
            );
            for entry in &report.passed {
                if let Some(path) = &entry.packet_path {
                    println!("  -> {path}");
                }
            }
            for entry in &report.oversized {
                println!(
                    "  !! oversized: {} ({} lines, {} file(s))",
                    entry.dir, entry.total_lines, entry.file_count
                );
            }
            Ok(())
        }
        Command::SeedExploration {
            instruction_file,
            evidence,
            goal,
            perspectives,
            output_dir,
        } => {
            println!(
                "Seeding exploration round for goal '{goal}' from {}...",
                instruction_file.display()
            );
            let evidence: Vec<String> = evidence.iter().map(|p| p.display().to_string()).collect();
            let written = exploration_seed::run(
                &instruction_file,
                evidence,
                &goal,
                perspectives,
                &output_dir,
            )?;
            println!("Emitted {} exploration packet(s):", written.len());
            for path in &written {
                println!("  -> {}", path.display());
            }
            Ok(())
        }
        Command::FrontierDiscoverServe {
            queries,
            output_dir,
            limit,
            interval_minutes,
            metadata_dir,
        } => frontier_discovery::serve(
            &queries,
            &output_dir,
            limit,
            interval_minutes,
            metadata_dir.as_deref(),
        ),
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
        Command::LocalWorkerServe { config } => local_worker::serve(&config),
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
        Command::BatchOverlapCheck {
            candidates_dir,
            output,
        } => batch_overlap::check_directory(&candidates_dir, &output),
        Command::OutboxList {
            results,
            verdicts,
            output,
        } => outbox::run_list(&results, verdicts.as_deref(), &output),
        Command::OutboxShow {
            results,
            id,
            output,
        } => outbox::run_show(&results, &id, &output),
        Command::OutboxVerdict {
            results,
            verdicts,
            id,
            reviewer,
            verdict,
            notes,
        } => outbox::run_verdict(&results, &verdicts, &id, &reviewer, &verdict, &notes),
        Command::OutboxListVerdicts { verdicts, output } => {
            outbox::run_list_verdicts(&verdicts, &output)
        }
        Command::OutboxPromote {
            results,
            verdicts,
            registrar_config,
            id,
            crate_name,
            target_file,
            reviewer,
            justification,
            ping,
        } => outbox::run_promote(
            &results,
            &verdicts,
            &registrar_config,
            &id,
            &crate_name,
            &target_file,
            &reviewer,
            &justification,
            ping.as_deref(),
        ),
        Command::StageDraft {
            checkout,
            worktrees_root,
            id,
            target_file,
            draft_file,
            message,
            output,
        } => {
            let rust_source = fs::read_to_string(&draft_file)
                .with_context(|| format!("reading draft file {}", draft_file.display()))?;
            local_staging::run_stage_draft(
                &checkout,
                &worktrees_root,
                &id,
                &target_file,
                &rust_source,
                &message,
                &output,
            )
        }
        Command::ListStaged {
            worktrees_root,
            output,
        } => local_staging::run_list_staged(&worktrees_root, &output),
        Command::PruneStaged {
            checkout,
            worktrees_root,
            id,
        } => local_staging::prune_staged(&checkout, &worktrees_root, &id),
        Command::ScheduleReview {
            queue,
            program,
            output,
            primary_capacity,
            concurrence_report,
        } => scheduler::run(
            &queue,
            &program,
            &output,
            primary_capacity,
            concurrence_report.as_deref(),
        ),
        Command::PortfolioTriage {
            program,
            queue,
            output_dir,
        } => portfolio_jobs::run(&program, &queue, &output_dir).map(|_| ()),
        Command::PortfolioDigest {
            results_dir,
            archive_dir,
            output,
        } => portfolio_digest::generate_digest(&results_dir, archive_dir.as_deref(), &output)
            .map(|_| ()),
        Command::DiscoveryDigest {
            results_dir,
            archive_dir,
            output,
        } => discovery_digest::run(&results_dir, archive_dir.as_deref(), &output).map(|_| ()),
        Command::QueryCandidates {
            metadata_dir,
            like,
            kind,
            min_score,
            limit,
            json,
        } => candidate_retrieval::run(
            &metadata_dir,
            &like,
            kind.as_deref(),
            min_score,
            limit,
            json,
        ),
        Command::VerifyClaims {
            checkout,
            input,
            json,
        } => fact_checker::run_cli(&checkout, &input, json),
        Command::AuditRollup {
            results_dir,
            archive_dir,
            output,
        } => audit_rollup::run_cli(&results_dir, &archive_dir, &output),
        Command::ConsensusReadiness {
            queue,
            ledger,
            output,
        } => consensus_readiness::run(&queue, &ledger, &output),
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
        Command::VerifyDraft {
            checkout,
            target_file,
            draft_file,
            verify_command,
            output,
        } => {
            if !draft_file.is_absolute() {
                bail!(
                    "draft_file must be an absolute path: {}",
                    draft_file.display()
                );
            }
            let draft = intake::read_bounded(&draft_file, 65536)
                .with_context(|| format!("failed to read {}", draft_file.display()))?;
            dogfood::verify_draft(&checkout, &target_file, &draft, &verify_command, &output)
        }
        Command::CratifyAuditDraft {
            checkout,
            target_file,
            draft_file,
            cratify_target,
            output,
        } => {
            if !draft_file.is_absolute() {
                bail!(
                    "draft_file must be an absolute path: {}",
                    draft_file.display()
                );
            }
            let draft = intake::read_bounded(&draft_file, 65536)
                .with_context(|| format!("failed to read {}", draft_file.display()))?;
            dogfood::cratify_audit(&checkout, &target_file, &draft, &cratify_target, &output)
        }
        Command::VerifyDraftWithProfile {
            checkout,
            target_file,
            draft_file,
            profile,
            cratify_target,
            verify_command,
            output,
        } => {
            if !draft_file.is_absolute() {
                bail!(
                    "draft_file must be an absolute path: {}",
                    draft_file.display()
                );
            }
            let profile = match profile {
                ProfileKind::AaroneousStrict => {
                    let cratify_target = cratify_target.context(
                        "--cratify-target is required when --profile is aaroneous-strict",
                    )?;
                    dogfood::ComplianceProfile::AaroneousStrict { cratify_target }
                }
                ProfileKind::DevtoolsOperatingModel => {
                    if cratify_target.is_some() {
                        bail!(
                            "--cratify-target is not used when --profile is devtools-operating-model"
                        );
                    }
                    dogfood::ComplianceProfile::DevtoolsOperatingModel
                }
                ProfileKind::Custom => {
                    if verify_command.is_empty() {
                        bail!("--verify-command is required when --profile is custom");
                    }
                    dogfood::ComplianceProfile::Custom(verify_command)
                }
            };
            let draft = intake::read_bounded(&draft_file, 65536)
                .with_context(|| format!("failed to read {}", draft_file.display()))?;
            dogfood::verify_draft_with_profile(&checkout, &target_file, &draft, &profile, &output)
        }
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
        Command::RecordExplorationOutcome {
            results_dir,
            job_id,
            outcome,
            reason,
        } => local_worker::record_exploration_outcome(&results_dir, &job_id, &outcome, reason),
        Command::SelectBranch {
            results_dir,
            job_id,
            branch,
            rationale,
            traces_dir,
        } => decision_trace::record_branch_selection(
            &results_dir,
            &job_id,
            &branch,
            &rationale,
            &traces_dir,
        )
        .map(|_| ()),
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
