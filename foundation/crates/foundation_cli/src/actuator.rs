use anyhow::Result;
use std::path::Path;

pub fn run(config_path: &Path) -> Result<()> {
    use crate::ledger::{IterationAttempt, JobState, Ledger};

    let c: crate::local_worker::Config =
        toml::from_str(&crate::intake::read_bounded(config_path, 16384)?)?;
    require_isolated_clean_worktree(&c.checkout)?;
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
                    if !is_safe_relative(rel_path) {
                        apply_failed = true;
                        apply_error.push_str(&format!(
                            "Rejected unsafe target path: {}
",
                            modif.file_path
                        ));
                        continue;
                    }
                    let target_path = c.checkout.join(rel_path);

                    if let Ok(content) = std::fs::read_to_string(&target_path) {
                        if content.contains(&modif.search_block) {
                            let new_content =
                                content.replace(&modif.search_block, &modif.replace_block);
                            let _ = std::fs::write(&target_path, new_content);
                        } else {
                            apply_failed = true;
                            apply_error.push_str(&format!(
                                "Search block not found in main workspace file: {}\n",
                                modif.file_path
                            ));
                        }
                    } else {
                        apply_failed = true;
                        apply_error.push_str(&format!(
                            "File not found in main workspace: {}\n",
                            modif.file_path
                        ));
                    }
                }
            }

            if apply_failed {
                println!(
                    "Actuator: apply failed for {}. Bouncing to Drafting due to conflict.",
                    job.job_id
                );

                // Reset changes that might have partially applied
                let _ = std::process::Command::new("git")
                    .current_dir(&c.checkout)
                    .arg("reset")
                    .arg("--hard")
                    .arg("HEAD")
                    .status();

                let attempt = IterationAttempt {
                    attempt_number: job.iteration_history.len() as u32 + 1,
                    submitted_at_secs: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
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
                    println!(
                        "Actuator: Main workspace verification failed for {}. Reverting and bouncing.",
                        job.job_id
                    );

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
                        submitted_at_secs: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        draft: job.current_draft.clone(),
                        compiler_output: Some(format!(
                            "ACTUATOR GATE FAILED ON MAIN:\nSTDOUT:\n{}\n\nSTDERR:\n{}",
                            stdout, stderr
                        )),
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

/// The actuator applies model drafts and, on failure, runs `git reset --hard
/// HEAD` in `checkout`. That is only safe in a disposable checkout, so refuse
/// to run unless `checkout` is a *linked* Git worktree (not a primary
/// checkout someone works in) with no uncommitted changes. Point the config's
/// `checkout` at a worktree created for actuation (see `local_staging`), never
/// at the owner's working copy.
fn require_isolated_clean_worktree(checkout: &Path) -> Result<()> {
    let git = |args: &[&str]| -> Result<String> {
        let out = std::process::Command::new("git")
            .current_dir(checkout)
            .args(args)
            .output()?;
        if !out.status.success() {
            anyhow::bail!(
                "git {} failed in {}: {}",
                args.join(" "),
                checkout.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    let git_dir = git(&["rev-parse", "--absolute-git-dir"])?;
    let common_dir = git(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
    if std::path::Path::new(&git_dir) == std::path::Path::new(&common_dir) {
        anyhow::bail!(
            "refusing to actuate into {}: it is a primary checkout, not a linked worktree. Create a dedicated worktree for actuation and point `checkout` at it.",
            checkout.display()
        );
    }
    if !git(&["status", "--porcelain"])?.is_empty() {
        anyhow::bail!(
            "refusing to actuate into {}: the worktree has uncommitted changes that `git reset --hard` would destroy.",
            checkout.display()
        );
    }
    Ok(())
}

/// A draft may only target a plain relative path inside the checkout.
fn is_safe_relative(rel_path: &str) -> bool {
    let path = std::path::Path::new(rel_path);
    !rel_path.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_and_absolute_paths() {
        assert!(is_safe_relative("crates/paths/src/lib.rs"));
        assert!(!is_safe_relative("../outside.rs"));
        assert!(!is_safe_relative("crates/../../outside.rs"));
        assert!(!is_safe_relative("C:/Windows/system.ini"));
        assert!(!is_safe_relative(""));
    }

    #[test]
    fn refuses_primary_checkout() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ok = std::process::Command::new("git")
            .current_dir(dir.path())
            .args(["init", "-q"])
            .status()
            .expect("git init");
        assert!(ok.success());
        let err = require_isolated_clean_worktree(dir.path()).expect_err("must refuse");
        assert!(err.to_string().contains("primary checkout"));
    }
}
