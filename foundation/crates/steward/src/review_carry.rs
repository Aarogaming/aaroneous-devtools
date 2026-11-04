//! Which files of a pull request are still the files a reviewer saw? (lane C190)
//!
//! A review is bound to one head. When the branch is later updated (typically by merging main into it) the reviewer must decide
//! what, if anything, needs a fresh look. [`compare`] answers the mechanical half of that question from four trees: the reviewed
//! head, optionally the base it was reviewed against, the current head, and the current tip of main. For every path where the
//! current head differs from main (the pull request's own change today) it says whether the content is byte-identical to what was
//! reviewed. It is a re-review aid, not an acceptance: identical files can still interact with a changed main, and nothing here
//! reads or writes a review record.
//!
//! [`compare`] is pure. [`compare_commits`] is the thin adapter that lists the trees through [`crate::release_tree::list_tree`],
//! which already hardens the git environment, requires full object ids that name commits, and bounds the output it buffers.

use crate::release::Entry;
use crate::release_tree::list_tree;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// What happened to one path between the reviewed head and the current head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The pull request's own change at this path today is byte-identical (mode and object id) to the reviewed head.
    Carried,
    /// The current head differs from main here and from the reviewed head, and either main did not change this path since the
    /// review base or no base was given, so the author is the likely source of the difference.
    Changed,
    /// As [`Status::Changed`], but main also changed this path since the reviewed base, so the difference may come from main rather
    /// than from the author. It still needs a look; the reason is only a hint.
    ChangedMainAlsoMoved,
    /// The path is part of the pull request's own change today and was not in the reviewed head at all.
    NewSinceReview,
    /// The reviewed head itself changed this path (it differs from the review base), but the current head now equals main: the
    /// change was dropped or has since landed on main. Informational; listed only when the review base is given.
    ConvergedToMain,
}

impl Status {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Status::Carried => "carried",
            Status::Changed => "changed",
            Status::ChangedMainAlsoMoved => "changed_main_also_moved",
            Status::NewSinceReview => "new_since_review",
            Status::ConvergedToMain => "converged_to_main",
        }
    }

    /// True when the path needs a reviewer to look again.
    #[must_use]
    pub fn needs_review(self) -> bool {
        matches!(
            self,
            Status::Changed | Status::ChangedMainAlsoMoved | Status::NewSinceReview
        )
    }
}

/// One path and what became of it. Paths that are identical in every tree are not listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub path: String,
    pub status: Status,
}

/// The result of [`compare`]: the rows in path order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub rows: Vec<Row>,
}

impl Report {
    /// True when no path needs a reviewer to look again.
    #[must_use]
    pub fn all_carried(&self) -> bool {
        !self.rows.iter().any(|row| row.status.needs_review())
    }

    /// Row count for one status.
    #[must_use]
    pub fn count(&self, status: Status) -> usize {
        self.rows.iter().filter(|row| row.status == status).count()
    }
}

type Index<'a> = BTreeMap<&'a str, (u32, &'a str)>;

fn index(entries: &[Entry]) -> Index<'_> {
    entries
        .iter()
        .map(|entry| (entry.path.as_str(), (entry.mode, entry.oid.as_str())))
        .collect()
}

/// Compare the four trees. `reviewed_base` is the tree of the base the review was done against; without it a changed path whose
/// difference might come from main is reported as plain [`Status::Changed`].
#[must_use]
pub fn compare(
    reviewed: &[Entry],
    reviewed_base: Option<&[Entry]>,
    current: &[Entry],
    main: &[Entry],
) -> Report {
    let reviewed = index(reviewed);
    let base = reviewed_base.map(index);
    let current = index(current);
    let main = index(main);
    let paths: BTreeSet<&str> = reviewed
        .keys()
        .chain(current.keys())
        .chain(main.keys())
        .copied()
        .collect();
    let mut rows = Vec::new();
    for path in paths {
        let (r, c, m) = (reviewed.get(path), current.get(path), main.get(path));
        let status = if c == m {
            // Only a path the reviewed head itself changed (it differs from the review base) can have converged; without the
            // base every path main touched since the review would look like one, so none is listed.
            let reviewer_saw_a_change = base.as_ref().is_some_and(|b| b.get(path) != r);
            if r == c || !reviewer_saw_a_change {
                continue;
            }
            Status::ConvergedToMain
        } else if r == c {
            Status::Carried
        } else if r.is_none() {
            Status::NewSinceReview
        } else if base.as_ref().is_some_and(|b| b.get(path) != m) {
            Status::ChangedMainAlsoMoved
        } else {
            Status::Changed
        };
        rows.push(Row {
            path: path.to_owned(),
            status,
        });
    }
    Report { rows }
}

/// The report as JSON: counts per status, whether everything carried, and every listed path with its status.
#[must_use]
pub fn summarize(report: &Report) -> Value {
    let counts: BTreeMap<&str, usize> = [
        Status::Carried,
        Status::Changed,
        Status::ChangedMainAlsoMoved,
        Status::NewSinceReview,
        Status::ConvergedToMain,
    ]
    .into_iter()
    .map(|status| (status.name(), report.count(status)))
    .collect();
    let rows: Vec<Value> = report
        .rows
        .iter()
        .map(|row| json!({"path": row.path, "status": row.status.name()}))
        .collect();
    json!({
        "all_carried": report.all_carried(),
        "counts": counts,
        "rows": rows,
    })
}

/// List the four trees with git and [`compare`] them. `git` and `repo` must be absolute paths; each commit must be a full
/// lower-case object id of a commit (see [`list_tree`]).
///
/// # Errors
/// A sentence naming what failed.
pub fn compare_commits(
    git: &Path,
    repo: &Path,
    reviewed: &str,
    reviewed_base: Option<&str>,
    current: &str,
    main: &str,
) -> Result<Report, String> {
    let reviewed_tree = list_tree(git, repo, reviewed)?;
    let base_tree = reviewed_base
        .map(|commit| list_tree(git, repo, commit))
        .transpose()?;
    let current_tree = list_tree(git, repo, current)?;
    let main_tree = list_tree(git, repo, main)?;
    Ok(compare(
        &reviewed_tree,
        base_tree.as_deref(),
        &current_tree,
        &main_tree,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, oid: &str) -> Entry {
        Entry {
            path: path.to_owned(),
            mode: 0o100_644,
            oid: oid.to_owned(),
        }
    }

    fn status_of(report: &Report, path: &str) -> Option<Status> {
        report
            .rows
            .iter()
            .find(|row| row.path == path)
            .map(|row| row.status)
    }

    #[test]
    fn a_path_identical_everywhere_is_not_listed() {
        let tree = [entry("a", "1")];
        let report = compare(&tree, None, &tree, &tree);
        assert!(report.rows.is_empty());
        assert!(report.all_carried());
    }

    #[test]
    fn the_pull_requests_own_unchanged_file_is_carried() {
        let reviewed = [entry("a", "2")];
        let main = [entry("a", "1")];
        let report = compare(&reviewed, None, &reviewed, &main);
        assert_eq!(status_of(&report, "a"), Some(Status::Carried));
        assert!(report.all_carried());
    }

    #[test]
    fn a_file_the_author_changed_since_the_review_is_changed() {
        let reviewed = [entry("a", "2")];
        let current = [entry("a", "3")];
        let main = [entry("a", "1")];
        let report = compare(&reviewed, None, &current, &main);
        assert_eq!(status_of(&report, "a"), Some(Status::Changed));
        assert!(!report.all_carried());
    }

    #[test]
    fn a_difference_where_main_also_moved_is_flagged_as_possibly_from_main() {
        let base = [entry("a", "1")];
        let reviewed = [entry("a", "2")];
        let current = [entry("a", "4")];
        let main = [entry("a", "9")];
        let report = compare(&reviewed, Some(&base), &current, &main);
        assert_eq!(status_of(&report, "a"), Some(Status::ChangedMainAlsoMoved));
        assert!(!report.all_carried());
    }

    #[test]
    fn a_difference_where_main_did_not_move_stays_plain_changed() {
        let base = [entry("a", "1")];
        let reviewed = [entry("a", "2")];
        let current = [entry("a", "4")];
        let report = compare(&reviewed, Some(&base), &current, &base);
        assert_eq!(status_of(&report, "a"), Some(Status::Changed));
    }

    #[test]
    fn a_file_added_after_the_review_is_new() {
        let reviewed = [entry("a", "2")];
        let current = [entry("a", "2"), entry("b", "5")];
        let main = [entry("a", "1")];
        let report = compare(&reviewed, None, &current, &main);
        assert_eq!(status_of(&report, "a"), Some(Status::Carried));
        assert_eq!(status_of(&report, "b"), Some(Status::NewSinceReview));
        assert!(!report.all_carried());
    }

    #[test]
    fn a_change_that_reached_main_is_informational_only() {
        let base = [entry("a", "1")];
        let reviewed = [entry("a", "2")];
        let current = [entry("a", "2")];
        let main = [entry("a", "2")];
        // The reviewed head carried the change, and main now has it: identical everywhere, nothing listed.
        assert!(
            compare(&reviewed, Some(&base), &current, &main)
                .rows
                .is_empty()
        );

        // The reviewed head carried a change the current head no longer has because main absorbed it in another form.
        let current = [entry("a", "3")];
        let main = [entry("a", "3")];
        let report = compare(&reviewed, Some(&base), &current, &main);
        assert_eq!(status_of(&report, "a"), Some(Status::ConvergedToMain));
        assert!(report.all_carried());
    }

    #[test]
    fn a_path_main_changed_after_the_review_is_not_listed_as_converged() {
        // The reviewer never saw a change here (reviewed equals base), and main has since moved and the branch merged it.
        let base = [entry("a", "1")];
        let reviewed = [entry("a", "1")];
        let current = [entry("a", "5")];
        let main = [entry("a", "5")];
        assert!(
            compare(&reviewed, Some(&base), &current, &main)
                .rows
                .is_empty()
        );
        // Without the base the same situation must not be listed either.
        assert!(compare(&reviewed, None, &current, &main).rows.is_empty());
    }

    #[test]
    fn a_file_the_pull_request_now_deletes_counts_as_changed() {
        let reviewed = [entry("a", "2")];
        let main = [entry("a", "1")];
        let report = compare(&reviewed, None, &[], &main);
        assert_eq!(status_of(&report, "a"), Some(Status::Changed));
    }

    #[test]
    fn a_mode_change_is_a_difference() {
        let mut executable = entry("a", "2");
        executable.mode = 0o100_755;
        let reviewed = [entry("a", "2")];
        let current = [executable];
        let main = [entry("a", "1")];
        let report = compare(&reviewed, None, &current, &main);
        assert_eq!(status_of(&report, "a"), Some(Status::Changed));
    }

    #[test]
    fn rows_come_out_in_path_order_with_counts_and_a_verdict() {
        let reviewed = [entry("z", "2"), entry("m", "2")];
        let current = [entry("z", "2"), entry("m", "7"), entry("b", "5")];
        let main = [entry("z", "1"), entry("m", "1")];
        let report = compare(&reviewed, None, &current, &main);
        let paths: Vec<&str> = report.rows.iter().map(|row| row.path.as_str()).collect();
        assert_eq!(paths, ["b", "m", "z"]);
        let summary = summarize(&report);
        assert_eq!(summary["all_carried"], json!(false));
        assert_eq!(summary["counts"]["carried"], json!(1));
        assert_eq!(summary["counts"]["changed"], json!(1));
        assert_eq!(summary["counts"]["new_since_review"], json!(1));
        assert_eq!(summary["rows"][0]["path"], json!("b"));
        assert_eq!(summary["rows"][0]["status"], json!("new_since_review"));
    }

    #[test]
    fn the_adapter_refuses_relative_paths_before_running_anything() {
        let id = "0".repeat(40);
        let error =
            compare_commits(Path::new("git"), Path::new("repo"), &id, None, &id, &id).unwrap_err();
        assert!(error.contains("absolute"), "{error}");
    }
}
