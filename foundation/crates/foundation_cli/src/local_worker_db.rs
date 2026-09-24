use anyhow::Result;
use std::path::Path;

pub fn serve_db(config_path: &Path) -> Result<()> {
    use crate::ledger::{IterationAttempt, JobRecord, JobState, Ledger};

    let c: crate::local_worker::Config =
        toml::from_str(&crate::intake::read_bounded(config_path, 16384)?)?;
    let db_path = c.reservation_directory.join("ledger_db");
    let ledger = Ledger::new(&db_path)?;

    println!(
        "local-worker-db-serve: starting on ledger at {}",
        db_path.display()
    );

    loop {
        // 1. Ingestion: scan worker-jobs/*.toml and ingest into DB
        if let Ok(entries) = std::fs::read_dir(&c.jobs_directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    let job_id = path
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .to_string()
                        .to_uppercase();

                    if ledger.get_job(&job_id).unwrap_or(None).is_none() {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let record = JobRecord {
                                job_id: job_id.clone(),
                                state: JobState::Discovered,
                                input_packet: content,
                                current_draft: None,
                                rust_draft: None,
                                iteration_history: Vec::new(),
                                created_at_secs: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                updated_at_secs: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                                rejection_reason: None,
                            };
                            let _ = ledger.insert_job(&record);
                            println!("Ingested {} into ledger.", job_id);
                        }
                    }
                }
            }
        }

        // 2. Transition Discovered -> Drafting
        if let Ok(jobs) = ledger.get_jobs_by_state(JobState::Discovered) {
            for mut job in jobs {
                job.state = JobState::Drafting;
                let _ = ledger.insert_job(&job);
            }
        }

        // 3. Drafting -> Gate1Syntax (Run LLM generation)
        if let Ok(jobs) = ledger.get_jobs_by_state(JobState::Drafting) {
            for mut job in jobs {
                println!("Drafting job: {}", job.job_id);

                let is_retry = !job.iteration_history.is_empty();
                let system_prompt = crate::local_worker::build_system_prompt(
                    true,
                    is_retry,
                    job.rejection_reason.is_some(),
                );

                // Inject previous compiler errors if it's a retry
                let mut input_prompt = job.input_packet.clone();
                if let Some(last_attempt) = job.iteration_history.last() {
                    if let Some(err) = &last_attempt.compiler_output {
                        input_prompt.push_str(&format!("\n\n[SELF-CORRECTION REQUIRED]: Your previous proposal failed with the following errors:\n{}\n\n", err));
                    }
                    if let Some(draft) = &last_attempt.draft {
                        input_prompt.push_str(&format!(
                            "=== YOUR PREVIOUS FAILED CODE ===\n{}\n",
                            serde_json::to_string_pretty(draft).unwrap_or_default()
                        ));
                    }
                }

                if let Ok(draft) = crate::local_worker::delegate_tuned(
                    &c.endpoint,
                    &c.model,
                    &system_prompt,
                    &input_prompt,
                    is_retry,
                ) {
                    job.current_draft = draft.implementation_plan;
                    job.state = JobState::Gate1Syntax;
                    let _ = ledger.insert_job(&job);
                    println!("LLM drafted {}; moved to Gate1Syntax.", job.job_id);
                } else {
                    println!("LLM query failed for {}; retrying later.", job.job_id);
                }
            }
        }

        // 4. Gate1Syntax -> Gate2Types / Drafting (Verify with ast_auditor)
        if let Ok(jobs) = ledger.get_jobs_by_state(JobState::Gate1Syntax) {
            for mut job in jobs {
                println!("Gate1Syntax checking: {}", job.job_id);

                // Construct a temporary Draft to pass to ast_auditor
                let temp_draft = crate::local_worker::Draft {
                    task: job.job_id.clone(),
                    observations: vec![],
                    risks: vec![],
                    test_plan: vec![],
                    recommended_next_step: "actuate".into(),
                    rust_draft: None,
                    implementation_plan: job.current_draft.clone(),
                    solution_tree: None,
                };

                let violations = crate::local_worker::verify_required_output(&temp_draft, true);

                if violations.is_empty() {
                    println!("Gate1 passed for {}. Moving to Gate2Types.", job.job_id);
                    job.state = JobState::Gate2Types;
                } else {
                    println!("Gate1 failed for {}. Bouncing to Drafting.", job.job_id);
                    let attempt = IterationAttempt {
                        attempt_number: job.iteration_history.len() as u32 + 1,
                        submitted_at_secs: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        draft: job.current_draft.clone(),
                        compiler_output: Some(violations.join("\n")),
                    };
                    job.iteration_history.push(attempt);
                    job.state = JobState::Drafting;
                }
                let _ = ledger.insert_job(&job);
            }
        }

        // 5. Gate2Types -> PendingReview / Drafting (cargo xtask gate in shadow sandbox)
        if let Ok(jobs) = ledger.get_jobs_by_state(JobState::Gate2Types) {
            let sandbox_dir = c.reservation_directory.join("shadow-sandbox");

            for mut job in jobs {
                println!("Gate2Types checking: {}", job.job_id);

                // Ensure sandbox exists and is clean
                if !sandbox_dir.exists() {
                    let _ = std::process::Command::new("git")
                        .arg("clone")
                        .arg("--local")
                        .arg(&c.checkout)
                        .arg(&sandbox_dir)
                        .status();
                } else {
                    let _ = std::process::Command::new("git")
                        .current_dir(&sandbox_dir)
                        .arg("reset")
                        .arg("--hard")
                        .arg("HEAD")
                        .status();
                    let _ = std::process::Command::new("git")
                        .current_dir(&sandbox_dir)
                        .arg("clean")
                        .arg("-fd")
                        .status();
                }

                // Apply FileModifications
                let mut apply_failed = false;
                let mut apply_error = String::new();

                if let Some(draft) = &job.current_draft {
                    for modif in draft {
                        // Strip leading slash if any to make it relative
                        let rel_path = modif.file_path.trim_start_matches('/');

                        // 1. Path Traversal Protection
                        if rel_path.contains("..") {
                            apply_failed = true;
                            apply_error.push_str(&format!("error[E-SEC01]: sandbox security violation\n  --> {}\n  |\n  = note: Path traversal ('..') is strictly forbidden.\n  = help: Only target files explicitly within the component workspace.\n\n", modif.file_path));
                            continue;
                        }

                        // 2. Boundary Protection
                        if rel_path.starts_with(".cargo")
                            || rel_path.starts_with(".github")
                            || rel_path.starts_with("xtask")
                            || rel_path == "build.rs"
                        {
                            apply_failed = true;
                            apply_error.push_str(&format!("error[E-SEC02]: framework boundary violation\n  --> {}\n  |\n  = note: Modification of infrastructure, workflow, or build files is banned.\n  = help: Focus your modifications strictly within the src/ or core/ runtime files.\n\n", modif.file_path));
                            continue;
                        }

                        let target_path = sandbox_dir.join(rel_path);

                        if let Ok(content) = std::fs::read_to_string(&target_path) {
                            let matches = content.matches(&modif.search_block).count();

                            if matches == 0 {
                                apply_failed = true;
                                apply_error.push_str(&format!("error[E-PTC01]: orphaned search block\n  --> {}\n  |\n  = note: The exact string block provided in 'search_block' could not be found in the file.\n  = help: Check for missing/extra leading whitespace, or fetch the latest version of the file. The search block must match the existing file verbatim.\n\n", modif.file_path));
                            } else if matches > 1 {
                                apply_failed = true;
                                apply_error.push_str(&format!("error[E-PTC02]: ambiguous search block\n  --> {}\n  |\n  = note: The provided 'search_block' matched {} times in the file. Patches must be deterministic.\n  = help: Expand your 'search_block' to include unique surrounding lines (like a function signature or specific comment) so it only matches exactly once.\n\n", modif.file_path, matches));
                            } else {
                                let new_content =
                                    content.replace(&modif.search_block, &modif.replace_block);
                                let _ = std::fs::write(&target_path, new_content);
                            }
                        } else {
                            apply_failed = true;
                            apply_error.push_str(&format!("error[E-IO01]: target file not found\n  --> {}\n  |\n  = note: The file does not exist in the repository checkout.\n  = help: Verify the file path is correct relative to the repository root.\n\n", modif.file_path));
                        }
                    }
                }

                if apply_failed {
                    println!(
                        "Gate2 apply failed for {}. Bouncing to Drafting.",
                        job.job_id
                    );
                    let attempt = IterationAttempt {
                        attempt_number: job.iteration_history.len() as u32 + 1,
                        submitted_at_secs: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        draft: job.current_draft.clone(),
                        compiler_output: Some(apply_error),
                    };
                    job.iteration_history.push(attempt);
                    job.state = JobState::Drafting;
                    let _ = ledger.insert_job(&job);
                    continue;
                }

                // Run cargo xtask gate
                println!("Running cargo xtask gate in shadow sandbox...");
                if let Ok(output) = std::process::Command::new("cargo")
                    .current_dir(&sandbox_dir)
                    .arg("xtask")
                    .arg("gate")
                    .output()
                {
                    if output.status.success() {
                        println!("Gate2 passed for {}. Moving to PendingReview.", job.job_id);
                        job.state = JobState::PendingReview; // xtask gate covers types AND tests
                    } else {
                        println!("Gate2 failed for {}. Bouncing to Drafting.", job.job_id);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);

                        let attempt = IterationAttempt {
                            attempt_number: job.iteration_history.len() as u32 + 1,
                            submitted_at_secs: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            draft: job.current_draft.clone(),
                            compiler_output: Some(format!(
                                "STDOUT:\n{}\n\nSTDERR:\n{}",
                                stdout, stderr
                            )),
                        };
                        job.iteration_history.push(attempt);
                        job.state = JobState::Drafting;
                    }
                } else {
                    println!(
                        "Gate2 command execution failed for {}. Bouncing.",
                        job.job_id
                    );
                    job.state = JobState::Drafting;
                }

                let _ = ledger.insert_job(&job);
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
