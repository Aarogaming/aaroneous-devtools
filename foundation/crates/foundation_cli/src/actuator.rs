use std::path::Path;
use anyhow::Result;

pub fn run(config_path: &Path) -> Result<()> {
    use crate::ledger::{Ledger, JobState, IterationAttempt};
    
    let c: crate::local_worker::Config = toml::from_str(&crate::intake::read_bounded(config_path, 16384)?)?;
    let db_path = c.reservation_directory.join("ledger_db");
    let ledger = Ledger::new(&db_path)?;

    println!("Actuator: starting on ledger at {}", db_path.display());

    if let Ok(jobs) = ledger.get_jobs_by_state(JobState::Accepted) {
        if jobs.is_empty() {
            println!("Actuator: No Accepted jobs found.");
            return Ok(());
        }

        for mut job in jobs {
            println!("Actuator processing: {}", job.job_id);
            
            // Apply FileModifications to MAIN checkout
            let mut apply_failed = false;
            let mut apply_error = String::new();
            
            if let Some(draft) = &job.current_draft {
                for modif in draft {
                    let rel_path = modif.file_path.trim_start_matches('/');
                    let target_path = c.checkout.join(rel_path);
                    
                    if let Ok(content) = std::fs::read_to_string(&target_path) {
                        if content.contains(&modif.search_block) {
                            let new_content = content.replace(&modif.search_block, &modif.replace_block);
                            let _ = std::fs::write(&target_path, new_content);
                        } else {
                            apply_failed = true;
                            apply_error.push_str(&format!("Search block not found in main workspace file: {}\n", modif.file_path));
                        }
                    } else {
                        apply_failed = true;
                        apply_error.push_str(&format!("File not found in main workspace: {}\n", modif.file_path));
                    }
                }
            }

            if apply_failed {
                println!("Actuator: apply failed for {}. Bouncing to Drafting due to conflict.", job.job_id);
                
                // Reset changes that might have partially applied
                let _ = std::process::Command::new("git")
                    .current_dir(&c.checkout)
                    .arg("reset")
                    .arg("--hard")
                    .arg("HEAD")
                    .status();

                let attempt = IterationAttempt {
                    attempt_number: job.iteration_history.len() as u32 + 1,
                    submitted_at_secs: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                    draft: job.current_draft.clone(),
                    compiler_output: Some(format!("ACTUATOR MERGE CONFLICT:\n{}", apply_error)),
                };
                job.iteration_history.push(attempt);
                job.state = JobState::Drafting;
                let _ = ledger.insert_job(&job);
                continue;
            }

            // Final safety net: Run cargo xtask gate on main
            println!("Actuator: running final cargo xtask gate on main workspace...");
            if let Ok(output) = std::process::Command::new("cargo")
                .current_dir(&c.checkout)
                .arg("xtask")
                .arg("gate")
                .output() 
            {
                if output.status.success() {
                    println!("Actuator: Success! {} actuated.", job.job_id);
                    job.state = JobState::Actuated;
                    let _ = ledger.insert_job(&job);
                } else {
                    println!("Actuator: Main workspace verification failed for {}. Reverting and bouncing.", job.job_id);
                    
                    // Revert Main Workspace
                    let _ = std::process::Command::new("git")
                        .current_dir(&c.checkout)
                        .arg("reset")
                        .arg("--hard")
                        .arg("HEAD")
                        .status();

                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    
                    let attempt = IterationAttempt {
                        attempt_number: job.iteration_history.len() as u32 + 1,
                        submitted_at_secs: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                        draft: job.current_draft.clone(),
                        compiler_output: Some(format!("ACTUATOR GATE FAILED ON MAIN:\nSTDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr)),
                    };
                    job.iteration_history.push(attempt);
                    job.state = JobState::Drafting;
                    let _ = ledger.insert_job(&job);
                }
            } else {
                println!("Actuator: Command execution failed.");
            }
        }
    }
    
    Ok(())
}
