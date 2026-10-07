//! Is a pull request head covered by an independent whole-diff accept? Port of `review_coverage.independent`,
//! `canonical_reviewer_seat` and `evaluate` (vector family `coverage`), as they are with PRs #111 and #114 applied.

use crate::seats::seat_names;
use std::collections::BTreeSet;

const ACCEPT: [&str; 1] = ["accept"];
const REJECT: [&str; 2] = ["changes_requested", "needs_revision"];

/// A validated attestation record, reduced to the fields the decision reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    pub head: String,
    pub base_sha: String,
    /// `"whole"` or `"delta"`.
    pub scope: String,
    pub reviewer: String,
    pub implementers: Vec<String>,
    pub verdict: String,
    /// A delta's starting head; `None` for a whole record.
    pub from: Option<String>,
}

/// The coverage decision. `counted` holds indexes into the records passed in, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decision {
    pub covered: bool,
    pub reasons: Vec<String>,
    pub accepts_at_head: usize,
    pub counted: Vec<usize>,
    pub accept_seats: Vec<String>,
}

/// The single seat a reviewer string resolves to: `None` when it names several seats ("Opus/gpt-5.5") or none, so an
/// ambiguous reviewer can neither be counted nor contribute two seats.
#[must_use]
pub fn canonical_reviewer_seat(name: &str) -> Option<String> {
    let seats = seat_names(name);
    if seats.len() == 1 {
        seats.into_iter().next()
    } else {
        None
    }
}

/// Python's `repr` of a list of strings, for the one reason message that prints the implementers.
fn py_repr_list(items: &[String]) -> String {
    let quoted: Vec<String> = items
        .iter()
        .map(|item| {
            let quote = if item.contains('\'') && !item.contains('"') {
                '"'
            } else {
                '\''
            };
            let mut out = String::new();
            out.push(quote);
            for c in item.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if c == quote => {
                        out.push('\\');
                        out.push(c);
                    }
                    c if c.is_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push(quote);
            out
        })
        .collect();
    format!("[{}]", quoted.join(", "))
}

fn short(sha: &str) -> String {
    sha.chars().take(12).collect()
}

/// The reviewer is not among the implementers, comparing seats: reviewer `gemini` with implementers `["Gemini"]`, or
/// `Gemini/Antigravity` with `["gemini"]`, is the same seat and never independent.
#[must_use]
pub fn independent(record: &Record) -> bool {
    let implementers: BTreeSet<String> = record
        .implementers
        .iter()
        .flat_map(|i| seat_names(i))
        .collect();
    seat_names(&record.reviewer).is_disjoint(&implementers)
}

/// Pure coverage decision for one PR head. `is_ancestor(a, b)` says whether commit `a` is an ancestor of commit `b`. A
/// delta accept extends coverage only when its `from` is a covered head AND an ancestor of the delta's own head, so a
/// rebased or flattened head cannot inherit coverage by naming an old SHA. A merge base that is `None`, empty or
/// `unknown` means coverage cannot be judged.
#[must_use]
pub fn evaluate(
    head: &str,
    merge_base: Option<&str>,
    records: &[Record],
    is_ancestor: &dyn Fn(&str, &str) -> bool,
) -> Decision {
    let merge_base = match merge_base {
        None | Some("" | "unknown") => {
            return Decision {
                reasons: vec!["merge-base unresolved: coverage cannot be judged".to_owned()],
                ..Decision::default()
            };
        }
        Some(base) => base,
    };
    let at_head: Vec<&Record> = records.iter().filter(|r| r.head == head).collect();
    let blockers: Vec<&Record> = at_head
        .iter()
        .copied()
        .filter(|r| REJECT.contains(&r.verdict.as_str()) && r.base_sha == merge_base)
        .collect();
    let mut reasons = Vec::new();
    if let Some(first) = blockers.first() {
        reasons.push(format!(
            "standing {} at head by {}",
            first.verdict, first.reviewer
        ));
    }
    // A record is valid only while the merge base equals its base_sha, so a rejection naming another base does not
    // block; it is reported, because a hand-typed base_sha is the likelier cause.
    for r in &at_head {
        if REJECT.contains(&r.verdict.as_str()) && r.base_sha != merge_base {
            reasons.push(format!(
                "a rejection by {} names base_sha {}, but the merge base is {}, so it is ignored as stale; if the \
                 base_sha was mistyped, that reviewer must re-attest against the current merge base",
                r.reviewer,
                short(&r.base_sha),
                short(merge_base)
            ));
        }
    }
    let valid: Vec<usize> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            ACCEPT.contains(&r.verdict.as_str())
                && independent(r)
                && r.base_sha == merge_base
                && canonical_reviewer_seat(&r.reviewer).is_some()
        })
        .map(|(i, _)| i)
        .collect();
    let mut covered: BTreeSet<&str> = valid
        .iter()
        .map(|&i| &records[i])
        .filter(|r| r.scope == "whole")
        .map(|r| r.head.as_str())
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for &i in &valid {
            let r = &records[i];
            let Some(from) = r.from.as_deref() else {
                continue;
            };
            if r.scope == "delta"
                && covered.contains(from)
                && !covered.contains(r.head.as_str())
                && is_ancestor(from, &r.head)
            {
                covered.insert(r.head.as_str());
                changed = true;
            }
        }
    }
    let ok = covered.contains(head) && blockers.is_empty();
    if !covered.contains(head) {
        if records
            .iter()
            .any(|r| ACCEPT.contains(&r.verdict.as_str()) && r.head == head)
        {
            reasons.push(
                "accept at head exists but is not chained to a whole accept at this base"
                    .to_owned(),
            );
            for r in &at_head {
                if !ACCEPT.contains(&r.verdict.as_str()) || valid.iter().any(|&i| records[i] == **r)
                {
                    continue;
                }
                if r.base_sha != merge_base {
                    reasons.push(format!(
                        "the accept by {} names base_sha {}, but the merge base is {}: re-attest against the \
                         current merge base",
                        r.reviewer,
                        short(&r.base_sha),
                        short(merge_base)
                    ));
                } else if !independent(r) {
                    let mut implementers = r.implementers.clone();
                    implementers.sort();
                    reasons.push(format!(
                        "the accept by {} is not independent of its implementers {}",
                        r.reviewer,
                        py_repr_list(&implementers)
                    ));
                }
            }
        } else {
            reasons.push("no accept at the current head".to_owned());
        }
    }
    let counted: Vec<usize> = valid
        .iter()
        .copied()
        .filter(|&i| records[i].head == head)
        .collect();
    let seats: BTreeSet<String> = counted
        .iter()
        .filter_map(|&i| canonical_reviewer_seat(&records[i].reviewer))
        .collect();
    Decision {
        covered: ok,
        reasons,
        accepts_at_head: at_head.len(),
        counted,
        accept_seats: seats.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Record, evaluate, independent};

    const MB: &str = "mmmm";

    fn whole(head: &str, reviewer: &str, implementers: &[&str]) -> Record {
        Record {
            head: head.into(),
            base_sha: MB.into(),
            scope: "whole".into(),
            reviewer: reviewer.into(),
            implementers: implementers.iter().map(|s| (*s).to_owned()).collect(),
            verdict: "accept".into(),
            from: None,
        }
    }

    fn never(_: &str, _: &str) -> bool {
        false
    }

    #[test]
    fn a_whole_accept_from_another_seat_covers() {
        let d = evaluate("a", Some(MB), &[whole("a", "mimo", &["claude"])], &never);
        assert!(d.covered);
        assert_eq!(d.accept_seats, ["mimo"]);
        assert_eq!(d.counted, [0]);
    }

    #[test]
    fn an_implementer_in_any_spelling_cannot_accept_its_own_work() {
        assert!(!independent(&whole("a", "Gemini/Antigravity", &["gemini"])));
        assert!(!independent(&whole("a", "Claude", &["claude-sonnet-5-5"])));
        assert!(independent(&whole("a", "Codex", &["Claude, Mimo"])));
        let d = evaluate("a", Some(MB), &[whole("a", "claude", &["claude"])], &never);
        assert!(!d.covered);
        assert_eq!(
            d.reasons,
            [
                "accept at head exists but is not chained to a whole accept at this base",
                "the accept by claude is not independent of its implementers ['claude']"
            ]
        );
    }

    #[test]
    fn a_delta_chains_only_from_a_covered_ancestor() {
        let delta = Record {
            scope: "delta".into(),
            from: Some("a".into()),
            ..whole("b", "mimo", &["claude"])
        };
        let records = [whole("a", "mimo", &["claude"]), delta];
        assert!(evaluate("b", Some(MB), &records, &|x, y| x == "a" && y == "b").covered);
        assert!(!evaluate("b", Some(MB), &records, &never).covered);
    }

    #[test]
    fn a_standing_rejection_on_the_merge_base_blocks() {
        let reject = Record {
            verdict: "needs_revision".into(),
            reviewer: "gemini".into(),
            ..whole("a", "x", &[])
        };
        let d = evaluate(
            "a",
            Some(MB),
            &[whole("a", "mimo", &["claude"]), reject],
            &never,
        );
        assert!(!d.covered);
        assert_eq!(d.reasons, ["standing needs_revision at head by gemini"]);
    }

    #[test]
    fn an_unresolved_merge_base_cannot_be_judged() {
        for base in [None, Some(""), Some("unknown")] {
            let d = evaluate("a", base, &[whole("a", "mimo", &["claude"])], &never);
            assert!(!d.covered);
            assert_eq!(d.accepts_at_head, 0);
        }
    }
}
