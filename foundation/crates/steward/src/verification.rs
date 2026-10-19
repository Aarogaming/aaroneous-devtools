//! Platform verification: routing a request to the machine that can verify it, and deciding from the signed results
//! whether a head is verified on every platform it must be (lane C129; design in
//! `governance/proposals/PLATFORM_VERIFICATION_ROUTING.md`).
//!
//! A request names a commit, a platform and a suite; it never carries a command. The node that receives it runs the
//! suite as defined by the base branch's own manifest (pinned by `suite_digest`), so a pull request cannot change what
//! verifies it. The node's result is signed and bound to the exact head and suite digest, so it needs no expiry.
//!
//! Pure functions: no I/O, no clock, no environment. Time arrives as ticks, signatures are verified by the adapter
//! (which sets `signature_verified`), and the node roster and the hall's admitted attempts are inputs. Every rule fails
//! closed: an unknown platform, an unauthorised node, an unverified signature, a stale head, a result for another request,
//! a late result from an earlier attempt, or two results that disagree never count as a pass.
//!
//! Delivery is at-least-once and routing is not execution: `route` picks a holder, it does not stop an offline holder from
//! still running. Exactly one *effect* needs durable request and attempt state with atomic admission and fencing in the hall;
//! this module only guarantees that whatever arrives is judged against what the hall admitted.

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

/// Which candidate a piece of evidence is about: the repository, the exact head and base, and the suite and request identity per
/// platform. It is set by [`evidence`] from the validated requests it was given, never by a caller, so a green result cannot be
/// carried to a different commit without the mismatch being visible (a release decision compares it to its own candidate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    pub repo: String,
    pub head: String,
    pub base_sha: String,
    /// One entry per platform, sorted by platform: `(platform, suite, suite_digest, request_id)`.
    pub requests: Vec<(String, String, String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    /// True only when at least one platform is required and every required platform passed.
    pub ok: bool,
    pub platforms: Vec<PlatformStatus>,
    /// What this evidence is about. `None` when the expectation itself was empty, malformed or self-contradictory: such
    /// evidence is never ok and identifies nothing.
    pub subject: Option<Subject>,
}

/// What the hall has issued for one request: the attempt that is current and the node it was assigned to. This is the hall's
/// own durable state; a node cannot create or change it. An outcome counts only for the admitted attempt on the assigned
/// node, so a late result from an earlier attempt, or from a node the request was not assigned to, can never turn a newer
/// pending attempt green.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admission {
    pub attempt: u32,
    pub node: String,
}

/// Decide whether every expected request is verified.
///
/// `expected` is the set of validated requests the hall issued for one candidate (one per required platform: same repository,
/// head and base, different platforms). Each outcome is matched to its request by the request's **canonical id**, so a result
/// for a different repository, base, suite or digest never counts, however well its own head and platform fit. `admitted`
/// says which attempt is current and which node holds it. `authorised` maps each registered node to the platforms it may attest.
///
/// A result counts only if its id, head, platform and suite digest equal the request's, it is for the admitted attempt on the
/// assigned node, its signature was verified, and the node is authorised for the platform. Two different verdicts for that
/// attempt are a conflict, not a pass; `error` is never a pass; a request the hall has not admitted is pending. An empty,
/// malformed, duplicated-platform or self-contradictory expectation is a misconfigured gate and is never ok.
#[must_use]
pub fn evidence(
    expected: &[Request],
    outcomes: &[Outcome],
    authorised: &BTreeMap<String, BTreeSet<String>>,
    admitted: &BTreeMap<String, Admission>,
) -> Evidence {
    let broken = |detail: String| Evidence {
        ok: false,
        platforms: vec![PlatformStatus {
            platform: "(expectation)".to_owned(),
            state: State::Error,
            detail,
        }],
        subject: None,
    };
    let Some(first) = expected.first() else {
        return broken("no platform is required, which is a misconfigured gate".to_owned());
    };
    let mut seen = BTreeSet::new();
    for request in expected {
        if let Some(problem) = request.problem() {
            return broken(format!(
                "expected request for {} is malformed: {problem}",
                request.platform
            ));
        }
        if !seen.insert(request.platform.as_str()) {
            return broken(format!("platform {} is expected twice", request.platform));
        }
        if request.repo != first.repo
            || request.head != first.head
            || request.base_sha != first.base_sha
        {
            return broken(
                "the expected requests disagree about the repository, head or base".to_owned(),
            );
        }
    }
    let mut platforms = Vec::new();
    let mut ordered: Vec<&Request> = expected.iter().collect();
    ordered.sort_by(|a, b| a.platform.cmp(&b.platform));
    for request in ordered {
        let id = request.id();
        let platform = request.platform.clone();
        let pending = |detail: String| PlatformStatus {
            platform: platform.clone(),
            state: State::Pending,
            detail,
        };
        let Some(admission) = admitted.get(&id) else {
            platforms.push(pending("the hall has not admitted this request".to_owned()));
            continue;
        };
        let bound: Vec<&Outcome> = outcomes
            .iter()
            .filter(|o| {
                o.request_id == id
                    && o.head == request.head
                    && o.platform == request.platform
                    && o.suite_digest == request.suite_digest
            })
            .collect();
        let current: Vec<&Outcome> = bound
            .iter()
            .copied()
            .filter(|o| o.attempt == admission.attempt && o.node == admission.node)
            .collect();
        let stale = bound.len() - current.len();
        let unverified = current.iter().filter(|o| !o.signature_verified).count();
        let counted: Vec<&Outcome> = current
            .iter()
            .copied()
            .filter(|o| {
                o.signature_verified
                    && authorised
                        .get(&o.node)
                        .is_some_and(|set| set.contains(&request.platform))
            })
            .collect();
        let unauthorised = current.len() - unverified - counted.len();
        if counted.is_empty() {
            let mut notes = Vec::new();
            if stale > 0 {
                notes.push(format!(
                    "{stale} result(s) from an earlier attempt or an unassigned node ignored"
                ));
            }
            if unverified > 0 {
                notes.push(format!("{unverified} unverified result(s) ignored"));
            }
            if unauthorised > 0 {
                notes.push(format!(
                    "{unauthorised} result(s) from a node not authorised for this platform ignored"
                ));
            }
            let detail = if notes.is_empty() {
                format!(
                    "attempt {} on {} has not reported",
                    admission.attempt, admission.node
                )
            } else {
                format!(
                    "attempt {} on {}: {}",
                    admission.attempt,
                    admission.node,
                    notes.join("; ")
                )
            };
            platforms.push(pending(detail));
            continue;
        }
        let verdicts: BTreeSet<&str> = counted
            .iter()
            .map(|o| match o.verdict {
                Verdict::Pass => "pass",
                Verdict::Fail => "fail",
                Verdict::Error => "error",
            })
            .collect();
        let at = format!("attempt {} on {}", admission.attempt, admission.node);
        let (state, detail) = if verdicts.len() > 1 {
            (
                State::Conflict,
                format!("{at}: the node reported different verdicts"),
            )
        } else {
            match counted[0].verdict {
                Verdict::Pass => (State::Pass, format!("{at} passed")),
                Verdict::Fail => (State::Fail, format!("{at} failed")),
                Verdict::Error => (State::Error, format!("{at} could not run")),
            }
        };
        platforms.push(PlatformStatus {
            platform,
            state,
            detail,
        });
    }
    let ok = platforms.iter().all(|p| p.state == State::Pass);
    let mut requests: Vec<(String, String, String, String)> = expected
        .iter()
        .map(|r| {
            (
                r.platform.clone(),
                r.suite.clone(),
                r.suite_digest.clone(),
                r.id(),
            )
        })
        .collect();
    requests.sort();
    let subject = Some(Subject {
        repo: first.repo.clone(),
        head: first.head.clone(),
        base_sha: first.base_sha.clone(),
        requests,
    });
    Evidence {
        ok,
        platforms,
        subject,
    }
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

    fn req(platforms: &[&str]) -> Vec<String> {
        platforms.iter().map(|s| (*s).to_owned()).collect()
    }

    fn request_on(platform: &str) -> Request {
        Request {
            platform: platform.into(),
            ..request()
        }
    }

    /// The expected requests for a candidate verified on Linux and Windows.
    fn both() -> Vec<Request> {
        vec![request_on("linux-x86_64"), request_on("windows-x86_64")]
    }

    fn linux() -> Request {
        request_on("linux-x86_64")
    }

    fn windows() -> Request {
        request_on("windows-x86_64")
    }

    /// A signed, verified pass for `request`, from `node`, for `attempt`, with its canonical id.
    fn outcome_for(request: &Request, node: &str, attempt: u32, verdict: Verdict) -> Outcome {
        Outcome {
            request_id: request.id(),
            head: request.head.clone(),
            platform: request.platform.clone(),
            suite_digest: request.suite_digest.clone(),
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
            ("ubuntu-2".to_owned(), set(&["linux-x86_64"])),
            ("windows-1".to_owned(), set(&["windows-x86_64"])),
        ])
    }

    /// The hall admitted attempt `attempt` of each request on the given node.
    fn admitted(entries: &[(&Request, u32, &str)]) -> BTreeMap<String, Admission> {
        entries
            .iter()
            .map(|(request, attempt, node)| {
                (
                    request.id(),
                    Admission {
                        attempt: *attempt,
                        node: (*node).to_owned(),
                    },
                )
            })
            .collect()
    }

    /// Both platforms admitted on their usual nodes at attempt 1.
    fn admitted_both() -> BTreeMap<String, Admission> {
        admitted(&[(&linux(), 1, "ubuntu-1"), (&windows(), 1, "windows-1")])
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
    fn a_head_is_verified_only_when_every_expected_platform_passed() {
        let linux_only = [outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass)];
        let result = evidence(&both(), &linux_only, &auth(), &admitted_both());
        assert!(!result.ok);
        assert_eq!(result.platforms[1].platform, "windows-x86_64");
        assert_eq!(result.platforms[1].state, State::Pending);
        let both_pass = [
            linux_only[0].clone(),
            outcome_for(&windows(), "windows-1", 1, Verdict::Pass),
        ];
        assert!(evidence(&both(), &both_pass, &auth(), &admitted_both()).ok);
    }

    #[test]
    fn evidence_names_the_candidate_it_is_about_and_only_the_constructor_sets_it() {
        let both_pass = [
            outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass),
            outcome_for(&windows(), "windows-1", 1, Verdict::Pass),
        ];
        let result = evidence(&both(), &both_pass, &auth(), &admitted_both());
        let subject = result
            .subject
            .clone()
            .expect("a well-formed expectation has a subject");
        assert_eq!(subject.repo, "Aarogaming/aaroneous-devtools");
        assert_eq!(subject.head, "a".repeat(40));
        assert_eq!(subject.base_sha, "b".repeat(40));
        assert_eq!(subject.requests.len(), 2);
        assert_eq!(subject.requests[0].0, "linux-x86_64");
        assert_eq!(subject.requests[0].1, "foundation-ci");
        assert_eq!(subject.requests[0].2, linux().suite_digest);
        assert_eq!(subject.requests[0].3, linux().id());
        // another head or base or suite digest or repository is a different subject
        for other in [
            Request {
                head: "d".repeat(40),
                ..request()
            },
            Request {
                base_sha: "d".repeat(40),
                ..request()
            },
            Request {
                suite_digest: "e".repeat(64),
                ..request()
            },
            Request {
                repo: "Aarogaming/aaroneous".into(),
                ..request()
            },
        ] {
            let pair = [
                Request {
                    platform: "linux-x86_64".into(),
                    ..other.clone()
                },
                Request {
                    platform: "windows-x86_64".into(),
                    ..other
                },
            ];
            let adm: BTreeMap<String, Admission> = pair
                .iter()
                .zip(["ubuntu-1", "windows-1"])
                .map(|(r, node)| {
                    (
                        r.id(),
                        Admission {
                            attempt: 1,
                            node: node.to_owned(),
                        },
                    )
                })
                .collect();
            let passes: Vec<Outcome> = pair
                .iter()
                .zip(["ubuntu-1", "windows-1"])
                .map(|(r, n)| outcome_for(r, n, 1, Verdict::Pass))
                .collect();
            let other_result = evidence(&pair, &passes, &auth(), &adm);
            assert!(other_result.ok);
            assert_ne!(other_result.subject, result.subject);
        }
        // the order of the expectation does not change the subject
        let mut swapped = both();
        swapped.reverse();
        assert_eq!(
            evidence(&swapped, &both_pass, &auth(), &admitted_both()).subject,
            result.subject
        );
        // a misconfigured expectation identifies nothing
        assert_eq!(
            evidence(&[], &both_pass, &auth(), &admitted_both()).subject,
            None
        );
        assert_eq!(
            evidence(&[linux(), linux()], &both_pass, &auth(), &admitted_both()).subject,
            None
        );
    }

    #[test]
    fn a_misconfigured_expectation_is_never_ok() {
        let pass = [outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass)];
        assert!(
            !evidence(&[], &pass, &auth(), &admitted_both()).ok,
            "nothing expected"
        );
        assert!(
            !evidence(&[linux(), linux()], &pass, &auth(), &admitted_both()).ok,
            "a platform expected twice"
        );
        assert!(
            !evidence(
                &[Request {
                    head: "x".into(),
                    ..linux()
                }],
                &pass,
                &auth(),
                &admitted_both()
            )
            .ok,
            "malformed request"
        );
        // The requests disagree about the candidate. Every one is admitted and every result is a genuine pass, so only the
        // disagreement itself can stop this from being ok.
        for (why, other) in [
            (
                "repository",
                Request {
                    repo: "Aarogaming/aaroneous".into(),
                    ..windows()
                },
            ),
            (
                "base",
                Request {
                    base_sha: "d".repeat(40),
                    ..windows()
                },
            ),
            (
                "head",
                Request {
                    head: "d".repeat(40),
                    ..windows()
                },
            ),
        ] {
            let pair = [linux(), other.clone()];
            let adm = admitted(&[(&linux(), 1, "ubuntu-1"), (&other, 1, "windows-1")]);
            let passes = [
                outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass),
                outcome_for(&other, "windows-1", 1, Verdict::Pass),
            ];
            assert!(
                !evidence(&pair, &passes, &auth(), &adm).ok,
                "requests that disagree about the {why}"
            );
        }
        // and the same setup with agreeing requests is ok, so the check above is what stopped it
        let adm = admitted_both();
        let passes = [
            outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass),
            outcome_for(&windows(), "windows-1", 1, Verdict::Pass),
        ];
        assert!(evidence(&both(), &passes, &auth(), &adm).ok);
    }

    #[test]
    fn a_result_for_another_request_does_not_count_however_well_its_head_and_platform_fit() {
        // Codex review of #197: a verified, authorised pass carrying another valid request's id made evidence ok.
        let intended = linux();
        let admitted = admitted(&[(&intended, 1, "ubuntu-1")]);
        let others = [
            (
                "another repository",
                Request {
                    repo: "Aarogaming/aaroneous".into(),
                    ..linux()
                },
            ),
            (
                "another base",
                Request {
                    base_sha: "d".repeat(40),
                    ..linux()
                },
            ),
            (
                "another suite",
                Request {
                    suite: "dashboard".into(),
                    ..linux()
                },
            ),
            (
                "another suite digest",
                Request {
                    suite_digest: "e".repeat(64),
                    ..linux()
                },
            ),
        ];
        for (why, other) in others {
            // the outcome keeps the intended head, platform and digest but carries the other request's id
            let borrowed = Outcome {
                request_id: other.id(),
                ..outcome_for(&intended, "ubuntu-1", 1, Verdict::Pass)
            };
            let result = evidence(
                std::slice::from_ref(&intended),
                &[borrowed],
                &auth(),
                &admitted,
            );
            assert!(!result.ok, "{why} must not count");
            assert_eq!(result.platforms[0].state, State::Pending, "{why}");
        }
        for bad_id in ["id", "not-an-id", "", &"0".repeat(64)] {
            let forged = Outcome {
                request_id: bad_id.to_owned(),
                ..outcome_for(&intended, "ubuntu-1", 1, Verdict::Pass)
            };
            assert!(
                !evidence(
                    std::slice::from_ref(&intended),
                    &[forged],
                    &auth(),
                    &admitted
                )
                .ok,
                "id {bad_id:?}"
            );
        }
        let genuine = outcome_for(&intended, "ubuntu-1", 1, Verdict::Pass);
        assert!(
            evidence(
                std::slice::from_ref(&intended),
                &[genuine],
                &auth(),
                &admitted
            )
            .ok
        );
    }

    #[test]
    fn a_result_that_cannot_count_is_ignored_and_says_why() {
        let one = [linux()];
        let adm = admitted(&[(&linux(), 1, "ubuntu-1")]);
        let good = outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass);
        let cases: Vec<(&str, Outcome)> = vec![
            (
                "another head",
                Outcome {
                    head: "d".repeat(40),
                    ..good.clone()
                },
            ),
            (
                "another suite digest",
                Outcome {
                    suite_digest: "d".repeat(64),
                    ..good.clone()
                },
            ),
            (
                "another platform",
                Outcome {
                    platform: "windows-x86_64".into(),
                    ..good.clone()
                },
            ),
            (
                "unverified signature",
                Outcome {
                    signature_verified: false,
                    ..good.clone()
                },
            ),
            (
                "a node not assigned the request",
                Outcome {
                    node: "ubuntu-2".into(),
                    ..good.clone()
                },
            ),
            (
                "an earlier attempt",
                Outcome {
                    attempt: 0,
                    ..good.clone()
                },
            ),
            (
                "a later attempt the hall never admitted",
                Outcome {
                    attempt: 2,
                    ..good.clone()
                },
            ),
        ];
        for (why, case) in cases {
            let result = evidence(&one, &[case], &auth(), &adm);
            assert!(!result.ok, "{why} must not pass");
            assert_eq!(result.platforms[0].state, State::Pending, "{why}");
        }
        // a node that is assigned but not registered for the platform
        let wrong_platform = admitted(&[(&linux(), 1, "windows-1")]);
        let result = evidence(
            &one,
            &[Outcome {
                node: "windows-1".into(),
                ..good.clone()
            }],
            &auth(),
            &wrong_platform,
        );
        assert_eq!(result.platforms[0].state, State::Pending);
        assert!(
            result.platforms[0].detail.contains("not authorised"),
            "{}",
            result.platforms[0].detail
        );
        let unknown = admitted(&[(&linux(), 1, "stranger")]);
        assert!(
            !evidence(
                &one,
                &[Outcome {
                    node: "stranger".into(),
                    ..good.clone()
                }],
                &auth(),
                &unknown
            )
            .ok
        );
        let detail = evidence(
            &one,
            &[Outcome {
                signature_verified: false,
                ..good
            }],
            &auth(),
            &adm,
        )
        .platforms[0]
            .detail
            .clone();
        assert!(detail.contains("unverified"), "{detail}");
    }

    #[test]
    fn a_request_the_hall_has_not_admitted_is_pending_whatever_arrives() {
        let pass = [outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass)];
        let result = evidence(&[linux()], &pass, &auth(), &BTreeMap::new());
        assert!(!result.ok);
        assert_eq!(result.platforms[0].state, State::Pending);
        assert!(result.platforms[0].detail.contains("not admitted"));
    }

    #[test]
    fn a_late_pass_from_an_earlier_attempt_cannot_turn_a_newer_pending_attempt_green() {
        // Codex review of #197: attempt 1's pass arrives after the hall reassigned the work as attempt 2.
        let late = outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass);
        let reassigned = admitted(&[(&linux(), 2, "ubuntu-2")]);
        let result = evidence(
            &[linux()],
            std::slice::from_ref(&late),
            &auth(),
            &reassigned,
        );
        assert!(!result.ok);
        assert_eq!(result.platforms[0].state, State::Pending);
        assert!(
            result.platforms[0].detail.contains("earlier attempt"),
            "{}",
            result.platforms[0].detail
        );
        // duplicate delivery of the same late packet changes nothing
        let twice = [late.clone(), late];
        assert!(!evidence(&[linux()], &twice, &auth(), &reassigned).ok);
        // once the admitted attempt reports, that result decides
        let fresh = outcome_for(&linux(), "ubuntu-2", 2, Verdict::Pass);
        let both_arrive = [twice[0].clone(), fresh.clone()];
        assert!(evidence(&[linux()], &both_arrive, &auth(), &reassigned).ok);
        // and a failing current attempt is not rescued by an earlier pass
        let failing = outcome_for(&linux(), "ubuntu-2", 2, Verdict::Fail);
        let result = evidence(
            &[linux()],
            &[twice[0].clone(), failing],
            &auth(),
            &reassigned,
        );
        assert_eq!(result.platforms[0].state, State::Fail);
    }

    #[test]
    fn the_assigned_node_reporting_two_verdicts_for_one_attempt_is_a_conflict() {
        let adm = admitted(&[(&linux(), 1, "ubuntu-1")]);
        let both_verdicts = [
            outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass),
            outcome_for(&linux(), "ubuntu-1", 1, Verdict::Fail),
        ];
        let result = evidence(&[linux()], &both_verdicts, &auth(), &adm);
        assert!(!result.ok);
        assert_eq!(result.platforms[0].state, State::Conflict);
        // the same verdict delivered twice is harmless
        let twice = [
            outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass),
            outcome_for(&linux(), "ubuntu-1", 1, Verdict::Pass),
        ];
        assert!(evidence(&[linux()], &twice, &auth(), &adm).ok);
    }

    #[test]
    fn a_node_error_is_never_a_pass() {
        let adm = admitted(&[(&linux(), 1, "ubuntu-1")]);
        let result = evidence(
            &[linux()],
            &[outcome_for(&linux(), "ubuntu-1", 1, Verdict::Error)],
            &auth(),
            &adm,
        );
        assert!(!result.ok);
        assert_eq!(result.platforms[0].state, State::Error);
    }

    #[test]
    fn evidence_does_not_depend_on_the_order_of_results() {
        let results = vec![
            outcome_for(&linux(), "ubuntu-1", 1, Verdict::Fail),
            outcome_for(&linux(), "ubuntu-1", 2, Verdict::Pass),
            outcome_for(&windows(), "windows-1", 1, Verdict::Pass),
            outcome_for(&windows(), "windows-1", 1, Verdict::Pass),
        ];
        let adm = admitted(&[(&linux(), 2, "ubuntu-1"), (&windows(), 1, "windows-1")]);
        let expected = evidence(&both(), &results, &auth(), &adm);
        assert!(expected.ok);
        let mut shuffled = results.clone();
        for rotation in 0..shuffled.len() {
            shuffled.rotate_left(rotation);
            assert_eq!(evidence(&both(), &shuffled, &auth(), &adm), expected);
        }
        let mut reversed = results;
        reversed.reverse();
        assert_eq!(evidence(&both(), &reversed, &auth(), &adm), expected);
        let mut swapped = both();
        swapped.reverse();
        assert_eq!(
            evidence(&swapped, &shuffled, &auth(), &adm),
            expected,
            "the order of the expectation does not matter either"
        );
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
