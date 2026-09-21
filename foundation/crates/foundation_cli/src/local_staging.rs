//! Automated progression worktrees (M62 —
//! `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`): lets the local worker
//! iterate freely -- generate, commit, generate again -- entirely on local
//! disk, without ever touching a shared remote.
//!
//! This module contains no `git push` call anywhere in its source, and
//! that's the property that makes unsupervised iteration here safe, not
//! just a design intention: `no_push_call_anywhere_in_this_module` below
//! greps this file's own compiled-in source for the quoted argument a real
//! git-push invocation would use, and fails the build's test suite if it
//! ever appears, so reintroducing one isn't just discouraged, it's caught.
//! Nothing this
//! module does is visible to another agent or human until a separate,
//! later, discrete action (`outbox::run_promote`) deliberately pushes one
//! specific staged commit to a shared remote — unchanged, same gate as
//! before this module existed.
//!
//! A staging worktree is local-only iteration history, not a proposal in
//! its own right. It never updates a `local_worker::Record`'s outcome and
//! never marks anything `pending-owner-review` — that pipeline is
//! untouched; this is purely additive local iteration space, the "farm
//! sidequests without touching the main questline" half of M61/M62.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedDraft {
    pub job_id: String,
    pub branch: String,
    pub worktree_path: String,
    pub commit_hash: String,
    pub iteration: u32,
}

fn require_absolute(label: &str, path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("{label} must be an absolute path: {}", path.display());
    }
    Ok(())
}

fn require_safe_job_id(job_id: &str) -> Result<()> {
    if job_id.trim().is_empty()
        || job_id.len() > 64
        || !job_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        bail!("job_id must be a short alphanumeric/hyphen identifier: {job_id}");
    }
    Ok(())
}

fn require_safe_relative_path(target_file: &str) -> Result<()> {
    let path = Path::new(target_file);
    if path.is_absolute() || target_file.contains("..") || target_file.trim().is_empty() {
        bail!("target_file must be a safe relative path: {target_file}");
    }
    Ok(())
}

fn worktree_exists(worktree_path: &Path) -> bool {
    worktree_path.join(".git").exists()
}

fn commit_count(worktree_path: &Path) -> Result<u32> {
    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(worktree_path)
        .output()
        .context("failed to run git rev-list --count HEAD")?;
    if !output.status.success() {
        bail!(
            "git rev-list --count HEAD failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .context("could not parse commit count")
}

fn head_commit_hash(worktree_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(worktree_path)
        .output()
        .context("failed to run git rev-parse HEAD")?;
    if !output.status.success() {
        bail!(
            "git rev-parse HEAD failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Stages `rust_source` at `target_file` inside a dedicated, local-only
/// git worktree/branch (`local-staging/{job_id}`) for `job_id`, committing
/// it. Reuses the same worktree across repeated calls for the same
/// `job_id`, so multiple iterations accumulate as real local commit
/// history instead of each call silently overwriting the last with no
/// record of what changed between them.
///
/// `checkout` is the real Aaroneous checkout the worktree is created from
/// (its currently checked-out ref becomes the staging branch's base
/// point, only on first creation); `worktrees_root` is the directory
/// staging worktrees live under. Both must already exist and be absolute
/// — no ambient path inference, matching this crate's convention
/// throughout. Refuses a `target_file` that isn't a safe relative path,
/// and refuses (as a no-op error, not a forced empty commit) staging
/// content identical to what's already committed — that's not a real
/// iteration.
pub(crate) fn stage_draft(
    checkout: &Path,
    worktrees_root: &Path,
    job_id: &str,
    target_file: &str,
    rust_source: &str,
    commit_message: &str,
) -> Result<StagedDraft> {
    require_absolute("checkout", checkout)?;
    require_absolute("worktrees-root", worktrees_root)?;
    require_safe_job_id(job_id)?;
    require_safe_relative_path(target_file)?;
    if !checkout.join(".git").exists() {
        bail!("checkout is not a git repository: {}", checkout.display());
    }
    if !worktrees_root.is_dir() {
        bail!(
            "worktrees-root does not exist: {}",
            worktrees_root.display()
        );
    }

    let branch = format!("local-staging/{job_id}");
    let worktree_path = worktrees_root.join(job_id);

    if !worktree_exists(&worktree_path) {
        let status = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                &branch,
                &worktree_path.display().to_string(),
            ])
            .current_dir(checkout)
            .status()
            .context("failed to run git worktree add")?;
        if !status.success() {
            bail!("git worktree add failed for branch {branch}");
        }
    }

    let full_target = worktree_path.join(target_file);
    if let Some(parent) = full_target.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to prepare {}", parent.display()))?;
    }
    std::fs::write(&full_target, rust_source)
        .with_context(|| format!("failed to write {}", full_target.display()))?;

    let add_status = Command::new("git")
        .args(["add", "--", target_file])
        .current_dir(&worktree_path)
        .status()
        .context("failed to run git add in staging worktree")?;
    if !add_status.success() {
        bail!("git add failed for {target_file} in staging worktree");
    }

    let commit_out = Command::new("git")
        .args(["commit", "-m", commit_message])
        .current_dir(&worktree_path)
        .output()
        .context("failed to run git commit in staging worktree")?;
    if !commit_out.status.success() {
        let stderr = String::from_utf8_lossy(&commit_out.stderr);
        let stdout = String::from_utf8_lossy(&commit_out.stdout);
        if stdout.contains("nothing to commit") || stderr.contains("nothing to commit") {
            bail!(
                "no change to stage for {job_id}: {target_file} is already identical to the \
                 last staged iteration"
            );
        }
        bail!("git commit failed in staging worktree: {stderr}{stdout}");
    }

    Ok(StagedDraft {
        job_id: job_id.to_string(),
        branch,
        worktree_path: worktree_path.display().to_string(),
        commit_hash: head_commit_hash(&worktree_path)?,
        iteration: commit_count(&worktree_path)?,
    })
}

/// Lists every existing staging worktree under `worktrees_root`, current
/// HEAD and iteration count for each. Read-only.
pub(crate) fn list_staged(worktrees_root: &Path) -> Result<Vec<StagedDraft>> {
    require_absolute("worktrees-root", worktrees_root)?;
    if !worktrees_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(worktrees_root)
        .with_context(|| format!("reading {}", worktrees_root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() || !worktree_exists(&path) {
            continue;
        }
        let job_id = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        out.push(StagedDraft {
            job_id: job_id.clone(),
            branch: format!("local-staging/{job_id}"),
            worktree_path: path.display().to_string(),
            commit_hash: head_commit_hash(&path)?,
            iteration: commit_count(&path)?,
        });
    }
    out.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    Ok(out)
}

/// Removes a staging worktree and its local-only branch. Never touches a
/// remote (there is nothing to touch — the branch was never pushed).
/// Bounded pruning for what would otherwise be unbounded local disk/branch
/// growth, matching the same "forgetting" discipline the rest of this
/// system already applies to `results/` archival.
pub(crate) fn prune_staged(checkout: &Path, worktrees_root: &Path, job_id: &str) -> Result<()> {
    require_absolute("checkout", checkout)?;
    require_absolute("worktrees-root", worktrees_root)?;
    require_safe_job_id(job_id)?;
    let worktree_path = worktrees_root.join(job_id);
    if !worktree_exists(&worktree_path) {
        bail!("no staging worktree found for {job_id}");
    }
    let remove_status = Command::new("git")
        .args([
            "worktree",
            "remove",
            "--force",
            &worktree_path.display().to_string(),
        ])
        .current_dir(checkout)
        .status()
        .context("failed to run git worktree remove")?;
    if !remove_status.success() {
        bail!("git worktree remove failed for {job_id}");
    }
    let branch = format!("local-staging/{job_id}");
    let _ = Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(checkout)
        .status();
    Ok(())
}

fn require_absolute_output(output: &Path) -> Result<()> {
    if !output.is_absolute() {
        bail!("output must be an absolute path: {}", output.display());
    }
    let parent = output
        .parent()
        .context("output path must have an existing parent directory")?;
    if !parent.is_dir() {
        bail!(
            "output parent directory does not exist: {}",
            parent.display()
        );
    }
    Ok(())
}

/// `stage_draft`, writing the resulting [`StagedDraft`] as JSON to `output`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_stage_draft(
    checkout: &Path,
    worktrees_root: &Path,
    job_id: &str,
    target_file: &str,
    rust_source: &str,
    commit_message: &str,
    output: &Path,
) -> Result<()> {
    require_absolute_output(output)?;
    let staged = stage_draft(
        checkout,
        worktrees_root,
        job_id,
        target_file,
        rust_source,
        commit_message,
    )?;
    println!(
        "staged {job_id} at {} ({}), iteration {}",
        staged.worktree_path, staged.commit_hash, staged.iteration
    );
    std::fs::write(output, serde_json::to_vec_pretty(&staged)?)
        .with_context(|| format!("writing {}", output.display()))
}

/// `list_staged`, writing the result as a JSON array to `output`.
pub(crate) fn run_list_staged(worktrees_root: &Path, output: &Path) -> Result<()> {
    require_absolute_output(output)?;
    let staged = list_staged(worktrees_root)?;
    std::fs::write(output, serde_json::to_vec_pretty(&staged)?)
        .with_context(|| format!("writing {}", output.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The actual safety property this module relies on, checked
    /// mechanically rather than only asserted in a doc comment: no
    /// `Command` invocation anywhere in this file passes the quoted git
    /// subcommand that would send a branch to a remote. If this ever
    /// fails, someone added that path to what is supposed to be
    /// structurally incapable of reaching a remote.
    #[test]
    fn no_push_call_anywhere_in_this_module() {
        let source = include_str!("local_staging.rs");
        // Built by concatenation, not as a literal, so this check line
        // itself (which necessarily names the forbidden pattern somehow)
        // doesn't trip its own assertion -- a real occurrence elsewhere in
        // the file, e.g. an args array literally containing it, is still a
        // contiguous substring of the compiled-in source and still caught.
        let forbidden_arg: String = ["\"pu", "sh\""].concat();
        assert!(
            !source.contains(&forbidden_arg),
            "local_staging.rs must never contain a git push call -- that's the entire safety \
             property this module depends on"
        );
    }

    fn init_bare_style_checkout(dir: &Path) {
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(dir)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?} failed");
        };
        run(&["init", "-q"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        std::fs::write(dir.join("README.md"), "seed\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "seed"]);
    }

    #[test]
    fn stages_and_iterates_a_draft_as_real_local_commits() {
        let checkout = tempfile::tempdir().unwrap();
        init_bare_style_checkout(checkout.path());
        let worktrees_root = tempfile::tempdir().unwrap();

        let first = stage_draft(
            checkout.path(),
            worktrees_root.path(),
            "JOB1",
            "crates/foo/src/lib.rs",
            "fn v1() {}",
            "first attempt",
        )
        .unwrap();
        assert_eq!(first.iteration, 2); // seed commit + this one
        assert_eq!(first.branch, "local-staging/JOB1");

        let second = stage_draft(
            checkout.path(),
            worktrees_root.path(),
            "JOB1",
            "crates/foo/src/lib.rs",
            "fn v2() {}",
            "second attempt",
        )
        .unwrap();
        assert_eq!(second.iteration, 3);
        assert_ne!(second.commit_hash, first.commit_hash);

        let content =
            std::fs::read_to_string(Path::new(&second.worktree_path).join("crates/foo/src/lib.rs"))
                .unwrap();
        assert_eq!(content, "fn v2() {}");
    }

    #[test]
    fn identical_redraft_is_refused_not_force_committed() {
        let checkout = tempfile::tempdir().unwrap();
        init_bare_style_checkout(checkout.path());
        let worktrees_root = tempfile::tempdir().unwrap();

        stage_draft(
            checkout.path(),
            worktrees_root.path(),
            "JOB2",
            "crates/foo/src/lib.rs",
            "fn same() {}",
            "attempt",
        )
        .unwrap();

        let err = stage_draft(
            checkout.path(),
            worktrees_root.path(),
            "JOB2",
            "crates/foo/src/lib.rs",
            "fn same() {}",
            "attempt again, unchanged",
        )
        .unwrap_err();
        assert!(err.to_string().contains("no change to stage"));
    }

    #[test]
    fn rejects_unsafe_job_id_and_target_file() {
        let checkout = tempfile::tempdir().unwrap();
        init_bare_style_checkout(checkout.path());
        let worktrees_root = tempfile::tempdir().unwrap();

        let err = stage_draft(
            checkout.path(),
            worktrees_root.path(),
            "job/with/slashes",
            "crates/foo/src/lib.rs",
            "fn x() {}",
            "m",
        )
        .unwrap_err();
        assert!(err.to_string().contains("job_id"));

        let err = stage_draft(
            checkout.path(),
            worktrees_root.path(),
            "JOB3",
            "../../etc/passwd",
            "fn x() {}",
            "m",
        )
        .unwrap_err();
        assert!(err.to_string().contains("target_file"));
    }

    #[test]
    fn list_and_prune_round_trip() {
        let checkout = tempfile::tempdir().unwrap();
        init_bare_style_checkout(checkout.path());
        let worktrees_root = tempfile::tempdir().unwrap();

        assert!(list_staged(worktrees_root.path()).unwrap().is_empty());

        stage_draft(
            checkout.path(),
            worktrees_root.path(),
            "JOB4",
            "crates/foo/src/lib.rs",
            "fn x() {}",
            "m",
        )
        .unwrap();

        let staged = list_staged(worktrees_root.path()).unwrap();
        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].job_id, "JOB4");

        prune_staged(checkout.path(), worktrees_root.path(), "JOB4").unwrap();
        assert!(list_staged(worktrees_root.path()).unwrap().is_empty());
    }
}
