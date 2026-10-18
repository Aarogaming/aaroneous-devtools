//! Release export: what leaves the private, authoritative repository, and whether a release may be cut (lane C136; design in
//! `governance/proposals/PRIVATE_AUTHORITY_AND_RELEASE_EXPORT.md`).
//!
//! The private repository keeps everything: governance, the queue, attestations, operational history. A public repository, if
//! there is one, receives only deliberate releases. This module is the pure core of that boundary: `plan_export` decides, path by
//! path, what may be published (default-deny: a path must be in an included area, outside every excluded one, writable under one
//! canonical interpretation on every platform, not a link, and not secret-looking), and `decide` says whether a release candidate
//! may be cut at all.
//!
//! Pure functions: no I/O, no clock, no environment. The adapter lists the tracked tree of a commit (path, git mode and object id),
//! runs the publication scanners over the export tree, and asks the verification evidence; this module only combines what it is
//! given. It fails closed: a path it cannot judge is withheld, and anything that looks like a mistake blocks the release instead of
//! being silently skipped. The results it is handed are bound to what they are about: the verification evidence names its commit,
//! and the scan result names the exact tree digest it scanned, so a green result computed for another commit or another tree is
//! refused, not trusted.

use crate::chain::digest;
use crate::verification::Evidence;
use std::collections::BTreeMap;

/// One tracked path of the commit being exported: its git file mode (`100644`, `100755`, `120000` symlink, `160000` submodule,
/// written as the number the octal string denotes) and its git object id (the content identity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub mode: u32,
    pub oid: String,
}

/// What may be published, as data. A rule is a path prefix ending in `/` (everything below it) or an exact file path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportPolicy {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

/// Why a path is not published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Withheld {
    /// The path (or its object id) is dangerous or ambiguous to write out: traversal, a git-metadata alias, a platform alias, a
    /// reserved device name, a name that collides with another on a case-insensitive file system. Blocks a release.
    Unsafe(String),
    /// A symbolic link or a submodule: the target is not part of the export. Withheld quietly; it is not a mistake.
    Link,
    /// Matches an exclude rule, which wins over any include.
    Excluded(String),
    /// Not inside any included area. The default.
    NotIncluded,
    /// Inside a published area but shaped like a credential, key or private state. Blocks a release.
    Sensitive(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Paths to publish, sorted.
    pub publish: Vec<String>,
    /// Everything else with the reason, sorted by path.
    pub withheld: Vec<(String, Withheld)>,
    /// SHA-256 over every published entry's mode, path and object id, in path order. The scanners' result must name this value:
    /// it is how a clean scan is tied to the exact tree that would be published.
    pub tree_digest: String,
}

fn matches_rule(path: &str, rule: &str) -> bool {
    if rule.ends_with('/') {
        path.starts_with(rule)
    } else {
        path == rule
    }
}

const RESERVED_DEVICE_NAMES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// A path with one canonical interpretation on Linux, macOS and Windows: no traversal, no drive or absolute form, no characters
/// Windows forbids, no trailing dot or space (Windows drops them, so `credentials.json.` and `credentials.json` are one file there),
/// no reserved device name even with an extension, no `.git` alias. A name that is not exactly its canonical form is rejected, never
/// renamed: renaming could change the reviewed tree or collide with another path.
fn path_problem(path: &str) -> Option<String> {
    if path.is_empty() || path.len() > 400 {
        return Some("empty or over 400 characters".to_owned());
    }
    if path
        .chars()
        .any(|c| c == '\0' || c.is_control() || c == '\\')
    {
        return Some("contains a control character or a backslash".to_owned());
    }
    if path.starts_with('/') || path.as_bytes().get(1) == Some(&b':') {
        return Some("absolute or drive-qualified".to_owned());
    }
    for segment in path.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Some("has an empty, '.' or '..' segment".to_owned());
        }
        if segment.ends_with('.') || segment.ends_with(' ') {
            return Some(format!(
                "segment {segment:?} ends in a dot or space, which Windows drops (an alias of another name)"
            ));
        }
        if segment.starts_with(' ') {
            return Some(format!("segment {segment:?} starts with a space"));
        }
        if let Some(bad) = segment
            .chars()
            .find(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
        {
            return Some(format!(
                "segment {segment:?} contains {bad:?}, which Windows forbids (or ':' for an alternate data stream)"
            ));
        }
        let lowered = segment.to_ascii_lowercase();
        if lowered == ".git" || lowered.starts_with("git~") {
            return Some(format!("{segment:?} names git metadata on some platforms"));
        }
        let stem = lowered.split('.').next().unwrap_or("");
        if RESERVED_DEVICE_NAMES.contains(&stem) {
            return Some(format!("{segment:?} is a reserved Windows device name"));
        }
    }
    None
}

/// Names that should never sit in a published area, whatever the include rules say. Only ever called with a path that already
/// passed [`path_problem`], so there is one spelling to compare, not several aliases.
fn sensitive(path: &str) -> Option<String> {
    let lowered = path.to_ascii_lowercase();
    for segment in lowered.split('/') {
        if matches!(segment, ".ssh" | ".steward" | ".gnupg" | ".aws" | ".kube") {
            return Some(format!("a {segment} directory"));
        }
    }
    let name = lowered.rsplit('/').next().unwrap_or("");
    if name == ".env" || name.starts_with(".env.") {
        return Some("an .env file".to_owned());
    }
    for suffix in [
        ".pem",
        ".key",
        ".p12",
        ".pfx",
        ".kdbx",
        ".jks",
        ".keystore",
        ".pgp",
        ".asc",
    ] {
        if name.ends_with(suffix) {
            return Some(format!("a {suffix} file"));
        }
    }
    for prefix in ["id_rsa", "id_dsa", "id_ecdsa", "id_ed25519"] {
        if name.starts_with(prefix) && !name.ends_with(".pub") {
            return Some("a private key file".to_owned());
        }
    }
    if matches!(
        name,
        "credentials"
            | "credentials.json"
            | "secrets.toml"
            | "secrets.json"
            | "secrets.yml"
            | "secrets.yaml"
            | ".netrc"
            | ".npmrc"
            | ".pypirc"
    ) {
        return Some("a credentials file".to_owned());
    }
    None
}

fn is_oid(text: &str) -> bool {
    (text.len() == 40 || text.len() == 64)
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Paths that name the same file or directory on a case-insensitive file system, or whose directory name equals another path's file
/// name there. Both sides of every collision are returned with the path they collide with.
fn collisions(entries: &[Entry]) -> BTreeMap<String, String> {
    let mut files: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for entry in entries {
        files
            .entry(entry.path.to_lowercase())
            .or_default()
            .push(entry.path.as_str());
    }
    let mut found = BTreeMap::new();
    for paths in files.values() {
        if paths.len() > 1 {
            for path in paths {
                let other = paths.iter().find(|p| *p != path).copied().unwrap_or("");
                found.insert((*path).to_owned(), other.to_owned());
            }
        }
    }
    // a directory component that equals another path's file name, ignoring case
    for entry in entries {
        let lowered = entry.path.to_lowercase();
        let mut prefix = String::new();
        let parts: Vec<&str> = lowered.split('/').collect();
        for part in &parts[..parts.len().saturating_sub(1)] {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if let Some(file_paths) = files.get(&prefix) {
                for file in file_paths {
                    found.insert(entry.path.clone(), (*file).to_owned());
                    found.insert((*file).to_owned(), entry.path.clone());
                }
            }
        }
    }
    found
}

/// Decide, for every tracked path, whether it may be published. The order is fixed so the same inputs always give the same plan:
/// an unsafe path shape or object id, a collision, links and submodules, exclude rules, the include requirement, then secret-shaped
/// names. Unsafe shapes come first and are checked before any rule or name comparison, so an alias of an excluded or secret name is
/// rejected as unsafe instead of slipping past the comparison that only knew the canonical spelling.
#[must_use]
pub fn plan_export(policy: &ExportPolicy, entries: &[Entry]) -> Plan {
    let clashes = collisions(entries);
    let mut publish: BTreeMap<String, (u32, String)> = BTreeMap::new();
    let mut withheld = Vec::new();
    for entry in entries {
        let path = entry.path.as_str();
        let reason = if let Some(problem) = path_problem(path) {
            Some(Withheld::Unsafe(problem))
        } else if !is_oid(&entry.oid) {
            Some(Withheld::Unsafe(
                "its object id is not a git object id".to_owned(),
            ))
        } else if let Some(other) = clashes.get(path) {
            Some(Withheld::Unsafe(format!(
                "collides with {other:?} on a case-insensitive file system"
            )))
        } else if entry.mode == 0o120_000 || entry.mode == 0o160_000 {
            Some(Withheld::Link)
        } else if let Some(rule) = policy.exclude.iter().find(|r| matches_rule(path, r)) {
            Some(Withheld::Excluded(rule.clone()))
        } else if !policy.include.iter().any(|r| matches_rule(path, r)) {
            Some(Withheld::NotIncluded)
        } else {
            sensitive(path).map(Withheld::Sensitive)
        };
        match reason {
            Some(reason) => withheld.push((entry.path.clone(), reason)),
            None => {
                publish.insert(entry.path.clone(), (entry.mode, entry.oid.clone()));
            }
        }
    }
    withheld.sort_by(|a, b| a.0.cmp(&b.0));
    let mut listing = String::from("release-tree-v1\n");
    for (path, (mode, oid)) in &publish {
        listing.push_str(&format!("{mode:o} {oid} {path}\n"));
    }
    Plan {
        publish: publish.keys().cloned().collect(),
        withheld,
        tree_digest: digest(&listing),
    }
}

/// A release candidate as the adapter assembled it.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// The repository the release is cut from (`OWNER/NAME`).
    pub repo: String,
    /// The full SHA of the private commit being released.
    pub commit: String,
    /// The base the verification requests were issued against.
    pub base_sha: String,
    /// `vMAJOR.MINOR.PATCH`, optionally with a `-pre.release` suffix of lower-case letters, digits and dots.
    pub tag: String,
    /// The platforms the release rule requires, sorted. The verification must have covered exactly these.
    pub required_platforms: Vec<String>,
    /// The platform verification evidence (see `verification::evidence`). It carries its own subject, which must be this candidate.
    pub verification: Evidence,
    /// The tree digest the publication scanners (secret scan, advisories, the publication check) ran on and found clean, or `None`
    /// if they did not run clean. It must equal the export plan's `tree_digest`.
    pub publication_clean_for_tree: Option<String>,
    /// Findings still open against this commit that the release rule treats as blocking.
    pub open_blockers: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub ok: bool,
    pub blockers: Vec<String>,
}

fn valid_tag(tag: &str) -> bool {
    let Some(rest) = tag.strip_prefix('v') else {
        return false;
    };
    let (core, pre) = match rest.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (rest, None),
    };
    let parts: Vec<&str> = core.split('.').collect();
    let numeric = |s: &&str| {
        !s.is_empty()
            && s.len() <= 6
            && s.bytes().all(|b| b.is_ascii_digit())
            && (s.len() == 1 || !s.starts_with('0'))
    };
    parts.len() == 3
        && parts.iter().all(numeric)
        && pre.is_none_or(|p| {
            !p.is_empty()
                && p.len() <= 32
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.')
                && !p.starts_with('.')
                && !p.ends_with('.')
                && !p.contains("..")
        })
}

/// May this candidate be cut as a release? Every blocker is listed, not just the first.
///
/// A release needs: a well-formed tag and full commit SHA; verification that passed on every required platform **for this exact
/// repository, commit and base** (the evidence's own subject is compared, so a pass computed for another commit is refused);
/// publication checks that ran clean **on the tree this plan would publish** (the scan's tree digest is compared); no open
/// blockers; something to publish; and an export plan with no unsafe path and no secret-shaped file inside a published area (those
/// are mistakes to look at, not paths to skip).
#[must_use]
pub fn decide(candidate: &Candidate, plan: &Plan) -> Decision {
    let mut blockers = Vec::new();
    if !valid_tag(&candidate.tag) {
        blockers.push(format!("tag {:?} is not vMAJOR.MINOR.PATCH", candidate.tag));
    }
    if !is_oid(&candidate.commit) || candidate.commit.len() != 40 {
        blockers.push("commit must be a full 40-character lower-case SHA".to_owned());
    }
    if !candidate.verification.ok {
        let pending: Vec<String> = candidate
            .verification
            .platforms
            .iter()
            .filter(|p| p.state != crate::verification::State::Pass)
            .map(|p| format!("{} ({})", p.platform, p.detail))
            .collect();
        blockers.push(format!(
            "platform verification has not passed: {}",
            if pending.is_empty() {
                "no platform evidence".to_owned()
            } else {
                pending.join("; ")
            }
        ));
    }
    match &candidate.verification.subject {
        None => blockers.push("the verification evidence does not say what it is about".to_owned()),
        Some(subject) => {
            if subject.repo != candidate.repo
                || subject.head != candidate.commit
                || subject.base_sha != candidate.base_sha
            {
                blockers.push(format!(
                    "the verification evidence is for {} at {} (base {}), not for {} at {} (base {})",
                    subject.repo, subject.head, subject.base_sha, candidate.repo, candidate.commit, candidate.base_sha
                ));
            }
            let mut verified: Vec<&str> = subject.requests.iter().map(|r| r.0.as_str()).collect();
            verified.sort_unstable();
            let mut required: Vec<&str> = candidate
                .required_platforms
                .iter()
                .map(String::as_str)
                .collect();
            required.sort_unstable();
            if verified != required {
                blockers.push(format!("the release requires platforms {required:?} but the verification covered {verified:?}"));
            }
        }
    }
    match &candidate.publication_clean_for_tree {
        None => {
            blockers.push("the publication checks did not run clean on the export tree".to_owned())
        }
        Some(scanned) if *scanned != plan.tree_digest => {
            blockers.push("the publication checks ran on a different tree than the one this plan would publish".to_owned());
        }
        Some(_) => {}
    }
    if candidate.open_blockers > 0 {
        blockers.push(format!(
            "{} open blocker(s) against this commit",
            candidate.open_blockers
        ));
    }
    if plan.publish.is_empty() {
        blockers.push("the export plan publishes nothing".to_owned());
    }
    for (path, reason) in &plan.withheld {
        match reason {
            Withheld::Unsafe(why) => blockers.push(format!("unsafe path {path:?}: {why}")),
            Withheld::Sensitive(what) => blockers.push(format!(
                "{path:?} looks like {what} but sits in a published area"
            )),
            _ => {}
        }
    }
    Decision {
        ok: blockers.is_empty(),
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::{
        Admission, Outcome, PlatformStatus, Request, State, Subject, Verdict, evidence,
    };
    use std::collections::{BTreeMap, BTreeSet};

    const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const OTHER_COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const BASE: &str = "cccccccccccccccccccccccccccccccccccccccc";
    const OID: &str = "dddddddddddddddddddddddddddddddddddddddd";
    const REPO: &str = "Aarogaming/aaroneous-devtools";

    fn policy() -> ExportPolicy {
        ExportPolicy {
            include: vec![
                "foundation/".into(),
                "gui/".into(),
                "README.md".into(),
                "LICENSE".into(),
                "docs/".into(),
            ],
            exclude: vec![
                "foundation/target/".into(),
                "docs/internal/".into(),
                "gui/node_modules/".into(),
            ],
        }
    }

    fn file(path: &str) -> Entry {
        Entry {
            path: path.into(),
            mode: 0o100_644,
            oid: OID.into(),
        }
    }

    fn reason_of(plan: &Plan, path: &str) -> Option<Withheld> {
        plan.withheld
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, r)| r.clone())
    }

    #[test]
    fn only_included_areas_are_published_and_everything_else_is_withheld_by_default() {
        let entries = [
            file("foundation/Cargo.toml"),
            file("foundation/crates/steward/src/lib.rs"),
            file("README.md"),
            file("governance/COORDINATION_QUEUE.md"),
            file("scripts/merge_gate.py"),
            file("READMEx"),
            file("README.md.bak"),
        ];
        let plan = plan_export(&policy(), &entries);
        assert_eq!(
            plan.publish,
            [
                "README.md",
                "foundation/Cargo.toml",
                "foundation/crates/steward/src/lib.rs"
            ]
        );
        for private in [
            "governance/COORDINATION_QUEUE.md",
            "scripts/merge_gate.py",
            "READMEx",
            "README.md.bak",
        ] {
            assert_eq!(
                reason_of(&plan, private),
                Some(Withheld::NotIncluded),
                "{private}"
            );
        }
    }

    #[test]
    fn an_exclude_rule_beats_an_include_rule() {
        let plan = plan_export(
            &policy(),
            &[
                file("foundation/target/debug/x"),
                file("docs/internal/plan.md"),
                file("docs/guide.md"),
            ],
        );
        assert_eq!(plan.publish, ["docs/guide.md"]);
        assert_eq!(
            reason_of(&plan, "docs/internal/plan.md"),
            Some(Withheld::Excluded("docs/internal/".into()))
        );
    }

    #[test]
    fn a_prefix_rule_matches_a_directory_boundary_not_a_name_prefix() {
        let plan = plan_export(
            &policy(),
            &[
                file("foundationx/a.rs"),
                file("foundation-old/a.rs"),
                file("docs/internal-notes.md"),
                file("docs/internal/a.md"),
            ],
        );
        assert_eq!(
            reason_of(&plan, "foundationx/a.rs"),
            Some(Withheld::NotIncluded)
        );
        assert_eq!(
            reason_of(&plan, "foundation-old/a.rs"),
            Some(Withheld::NotIncluded)
        );
        assert!(
            plan.publish.contains(&"docs/internal-notes.md".to_owned()),
            "a file whose name merely starts with an excluded name is not excluded"
        );
        assert_eq!(
            reason_of(&plan, "docs/internal/a.md"),
            Some(Withheld::Excluded("docs/internal/".into()))
        );
    }

    #[test]
    fn dangerous_path_shapes_are_unsafe_whatever_the_rules_say() {
        let hostile = [
            "foundation/../../etc/passwd",
            "/foundation/a",
            "C:/foundation/a",
            "foundation\\a",
            "foundation//a",
            "foundation/./a",
            "foundation/.git/config",
            "foundation/.GIT/config",
            "foundation/.git./config",
            "foundation/.git /config",
            "foundation/GIT~1/config",
            "foundation/a:stream",
            "foundation/...",
            "foundation/a\nb",
            "foundation/a\0b",
            "",
        ];
        let entries: Vec<Entry> = hostile.iter().map(|p| file(p)).collect();
        let plan = plan_export(&policy(), &entries);
        assert!(plan.publish.is_empty(), "{:?}", plan.publish);
        for path in hostile {
            assert!(
                matches!(reason_of(&plan, path), Some(Withheld::Unsafe(_))),
                "{path:?} -> {:?}",
                reason_of(&plan, path)
            );
        }
    }

    #[test]
    fn windows_aliases_of_credentials_and_private_directories_cannot_slip_past_the_name_checks() {
        // Codex review of #205: with the first version these were published, because the sensitive-name comparison only knew the
        // canonical spelling while Windows treats a trailing dot or space as nothing. The control is the plain spelling, withheld.
        let control = plan_export(&policy(), &[file("foundation/credentials.json")]);
        assert!(matches!(
            reason_of(&control, "foundation/credentials.json"),
            Some(Withheld::Sensitive(_))
        ));
        let aliases = [
            "foundation/credentials.json.",
            "foundation/token.key.",
            "foundation/.env ",
            "foundation/.ssh./config",
            "foundation/.ssh /config",
            "foundation/id_rsa.",
            "docs/.steward./seats/x",
            "foundation/ leading",
        ];
        let entries: Vec<Entry> = aliases.iter().map(|p| file(p)).collect();
        let plan = plan_export(&policy(), &entries);
        assert!(plan.publish.is_empty(), "{:?}", plan.publish);
        for path in aliases {
            assert!(
                matches!(reason_of(&plan, path), Some(Withheld::Unsafe(_))),
                "{path:?} -> {:?}",
                reason_of(&plan, path)
            );
        }
    }

    #[test]
    fn reserved_windows_device_names_and_forbidden_characters_are_unsafe_even_with_an_extension() {
        let reserved = [
            "foundation/CON",
            "foundation/CON.txt",
            "foundation/con.tar.gz",
            "foundation/Aux.rs",
            "foundation/NUL",
            "foundation/COM1.log",
            "foundation/lpt9",
            "foundation/sub/PRN.md",
            "foundation/a<b",
            "foundation/a>b",
            "foundation/a\"b",
            "foundation/a|b",
            "foundation/a?b",
            "foundation/a*b",
        ];
        let entries: Vec<Entry> = reserved.iter().map(|p| file(p)).collect();
        let plan = plan_export(&policy(), &entries);
        assert!(plan.publish.is_empty(), "{:?}", plan.publish);
        for path in reserved {
            assert!(
                matches!(reason_of(&plan, path), Some(Withheld::Unsafe(_))),
                "{path:?}"
            );
        }
        // names that merely contain a reserved word are fine
        let plan = plan_export(
            &policy(),
            &[
                file("foundation/console.rs"),
                file("foundation/auxiliary.rs"),
                file("foundation/com10.rs"),
                file("foundation/lpt0"),
            ],
        );
        assert_eq!(plan.publish.len(), 4);
    }

    #[test]
    fn paths_that_collide_on_a_case_insensitive_file_system_are_unsafe_on_both_sides() {
        let entries = [
            file("foundation/Readme.md"),
            file("foundation/README.md"),
            file("foundation/ok.rs"),
            file("foundation/Lib/a.rs"),
            file("foundation/lib/b.rs"),
        ];
        let plan = plan_export(&policy(), &entries);
        assert_eq!(
            plan.publish,
            [
                "foundation/Lib/a.rs",
                "foundation/lib/b.rs",
                "foundation/ok.rs"
            ],
            "different directories with different files do not clash"
        );
        for path in ["foundation/Readme.md", "foundation/README.md"] {
            match reason_of(&plan, path) {
                Some(Withheld::Unsafe(why)) => assert!(why.contains("collides"), "{why}"),
                other => panic!("{path}: {other:?}"),
            }
        }
        // a file and a directory that differ only in case cannot both be written
        let plan = plan_export(
            &policy(),
            &[file("foundation/Docs"), file("foundation/docs/a.md")],
        );
        assert!(plan.publish.is_empty(), "{:?}", plan.publish);
        assert!(matches!(
            reason_of(&plan, "foundation/Docs"),
            Some(Withheld::Unsafe(_))
        ));
        assert!(matches!(
            reason_of(&plan, "foundation/docs/a.md"),
            Some(Withheld::Unsafe(_))
        ));
        // the same path listed twice is not a tree
        let plan = plan_export(
            &policy(),
            &[file("foundation/a.rs"), file("foundation/a.rs")],
        );
        assert!(plan.publish.is_empty());
    }

    #[test]
    fn an_object_id_that_is_not_one_makes_the_entry_unsafe() {
        for bad in [
            "",
            "short",
            &"G".repeat(40),
            &"a".repeat(41),
            &"A".repeat(40),
        ] {
            let entry = Entry {
                path: "foundation/a.rs".into(),
                mode: 0o100_644,
                oid: bad.to_string(),
            };
            assert!(
                matches!(
                    reason_of(&plan_export(&policy(), &[entry]), "foundation/a.rs"),
                    Some(Withheld::Unsafe(_))
                ),
                "{bad:?}"
            );
        }
        let sha256_oid = Entry {
            path: "foundation/a.rs".into(),
            mode: 0o100_644,
            oid: "a".repeat(64),
        };
        assert_eq!(
            plan_export(&policy(), &[sha256_oid]).publish,
            ["foundation/a.rs"],
            "SHA-256 repositories are fine"
        );
    }

    #[test]
    fn links_and_submodules_are_withheld_quietly() {
        let plan = plan_export(
            &policy(),
            &[
                Entry {
                    path: "foundation/link".into(),
                    mode: 0o120_000,
                    oid: OID.into(),
                },
                Entry {
                    path: "foundation/sub".into(),
                    mode: 0o160_000,
                    oid: OID.into(),
                },
                file("foundation/ok.rs"),
            ],
        );
        assert_eq!(plan.publish, ["foundation/ok.rs"]);
        assert_eq!(reason_of(&plan, "foundation/link"), Some(Withheld::Link));
        assert_eq!(reason_of(&plan, "foundation/sub"), Some(Withheld::Link));
        let ok = candidate_for(&plan);
        assert!(decide(&ok, &plan).ok, "a withheld link is not a mistake");
    }

    #[test]
    fn secret_shaped_files_inside_a_published_area_are_marked_and_block_a_release() {
        let secrets = [
            "foundation/.env",
            "foundation/.env.local",
            "gui/server.key",
            "gui/cert.PEM",
            "foundation/id_ed25519",
            "foundation/id_rsa",
            "foundation/credentials.json",
            "docs/.ssh/config",
            "docs/.steward/seats/claude/gh/hosts.yml",
            "foundation/.npmrc",
        ];
        let entries: Vec<Entry> = secrets.iter().map(|p| file(p)).collect();
        let plan = plan_export(&policy(), &entries);
        assert!(plan.publish.is_empty());
        for path in secrets {
            assert!(
                matches!(reason_of(&plan, path), Some(Withheld::Sensitive(_))),
                "{path}"
            );
        }
        let mut with_one_file = entries.clone();
        with_one_file.push(file("README.md"));
        let plan = plan_export(&policy(), &with_one_file);
        let result = decide(&candidate_for(&plan), &plan);
        assert!(!result.ok);
        assert_eq!(result.blockers.len(), secrets.len());
        // public keys are fine, and a secret-shaped file outside any published area is simply not published
        let plan = plan_export(
            &policy(),
            &[
                file("foundation/id_ed25519.pub"),
                file("governance/server.key"),
            ],
        );
        assert_eq!(plan.publish, ["foundation/id_ed25519.pub"]);
        assert_eq!(
            reason_of(&plan, "governance/server.key"),
            Some(Withheld::NotIncluded)
        );
    }

    #[test]
    fn the_plan_does_not_depend_on_the_order_of_the_entries() {
        let entries = vec![
            file("b/x"),
            file("foundation/z.rs"),
            file("foundation/a.rs"),
            file("README.md"),
            file("../up"),
        ];
        let expected = plan_export(&policy(), &entries);
        let mut reversed = entries.clone();
        reversed.reverse();
        assert_eq!(plan_export(&policy(), &reversed), expected);
        assert_eq!(
            expected.publish,
            ["README.md", "foundation/a.rs", "foundation/z.rs"]
        );
    }

    #[test]
    fn the_tree_digest_changes_with_any_published_path_mode_or_content() {
        let base = plan_export(&policy(), &[file("foundation/a.rs"), file("README.md")]);
        assert_eq!(
            base.tree_digest,
            plan_export(&policy(), &[file("README.md"), file("foundation/a.rs")]).tree_digest
        );
        let changed_content = Entry {
            oid: "e".repeat(40),
            ..file("foundation/a.rs")
        };
        let changed_mode = Entry {
            mode: 0o100_755,
            ..file("foundation/a.rs")
        };
        let extra = file("foundation/b.rs");
        let variants = [
            plan_export(&policy(), &[changed_content, file("README.md")]),
            plan_export(&policy(), &[changed_mode, file("README.md")]),
            plan_export(
                &policy(),
                &[file("foundation/a.rs"), file("README.md"), extra],
            ),
            plan_export(&policy(), &[file("foundation/a.rs")]),
        ];
        for variant in variants {
            assert_ne!(variant.tree_digest, base.tree_digest);
        }
        // a withheld path does not change what is published, so it does not change the digest
        let with_private = plan_export(
            &policy(),
            &[
                file("foundation/a.rs"),
                file("README.md"),
                file("governance/x.md"),
            ],
        );
        assert_eq!(with_private.tree_digest, base.tree_digest);
    }

    fn request_on(platform: &str, commit: &str) -> Request {
        Request {
            repo: REPO.into(),
            head: commit.into(),
            base_sha: BASE.into(),
            platform: platform.into(),
            suite: "foundation-ci".into(),
            suite_digest: "f".repeat(64),
        }
    }

    /// Real evidence from the real constructor: both platforms passed for `commit`.
    fn passing_evidence_for(commit: &str) -> Evidence {
        let requests = [
            request_on("linux-x86_64", commit),
            request_on("windows-x86_64", commit),
        ];
        let nodes = [
            ("linux-x86_64", "ubuntu-1"),
            ("windows-x86_64", "windows-1"),
        ];
        let authorised: BTreeMap<String, BTreeSet<String>> = nodes
            .iter()
            .map(|(p, n)| ((*n).to_owned(), BTreeSet::from([(*p).to_owned()])))
            .collect();
        let admitted: BTreeMap<String, Admission> = requests
            .iter()
            .zip(nodes)
            .map(|(r, (_, n))| {
                (
                    r.id(),
                    Admission {
                        attempt: 1,
                        node: n.to_owned(),
                    },
                )
            })
            .collect();
        let outcomes: Vec<Outcome> = requests
            .iter()
            .zip(nodes)
            .map(|(r, (_, n))| Outcome {
                request_id: r.id(),
                head: r.head.clone(),
                platform: r.platform.clone(),
                suite_digest: r.suite_digest.clone(),
                node: n.to_owned(),
                attempt: 1,
                verdict: Verdict::Pass,
                signature_verified: true,
            })
            .collect();
        let result = evidence(&requests, &outcomes, &authorised, &admitted);
        assert!(result.ok);
        result
    }

    fn candidate_for(plan: &Plan) -> Candidate {
        Candidate {
            repo: REPO.into(),
            commit: COMMIT.into(),
            base_sha: BASE.into(),
            tag: "v1.2.3".into(),
            required_platforms: vec!["linux-x86_64".into(), "windows-x86_64".into()],
            verification: passing_evidence_for(COMMIT),
            publication_clean_for_tree: Some(plan.tree_digest.clone()),
            open_blockers: 0,
        }
    }

    fn good() -> (Candidate, Plan) {
        let plan = plan_export(&policy(), &[file("foundation/a.rs"), file("README.md")]);
        (candidate_for(&plan), plan)
    }

    #[test]
    fn a_verified_clean_candidate_with_something_to_publish_may_be_released() {
        let (candidate, plan) = good();
        assert_eq!(
            decide(&candidate, &plan),
            Decision {
                ok: true,
                blockers: vec![]
            }
        );
    }

    #[test]
    fn a_green_result_computed_for_another_commit_is_refused() {
        // Codex review of #205: evidence for commit A was accepted for candidate B, because Evidence had lost its subject.
        let (mut candidate, plan) = good();
        candidate.commit = OTHER_COMMIT.into();
        let result = decide(&candidate, &plan);
        assert!(!result.ok);
        assert!(
            result
                .blockers
                .iter()
                .any(|b| b.contains("verification evidence is for")),
            "{:?}",
            result.blockers
        );
        // genuinely verified for B, it is accepted; the same evidence is refused for A
        candidate.verification = passing_evidence_for(OTHER_COMMIT);
        assert!(decide(&candidate, &plan).ok);
        let for_a = Candidate {
            commit: COMMIT.into(),
            ..candidate.clone()
        };
        assert!(!decide(&for_a, &plan).ok);
        // the other parts of the subject are checked too
        for (why, changed) in [
            (
                "another repository",
                Candidate {
                    repo: "Aarogaming/aaroneous".into(),
                    ..candidate.clone()
                },
            ),
            (
                "another base",
                Candidate {
                    base_sha: "9".repeat(40),
                    ..candidate.clone()
                },
            ),
        ] {
            assert!(!decide(&changed, &plan).ok, "{why}");
        }
    }

    #[test]
    fn evidence_that_covers_other_platforms_than_the_release_requires_is_refused() {
        let (mut candidate, plan) = good();
        candidate.required_platforms = vec![
            "linux-x86_64".into(),
            "windows-x86_64".into(),
            "macos-aarch64".into(),
        ];
        let result = decide(&candidate, &plan);
        assert!(!result.ok);
        assert!(
            result.blockers.iter().any(|b| b.contains("macos-aarch64")),
            "{:?}",
            result.blockers
        );
        candidate.required_platforms = vec!["linux-x86_64".into()];
        assert!(
            !decide(&candidate, &plan).ok,
            "evidence for more platforms than required is a different claim, not silently fine"
        );
    }

    #[test]
    fn evidence_without_a_subject_identifies_nothing_and_is_refused() {
        let (mut candidate, plan) = good();
        candidate.verification = Evidence {
            subject: None,
            ..candidate.verification.clone()
        };
        let result = decide(&candidate, &plan);
        assert!(!result.ok);
        assert!(
            result
                .blockers
                .iter()
                .any(|b| b.contains("does not say what it is about"))
        );
        // and a hand-built subject for another commit cannot be mistaken for this one
        candidate.verification = Evidence {
            subject: Some(Subject {
                repo: REPO.into(),
                head: OTHER_COMMIT.into(),
                base_sha: BASE.into(),
                requests: vec![],
            }),
            ..candidate.verification.clone()
        };
        assert!(!decide(&candidate, &plan).ok);
    }

    #[test]
    fn a_clean_scan_of_a_different_tree_does_not_count() {
        let (mut candidate, plan) = good();
        let scanned_earlier = plan_export(&policy(), &[file("foundation/a.rs")]);
        candidate.publication_clean_for_tree = Some(scanned_earlier.tree_digest);
        let result = decide(&candidate, &plan);
        assert!(!result.ok);
        assert!(
            result.blockers.iter().any(|b| b.contains("different tree")),
            "{:?}",
            result.blockers
        );
        // the file changed after the scan: same paths, new content, so the old clean result no longer applies
        let drifted = plan_export(
            &policy(),
            &[
                Entry {
                    oid: "e".repeat(40),
                    ..file("foundation/a.rs")
                },
                file("README.md"),
            ],
        );
        let mut fresh = candidate_for(&plan);
        fresh.publication_clean_for_tree = Some(plan.tree_digest.clone());
        assert!(!decide(&fresh, &drifted).ok);
    }

    #[test]
    fn each_other_condition_blocks_on_its_own_and_every_blocker_is_reported() {
        let (good_candidate, good_plan) = good();
        let pending = Evidence {
            ok: false,
            platforms: vec![
                PlatformStatus {
                    platform: "linux-x86_64".into(),
                    state: State::Pass,
                    detail: "attempt 1 passed".into(),
                },
                PlatformStatus {
                    platform: "windows-x86_64".into(),
                    state: State::Pending,
                    detail: "attempt 1 on windows-1 has not reported".into(),
                },
            ],
            subject: good_candidate.verification.subject.clone(),
        };
        let cases: Vec<(&str, Candidate, Plan, &str)> = vec![
            (
                "bad tag",
                Candidate {
                    tag: "1.2.3".into(),
                    ..good_candidate.clone()
                },
                good_plan.clone(),
                "tag",
            ),
            (
                "tag with leading zero",
                Candidate {
                    tag: "v01.2.3".into(),
                    ..good_candidate.clone()
                },
                good_plan.clone(),
                "tag",
            ),
            (
                "tag with only two parts",
                Candidate {
                    tag: "v1.2".into(),
                    ..good_candidate.clone()
                },
                good_plan.clone(),
                "tag",
            ),
            (
                "tag injection",
                Candidate {
                    tag: "v1.2.3\nrm".into(),
                    ..good_candidate.clone()
                },
                good_plan.clone(),
                "tag",
            ),
            (
                "short commit",
                Candidate {
                    commit: "abc".into(),
                    ..good_candidate.clone()
                },
                good_plan.clone(),
                "40-character",
            ),
            (
                "verification pending",
                Candidate {
                    verification: pending,
                    ..good_candidate.clone()
                },
                good_plan.clone(),
                "windows-x86_64",
            ),
            (
                "publication checks did not run",
                Candidate {
                    publication_clean_for_tree: None,
                    ..good_candidate.clone()
                },
                good_plan.clone(),
                "publication checks",
            ),
            (
                "open blockers",
                Candidate {
                    open_blockers: 2,
                    ..good_candidate.clone()
                },
                good_plan.clone(),
                "2 open blocker",
            ),
            (
                "nothing to publish",
                good_candidate.clone(),
                Plan {
                    publish: vec![],
                    withheld: vec![],
                    tree_digest: good_plan.tree_digest.clone(),
                },
                "publishes nothing",
            ),
            (
                "unsafe path",
                good_candidate.clone(),
                plan_export(
                    &policy(),
                    &[file("foundation/a.rs"), file("foundation/../x")],
                ),
                "unsafe path",
            ),
        ];
        for (why, candidate, plan, needle) in cases {
            let result = decide(&candidate, &plan);
            assert!(!result.ok, "{why} must block");
            assert!(
                result.blockers.iter().any(|b| b.contains(needle)),
                "{why}: {:?}",
                result.blockers
            );
        }
        let everything_wrong = Candidate {
            commit: "x".into(),
            tag: "nope".into(),
            verification: Evidence {
                ok: false,
                platforms: vec![],
                subject: None,
            },
            publication_clean_for_tree: None,
            open_blockers: 1,
            ..good_candidate
        };
        let result = decide(
            &everything_wrong,
            &Plan {
                publish: vec![],
                withheld: vec![],
                tree_digest: String::new(),
            },
        );
        assert!(
            result.blockers.len() >= 6,
            "all blockers are listed: {:?}",
            result.blockers
        );
    }

    #[test]
    fn prerelease_tags_are_allowed_and_malformed_ones_are_not() {
        for ok in [
            "v0.0.1",
            "v1.2.3",
            "v10.20.30",
            "v1.2.3-rc.1",
            "v1.2.3-beta",
        ] {
            assert!(valid_tag(ok), "{ok}");
        }
        for bad in [
            "",
            "v",
            "1.2.3",
            "v1.2",
            "v1.2.3.4",
            "v1..3",
            "v1.2.x",
            "v1.2.3-",
            "v1.2.3-RC1",
            "v1.2.3-.x",
            "v1.2.3-a..b",
            "v1.2.3 ",
            "V1.2.3",
            "v1.2.3-a.",
            "v0001.2.3",
            "v1.2.3+meta",
        ] {
            assert!(!valid_tag(bad), "{bad:?}");
        }
    }
}
