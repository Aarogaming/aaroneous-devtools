use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

/// One row from `governance/MULTITHREAD_PROGRAM.md`'s "Priority portfolio"
/// table.
#[derive(Debug, Clone, PartialEq)]
struct PortfolioRow {
    priority: String,
    id: String,
    work: String,
    dependency: String,
    preferred_capability: String,
}

/// Parse the Priority portfolio table's rows. Mirrors `scheduler.rs`'s own
/// `queue_rows` style: split each `|`-delimited line into trimmed fields,
/// skip anything that isn't a real data row (header/separator rows, or a
/// first data column that isn't a bare ID token).
fn portfolio_rows(input: &str) -> Vec<PortfolioRow> {
    input
        .lines()
        .filter_map(|line| {
            let fields = line.split('|').map(str::trim).collect::<Vec<_>>();
            if fields.len() < 6 {
                return None;
            }
            let priority = fields[1];
            let id = fields[2];
            if priority.is_empty()
                || priority == "Priority"
                || priority.chars().all(|c| c == '-')
                || id.is_empty()
                || !id.chars().all(|c| c.is_ascii_alphanumeric())
            {
                return None;
            }
            Some(PortfolioRow {
                priority: priority.to_owned(),
                id: id.to_owned(),
                work: fields[3].to_owned(),
                dependency: fields[4].to_owned(),
                preferred_capability: fields[5].to_owned(),
            })
        })
        .collect()
}

/// Allowlist of open, unclaimed-by-default priority values. Anything else
/// (`Done`, `Retired`, or an unexpected future status) is skipped by
/// default rather than included -- an allowlist, not a denylist.
fn is_open_priority(priority: &str) -> bool {
    matches!(priority, "P0" | "P1" | "P2" | "P3")
}

/// Extract the `Scope` text of every `governance/COORDINATION_QUEUE.md` row
/// whose `Status` is `Claimed` or `Review`. Mirrors `local_worker.rs`'s own
/// `scope_reserved` table-parsing approach (split on `|`, trim fields).
fn claimed_or_review_scopes(input: &str) -> Vec<String> {
    input
        .lines()
        .filter_map(|line| {
            let fields = line.split('|').map(str::trim).collect::<Vec<_>>();
            if fields.len() < 4 {
                return None;
            }
            let id = fields[1];
            let status = fields[3];
            if id.is_empty() || id == "ID" || id.chars().all(|c| c == '-') {
                return None;
            }
            if matches!(status, "Claimed" | "Review") {
                Some(fields[2].to_owned())
            } else {
                None
            }
        })
        .collect()
}

/// Drop any portfolio row whose `id` (an `M##` token) appears as a
/// substring of any already-claimed/reviewed queue row's `Scope` text. This
/// mirrors how `local_worker.rs`'s existing `scope_reserved` does textual
/// matching against the queue rather than requiring an exact ID-column
/// match -- the two ID namespaces (`M##` portfolio IDs, `C##` queue IDs)
/// are different, and a queue row's `Scope` prose is where an `M##`
/// reference actually shows up (see e.g. queue row C35's Scope text, which
/// names "M40-M47" directly).
fn already_claimed(id: &str, claimed_scopes: &[String]) -> bool {
    claimed_scopes.iter().any(|scope| scope.contains(id))
}

/// Slugify a portfolio ID into an id-safe `exploration_group` key, mirroring
/// `frontier_discovery.rs`'s `slugify_query`. Unlike M46/M55's grouping
/// (where multiple candidates found for one query intentionally share a
/// group), each portfolio row here is an independent triage request, so
/// every surviving row gets its own group -- there's no shared join key.
fn slugify_id(id: &str) -> String {
    let mut slug = String::with_capacity(id.len());
    let mut last_was_sep = true;
    for ch in id.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_sep = false;
        } else if !last_was_sep {
            slug.push('_');
            last_was_sep = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    slug
}

/// Create one `worker-jobs/*.toml` analysis packet requesting a speculative
/// proposed approach for one surviving portfolio row. This is explicitly
/// **not** a claim: the local worker is proposal-only per
/// `governance/LOCAL_AGENT_CONTROL_PLANE.md`, and the emitted instruction
/// says so directly so nothing downstream could mistake this for
/// authorization to touch the row's actual scope.
fn create_portfolio_job(row: &PortfolioRow, output_dir: &Path) -> Result<PathBuf> {
    if !output_dir.is_absolute() {
        bail!("output_dir must be an absolute path");
    }

    let sanitized_id = row.id.to_uppercase();
    let job_id = format!("PORTFOLIO-{sanitized_id}");
    let filename = format!("portfolio-{}.toml", row.id.to_lowercase());
    let destination = output_dir.join(&filename);

    let instruction = format!(
        "Portfolio triage request for {id} (priority {priority}) from governance/MULTITHREAD_PROGRAM.md's Priority portfolio table.\n\
        Work: \"{work}\"\n\
        Dependency: \"{dependency}\"\n\
        Preferred capability: \"{preferred_capability}\"\n\n\
        This is a request for a speculative proposed approach only. It is NOT a claim on {id}, \
        does not authorize writing product code, and does not authorize touching the row's actual \
        scope -- claiming real work still goes through governance/COORDINATION_QUEUE.md per \
        governance/LOCAL_AGENT_CONTROL_PLANE.md.\n\n\
        Please propose: (1) a possible approach or plan for this work; (2) which files it would \
        likely touch; (3) risks or open questions; (4) based on your own read, whether this looks \
        like something worth a human or cloud session actually claiming.",
        id = row.id,
        priority = row.priority,
        work = row.work,
        dependency = row.dependency,
        preferred_capability = row.preferred_capability,
    );

    let manifest = serde_json::json!({
        "schema_version": 1,
        "id": job_id,
        "mode": "analysis",
        "evidence": ["governance/MULTITHREAD_PROGRAM.md"],
        "instruction": instruction,
        "perspective": "portfolio-triage",
        "goal": row.id,
        "exploration_group": slugify_id(&row.id),
    });

    let toml_str = toml::to_string_pretty(&manifest)?;
    fs::write(&destination, toml_str)?;
    Ok(destination)
}

fn require_absolute_file(label: &str, path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("{label} must be an absolute path: {}", path.display());
    }
    if !fs::metadata(path)
        .with_context(|| format!("{label} is unavailable: {}", path.display()))?
        .is_file()
    {
        bail!("{label} must be a file: {}", path.display());
    }
    Ok(())
}

fn require_absolute_dir(label: &str, path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("{label} must be an absolute path: {}", path.display());
    }
    if !fs::metadata(path)
        .with_context(|| format!("{label} is unavailable: {}", path.display()))?
        .is_dir()
    {
        bail!("{label} must be a directory: {}", path.display());
    }
    Ok(())
}

/// Read `program` and `queue` (explicit absolute paths), cross-reference the
/// Priority portfolio table against already-claimed/reviewed queue scopes,
/// and write one analysis packet per surviving row into `output_dir`.
/// Returns the paths of every packet written.
pub(crate) fn run(program: &Path, queue: &Path, output_dir: &Path) -> Result<Vec<PathBuf>> {
    require_absolute_file("program", program)?;
    require_absolute_file("queue", queue)?;
    require_absolute_dir("output_dir", output_dir)?;

    let program_text = fs::read_to_string(program)
        .with_context(|| format!("failed to read {}", program.display()))?;
    let queue_text =
        fs::read_to_string(queue).with_context(|| format!("failed to read {}", queue.display()))?;

    let claimed_scopes = claimed_or_review_scopes(&queue_text);
    let rows = portfolio_rows(&program_text)
        .into_iter()
        .filter(|row| is_open_priority(&row.priority))
        .filter(|row| !already_claimed(&row.id, &claimed_scopes));

    let mut written = Vec::new();
    for row in rows {
        let path = create_portfolio_job(&row, output_dir)?;
        println!("wrote portfolio triage job: {}", path.display());
        written.push(path);
    }
    Ok(written)
}

/// Create a progressive depth analysis packet for a milestone that has
/// completed its earlier triage pass. Injects findings from Pass 1 into
/// Pass 2 (Detailed Specification & Test Architecture) or Pass 2 into
/// Pass 3 (Reference Model & Invariant Proofs).
fn create_progressive_portfolio_job(
    row: &PortfolioRow,
    pass: u32,
    prior_summary: &str,
    output_dir: &Path,
) -> Result<PathBuf> {
    if !output_dir.is_absolute() {
        bail!("output_dir must be an absolute path");
    }

    let sanitized_id = row.id.to_uppercase();
    let job_id = format!("PORTFOLIO-{sanitized_id}-P{pass:04}");
    let filename = format!("portfolio-{}-p{pass:04}.toml", row.id.to_lowercase());
    let destination = output_dir.join(&filename);

    let context_block = if prior_summary.is_empty() {
        String::new()
    } else {
        format!(
            "\n[PRIOR TRIAGE CONTEXT - Pass {}]:\n{}\n",
            pass - 1,
            prior_summary
        )
    };

    let (instruction, mode) = if pass == 2 {
        (
            format!(
                "Portfolio deep-dive request for {id} (priority {priority}, Pass 2: Detailed Specification & Test Architecture) from governance/MULTITHREAD_PROGRAM.md's Priority portfolio table.\n\
            Work: \"{work}\"\n\
            Dependency: \"{dependency}\"\n\
            Preferred capability: \"{preferred_capability}\"\n\
            {context_block}\n\
            This is a request for a speculative proposed technical specification only. It is NOT a claim on {id}, \
            does not authorize writing product code, and does not authorize touching the row's actual scope -- \
            claiming real work still goes through governance/COORDINATION_QUEUE.md per governance/LOCAL_AGENT_CONTROL_PLANE.md.\n\n\
            Building upon Pass 1's triage recommendations, please propose a Solution Tree with 2-3 distinct architectural branches (e.g. Branch-A: Compile-time typestate/zero-alloc, Branch-B: Bounded SWMR ring buffer, Branch-C: Decoupled adapter facade):\n\
            (1) Concrete architectural design and trait/struct interface boundaries for each branch;\n\
            (2) Invariants that must hold across state transitions (zero-heap, zero ambient authority, deterministic state machines per AGENTS.md);\n\
            (3) Concrete trade-offs, blast radius, and code sketches comparing the branches in solution_tree;\n\
            (4) Detailed checklist of step-by-step implementation phases for the recommended branch.",
                id = row.id,
                priority = row.priority,
                work = row.work,
                dependency = row.dependency,
                preferred_capability = row.preferred_capability,
                context_block = context_block,
            ),
            "analysis",
        )
    } else if pass == 3 {
        (
            format!(
                "Portfolio progressive verification request for {id} (priority {priority}, Pass {pass}: Reference Architecture & Contract Proofs) from governance/MULTITHREAD_PROGRAM.md's Priority portfolio table.\n\
            Work: \"{work}\"\n\
            Dependency: \"{dependency}\"\n\
            Preferred capability: \"{preferred_capability}\"\n\
            {context_block}\n\
            This is a request for speculative reference modeling only. It is NOT a claim on {id}, \
            does not authorize writing product code, and does not authorize touching the row's actual scope.\n\n\
            Please propose:\n\
            (1) A draft reference implementation sketch (in rust_draft or solution_tree) or mock interface;\n\
            (2) Proof that the design satisfies sequential verification gates and AST invariants (zero-heap, zero ambient authority);\n\
            (3) Isolated test fixture scenarios and failure reproduction cases.",
                id = row.id,
                priority = row.priority,
                work = row.work,
                dependency = row.dependency,
                preferred_capability = row.preferred_capability,
                context_block = context_block,
            ),
            "analysis",
        )
    } else {
        (
            format!(
                "Portfolio progressive verification request for {id} (priority {priority}, Pass {pass}: Isolated Prototype Implementation & AST Invariant Proofs) from governance/MULTITHREAD_PROGRAM.md's Priority portfolio table.\n\
            Work: \"{work}\"\n\
            Dependency: \"{dependency}\"\n\
            Preferred capability: \"{preferred_capability}\"\n\
            {context_block}\n\
            Synthesize a self-contained, complete Rust draft implementation for this milestone's selected architecture.\n\
            CONSTRAINTS:\n\
            - Strictly zero heap allocations on hot paths (no String, Vec, Box, format!, or unbounded collections).\n\
            - Strictly zero ambient authority (no std::env::*, no direct clock reads outside inputs, explicit constructor injection).\n\
            - Zero stubs (no todo! or unimplemented!). All error paths must propagate Result.\n\
            - Include thorough unit test assertions proving determinism and contract compliance in test_plan and code.",
                id = row.id,
                priority = row.priority,
                work = row.work,
                dependency = row.dependency,
                preferred_capability = row.preferred_capability,
                context_block = context_block,
            ),
            "code-draft",
        )
    };

    let manifest = serde_json::json!({
        "schema_version": 1,
        "id": job_id,
        "mode": mode,
        "evidence": ["governance/MULTITHREAD_PROGRAM.md"],
        "instruction": instruction,
        "perspective": "portfolio-triage",
        "goal": row.id,
        "exploration_group": slugify_id(&row.id),
    });

    let toml_str = toml::to_string_pretty(&manifest)?;
    fs::write(&destination, toml_str)?;
    Ok(destination)
}

/// Automatically refills progressive depth passes for unblocked milestones
/// when previous passes have completed. Bounded by `max_batch` to avoid flooding.
pub(crate) fn refill_portfolio_progression_jobs(
    program: &Path,
    queue: &Path,
    output_dir: &Path,
    max_batch: usize,
) -> Result<Vec<PathBuf>> {
    require_absolute_file("program", program)?;
    require_absolute_file("queue", queue)?;
    require_absolute_dir("output_dir", output_dir)?;

    let program_text = fs::read_to_string(program)
        .with_context(|| format!("failed to read {}", program.display()))?;
    let queue_text =
        fs::read_to_string(queue).with_context(|| format!("failed to read {}", queue.display()))?;

    let claimed_scopes = claimed_or_review_scopes(&queue_text);
    let rows: Vec<_> = portfolio_rows(&program_text)
        .into_iter()
        .filter(|row| is_open_priority(&row.priority))
        .filter(|row| !already_claimed(&row.id, &claimed_scopes))
        .collect();

    let mut candidate_dirs = vec![output_dir.join("results"), output_dir.join("archive")];
    if let Some(parent) = output_dir.parent() {
        candidate_dirs.push(parent.join("results"));
        candidate_dirs.push(parent.join("archive"));
        candidate_dirs.push(
            parent
                .join("artifacts")
                .join("local-worker-service")
                .join("results"),
        );
        candidate_dirs.push(
            parent
                .join("artifacts")
                .join("local-worker-service")
                .join("archive"),
        );
    }

    // Map: milestone_id -> (highest_completed_pass, latest_summary)
    let mut milestone_progress: std::collections::HashMap<String, (u32, String)> =
        std::collections::HashMap::new();

    for dir in candidate_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(text) = fs::read_to_string(&path) {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(job_id) = value["job_id"].as_str() {
                                let upper = job_id.to_uppercase();
                                if let Some(stripped) = upper.strip_prefix("PORTFOLIO-") {
                                    let (m_id, pass) = if let Some(idx) = stripped.rfind("-P") {
                                        let m = &stripped[..idx];
                                        let p = stripped[idx + 2..].parse::<u32>().unwrap_or(1);
                                        (m.to_string(), p)
                                    } else {
                                        (stripped.to_string(), 1)
                                    };

                                    let mut summary = String::new();
                                    if let Some(proposal) =
                                        value.get("proposal").and_then(|p| p.as_object())
                                    {
                                        let mut parts = Vec::new();
                                        if let Some(next_step) = proposal
                                            .get("recommended_next_step")
                                            .and_then(|s| s.as_str())
                                        {
                                            parts.push(format!("Recommendation: {next_step}"));
                                        }
                                        if let Some(obs) =
                                            proposal.get("observations").and_then(|v| v.as_array())
                                        {
                                            for o in obs.iter().filter_map(|x| x.as_str()) {
                                                parts.push(format!("Obs: {o}"));
                                            }
                                        }
                                        if let Some(selected) =
                                            value.get("selected_branch").and_then(|s| s.as_str())
                                        {
                                            let rat = value
                                                .get("selection_rationale")
                                                .and_then(|s| s.as_str())
                                                .unwrap_or("");
                                            parts.push(format!("Selected Architecture: {selected} | Rationale: {rat}"));
                                        } else if let Some(tree) =
                                            proposal.get("solution_tree").and_then(|t| t.as_array())
                                        {
                                            let branch_summaries: Vec<_> = tree
                                                .iter()
                                                .filter_map(|b| {
                                                    let obj = b.as_object()?;
                                                    let name = obj
                                                        .get("branch_name")
                                                        .and_then(|s| s.as_str())?;
                                                    let app = obj
                                                        .get("approach")
                                                        .and_then(|s| s.as_str())?;
                                                    Some(format!("{name}: {app}"))
                                                })
                                                .collect();
                                            if !branch_summaries.is_empty() {
                                                parts.push(format!(
                                                    "Proposed Branches: {}",
                                                    branch_summaries.join("; ")
                                                ));
                                            }
                                        }
                                        summary = parts.join(" | ");
                                    }

                                    let entry = milestone_progress
                                        .entry(m_id)
                                        .or_insert((0, String::new()));
                                    if pass >= entry.0 {
                                        entry.0 = pass;
                                        if !summary.is_empty() {
                                            entry.1 = summary;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check existing files in output_dir
    let existing_files: std::collections::HashSet<String> = fs::read_dir(output_dir)
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    if name.ends_with(".toml") {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let mut written = Vec::new();

    // Priority sort: P0 first, then P1, P2, P3
    let mut sorted_rows = rows;
    sorted_rows.sort_by_key(|r| match r.priority.as_str() {
        "P0" => 0,
        "P1" => 1,
        "P2" => 2,
        "P3" => 3,
        _ => 4,
    });

    for row in sorted_rows {
        if written.len() >= max_batch {
            break;
        }
        let m_id = row.id.to_uppercase();
        let (completed_pass, summary) = milestone_progress
            .get(&m_id)
            .cloned()
            .unwrap_or((0, String::new()));

        // Advance to next pass if previous pass completed and next pass file doesn't exist
        let next_pass = completed_pass + 1;
        if next_pass > 4 {
            // Bounded to 4 progressive depth tiers per milestone to prevent unbounded expansion
            continue;
        }

        let target_filename = if next_pass == 1 {
            format!("portfolio-{}.toml", row.id.to_lowercase())
        } else {
            format!("portfolio-{}-p{next_pass:04}.toml", row.id.to_lowercase())
        };

        if existing_files.contains(&target_filename) || output_dir.join(&target_filename).exists() {
            continue;
        }

        let job_path = if next_pass == 1 {
            create_portfolio_job(&row, output_dir)?
        } else {
            create_progressive_portfolio_job(&row, next_pass, &summary, output_dir)?
        };

        println!("refilled progressive portfolio job: {}", job_path.display());
        written.push(job_path);
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROGRAM_FIXTURE: &str = "## Priority portfolio\n\n\
        | Priority | ID | Work | Dependency | Preferred capability |\n\
        | --- | --- | --- | --- | --- |\n\
        | P0 | M1 | Converge and verify the active integration branch | CI result | Codex integration |\n\
        | Done | M2 | Port the text-encoding checker | C1 | Rust CLI / tests |\n\
        | P1 | M8 | Establish the bounded reducer reference path | A2 | Deterministic systems testing |\n\
        | P2 | M10 | Enforce execution-host dependency policy | M8 | Architecture and Cargo policy |\n\
        | Retired | M24 | Consolidate roadmap docs | none | Documentation architecture |\n";

    #[test]
    fn portfolio_rows_parses_real_shaped_rows() {
        let rows = portfolio_rows(PROGRAM_FIXTURE);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].priority, "P0");
        assert_eq!(rows[0].id, "M1");
        assert_eq!(
            rows[0].work,
            "Converge and verify the active integration branch"
        );
        assert_eq!(rows[0].dependency, "CI result");
        assert_eq!(rows[0].preferred_capability, "Codex integration");
    }

    #[test]
    fn is_open_priority_allowlist() {
        assert!(is_open_priority("P0"));
        assert!(is_open_priority("P1"));
        assert!(is_open_priority("P2"));
        assert!(is_open_priority("P3"));
        assert!(!is_open_priority("Done"));
        assert!(!is_open_priority("Retired"));
        assert!(!is_open_priority("SomeFuturePriority"));
    }

    #[test]
    fn claimed_or_review_scopes_extracts_only_matching_status() {
        let queue = "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |\n\
            | --- | --- | --- | --- | --- | --- | --- |\n\
            | C35 | M40-M47 local/cloud orchestration plan | Review | Someone | branch | 2026-09-20T | evidence |\n\
            | C10 | M10 execution-host dependency policy | Claimed | Someone | branch | 2026-09-19T | evidence |\n\
            | C1 | M2 text-encoding checker | Done | Someone | main | 2026-09-01T | evidence |\n";
        let scopes = claimed_or_review_scopes(queue);
        assert_eq!(scopes.len(), 2);
        assert!(scopes[0].contains("M40-M47"));
        assert!(scopes[1].contains("M10"));
    }

    #[test]
    fn run_p1_row_not_claimed_produces_a_job() {
        let temp = tempfile::tempdir().expect("temp dir");
        let program = temp.path().join("program.md");
        let queue = temp.path().join("queue.md");
        let output_dir = temp.path().join("worker-jobs");
        fs::create_dir_all(&output_dir).expect("create output dir");
        fs::write(&program, PROGRAM_FIXTURE).expect("write program fixture");
        fs::write(
            &queue,
            "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n",
        )
        .expect("write queue fixture");

        let written = run(&program, &queue, &output_dir).expect("run");
        let m8_job = written
            .iter()
            .find(|p| p.file_name().unwrap().to_string_lossy().contains("m8"))
            .expect("M8 job written");
        let text = fs::read_to_string(m8_job).expect("read job");
        assert!(text.contains("PORTFOLIO-M8"));
        assert!(text.contains("mode = \"analysis\""));
    }

    #[test]
    fn run_done_row_produces_no_job() {
        let temp = tempfile::tempdir().expect("temp dir");
        let program = temp.path().join("program.md");
        let queue = temp.path().join("queue.md");
        let output_dir = temp.path().join("worker-jobs");
        fs::create_dir_all(&output_dir).expect("create output dir");
        fs::write(&program, PROGRAM_FIXTURE).expect("write program fixture");
        fs::write(
            &queue,
            "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n",
        )
        .expect("write queue fixture");

        let written = run(&program, &queue, &output_dir).expect("run");
        assert!(
            !written
                .iter()
                .any(|p| p.file_name().unwrap().to_string_lossy().contains("m2"))
        );
    }

    #[test]
    fn run_claimed_row_produces_no_job() {
        let temp = tempfile::tempdir().expect("temp dir");
        let program = temp.path().join("program.md");
        let queue = temp.path().join("queue.md");
        let output_dir = temp.path().join("worker-jobs");
        fs::create_dir_all(&output_dir).expect("create output dir");
        fs::write(&program, PROGRAM_FIXTURE).expect("write program fixture");
        fs::write(
            &queue,
            "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n\
             | C10 | M10 execution-host dependency policy | Claimed | Someone | branch | 2026-09-19T | evidence |\n",
        )
        .expect("write queue fixture");

        let written = run(&program, &queue, &output_dir).expect("run");
        assert!(
            !written
                .iter()
                .any(|p| p.file_name().unwrap().to_string_lossy().contains("m10"))
        );
        // M8 (P1, not claimed) should still be produced.
        assert!(
            written
                .iter()
                .any(|p| p.file_name().unwrap().to_string_lossy().contains("m8"))
        );
    }

    #[test]
    fn job_instruction_contains_id_and_not_a_claim_statement() {
        let temp = tempfile::tempdir().expect("temp dir");
        let output_dir = temp.path().to_path_buf();
        let row = PortfolioRow {
            priority: "P1".to_owned(),
            id: "M8".to_owned(),
            work: "Establish the bounded reducer reference path".to_owned(),
            dependency: "A2".to_owned(),
            preferred_capability: "Deterministic systems testing".to_owned(),
        };
        let job_path = create_portfolio_job(&row, &output_dir).expect("create job");
        let text = fs::read_to_string(&job_path).expect("read job");
        assert!(text.contains("mode = \"analysis\""));
        assert!(text.contains("M8"));
        assert!(text.contains("NOT a claim") || text.contains("not a claim"));
        assert!(text.contains("does not authorize"));
    }

    #[test]
    fn multiple_surviving_rows_get_distinct_exploration_groups() {
        let temp = tempfile::tempdir().expect("temp dir");
        let program = temp.path().join("program.md");
        let queue = temp.path().join("queue.md");
        let output_dir = temp.path().join("worker-jobs");
        fs::create_dir_all(&output_dir).expect("create output dir");
        fs::write(&program, PROGRAM_FIXTURE).expect("write program fixture");
        fs::write(
            &queue,
            "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n",
        )
        .expect("write queue fixture");

        let written = run(&program, &queue, &output_dir).expect("run");
        assert!(written.len() >= 2);

        let mut groups = Vec::new();
        for path in &written {
            let text = fs::read_to_string(path).expect("read job");
            let value: toml::Value = toml::from_str(&text).expect("parse toml");
            groups.push(
                value["exploration_group"]
                    .as_str()
                    .expect("exploration_group present")
                    .to_owned(),
            );
        }
        let unique: std::collections::HashSet<_> = groups.iter().collect();
        assert_eq!(
            unique.len(),
            groups.len(),
            "each row must get its own exploration_group"
        );
    }

    #[test]
    fn test_refill_portfolio_progression_advances_to_pass_2() {
        let temp = tempfile::tempdir().expect("temp dir");
        let program = temp.path().join("program.md");
        let queue = temp.path().join("queue.md");
        let output_dir = temp.path().join("worker-jobs");
        let results_dir = output_dir.join("results");
        fs::create_dir_all(&output_dir).expect("create output dir");
        fs::create_dir_all(&results_dir).expect("create results dir");

        fs::write(&program, PROGRAM_FIXTURE).expect("write program");
        fs::write(
            &queue,
            "| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n",
        )
        .expect("write queue");

        // Simulate that M8 has completed Pass 1
        let pass_1_record = serde_json::json!({
            "schema_version": 1,
            "job_id": "PORTFOLIO-M8",
            "outcome": "pending-owner-review",
            "submitted_at_secs": 1000,
            "proposal": {
                "task": "PORTFOLIO-M8",
                "observations": ["Reference path needs deterministic state transitions"],
                "risks": ["Dependency on A2"],
                "test_plan": ["Run reducer harness"],
                "recommended_next_step": "Define reducer trait boundary"
            }
        });
        fs::write(
            results_dir.join("PORTFOLIO-M8-0000.json"),
            serde_json::to_string_pretty(&pass_1_record).unwrap(),
        )
        .expect("write result");

        let refilled =
            refill_portfolio_progression_jobs(&program, &queue, &output_dir, 2).expect("refill");
        assert!(!refilled.is_empty());

        let m8_p2 = refilled.iter().find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("m8-p0002")
        });
        assert!(m8_p2.is_some(), "M8 must progress to Pass 2");

        let text = fs::read_to_string(m8_p2.unwrap()).expect("read m8-p0002");
        assert!(text.contains("PORTFOLIO-M8-P0002"));
        assert!(text.contains("Pass 2: Detailed Specification"));
        assert!(text.contains("PRIOR TRIAGE CONTEXT"));
        assert!(text.contains("Define reducer trait boundary"));
    }
}
