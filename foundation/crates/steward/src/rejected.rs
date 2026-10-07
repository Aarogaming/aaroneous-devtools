//! Verdict comments the coverage tool did not count at a PR's current head, with the seat each one is from.
//! Port of `collect_state.rejected_verdicts`, `explain_problem` and `phantom_head` (vector family `rejected`).

use crate::attribution::Attribution;
use crate::heads::cites_head;
use regex::Regex;

pub const LEGACY_REASON: &str =
    "the comment reads like a verdict but has no fenced attestation block, so nothing was counted";
/// Whether a commit is present in the steward's clone: `Some(true)`, `Some(false)`, or `None` when the lookup failed.
pub type CommitExists<'a> = &'a dyn Fn(&str) -> Option<bool>;

const MAX_REJECTED_PER_SEAT: usize = 2;
const NEAR_MISS_PREFIX: usize = 7;

/// A pull-request comment. A comment that is not an object in the snapshot is represented as `body: ""`, `id: None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Comment {
    pub id: Option<i64>,
    pub body: String,
}

/// An attestation block the coverage tool refused: the comment index it was in and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
    pub comment: Option<i64>,
    pub problem: String,
}

/// One entry of the repair list a card shows its author.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    pub comment_id: Option<i64>,
    pub seat: String,
    pub reason: String,
}

/// The coverage tool's rejection reason in words an author can act on.
#[must_use]
pub fn explain_problem(raw: &str) -> String {
    if raw.starts_with("invalid JSON") || raw.starts_with("not a JSON object") {
        "the attestation block is not valid JSON; it must be one JSON object, not YAML or prose"
            .to_owned()
    } else if raw.starts_with("missing fields") {
        format!("the attestation block is {raw}")
    } else if raw.starts_with("implementers must be") {
        "implementers must be a non-empty list such as [\"Claude\"]".to_owned()
    } else if raw.starts_with("unknown scope") {
        "scope must be \"whole\" or \"delta\"".to_owned()
    } else if raw.contains("does not match") {
        "the block names a different repository or PR than the one it was posted on".to_owned()
    } else if raw.starts_with("unsigned or unverifiable") {
        "signatures are required and this block is unsigned or does not verify against the registered key".to_owned()
    } else {
        raw.to_owned()
    }
}

/// The 40-character hex tokens of `text` (`\b[0-9a-f]{40}\b`, matched on lower-cased text).
fn hex40_tokens<'a>(re: &Regex, text: &'a str) -> Vec<&'a str> {
    re.find_iter(text).map(|m| m.as_str()).collect()
}

/// A full 40-character token in the comment that agrees with the head in its first seven characters, differs after
/// that, and is reported absent: a probably mistyped or invented head. `commit_exists` answers `Some(true)`,
/// `Some(false)` or `None` for a failed lookup; only `Some(false)` flags, so a git failure never turns into an
/// accusation. Without it nothing is flagged.
fn phantom_head(
    hex40: &Regex,
    body: &str,
    head: &str,
    commit_exists: Option<CommitExists<'_>>,
) -> Option<String> {
    let head = head.to_lowercase();
    let exists = commit_exists?;
    if head.is_empty() {
        return None;
    }
    let body = body.to_lowercase();
    for token in hex40_tokens(hex40, &body) {
        let agree = token
            .chars()
            .zip(head.chars())
            .take_while(|(a, b)| a == b)
            .count();
        if token != head && agree >= NEAR_MISS_PREFIX && exists(token) == Some(false) {
            return Some(token.to_owned());
        }
    }
    None
}

pub struct Rules {
    attribution: Attribution,
    hex40: Regex,
}

impl Rules {
    /// # Errors
    /// Returns the regex error if a fixed pattern fails to compile (a build defect).
    pub fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            attribution: Attribution::new()?,
            hex40: Regex::new(r"\b[0-9a-f]{40}\b")?,
        })
    }

    /// Verdict comments the coverage tool did not count at the PR's CURRENT head, each attributed to a seat where the
    /// text says which. A comment that does not cite the current head is history, not a pending repair, and at most
    /// the latest two per seat are kept so a card stays short. The exception is a comment citing a head that does not
    /// exist but almost matches the current one: that is a mistyped head, so its author is told.
    #[must_use]
    pub fn rejected_verdicts(
        &self,
        comments: &[Comment],
        head: &str,
        malformed: &[Malformed],
        legacy: &[Option<i64>],
        commit_exists: Option<CommitExists<'_>>,
    ) -> Vec<Rejected> {
        let mut found: Vec<(usize, Rejected)> = Vec::new();
        let mut entry = |index: Option<i64>, reason: String| {
            let Some(index) = index
                .and_then(|i| usize::try_from(i).ok())
                .filter(|i| *i < comments.len())
            else {
                return;
            };
            let comment = &comments[index];
            let mut reason = reason;
            if !cites_head(&comment.body, head) {
                let Some(phantom) = phantom_head(&self.hex40, &comment.body, head, commit_exists)
                else {
                    return;
                };
                reason = format!(
                    "it cites {phantom}, which was not found as a commit in the steward's clone (a suspected head \
                     mismatch, not proof); the PR's current head is {head} and only the first characters match, so it \
                     looks mistyped. Copy the head from the card. Also: {reason}"
                );
            }
            found.push((
                index,
                Rejected {
                    comment_id: comment.id,
                    seat: self.attribution.comment_seat(&comment.body, true),
                    reason,
                },
            ));
        };
        for problem in malformed {
            if problem.comment.is_some() {
                entry(problem.comment, explain_problem(&problem.problem));
            }
        }
        for packet in legacy {
            entry(*packet, LEGACY_REASON.to_owned());
        }
        found.sort_by_key(|(index, _)| *index);
        let mut kept: Vec<Rejected> = Vec::new();
        for (_, item) in found.into_iter().rev() {
            let count = kept.iter().filter(|k| k.seat == item.seat).count();
            if count < MAX_REJECTED_PER_SEAT {
                kept.push(item);
            }
        }
        kept.reverse();
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::{Comment, LEGACY_REASON, Malformed, Rules, explain_problem};

    const HEAD: &str = "cccccccccccccccccccccccccccccccccccccccc";

    fn comment(id: i64, body: &str) -> Comment {
        Comment {
            id: Some(id),
            body: body.to_owned(),
        }
    }

    #[test]
    fn a_phantom_head_is_told_to_its_author_only_when_git_says_the_commit_is_absent() {
        let Ok(rules) = Rules::new() else {
            panic!("patterns")
        };
        let mistyped = format!("{}dddddddddddddddddddddddddddddddd", &HEAD[..8]);
        let comments = [comment(
            7,
            &format!("Seat / agent: Codex\nhead {mistyped}\nVERDICT: accept"),
        )];
        let missing = |_: &str| Some(false);
        let failed = |_: &str| None;
        let present = |_: &str| Some(true);
        let told = rules.rejected_verdicts(&comments, HEAD, &[], &[Some(0)], Some(&missing));
        assert_eq!(told.len(), 1);
        assert!(
            told[0].reason.contains("looks mistyped"),
            "{}",
            told[0].reason
        );
        assert!(told[0].reason.ends_with(LEGACY_REASON));
        assert!(
            rules
                .rejected_verdicts(&comments, HEAD, &[], &[Some(0)], Some(&failed))
                .is_empty()
        );
        assert!(
            rules
                .rejected_verdicts(&comments, HEAD, &[], &[Some(0)], Some(&present))
                .is_empty()
        );
        assert!(
            rules
                .rejected_verdicts(&comments, HEAD, &[], &[Some(0)], None)
                .is_empty()
        );
    }

    #[test]
    fn an_index_outside_the_comments_is_ignored() {
        let Ok(rules) = Rules::new() else {
            panic!("patterns")
        };
        let comments = [comment(1, &format!("Seat / agent: Mimo\nhead {HEAD}"))];
        let bad = [
            Malformed {
                comment: Some(-1),
                problem: "x".into(),
            },
            Malformed {
                comment: Some(9),
                problem: "x".into(),
            },
        ];
        assert!(
            rules
                .rejected_verdicts(&comments, HEAD, &bad, &[], None)
                .is_empty()
        );
    }

    #[test]
    fn problems_read_in_plain_words() {
        assert!(explain_problem("invalid JSON: Expecting value").contains("not valid JSON"));
        assert_eq!(explain_problem("something new"), "something new");
    }
}
