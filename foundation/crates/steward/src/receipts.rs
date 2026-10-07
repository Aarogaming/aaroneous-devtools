//! Which cards a comment completes. Port of `collect_state.done_receipts` and `eligible_completer`
//! (vector family `receipts`).
//!
//! A completion receipt is an explicit, positive statement bound to the exact full head: `[done:KIND:PR:<40-hex head>]`
//! as the first thing in a comment. One quoted further down, one for another PR or head, one whose first line says the
//! work was not done, or one in a comment that also names a different full head sharing the first eight characters
//! completes nothing.

use crate::attribution::Attribution;
use crate::rejected::Comment;
use crate::text::{Lexicon, splitlines, strip};
use regex::Regex;
use std::collections::BTreeMap;

/// Kinds whose completion is visible on GitHub itself; a receipt for one would close the card while the item is open.
const OBSERVABLE_KINDS: [&str; 4] = [
    "resolve_threads",
    "address_review",
    "identify_implementers",
    "not_mergeable",
];

/// A pull request as the steward's snapshot holds it.
#[derive(Debug, Clone, Default)]
pub struct Pr {
    pub number: i64,
    pub head: String,
    pub comments: Vec<Comment>,
    pub implementers: Vec<String>,
}

pub struct Receipts {
    lexicon: Lexicon,
    attribution: Attribution,
    start: Regex,
    negative: Regex,
    hex40: Regex,
}

/// Whether this seat may complete a card of this kind. It fails closed: an unattributed comment never completes a
/// card; a review needs KNOWN implementers and a seat that is not one of them; only the author seat completes its own
/// thread fixes; only a seat whose verdict was rejected completes that repair.
#[must_use]
pub fn eligible_completer(
    kind: &str,
    seat: &str,
    implementers: &[String],
    repair_owners: &[String],
) -> bool {
    if seat.is_empty() {
        return false;
    }
    let is_implementer = implementers.iter().any(|i| i == seat);
    let is_author = implementers.first().is_some_and(|first| first == seat);
    match kind {
        "needs_review" | "resolve_threads" => !implementers.is_empty() && !is_implementer,
        "fix_threads" | "address_review" => !implementers.is_empty() && is_author,
        "repair_verdict" => repair_owners.iter().any(|owner| owner == seat),
        _ => true,
    }
}

impl Receipts {
    /// # Errors
    /// Returns the regex error if a fixed pattern fails to compile (a build defect).
    pub fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            lexicon: Lexicon::new()?,
            attribution: Attribution::new()?,
            start: Regex::new(r"(?i)\A[\s\x1c-\x1f]*\[done:([a-z_]+):([0-9]+):([0-9a-f]{40})\]")?,
            negative: Regex::new(
                r"(?i)\b(not done|not complete[d]?|blocked|could not|cannot|unable|failed)\b",
            )?,
            hex40: Regex::new(r"\b[0-9a-f]{40}\b")?,
        })
    }

    /// `{card key: the seat whose receipt completed it}`. The first eligible receipt for a key names the seat.
    /// `implementers_by_pr` wins over the PR's own list when it holds a non-empty entry.
    #[must_use]
    pub fn done_receipts(
        &self,
        prs: &[Pr],
        repair_owners: &BTreeMap<i64, Vec<String>>,
        implementers_by_pr: &BTreeMap<i64, Vec<String>>,
    ) -> BTreeMap<String, String> {
        let mut keys = BTreeMap::new();
        for pr in prs {
            let head = pr.head.to_lowercase();
            let implementers = match implementers_by_pr.get(&pr.number) {
                Some(list) if !list.is_empty() => list.as_slice(),
                _ => pr.implementers.as_slice(),
            };
            let no_owners = Vec::new();
            let owners = repair_owners.get(&pr.number).unwrap_or(&no_owners);
            for comment in &pr.comments {
                let Some(captures) = self.start.captures(&comment.body) else {
                    continue;
                };
                let kind = captures[1].to_lowercase();
                let receipt_head = captures[3].to_lowercase();
                if OBSERVABLE_KINDS.contains(&kind.as_str()) {
                    continue;
                }
                let number: Option<i64> = captures[2].parse().ok();
                if number != Some(pr.number) || receipt_head != head {
                    continue;
                }
                let first = splitlines(&comment.body)
                    .into_iter()
                    .find(|line| !strip(line).is_empty())
                    .unwrap_or("");
                if self.negative.is_match(&self.lexicon.pyfy(first)) {
                    continue;
                }
                let lowered = self.lexicon.pyfy(&comment.body.to_lowercase());
                let conflicting = self.hex40.find_iter(&lowered).any(|m| {
                    let token = m.as_str();
                    token != head && token.chars().take(8).eq(head.chars().take(8))
                });
                if conflicting {
                    continue;
                }
                let seat = self.attribution.comment_seat(&comment.body, true);
                if !eligible_completer(&kind, &seat, implementers, owners) {
                    continue;
                }
                let prefix: String = head.chars().take(8).collect();
                keys.entry(format!("{kind}:{}:{prefix}", pr.number))
                    .or_insert(seat);
            }
        }
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::{Pr, Receipts, eligible_completer};
    use crate::rejected::Comment;
    use std::collections::BTreeMap;

    const HEAD: &str = "cccccccccccccccccccccccccccccccccccccccc";

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn run(body: &str) -> BTreeMap<String, String> {
        let Ok(receipts) = Receipts::new() else {
            panic!("patterns")
        };
        let pr = Pr {
            number: 7,
            head: HEAD.to_owned(),
            comments: vec![Comment {
                id: Some(1),
                body: body.to_owned(),
            }],
            implementers: Vec::new(),
        };
        let by_pr = BTreeMap::from([(7, strings(&["claude"]))]);
        receipts.done_receipts(&[pr], &BTreeMap::new(), &by_pr)
    }

    #[test]
    fn the_author_completes_its_thread_fix() {
        let done = run(&format!("[done:fix_threads:7:{HEAD}] Claude seat: ok"));
        assert_eq!(
            done.get("fix_threads:7:cccccccc").map(String::as_str),
            Some("claude")
        );
    }

    #[test]
    fn a_non_ascii_pr_number_never_matches() {
        assert!(
            run(&format!(
                "[done:fix_threads:\u{667}:{HEAD}] Claude seat: ok"
            ))
            .is_empty()
        );
    }

    #[test]
    fn a_receipt_must_open_the_comment() {
        assert!(run(&format!("> [done:fix_threads:7:{HEAD}] Claude seat: ok")).is_empty());
        let opened = format!("\n  [done:fix_threads:7:{HEAD}] Claude seat: ok");
        assert_eq!(run(&opened).len(), 1, "leading whitespace is allowed");
    }

    #[test]
    fn completers_fail_closed() {
        let claude = strings(&["claude"]);
        assert!(!eligible_completer("needs_review", "claude", &claude, &[]));
        assert!(!eligible_completer("needs_review", "gemini", &[], &[]));
        assert!(eligible_completer("needs_review", "gemini", &claude, &[]));
        assert!(!eligible_completer("fix_threads", "codex", &claude, &[]));
        assert!(!eligible_completer("anything", "", &claude, &[]));
        assert!(eligible_completer(
            "repair_verdict",
            "gemini",
            &claude,
            &strings(&["gemini"])
        ));
        assert!(eligible_completer("unlisted_kind", "gemini", &[], &[]));
    }
}
