//! Release export: what leaves the private, authoritative repository, and whether a release may be cut (lane C135; design in
//! `governance/proposals/PRIVATE_AUTHORITY_AND_RELEASE_EXPORT.md`).
//!
//! The private repository keeps everything: governance, the queue, attestations, operational history. A public repository, if
//! there is one, receives only deliberate releases. This module is the pure core of that boundary: `plan_export` decides, path by
//! path, what may be published (default-deny: a path must be in an included area, outside every excluded one, safe to write on any
//! platform, not a link, and not secret-looking), and `decide` says whether a release candidate may be cut at all.
//!
//! Pure functions: no I/O, no clock, no environment. The adapter lists the tracked tree of a commit (path and git mode), runs the
//! publication scanners, and asks the verification evidence; this module only combines what it is given. It fails closed: a path
//! it cannot judge is withheld, and anything that looks like a mistake blocks the release instead of being silently skipped.

use crate::verification::Evidence;
use std::collections::BTreeSet;

/// One tracked path of the commit being exported, with its git file mode (`100644`, `100755`, `120000` symlink, `160000` submodule).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub mode: u32,
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
    /// The path itself is dangerous to write out (traversal, a git-metadata alias, a platform alias). Blocks a release.
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
}

fn matches_rule(path: &str, rule: &str) -> bool {
    if rule.ends_with('/') {
        path.starts_with(rule)
    } else {
        path == rule
    }
}

/// A path that is safe to create on Linux, macOS and Windows without escaping the export directory or touching git metadata.
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
        // Windows drops trailing dots and spaces and folds case, so ".git." and ".GIT " both name the git directory there.
        let folded = segment.trim_end_matches(['.', ' ']).to_ascii_lowercase();
        if folded == ".git" || folded.starts_with("git~") {
            return Some(format!("{segment:?} names git metadata on some platforms"));
        }
        if folded.is_empty() {
            return Some("a segment is only dots or spaces".to_owned());
        }
        if segment.contains(':') {
            return Some("a segment contains ':' (an alternate data stream on Windows)".to_owned());
        }
    }
    None
}

/// Names that should never sit in a published area, whatever the include rules say.
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

/// Decide, for every tracked path, whether it may be published. The order is fixed so the same inputs always give the same plan:
/// an unsafe path shape, then links and submodules, then exclude rules, then the include requirement, then secret-shaped names.
#[must_use]
pub fn plan_export(policy: &ExportPolicy, entries: &[Entry]) -> Plan {
    let mut publish = BTreeSet::new();
    let mut withheld = Vec::new();
    for entry in entries {
        let path = entry.path.as_str();
        let reason = if let Some(problem) = path_problem(path) {
            Some(Withheld::Unsafe(problem))
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
                publish.insert(entry.path.clone());
            }
        }
    }
    withheld.sort_by(|a, b| a.0.cmp(&b.0));
    Plan {
        publish: publish.into_iter().collect(),
        withheld,
    }
}

/// A release candidate as the adapter assembled it.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// The full SHA of the private commit being released.
    pub commit: String,
    /// `vMAJOR.MINOR.PATCH`, optionally with a `-pre.release` suffix of lower-case letters, digits and dots.
    pub tag: String,
    /// The platform verification evidence for exactly this commit (see `verification::evidence`).
    pub verification: Evidence,
    /// True only when the publication scanners (secret scan of the export tree, dependency advisories, the publication check)
    /// all ran on the export tree and found nothing.
    pub publication_checks_clean: bool,
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
/// A release needs: a well-formed tag and full commit SHA; verification that passed on every required platform for this exact
/// commit; clean publication checks on the export tree; no open blockers; something to publish; and an export plan with no
/// unsafe path and no secret-shaped file inside a published area (those are mistakes to look at, not paths to skip).
#[must_use]
pub fn decide(candidate: &Candidate, plan: &Plan) -> Decision {
    let mut blockers = Vec::new();
    if !valid_tag(&candidate.tag) {
        blockers.push(format!("tag {:?} is not vMAJOR.MINOR.PATCH", candidate.tag));
    }
    if candidate.commit.len() != 40
        || !candidate
            .commit
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
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
    if !candidate.publication_checks_clean {
        blockers.push("the publication checks did not run clean on the export tree".to_owned());
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
    use crate::verification::{PlatformStatus, State};

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
    fn links_and_submodules_are_withheld_quietly() {
        let plan = plan_export(
            &policy(),
            &[
                Entry {
                    path: "foundation/link".into(),
                    mode: 0o120_000,
                },
                Entry {
                    path: "foundation/sub".into(),
                    mode: 0o160_000,
                },
                file("foundation/ok.rs"),
            ],
        );
        assert_eq!(plan.publish, ["foundation/ok.rs"]);
        assert_eq!(reason_of(&plan, "foundation/link"), Some(Withheld::Link));
        assert_eq!(reason_of(&plan, "foundation/sub"), Some(Withheld::Link));
        let ok = Candidate {
            commit: "a".repeat(40),
            tag: "v0.1.0".into(),
            verification: passing(),
            publication_checks_clean: true,
            open_blockers: 0,
        };
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
        let candidate = Candidate {
            commit: "a".repeat(40),
            tag: "v0.1.0".into(),
            verification: passing(),
            publication_checks_clean: true,
            open_blockers: 0,
        };
        let mut with_one_file = entries.clone();
        with_one_file.push(file("README.md"));
        let result = decide(&candidate, &plan_export(&policy(), &with_one_file));
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

    fn passing() -> Evidence {
        Evidence {
            ok: true,
            platforms: vec![
                PlatformStatus {
                    platform: "linux-x86_64".into(),
                    state: State::Pass,
                    detail: "attempt 1 passed".into(),
                },
                PlatformStatus {
                    platform: "windows-x86_64".into(),
                    state: State::Pass,
                    detail: "attempt 1 passed".into(),
                },
            ],
        }
    }

    fn good() -> (Candidate, Plan) {
        let plan = plan_export(&policy(), &[file("foundation/a.rs"), file("README.md")]);
        (
            Candidate {
                commit: "a".repeat(40),
                tag: "v1.2.3".into(),
                verification: passing(),
                publication_checks_clean: true,
                open_blockers: 0,
            },
            plan,
        )
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
    fn each_condition_blocks_on_its_own_and_every_blocker_is_reported() {
        let (good_candidate, good_plan) = good();
        let failing = Evidence {
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
                    verification: failing.clone(),
                    ..good_candidate.clone()
                },
                good_plan.clone(),
                "windows-x86_64",
            ),
            (
                "publication checks",
                Candidate {
                    publication_checks_clean: false,
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
            verification: failing,
            publication_checks_clean: false,
            open_blockers: 1,
        };
        let result = decide(
            &everything_wrong,
            &Plan {
                publish: vec![],
                withheld: vec![],
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
