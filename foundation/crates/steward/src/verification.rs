//! Platform verification: routing a request to the machine that can verify it, and deciding from the signed results
//! whether a head is verified on every platform it must be (lane C129; design in
//! `governance/proposals/PLATFORM_VERIFICATION_ROUTING.md`).
//!
//! A request names a commit, a platform and a suite; it never carries a command. The node that receives it runs the
//! suite as defined by the base branch's own manifest (pinned by `suite_digest`), so a pull request cannot change what
//! verifies it. The node's result is signed and bound to the exact head and suite digest, so it needs no expiry.
//!
//! Pure functions: no I/O, no clock, no environment. Time arrives as ticks, signatures are verified by the adapter
//! (which sets `signature_verified`), and the node roster is an input. Every rule fails closed: an unknown platform,
//! an unauthorised node, an unverified signature, a stale head, or two results that disagree never count as a pass.

use crate::chain::digest;
use std::collections::{BTreeMap, BTreeSet};

/// Wire version of the request and result packets.
pub const PACKET_VERSION: u32 = 1;

fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A label such as `linux-x86_64` or `foundation-ci`: a lower-case letter, then letters, digits, `-`, `_` or `.`.
fn is_label(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some('a'..='z'))
        && text.chars().count() <= 48
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
}

fn is_repo(text: &str) -> bool {
    let valid = |part: &str| {
        !part.is_empty()
            && part.len() <= 100
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    text.split_once('/')
        .is_some_and(|(owner, name)| valid(owner) && valid(name) && !name.contains('/'))
}

/// Ask one platform to run one suite on one commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub repo: String,
    pub head: String,
    pub base_sha: String,
    pub platform: String,
    pub suite: String,
    /// SHA-256 of the suite manifest as it stands at `base_sha`. The node recomputes it from git and refuses a mismatch.
    pub suite_digest: String,
}

impl Request {
    /// Why the request is malformed, or `None`. Strict on purpose: the fields are joined into the id with newlines, so a
    /// field that could contain one must never get past this.
    #[must_use]
    pub fn problem(&self) -> Option<String> {
        if !is_repo(&self.repo) {
            return Some(format!("repo {:?} must be OWNER/NAME", self.repo));
        }
        for (name, value) in [("head", &self.head), ("base_sha", &self.base_sha)] {
            if !is_hex(value, 40) {
                return Some(format!("{name} must be a full 40-character lower-case SHA"));
            }
        }
        if !is_hex(&self.suite_digest, 64) {
            return Some("suite_digest must be a 64-character lower-case SHA-256".to_owned());
        }
        for (name, value) in [("platform", &self.platform), ("suite", &self.suite)] {
            if !is_label(value) {
                return Some(format!("{name} {value:?} must be a short lower-case label"));
            }
        }
        None
    }

    /// The request's identity: the same repository, commits, platform, suite and suite digest always give the same id, so
    /// a retried or duplicated request is recognised and routed to the same node.
    #[must_use]
    pub fn id(&self) -> String {
        digest(&format!(
            "v{PACKET_VERSION}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            self.repo, self.head, self.base_sha, self.platform, self.suite, self.suite_digest
        ))
    }
}

/// What the node reports about one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    /// The suite ran and a step failed.
    Fail,
    /// The suite could not run (a node problem, not a verdict about the code). Never a pass.
    Error,
}

/// One signed result as the adapter read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub request_id: String,
    pub head: String,
    pub platform: String,
    pub suite_digest: String,
    pub node: String,
    /// A retry of the same request carries a higher number; the highest attempt decides.
    pub attempt: u32,
    pub verdict: Verdict,
    /// Set by the adapter after checking the node's signature against the registry. Unverified results never count.
    pub signature_verified: bool,
}

/// How one required platform stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Pass,
    Fail,
    /// The suite could not run on the deciding attempt.
    Error,
    /// No result for this head and suite digest yet (or only ones that cannot count).
    Pending,
    /// The deciding attempt has two different verdicts.
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformStatus {
    pub platform: String,
    pub state: State,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    /// True only when at least one platform is required and every required platform passed.
    pub ok: bool,
    pub platforms: Vec<PlatformStatus>,
}

/// Decide whether `head` is verified on every `required` platform.
///
/// `authorised` maps each registered node to the platforms it may attest. A result counts only if its head, platform and
/// suite digest match, its signature was verified, and its node is authorised for that platform. Of the results that
/// count, the highest attempt decides; two different verdicts on that attempt are a conflict, not a pass. No required
/// platform at all is a misconfigured gate and is never ok.
#[must_use]
pub fn evidence(
    head: &str,
    suite_digest: &str,
    required: &[String],
    outcomes: &[Outcome],
    authorised: &BTreeMap<String, BTreeSet<String>>,
) -> Evidence {
    let wanted: BTreeSet<&String> = required.iter().collect();
    let mut platforms = Vec::new();
    for platform in wanted {
        let matching: Vec<&Outcome> = outcomes
            .iter()
            .filter(|o| o.head == head && o.platform == *platform && o.suite_digest == suite_digest)
            .collect();
        let unverified = matching.iter().filter(|o| !o.signature_verified).count();
        let unauthorised = matching
            .iter()
            .filter(|o| {
                o.signature_verified
                    && !authorised
                        .get(&o.node)
                        .is_some_and(|set| set.contains(platform))
            })
            .count();
        let counted: Vec<&Outcome> = matching
            .iter()
            .copied()
            .filter(|o| {
                o.signature_verified
                    && authorised
                        .get(&o.node)
                        .is_some_and(|set| set.contains(platform))
            })
            .collect();
        let status = match counted.iter().map(|o| o.attempt).max() {
            None => {
                let mut notes = Vec::new();
                if unverified > 0 {
                    notes.push(format!("{unverified} unverified result(s) ignored"));
                }
                if unauthorised > 0 {
                    notes.push(format!("{unauthorised} result(s) from a node not authorised for this platform ignored"));
                }
                PlatformStatus {
                    platform: platform.clone(),
                    state: State::Pending,
                    detail: if notes.is_empty() {
                        "no result yet".to_owned()
                    } else {
                        notes.join("; ")
                    },
                }
            }
            Some(top) => {
                let deciding: Vec<&&Outcome> =
                    counted.iter().filter(|o| o.attempt == top).collect();
                let verdicts: BTreeSet<&str> = deciding
                    .iter()
                    .map(|o| match o.verdict {
                        Verdict::Pass => "pass",
                        Verdict::Fail => "fail",
                        Verdict::Error => "error",
                    })
                    .collect();
                let nodes: BTreeSet<&str> = deciding.iter().map(|o| o.node.as_str()).collect();
                let (state, detail) = if verdicts.len() > 1 {
                    (
                        State::Conflict,
                        format!(
                            "attempt {top}: {} disagree",
                            nodes.iter().copied().collect::<Vec<_>>().join(", ")
                        ),
                    )
                } else {
                    match deciding[0].verdict {
                        Verdict::Pass => (
                            State::Pass,
                            format!(
                                "attempt {top} passed on {}",
                                nodes.iter().copied().collect::<Vec<_>>().join(", ")
                            ),
                        ),
                        Verdict::Fail => (
                            State::Fail,
                            format!(
                                "attempt {top} failed on {}",
                                nodes.iter().copied().collect::<Vec<_>>().join(", ")
                            ),
                        ),
                        Verdict::Error => (
                            State::Error,
                            format!(
                                "attempt {top} could not run on {}",
                                nodes.iter().copied().collect::<Vec<_>>().join(", ")
                            ),
                        ),
                    }
                };
                PlatformStatus {
                    platform: platform.clone(),
                    state,
                    detail,
                }
            }
        };
        platforms.push(status);
    }
    let ok = !platforms.is_empty() && platforms.iter().all(|p| p.state == State::Pass);
    Evidence { ok, platforms }
}

/// Which platforms a change must be verified on: `default`, plus the platform of every rule whose path prefix a changed
/// path starts with. Data, not code, so the rules live in a reviewed policy file.
#[must_use]
pub fn required_platforms(
    default: &[String],
    rules: &[(String, String)],
    changed_paths: &[String],
) -> Vec<String> {
    let mut required: BTreeSet<String> = default.iter().cloned().collect();
    for (prefix, platform) in rules {
        if changed_paths
            .iter()
            .any(|path| path.starts_with(prefix.as_str()))
        {
            required.insert(platform.clone());
        }
    }
    required.into_iter().collect()
}

/// One verification node as the hall sees it after its latest heartbeat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    pub platforms: BTreeSet<String>,
    pub online: bool,
    pub free_slots: u32,
    pub queue_depth: u32,
}

/// A node is online while its last heartbeat is recent in the hall's own ticks (no clock comparison across machines).
#[must_use]
pub fn is_online(now_tick: u64, last_beat_tick: u64, offline_after_ticks: u64) -> bool {
    now_tick >= last_beat_tick && now_tick - last_beat_tick <= offline_after_ticks
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Send the request to this node.
    Assign(String),
    /// A node exists for the platform but none can take it now; keep it queued and try again.
    Defer(String),
    /// No registered node supports the platform. Retrying will not help until one is registered.
    Unsupported(String),
}

/// Pick the node for one request. A node that already holds the request's lease keeps it while it is online and still
/// supports the platform, so a duplicate packet is never run twice. Otherwise the online node with a free slot and the
/// shortest queue wins, ties broken by name, so the same roster always gives the same answer.
#[must_use]
pub fn route(platform: &str, roster: &[Node], current_lease: Option<&str>) -> Route {
    if let Some(holder) = current_lease {
        if roster
            .iter()
            .any(|n| n.name == holder && n.online && n.platforms.contains(platform))
        {
            return Route::Assign(holder.to_owned());
        }
    }
    let supporting: Vec<&Node> = roster
        .iter()
        .filter(|n| n.platforms.contains(platform))
        .collect();
    if supporting.is_empty() {
        return Route::Unsupported(format!("no registered node supports {platform}"));
    }
    let mut ready: Vec<&Node> = supporting
        .iter()
        .copied()
        .filter(|n| n.online && n.free_slots > 0)
        .collect();
    ready.sort_by(|a, b| {
        a.queue_depth
            .cmp(&b.queue_depth)
            .then_with(|| a.name.cmp(&b.name))
    });
    match ready.first() {
        Some(node) => Route::Assign(node.name.clone()),
        None => Route::Defer(format!(
            "{} node(s) support {platform} but none is online with a free slot",
            supporting.len()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const BASE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn request() -> Request {
        Request {
            repo: "Aarogaming/aaroneous-devtools".into(),
            head: HEAD.into(),
            base_sha: BASE.into(),
            platform: "linux-x86_64".into(),
            suite: "foundation-ci".into(),
            suite_digest: DIGEST.into(),
        }
    }

    fn outcome(node: &str, platform: &str, attempt: u32, verdict: Verdict) -> Outcome {
        Outcome {
            request_id: "id".into(),
            head: HEAD.into(),
            platform: platform.into(),
            suite_digest: DIGEST.into(),
            node: node.into(),
            attempt,
            verdict,
            signature_verified: true,
        }
    }

    fn auth() -> BTreeMap<String, BTreeSet<String>> {
        let set = |items: &[&str]| {
            items
                .iter()
                .map(|s| (*s).to_owned())
                .collect::<BTreeSet<_>>()
        };
        BTreeMap::from([
            ("ubuntu-1".to_owned(), set(&["linux-x86_64"])),
            ("windows-1".to_owned(), set(&["windows-x86_64"])),
        ])
    }

    fn req(platforms: &[&str]) -> Vec<String> {
        platforms.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn the_request_id_is_stable_and_every_field_changes_it() {
        let base = request();
        assert_eq!(base.id(), request().id());
        assert_eq!(base.id().len(), 64);
        for variant in [
            Request {
                head: "d".repeat(40),
                ..request()
            },
            Request {
                base_sha: "d".repeat(40),
                ..request()
            },
            Request {
                platform: "windows-x86_64".into(),
                ..request()
            },
            Request {
                suite: "dashboard".into(),
                ..request()
            },
            Request {
                suite_digest: "d".repeat(64),
                ..request()
            },
            Request {
                repo: "Aarogaming/aaroneous".into(),
                ..request()
            },
        ] {
            assert_ne!(base.id(), variant.id());
        }
    }

    #[test]
    fn a_malformed_request_is_refused_before_it_can_be_hashed_ambiguously() {
        assert_eq!(request().problem(), None);
        for bad in [
            Request {
                repo: "no-slash".into(),
                ..request()
            },
            Request {
                repo: "a/b/c".into(),
                ..request()
            },
            Request {
                head: HEAD[..39].into(),
                ..request()
            },
            Request {
                head: HEAD.to_uppercase(),
                ..request()
            },
            Request {
                platform: "Linux".into(),
                ..request()
            },
            Request {
                platform: "linux\nwindows".into(),
                ..request()
            },
            Request {
                suite: "".into(),
                ..request()
            },
            Request {
                suite_digest: "x".repeat(64),
                ..request()
            },
        ] {
            assert!(bad.problem().is_some(), "{bad:?}");
        }
    }

    #[test]
    fn a_head_is_verified_only_when_every_required_platform_passed() {
        let both = req(&["linux-x86_64", "windows-x86_64"]);
        let linux_only = [outcome("ubuntu-1", "linux-x86_64", 1, Verdict::Pass)];
        let result = evidence(HEAD, DIGEST, &both, &linux_only, &auth());
        assert!(!result.ok);
        assert_eq!(result.platforms[1].platform, "windows-x86_64");
        assert_eq!(result.platforms[1].state, State::Pending);
        let both_pass = [
            linux_only[0].clone(),
            outcome("windows-1", "windows-x86_64", 1, Verdict::Pass),
        ];
        assert!(evidence(HEAD, DIGEST, &both, &both_pass, &auth()).ok);
    }

    #[test]
    fn nothing_required_is_a_misconfigured_gate_never_ok() {
        assert!(
            !evidence(
                HEAD,
                DIGEST,
                &[],
                &[outcome("ubuntu-1", "linux-x86_64", 1, Verdict::Pass)],
                &auth()
            )
            .ok
        );
    }

    #[test]
    fn a_result_that_cannot_count_is_ignored_and_says_why() {
        let need = req(&["linux-x86_64"]);
        let cases: Vec<(&str, Outcome)> = vec![
            (
                "another head",
                Outcome {
                    head: "d".repeat(40),
                    ..outcome("ubuntu-1", "linux-x86_64", 1, Verdict::Pass)
                },
            ),
            (
                "another suite digest",
                Outcome {
                    suite_digest: "d".repeat(64),
                    ..outcome("ubuntu-1", "linux-x86_64", 1, Verdict::Pass)
                },
            ),
            (
                "another platform",
                outcome("windows-1", "windows-x86_64", 1, Verdict::Pass),
            ),
            (
                "unverified signature",
                Outcome {
                    signature_verified: false,
                    ..outcome("ubuntu-1", "linux-x86_64", 1, Verdict::Pass)
                },
            ),
            (
                "unauthorised node",
                outcome("windows-1", "linux-x86_64", 1, Verdict::Pass),
            ),
            (
                "unknown node",
                outcome("stranger", "linux-x86_64", 1, Verdict::Pass),
            ),
        ];
        for (why, case) in cases {
            let result = evidence(HEAD, DIGEST, &need, &[case], &auth());
            assert!(!result.ok, "{why} must not pass");
            assert_eq!(result.platforms[0].state, State::Pending, "{why}");
        }
        let detail = evidence(
            HEAD,
            DIGEST,
            &need,
            &[Outcome {
                signature_verified: false,
                ..outcome("ubuntu-1", "linux-x86_64", 1, Verdict::Pass)
            }],
            &auth(),
        )
        .platforms[0]
            .detail
            .clone();
        assert!(detail.contains("unverified"), "{detail}");
    }

    #[test]
    fn the_highest_attempt_decides_and_a_disagreement_is_a_conflict() {
        let need = req(&["linux-x86_64"]);
        let retry_passes = [
            outcome("ubuntu-1", "linux-x86_64", 1, Verdict::Fail),
            outcome("ubuntu-1", "linux-x86_64", 2, Verdict::Pass),
        ];
        assert!(evidence(HEAD, DIGEST, &need, &retry_passes, &auth()).ok);
        let later_fails = [
            outcome("ubuntu-1", "linux-x86_64", 1, Verdict::Pass),
            outcome("ubuntu-1", "linux-x86_64", 2, Verdict::Fail),
        ];
        let result = evidence(HEAD, DIGEST, &need, &later_fails, &auth());
        assert!(!result.ok);
        assert_eq!(result.platforms[0].state, State::Fail);
        let mut auth_two = auth();
        auth_two.insert(
            "ubuntu-2".to_owned(),
            BTreeSet::from(["linux-x86_64".to_owned()]),
        );
        let disagree = [
            outcome("ubuntu-1", "linux-x86_64", 3, Verdict::Pass),
            outcome("ubuntu-2", "linux-x86_64", 3, Verdict::Fail),
        ];
        let result = evidence(HEAD, DIGEST, &need, &disagree, &auth_two);
        assert!(!result.ok);
        assert_eq!(result.platforms[0].state, State::Conflict);
    }

    #[test]
    fn a_node_error_is_never_a_pass() {
        let result = evidence(
            HEAD,
            DIGEST,
            &req(&["linux-x86_64"]),
            &[outcome("ubuntu-1", "linux-x86_64", 1, Verdict::Error)],
            &auth(),
        );
        assert!(!result.ok);
        assert_eq!(result.platforms[0].state, State::Error);
    }

    #[test]
    fn evidence_does_not_depend_on_the_order_of_results() {
        let need = req(&["linux-x86_64", "windows-x86_64"]);
        let results = vec![
            outcome("ubuntu-1", "linux-x86_64", 1, Verdict::Fail),
            outcome("ubuntu-1", "linux-x86_64", 2, Verdict::Pass),
            outcome("windows-1", "windows-x86_64", 1, Verdict::Pass),
        ];
        let expected = evidence(HEAD, DIGEST, &need, &results, &auth());
        let mut shuffled = results.clone();
        for rotation in 0..shuffled.len() {
            shuffled.rotate_left(rotation);
            assert_eq!(evidence(HEAD, DIGEST, &need, &shuffled, &auth()), expected);
        }
        let mut reversed = results;
        reversed.reverse();
        assert_eq!(evidence(HEAD, DIGEST, &need, &reversed, &auth()), expected);
    }

    #[test]
    fn required_platforms_come_from_data_not_code() {
        let default = req(&["linux-x86_64"]);
        let rules = vec![(
            "foundation/crates/platform_bridge/".to_owned(),
            "windows-x86_64".to_owned(),
        )];
        assert_eq!(
            required_platforms(&default, &rules, &["governance/x.md".into()]),
            ["linux-x86_64"]
        );
        assert_eq!(
            required_platforms(
                &default,
                &rules,
                &["foundation/crates/platform_bridge/src/lib.rs".into()]
            ),
            ["linux-x86_64", "windows-x86_64"]
        );
        assert_eq!(required_platforms(&[], &[], &[]), Vec::<String>::new());
    }

    fn node(name: &str, platform: &str, online: bool, free: u32, queue: u32) -> Node {
        Node {
            name: name.into(),
            platforms: BTreeSet::from([platform.to_owned()]),
            online,
            free_slots: free,
            queue_depth: queue,
        }
    }

    #[test]
    fn a_request_goes_to_the_node_for_its_platform() {
        let roster = [
            node("ubuntu-1", "linux-x86_64", true, 1, 0),
            node("windows-1", "windows-x86_64", true, 1, 0),
        ];
        assert_eq!(
            route("linux-x86_64", &roster, None),
            Route::Assign("ubuntu-1".into())
        );
        assert_eq!(
            route("windows-x86_64", &roster, None),
            Route::Assign("windows-1".into())
        );
        assert!(matches!(
            route("macos-aarch64", &roster, None),
            Route::Unsupported(_)
        ));
    }

    #[test]
    fn an_offline_or_full_node_defers_instead_of_failing_or_misrouting() {
        for roster in [
            vec![node("windows-1", "windows-x86_64", false, 1, 0)],
            vec![node("windows-1", "windows-x86_64", true, 0, 0)],
        ] {
            assert!(matches!(
                route("windows-x86_64", &roster, None),
                Route::Defer(_)
            ));
        }
    }

    #[test]
    fn the_least_loaded_node_wins_ties_break_by_name_and_a_held_lease_sticks() {
        let roster = [
            node("b", "linux-x86_64", true, 1, 2),
            node("a", "linux-x86_64", true, 1, 2),
            node("c", "linux-x86_64", true, 1, 5),
        ];
        assert_eq!(
            route("linux-x86_64", &roster, None),
            Route::Assign("a".into())
        );
        let mut busier = roster.to_vec();
        busier[1].queue_depth = 9;
        assert_eq!(
            route("linux-x86_64", &busier, None),
            Route::Assign("b".into())
        );
        assert_eq!(
            route("linux-x86_64", &roster, Some("c")),
            Route::Assign("c".into()),
            "a duplicate packet stays on its holder"
        );
        let mut gone = roster.to_vec();
        gone[2].online = false;
        assert_eq!(
            route("linux-x86_64", &gone, Some("c")),
            Route::Assign("a".into()),
            "a dead holder's request moves on"
        );
    }

    #[test]
    fn a_node_is_online_while_its_last_beat_is_recent_in_the_halls_own_ticks() {
        assert!(is_online(10, 10, 3));
        assert!(is_online(13, 10, 3));
        assert!(!is_online(14, 10, 3));
        assert!(
            !is_online(5, 10, 3),
            "a beat from the future is not trusted"
        );
    }
}
