use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize)]
struct JobManifest {
    schema_version: u32,
    id: String,
    mode: String,
    evidence: Vec<String>,
    instruction: String,
}

pub(crate) struct CatalogItem {
    pub id: &'static str,
    pub mode: &'static str,
    pub evidence: &'static [&'static str],
    pub instruction: &'static str,
}

pub(crate) fn recurring_catalog() -> &'static [CatalogItem] {
    &[
        CatalogItem {
            id: "AUDIT-01-AST-SAFETY",
            mode: "analysis",
            evidence: &[
                "crates/ast_auditor/src/rules/text_encoding.rs",
                "xtask/src/gate.rs",
            ],
            instruction: "Audit AST auditor rules, UTF-8/LF line ending enforcement, and gate command alignment between xtask/src/gate.rs and AGENTS.md Section 6.",
        },
        CatalogItem {
            id: "AUDIT-02-HOT-PATH-ALLOC",
            mode: "analysis",
            evidence: &["core/hypervisor/src/lib.rs"],
            instruction: "Audit hypervisor core module for forbidden dynamic heap allocations (String, Vec, Box, HashMap), unwraps/panics, and missing zero-copy/#[repr(C)] memory alignment invariants.",
        },
        CatalogItem {
            id: "AUDIT-03-ZERO-AMBIENT-AUTH",
            mode: "analysis",
            evidence: &["crates/paths/src/lib.rs"],
            instruction: "Audit paths crate for ambient authority violations (banned functions std::env::var, temp_dir, canonicalize). Verify path normalization via paths::normalize_path and constructor injection.",
        },
        CatalogItem {
            id: "AUDIT-04-DEPRECATION-POLICY",
            mode: "analysis",
            evidence: &[
                "crates/adaptation_plane/src/capability_spec.rs",
                "docs/DEPRECATION_POLICY.md",
            ],
            instruction: "Audit deprecated biological/mythological type aliases against docs/DEPRECATION_POLICY.md. Verify zero production call sites remain and migration equivalence tests pass.",
        },
        CatalogItem {
            id: "AUDIT-05-TEMPORAL-SYNC",
            mode: "analysis",
            evidence: &[
                "crates/omni/src/matrix/sab_matrix.rs",
                "governance/TEMPORAL_TEST_SYNCHRONIZATION_GUIDANCE.md",
            ],
            instruction: "Audit test suites for fixed sleep hazard calls (thread::sleep, tokio::time::sleep). Verify deterministic synchronization (FileTimes backdating, join barriers, bounded polling).",
        },
        CatalogItem {
            id: "AUDIT-06-FOUNDATION-CLI",
            mode: "analysis",
            evidence: &["foundation/crates/foundation_cli/src/local_worker.rs"],
            instruction: "Audit local worker and foundation CLI for lock safety, response validation, transport bounds, and safe path traversal without parent directory escaping.",
        },
        CatalogItem {
            id: "AUDIT-07-WIRE-ABI-GEOMETRY",
            mode: "analysis",
            evidence: &["crates/ipc_bus/src/wire.rs", "crates/wire/src/lib.rs"],
            instruction: "Audit IPC bus and wire parser structs for #[repr(C)], bytemuck Pod/Zeroable derives, explicit padding fields (_pad0), and zero-copy SWMR ring buffer guarantees.",
        },
        CatalogItem {
            id: "AUDIT-08-ROADMAP-INVARIANTS",
            mode: "analysis",
            evidence: &["docs/roadmap.md", "governance/EVIDENCE_STANDARD.md"],
            instruction: "Audit roadmap work claims and milestone status labels against codebase evidence per governance/EVIDENCE_STANDARD.md.",
        },
        CatalogItem {
            id: "AUDIT-09-HOT-PATH-REDUCERS",
            mode: "analysis",
            evidence: &["dev/emulator_harness/src/lib.rs"],
            instruction: "Audit compute engine and emulator harness for zero-allocation trace reduction, bounded execution budgets, and deterministic trace replay guarantees.",
        },
        CatalogItem {
            id: "AUDIT-10-ORCHESTRATOR-BOUNDS",
            mode: "analysis",
            evidence: &["crates/orchestrator/src/lib.rs"],
            instruction: "Audit orchestrator crate for ambient authority violations, task routing bounds, and supervisor budget enforcement.",
        },
        CatalogItem {
            id: "PREP-01-IPC-BUS-RING-BUFFER",
            mode: "analysis",
            evidence: &["crates/ipc_bus/src/lib.rs", "crates/ipc_bus/src/wire.rs"],
            instruction: "Research and investigate IPC bus ring buffer zero-copy contract, lock-free memory alignment, and SWMR read/write invariants.",
        },
        CatalogItem {
            id: "PREP-02-COMPUTE-ENGINE-CONTAINER",
            mode: "analysis",
            evidence: &["crates/compute/src/lib.rs"],
            instruction: "Research and investigate compute engine JIT and .si container engine interfaces, execution bounds, and allocation limits.",
        },
        CatalogItem {
            id: "PREP-03-GOVERNANCE-SMT-INTERLOCKS",
            mode: "analysis",
            evidence: &["crates/governance/src/lib.rs"],
            instruction: "Research and investigate governance Z3 SMT verification rules, safety interlocks, and state reduction constraints.",
        },
        CatalogItem {
            id: "PREP-04-ORCHESTRATOR-TASK-SCHEDULER",
            mode: "analysis",
            evidence: &["crates/orchestrator/src/lib.rs"],
            instruction: "Research and investigate task scheduler core affinity, execution budgets, and supervisor tick boundaries.",
        },
        CatalogItem {
            id: "PREP-05-MAINTENANCE-UI-CONTRACTS",
            mode: "analysis",
            evidence: &[
                "foundation/crates/maintenance_ui/src/lib.rs",
                "governance/UTILITY.md",
            ],
            instruction: "Research and investigate maintenance UI packet server handlers, authentication constraints, and HTTP status contracts.",
        },
        CatalogItem {
            id: "PREP-06-STUDIO-HUD-PANIC-CONTAINMENT",
            mode: "analysis",
            evidence: &[
                "crates/studio_hud/src/plugin_api.rs",
                "docs/rfcs/RFC-0006-PLUGIN_LIFECYCLE_AND_STABLE_UI_CARTRIDGE_ABI.md",
            ],
            instruction: "Research and investigate studio_hud FFI boundary panic containment, catch_unwind wrappers, and TickResult fault handling.",
        },
        CatalogItem {
            id: "AUDIT-11-GIT-DISCOVERY",
            mode: "analysis",
            evidence: &["Cargo.toml", "docs/ARCHITECTURE.md"],
            instruction: "Perform Git discovery and architectural frontier audit: analyze workspace structure, discover open-source Rust patterns and candidate crates for dogfooding, and evaluate architectural influences without circular dependencies or ambient authority violations.",
        },
        CatalogItem {
            id: "FRONTIER-01-REPO-OUTSOURCING",
            mode: "analysis",
            evidence: &["docs/ARCHITECTURE.md", "governance/COORDINATION_QUEUE.md"],
            instruction: "Audit open-source repository opportunities and external crate outsourcing: identify mature ecosystem components (e.g. tracing, bytemuck, lock-free queues, SMT provers) that Aaroneous devtools or crates can leverage to accelerate development while adhering to zero-heap and zero-ambient authority rules.",
        },
    ]
}

pub(crate) fn convert(
    queue_path: &Path,
    task_id: &str,
    mode: &str,
    explicit_evidence: &[String],
    output_dir: &Path,
) -> Result<PathBuf> {
    if !queue_path.is_absolute() || !output_dir.is_absolute() {
        bail!("queue and output_dir must be absolute paths");
    }
    if !matches!(mode, "analysis" | "test-fixture" | "code-draft") {
        bail!("mode must be one of: analysis, test-fixture, code-draft");
    }
    let content = fs::read_to_string(queue_path)
        .with_context(|| format!("failed to read queue file: {}", queue_path.display()))?;

    let mut found_scope = explicit_evidence.to_vec();
    let mut found_instruction = String::new();
    let mut matched = false;

    for line in content.lines().filter(|l| l.starts_with('|')) {
        let cols: Vec<_> = line.split('|').map(str::trim).collect();
        if cols.len() > 2 && cols.iter().any(|col| col.eq_ignore_ascii_case(task_id)) {
            matched = true;
            if found_scope.is_empty() {
                for segment in line.split('`') {
                    let cleaned = segment.trim_matches(|c| c == '`' || c == '*' || c == ' ');
                    if crate::intake::relative_file(cleaned)
                        && (cleaned.contains('/')
                            || cleaned.ends_with(".rs")
                            || cleaned.ends_with(".toml")
                            || cleaned.ends_with(".md"))
                        && !cleaned.is_empty()
                        && !found_scope.contains(&cleaned.to_owned())
                    {
                        found_scope.push(cleaned.to_owned());
                    }
                }
            }
            if cols.len() > 2 {
                found_instruction = line.to_owned();
            }
            break;
        }
    }

    if !matched {
        bail!("task {task_id} not found in {}", queue_path.display());
    }
    if found_scope.is_empty() {
        bail!("task {task_id} has no valid declared relative file scopes");
    }

    let manifest = JobManifest {
        schema_version: 1,
        id: task_id.to_uppercase(),
        mode: mode.to_owned(),
        evidence: found_scope,
        instruction: if found_instruction.is_empty() {
            format!("Perform {mode} review on declared evidence.")
        } else {
            found_instruction
        },
    };

    let destination = output_dir.join(format!("{}.toml", task_id.to_lowercase()));
    let toml_str = toml::to_string_pretty(&manifest)?;
    fs::write(&destination, toml_str)?;
    println!("wrote worker job manifest: {}", destination.display());
    Ok(destination)
}

pub(crate) fn sync_worker_jobs(
    queue_path: &Path,
    default_mode: &str,
    output_dir: &Path,
) -> Result<Vec<PathBuf>> {
    if !queue_path.is_absolute() || !output_dir.is_absolute() {
        bail!("queue and output_dir must be absolute paths");
    }
    let content = fs::read_to_string(queue_path)
        .with_context(|| format!("failed to read queue file: {}", queue_path.display()))?;

    let mut created = Vec::new();

    for line in content.lines().filter(|l| l.starts_with('|')) {
        let cols: Vec<_> = line.split('|').map(str::trim).collect();
        if cols.len() > 3 && matches!(cols[3], "Ready" | "In Progress") {
            let task_id = cols[1];
            let destination = output_dir.join(format!("{}.toml", task_id.to_lowercase()));
            if destination.exists() {
                continue;
            }
            let mut evidence = Vec::new();
            for segment in line.split('`') {
                let cleaned = segment.trim_matches(|c| c == '`' || c == '*' || c == ' ');
                if crate::intake::relative_file(cleaned)
                    && (cleaned.contains('/')
                        || cleaned.ends_with(".rs")
                        || cleaned.ends_with(".toml")
                        || cleaned.ends_with(".md"))
                    && !cleaned.is_empty()
                    && !evidence.contains(&cleaned.to_owned())
                {
                    evidence.push(cleaned.to_owned());
                }
            }
            if evidence.is_empty() {
                continue;
            }
            let manifest = JobManifest {
                schema_version: 1,
                id: task_id.to_uppercase(),
                mode: default_mode.to_owned(),
                evidence,
                instruction: line.to_owned(),
            };
            let toml_str = toml::to_string_pretty(&manifest)?;
            fs::write(&destination, toml_str)?;
            println!("synced worker job manifest: {}", destination.display());
            created.push(destination);
        }
    }
    Ok(created)
}

pub(crate) fn refill_recurring_jobs(checkout: &Path, output_dir: &Path) -> Result<Vec<PathBuf>> {
    if !checkout.is_absolute() || !output_dir.is_absolute() {
        bail!("checkout and output_dir must be absolute paths");
    }
    let mut created = Vec::new();
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

    let mut completed_ids = std::collections::HashSet::new();
    let mut latest_completion_secs: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    let mut latest_completion_summary: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for dir in candidate_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(text) = fs::read_to_string(&path) {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(id) = value["job_id"].as_str() {
                                let upper = id.to_uppercase();
                                completed_ids.insert(upper.clone());
                                let submitted = value["submitted_at_secs"].as_u64().unwrap_or(0);
                                let base = if let Some(idx) = upper.rfind("-P") {
                                    if upper[idx + 2..].chars().all(|c| c.is_ascii_digit()) {
                                        &upper[..idx]
                                    } else {
                                        &upper
                                    }
                                } else {
                                    &upper
                                };
                                let entry =
                                    latest_completion_secs.entry(base.to_string()).or_insert(0);
                                if submitted >= *entry {
                                    *entry = submitted;
                                    if let Some(proposal) =
                                        value.get("proposal").and_then(|p| p.as_object())
                                    {
                                        let mut points = Vec::new();
                                        if let Some(obs) =
                                            proposal.get("observations").and_then(|v| v.as_array())
                                        {
                                            for o in obs.iter().filter_map(|x| x.as_str()) {
                                                points.push(format!("Obs: {o}"));
                                            }
                                        }
                                        if let Some(risks) =
                                            proposal.get("risks").and_then(|v| v.as_array())
                                        {
                                            for r in risks.iter().filter_map(|x| x.as_str()) {
                                                points.push(format!("Risk: {r}"));
                                            }
                                        }
                                        if !points.is_empty() {
                                            latest_completion_summary
                                                .insert(base.to_string(), points.join(" | "));
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

    for item in recurring_catalog() {
        let devtools_root = output_dir
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(checkout);
        let all_evidence_present = item.evidence.iter().all(|relative| {
            checkout.join(relative).is_file() || devtools_root.join(relative).is_file()
        });
        if !all_evidence_present {
            continue;
        }

        let prefix = item.id.to_lowercase();
        let existing_jobs: Vec<_> = fs::read_dir(output_dir)
            .ok()
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|e| {
                        let fname = e.file_name().to_string_lossy().to_lowercase();
                        if fname.starts_with(&prefix) && fname.ends_with(".toml") {
                            Some(e.path())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        for job_path in &existing_jobs {
            if let Ok(text) = fs::read_to_string(job_path) {
                if let Ok(val) = toml::from_str::<toml::Value>(&text) {
                    if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                        if completed_ids.contains(&id.to_uppercase()) {
                            let _ = fs::remove_file(job_path);
                        }
                    }
                }
            }
        }

        // Re-read existing jobs after removing completed ones
        let remaining_jobs: Vec<_> = fs::read_dir(output_dir)
            .ok()
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|e| {
                        let fname = e.file_name().to_string_lossy().to_lowercase();
                        if fname.starts_with(&prefix) && fname.ends_with(".toml") {
                            Some(e.path())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        if !remaining_jobs.is_empty() {
            continue;
        }

        let mut pass = 0;
        let mut target_id = item.id.to_string();
        let mut target_filename = format!("{}.toml", item.id.to_lowercase());

        while completed_ids.contains(&target_id) {
            pass += 1;
            target_id = format!("{}-P{:04}", item.id, pass);
            target_filename = format!("{}-p{:04}.toml", item.id.to_lowercase(), pass);
        }

        // If at least one pass has already completed, check if evidence files
        // have changed or if the 24h cooldown has elapsed. If neither, skip refilling.
        if pass > 0 {
            if let Some(&last_submitted) = latest_completion_secs.get(item.id) {
                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let evidence_modified = item.evidence.iter().any(|relative| {
                    let p = if checkout.join(relative).is_file() {
                        checkout.join(relative)
                    } else {
                        devtools_root.join(relative)
                    };
                    if let Ok(meta) = fs::metadata(&p) {
                        if let Ok(mtime) = meta.modified() {
                            if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                                return dur.as_secs() > last_submitted;
                            }
                        }
                    }
                    false
                });

                let cooldown_elapsed = now_secs.saturating_sub(last_submitted) >= 900;
                if !evidence_modified && !cooldown_elapsed {
                    continue;
                }
            }
        }

        let destination = output_dir.join(&target_filename);
        if destination.exists() {
            continue;
        }

        let mut instruction = item.instruction.to_owned();
        if pass > 0 {
            if let Some(prev_notes) = latest_completion_summary.get(item.id) {
                instruction.push_str(&format!(
                    "\n\n[PRIOR AUDIT CONTEXT - Pass {pass}]:\nIn previous invariant audit pass(es), the following observations were noted:\n{prev_notes}\nVerify whether these conditions remain active, have been remediated in the codebase, or if any new invariant violations have emerged since."
                ));
            }
        }

        let manifest = JobManifest {
            schema_version: 1,
            id: target_id,
            mode: item.mode.to_owned(),
            evidence: item.evidence.iter().map(|s| (*s).to_owned()).collect(),
            instruction,
        };
        let toml_str = toml::to_string_pretty(&manifest)?;
        fs::write(&destination, toml_str)?;
        println!("refilled recurring worker job: {}", destination.display());
        created.push(destination);
    }
    Ok(created)
}

pub(crate) fn sync_and_refill_jobs(
    checkout: &Path,
    queue_path: &Path,
    default_mode: &str,
    output_dir: &Path,
) -> Result<Vec<PathBuf>> {
    let mut created = sync_worker_jobs(queue_path, default_mode, output_dir)?;
    let refilled = refill_recurring_jobs(checkout, output_dir)?;
    created.extend(refilled);

    if let Some(gov_dir) = queue_path.parent() {
        let program_path = gov_dir.join("MULTITHREAD_PROGRAM.md");
        if program_path.is_file() {
            if let Ok(prog_jobs) = crate::portfolio_jobs::refill_portfolio_progression_jobs(
                &program_path,
                queue_path,
                output_dir,
                4,
            ) {
                created.extend(prog_jobs);
            }
        }
    }

    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_dummy_evidence(checkout: &Path, relative: &str) {
        let path = checkout.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, "dummy").expect("write dummy file");
    }

    #[test]
    fn converts_queue_row_to_valid_worker_job() {
        let temp = tempfile::tempdir().expect("test dir");
        let queue = temp.path().join("queue.md");
        fs::write(
            &queue,
            "| ID | Scope | Status |\n| M31 | `crates/paths/src/lib.rs` review | Ready |",
        )
        .expect("write queue");

        let out = convert(&queue, "M31", "analysis", &[], temp.path()).expect("convert");
        assert!(out.exists());
        let content = fs::read_to_string(&out).expect("read");
        let manifest: JobManifest = toml::from_str(&content).expect("toml parse");
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.id, "M31");
        assert_eq!(manifest.mode, "analysis");
        assert_eq!(manifest.evidence, vec!["crates/paths/src/lib.rs"]);
    }

    #[test]
    fn syncs_ready_queue_rows_to_worker_jobs() {
        let temp = tempfile::tempdir().expect("test dir");
        let queue = temp.path().join("queue.md");
        fs::write(
            &queue,
            "| ID | Scope | Status |\n\
             | C33 | `crates/orchestrator/src/lib.rs` | Ready |\n\
             | C34 | `crates/compute/src/lib.rs` | In Progress |\n\
             | C35 | `README.md` | Done |",
        )
        .expect("write queue");

        let synced = sync_worker_jobs(&queue, "analysis", temp.path()).expect("sync");
        assert_eq!(synced.len(), 2);
        assert!(temp.path().join("c33.toml").exists());
        assert!(temp.path().join("c34.toml").exists());
        assert!(!temp.path().join("c35.toml").exists());
    }

    #[test]
    fn refills_recurring_maintenance_catalog_jobs_when_evidence_is_present() {
        let temp = tempfile::tempdir().expect("test dir");
        let checkout = temp.path().join("checkout");
        create_dummy_evidence(&checkout, "crates/ast_auditor/src/rules/text_encoding.rs");
        create_dummy_evidence(&checkout, "xtask/src/gate.rs");

        let refilled = refill_recurring_jobs(&checkout, temp.path()).expect("refill");
        assert_eq!(refilled.len(), 1);
        assert!(temp.path().join("audit-01-ast-safety.toml").exists());
    }

    #[test]
    fn sync_and_refill_combines_queue_and_catalog() {
        let temp = tempfile::tempdir().expect("test dir");
        let checkout = temp.path().join("checkout");
        create_dummy_evidence(&checkout, "crates/ast_auditor/src/rules/text_encoding.rs");
        create_dummy_evidence(&checkout, "xtask/src/gate.rs");

        let queue = temp.path().join("queue.md");
        fs::write(
            &queue,
            "| ID | Scope | Status |\n| C33 | `crates/orchestrator/src/lib.rs` | Ready |",
        )
        .expect("write queue");

        let synced = sync_and_refill_jobs(&checkout, &queue, "analysis", temp.path())
            .expect("sync & refill");
        assert_eq!(synced.len(), 2);
        assert!(temp.path().join("c33.toml").exists());
        assert!(temp.path().join("audit-01-ast-safety.toml").exists());
    }
}
