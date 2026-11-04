//! `review_carry::compare_commits` and the `review-carry` binary against a real scratch git repository (lane C190).
//!
//! History built here, all in one repository:
//!
//! ```text
//! B  a=1 b=1 c=1                 the base a review was done against
//! R  a=2 b=2        (from B)     the reviewed head
//! M  c=main         (from B)     main moved: touched c only
//! N  b=x c=main     (from B)     main moved: touched b and c
//! C  a=2 b=3 c=main d=new (from R)  the current head: author changed b, added d, merged main's c
//! ```

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use steward::review_carry::{Status, compare_commits, summarize};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_git_bin() -> PathBuf {
    PathBuf::from(
        std::env::var("DEVTOOLS_TEST_GIT")
            .expect("explicit DEVTOOLS_TEST_GIT native executable required"),
    )
}

struct Repo {
    git: PathBuf,
    dir: PathBuf,
    base: String,
    reviewed: String,
    main_c_only: String,
    main_b_and_c: String,
    current: String,
}

impl Repo {
    fn git(git: &Path, dir: &Path, args: &[&str]) -> String {
        let mut child = Command::new(git)
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
            .args(["-c", "core.autocrlf=false"])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(b"").unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    /// Write `files` into the work tree, commit everything, and return the new commit id.
    fn commit(git: &Path, dir: &Path, files: &[(&str, &str)], message: &str) -> String {
        for (name, text) in files {
            fs::write(dir.join(name), text).unwrap();
        }
        Self::git(git, dir, &["add", "-A"]);
        Self::git(git, dir, &["commit", "-q", "-m", message]);
        Self::git(git, dir, &["rev-parse", "HEAD"])
    }

    fn new() -> Self {
        let git = test_git_bin();
        let base = std::env::var_os("DEVTOOLS_TEST_SUPPORT_DIR")
            .or_else(|| std::env::var_os("DEVTOOLS_TEST_TMP"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../tmp"));
        fs::create_dir_all(&base).unwrap();
        // `create_dir` (not `create_dir_all`) so a name that already exists fails loudly instead of being shared.
        let dir = base.canonicalize().unwrap().join(format!(
            "review-carry-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&dir).unwrap();
        Self::git(&git, &dir, &["init", "--quiet"]);
        let b = Self::commit(&git, &dir, &[("a", "1"), ("b", "1"), ("c", "1")], "B");
        let r = Self::commit(&git, &dir, &[("a", "2"), ("b", "2")], "R");
        Self::git(&git, &dir, &["checkout", "-q", "--detach", &b]);
        let m = Self::commit(&git, &dir, &[("c", "main")], "M");
        Self::git(&git, &dir, &["checkout", "-q", "--detach", &b]);
        let n = Self::commit(&git, &dir, &[("b", "x"), ("c", "main")], "N");
        Self::git(&git, &dir, &["checkout", "-q", "--detach", &r]);
        let c = Self::commit(
            &git,
            &dir,
            &[("a", "2"), ("b", "3"), ("c", "main"), ("d", "new")],
            "C",
        );
        Self {
            git,
            dir,
            base: b,
            reviewed: r,
            main_c_only: m,
            main_b_and_c: n,
            current: c,
        }
    }

    fn statuses(&self, main: &str, base: Option<&str>) -> Vec<(String, Status)> {
        compare_commits(
            &self.git,
            &self.dir,
            &self.reviewed,
            base,
            &self.current,
            main,
        )
        .unwrap()
        .rows
        .into_iter()
        .map(|row| (row.path, row.status))
        .collect()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn row(path: &str, status: Status) -> (String, Status) {
    (path.to_owned(), status)
}

#[test]
fn real_trees_give_carried_changed_and_new() {
    let repo = Repo::new();
    // Main touched only c, which the branch merged: a is the author's unchanged change, b the author changed after the review,
    // d is new. c equals main and the reviewer never saw a change there, so it is not listed.
    let rows = repo.statuses(&repo.main_c_only, Some(&repo.base));
    assert_eq!(
        rows,
        [
            row("a", Status::Carried),
            row("b", Status::Changed),
            row("d", Status::NewSinceReview),
        ]
    );
}

#[test]
fn a_changed_path_that_main_also_moved_is_flagged_as_such() {
    let repo = Repo::new();
    let rows = repo.statuses(&repo.main_b_and_c, Some(&repo.base));
    assert_eq!(
        rows,
        [
            row("a", Status::Carried),
            row("b", Status::ChangedMainAlsoMoved),
            row("d", Status::NewSinceReview),
        ]
    );
    // Without the base the same path cannot be attributed and reads as plain changed.
    let rows = repo.statuses(&repo.main_b_and_c, None);
    assert_eq!(rows[1], row("b", Status::Changed));
}

#[test]
fn the_binary_exits_two_when_something_needs_a_look_and_prints_the_summary() {
    let repo = Repo::new();
    let output = Command::new(env!("CARGO_BIN_EXE_review-carry"))
        .arg(&repo.git)
        .arg(&repo.dir)
        .args([&repo.reviewed, &repo.current, &repo.main_c_only, &repo.base])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let printed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(printed["all_carried"], false);
    assert_eq!(printed["counts"]["carried"], 1);
    assert_eq!(printed["counts"]["changed"], 1);
    assert_eq!(printed["counts"]["new_since_review"], 1);
    let report = compare_commits(
        &repo.git,
        &repo.dir,
        &repo.reviewed,
        Some(&repo.base),
        &repo.current,
        &repo.main_c_only,
    )
    .unwrap();
    assert_eq!(printed, summarize(&report));
}

#[test]
fn the_binary_exits_zero_when_the_head_is_the_reviewed_head() {
    let repo = Repo::new();
    let output = Command::new(env!("CARGO_BIN_EXE_review-carry"))
        .arg(&repo.git)
        .arg(&repo.dir)
        .args([&repo.reviewed, &repo.reviewed, &repo.main_c_only])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let printed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(printed["all_carried"], true);
}

#[test]
fn the_binary_exits_one_on_an_object_that_is_not_a_commit_id() {
    let repo = Repo::new();
    let output = Command::new(env!("CARGO_BIN_EXE_review-carry"))
        .arg(&repo.git)
        .arg(&repo.dir)
        .args(["HEAD", &repo.current, &repo.main_c_only])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
}
