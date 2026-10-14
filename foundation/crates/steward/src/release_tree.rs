//! Reading the tracked tree of a commit for the release export (lane C136, phase 2).
//!
//! [`parse_ls_tree`] is pure and strict: it turns the bytes of `git ls-tree -r -z --full-tree <commit>` into
//! [`crate::release::Entry`] values, and refuses anything it cannot represent exactly (a non-UTF-8 path, a malformed record, a mode
//! that does not match its type, an object id that is not one). [`list_tree`] is the thin adapter that runs git for one commit,
//! with the same environment hardening the other local adapters use: every inherited `GIT_*` variable removed, replace objects off,
//! system and global configuration neutralised, no prompts. [`summarize`] turns a plan into a dry-run report.

use crate::release::{ExportPolicy, Plan, Withheld, plan_export};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use crate::release::Entry;

/// Largest `ls-tree` output accepted, so a hostile repository cannot exhaust memory.
const LIMIT: usize = 64 * 1024 * 1024;

fn is_oid(text: &str) -> bool {
    (text.len() == 40 || text.len() == 64)
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Parse `git ls-tree -r -z --full-tree` output. Each record is `<mode> SP <type> SP <oid> TAB <path>` ended by NUL.
///
/// # Errors
/// A sentence naming the first record that cannot be represented exactly.
pub fn parse_ls_tree(output: &[u8]) -> Result<Vec<Entry>, String> {
    if output.len() > LIMIT {
        return Err("tree listing exceeds the size limit".to_owned());
    }
    let mut entries = Vec::new();
    if output.is_empty() {
        return Ok(entries);
    }
    let body = output
        .strip_suffix(&[0])
        .ok_or("tree listing is not NUL-terminated")?;
    for (index, record) in body.split(|b| *b == 0).enumerate() {
        let tab = record
            .iter()
            .position(|b| *b == b'\t')
            .ok_or_else(|| format!("record {index} has no tab"))?;
        let (meta, rest) = record.split_at(tab);
        let path_bytes = &rest[1..];
        let meta = std::str::from_utf8(meta)
            .map_err(|_| format!("record {index} metadata is not UTF-8"))?;
        let path = std::str::from_utf8(path_bytes)
            .map_err(|_| format!("record {index} has a path that is not valid UTF-8"))?
            .to_owned();
        let parts: Vec<&str> = meta.split(' ').collect();
        let [mode_text, kind, oid] = parts.as_slice() else {
            return Err(format!(
                "record {index} metadata is not '<mode> <type> <oid>'"
            ));
        };
        if mode_text.len() != 6 || !mode_text.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
            return Err(format!("record {index} has a malformed mode {mode_text:?}"));
        }
        let mode = u32::from_str_radix(mode_text, 8)
            .map_err(|_| format!("record {index} mode is not octal"))?;
        let consistent = match *kind {
            "blob" => matches!(mode, 0o100_644 | 0o100_755 | 0o120_000),
            "commit" => mode == 0o160_000,
            _ => false,
        };
        if !consistent {
            return Err(format!(
                "record {index} has type {kind:?} with mode {mode_text}, which git does not produce"
            ));
        }
        if !is_oid(oid) {
            return Err(format!("record {index} has a malformed object id"));
        }
        entries.push(Entry {
            path,
            mode,
            oid: (*oid).to_owned(),
        });
    }
    Ok(entries)
}

/// List the tracked tree of `commit` in `repo`. `git` must be an absolute path to a native git executable; `commit` must be a full
/// lower-case SHA, so it can never be read as an option or a ref name.
///
/// # Errors
/// A sentence naming what failed.
pub fn list_tree(git: &Path, repo: &Path, commit: &str) -> Result<Vec<Entry>, String> {
    if !git.is_absolute() || !repo.is_absolute() {
        return Err("explicit absolute git and repository paths required".to_owned());
    }
    if !is_oid(commit) {
        return Err("commit must be a full lower-case object id".to_owned());
    }
    let mut command = Command::new(git);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    let output = command
        .arg("-C")
        .arg(repo)
        .args([
            "ls-tree",
            "-r",
            "-z",
            "--full-tree",
            "--end-of-options",
            commit,
        ])
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "git ls-tree failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_ls_tree(&output.stdout)
}

fn reason_name(reason: &Withheld) -> &'static str {
    match reason {
        Withheld::Unsafe(_) => "unsafe",
        Withheld::Link => "link",
        Withheld::Excluded(_) => "excluded",
        Withheld::NotIncluded => "not_included",
        Withheld::Sensitive(_) => "sensitive",
    }
}

/// A dry-run report for a plan: counts, the paths that would block a release (unsafe or secret-shaped, each with its reason), the
/// tree digest, and how the withheld remainder splits by top-level area. It never lists published content, only paths.
#[must_use]
pub fn summarize(plan: &Plan) -> Value {
    let mut by_reason: BTreeMap<&str, usize> = BTreeMap::new();
    let mut blockers = Vec::new();
    let mut areas: BTreeMap<String, usize> = BTreeMap::new();
    for (path, reason) in &plan.withheld {
        *by_reason.entry(reason_name(reason)).or_default() += 1;
        match reason {
            Withheld::Unsafe(why) => {
                blockers.push(json!({"path": path, "kind": "unsafe", "why": why}))
            }
            Withheld::Sensitive(what) => {
                blockers.push(json!({"path": path, "kind": "sensitive", "why": what}))
            }
            Withheld::NotIncluded => {
                let area = path
                    .split_once('/')
                    .map_or("(root file)".to_owned(), |(top, _)| top.to_owned());
                *areas.entry(area).or_default() += 1;
            }
            _ => {}
        }
    }
    json!({
        "mode": "export-dry-run",
        "published": plan.publish.len(),
        "withheld": plan.withheld.len(),
        "withheld_by_reason": by_reason,
        "not_included_by_area": areas,
        "release_blockers": blockers,
        "tree_digest": plan.tree_digest,
    })
}

/// Plan the export of one commit: list its tree and apply the policy.
///
/// # Errors
/// When the tree cannot be listed or parsed exactly.
pub fn plan_commit(
    git: &Path,
    repo: &Path,
    commit: &str,
    policy: &ExportPolicy,
) -> Result<Plan, String> {
    Ok(plan_export(policy, &list_tree(git, repo, commit)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn record(mode: &str, kind: &str, oid: &str, path: &str) -> Vec<u8> {
        let mut bytes = format!("{mode} {kind} {oid}\t{path}").into_bytes();
        bytes.push(0);
        bytes
    }

    fn listing(records: &[Vec<u8>]) -> Vec<u8> {
        records.concat()
    }

    #[test]
    fn ordinary_records_parse_exactly() {
        let bytes = listing(&[
            record("100644", "blob", OID, "README.md"),
            record("100755", "blob", OID, "scripts/run.sh"),
            record("120000", "blob", OID, "link"),
            record("160000", "commit", OID, "vendor/sub"),
        ]);
        let entries = parse_ls_tree(&bytes).unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(
            entries[0],
            Entry {
                path: "README.md".into(),
                mode: 0o100_644,
                oid: OID.into()
            }
        );
        assert_eq!(entries[1].mode, 0o100_755);
        assert_eq!(entries[2].mode, 0o120_000);
        assert_eq!(entries[3].mode, 0o160_000);
        assert_eq!(parse_ls_tree(b"").unwrap(), vec![]);
    }

    #[test]
    fn paths_with_spaces_tabs_newlines_and_unicode_survive_the_nul_format() {
        let paths = [
            "a b/c d.txt",
            "tab\there",
            "line\nbreak",
            "caf\u{e9}/\u{4e2d}\u{6587}.md",
            "quote\"s",
            "back\\slash",
        ];
        let bytes = listing(
            &paths
                .iter()
                .map(|p| record("100644", "blob", OID, p))
                .collect::<Vec<_>>(),
        );
        let entries = parse_ls_tree(&bytes).unwrap();
        let got: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(got, paths);
    }

    #[test]
    fn a_hostile_or_malformed_listing_is_refused_not_guessed_at() {
        let good = record("100644", "blob", OID, "a");
        let cases: Vec<(&str, Vec<u8>)> = vec![
            (
                "not NUL terminated",
                b"100644 blob aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\ta".to_vec(),
            ),
            (
                "no tab",
                b"100644 blob aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa a\0".to_vec(),
            ),
            (
                "extra field",
                b"100644 blob aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa extra\ta\0".to_vec(),
            ),
            (
                "missing field",
                b"100644 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\ta\0".to_vec(),
            ),
            ("non octal mode", record("100649", "blob", OID, "a")),
            ("short mode", record("644", "blob", OID, "a")),
            ("tree entry under -r", record("040000", "tree", OID, "dir")),
            (
                "blob with a submodule mode",
                record("160000", "blob", OID, "a"),
            ),
            (
                "commit with a file mode",
                record("100644", "commit", OID, "a"),
            ),
            ("unknown mode", record("100600", "blob", OID, "a")),
            ("unknown type", record("100644", "tag", OID, "a")),
            ("short oid", record("100644", "blob", "abc", "a")),
            (
                "upper-case oid",
                record("100644", "blob", &OID.to_uppercase(), "a"),
            ),
            ("non UTF-8 path", {
                let mut bytes = format!("100644 blob {OID}\t").into_bytes();
                bytes.extend_from_slice(&[0xff, 0xfe]);
                bytes.push(0);
                bytes
            }),
            (
                "a good record then a bad one",
                [good.clone(), record("100644", "blob", "zz", "b")].concat(),
            ),
        ];
        for (why, bytes) in cases {
            assert!(parse_ls_tree(&bytes).is_err(), "{why} must be refused");
        }
        assert!(parse_ls_tree(&good).is_ok());
    }

    #[test]
    fn the_size_limit_is_enforced() {
        let huge = vec![b'x'; LIMIT + 1];
        assert!(parse_ls_tree(&huge).unwrap_err().contains("size limit"));
    }

    #[test]
    fn the_adapter_refuses_relative_paths_and_anything_that_is_not_a_full_object_id() {
        let git = if cfg!(windows) {
            Path::new("C:/git/git.exe")
        } else {
            Path::new("/usr/bin/git")
        };
        let repo = if cfg!(windows) {
            Path::new("C:/repo")
        } else {
            Path::new("/repo")
        };
        for bad in [
            "",
            "HEAD",
            "main",
            "--all",
            "abc",
            &"A".repeat(40),
            &format!("{OID} --stat"),
        ] {
            let error = list_tree(git, repo, bad).unwrap_err();
            assert!(
                error.contains("full lower-case object id"),
                "{bad:?} must be refused before git runs, got {error:?}"
            );
        }
        for (git, repo) in [(Path::new("git"), repo), (git, Path::new("repo"))] {
            let error = list_tree(git, repo, OID).unwrap_err();
            assert!(
                error.contains("absolute"),
                "a relative path must be refused before git runs, got {error:?}"
            );
        }
    }

    #[test]
    fn both_object_id_widths_are_accepted_and_nothing_between() {
        let sha256 = "b".repeat(64);
        let entries = parse_ls_tree(&record("100644", "blob", &sha256, "a")).unwrap();
        assert_eq!(entries[0].oid, sha256);
        for length in [0, 39, 41, 63, 65] {
            let bad = "c".repeat(length);
            assert!(
                parse_ls_tree(&record("100644", "blob", &bad, "a")).is_err(),
                "{length}"
            );
        }
    }

    #[test]
    fn the_dry_run_report_counts_reasons_lists_blockers_and_never_lists_content() {
        let policy = ExportPolicy {
            include: vec!["foundation/".into(), "README.md".into()],
            exclude: vec!["foundation/target/".into()],
        };
        let bytes = listing(&[
            record("100644", "blob", OID, "README.md"),
            record("100644", "blob", OID, "foundation/a.rs"),
            record("100644", "blob", OID, "foundation/.env"),
            record("100644", "blob", OID, "foundation/CON.txt"),
            record("100644", "blob", OID, "foundation/target/x"),
            record("120000", "blob", OID, "foundation/link"),
            record("100644", "blob", OID, "governance/q.md"),
            record("100644", "blob", OID, "scripts/a.py"),
            record("100644", "blob", OID, "LICENSE"),
        ]);
        let plan = plan_export(&policy, &parse_ls_tree(&bytes).unwrap());
        let report = summarize(&plan);
        assert_eq!(report["mode"], "export-dry-run");
        assert_eq!(report["published"], 2);
        assert_eq!(report["withheld"], 7);
        assert_eq!(report["withheld_by_reason"]["sensitive"], 1);
        assert_eq!(report["withheld_by_reason"]["unsafe"], 1);
        assert_eq!(report["withheld_by_reason"]["link"], 1);
        assert_eq!(report["withheld_by_reason"]["excluded"], 1);
        assert_eq!(report["withheld_by_reason"]["not_included"], 3);
        assert_eq!(report["not_included_by_area"]["governance"], 1);
        assert_eq!(report["not_included_by_area"]["scripts"], 1);
        assert_eq!(report["not_included_by_area"]["(root file)"], 1);
        let blockers = report["release_blockers"].as_array().unwrap();
        assert_eq!(blockers.len(), 2);
        assert!(
            blockers
                .iter()
                .any(|b| b["path"] == "foundation/.env" && b["kind"] == "sensitive")
        );
        assert!(
            blockers
                .iter()
                .any(|b| b["path"] == "foundation/CON.txt" && b["kind"] == "unsafe")
        );
        assert_eq!(report["tree_digest"], plan.tree_digest);
        let text = report.to_string();
        assert!(
            !text.contains("foundation/a.rs"),
            "published paths are counted, not listed"
        );
    }
}
