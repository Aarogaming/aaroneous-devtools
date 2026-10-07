//! Which seat a comment says it is from. Port of `collect_state.comment_seat` (vector family `attribution`).

use crate::text::{Lexicon, splitlines, strip};
use regex::Regex;

/// The compiled patterns, built once by the caller and passed in (no hidden global state).
pub struct Attribution {
    lexicon: Lexicon,
    in_text: Regex,
    seat_word: Regex,
    declared: Regex,
}

// Python's `\s` also counts the ASCII separators U+001C to U+001F, which Rust's does not.
const DECLARED: &str = r"(?:Seat[\s\x1c-\x1f]*/[\s\x1c-\x1f]*agent|Reviewer(?:[\s\x1c-\x1f]*/[\s\x1c-\x1f]*Seat|[\s\x1c-\x1f]+seat)?)\W{0,6}(Claude|Mimo|Codex|Gemini|Antigravity)";

impl Attribution {
    /// # Errors
    /// Returns the regex error if a pattern fails to compile (they are fixed literals, so this is a build defect).
    pub fn new() -> Result<Self, regex::Error> {
        // A seat name that opens a line, optionally after a markdown heading mark and the word "Independent",
        // is how the seats title their reviews.
        let in_text = format!(
            r"(?im){DECLARED}|^[\s\x1c-\x1f]*(?:#{{1,6}}[\s\x1c-\x1f]*)?(?:Independent[\s\x1c-\x1f]+)?(Claude|Codex|Gemini|Mimo|Antigravity)\b"
        );
        Ok(Self {
            lexicon: Lexicon::new()?,
            in_text: Regex::new(&in_text)?,
            seat_word: Regex::new(
                r"(?i)\b(Claude|Mimo|Codex|Gemini|Antigravity)[\s\x1c-\x1f]+seat\b",
            )?,
            declared: Regex::new(&format!("(?i){DECLARED}"))?,
        })
    }

    /// The seat a comment says it is from (`""` when it does not say). Antigravity is the Gemini seat.
    ///
    /// `strict` accepts only an explicit declaration ("Seat / agent: X", "Reviewer: X") or a seat name that opens
    /// the comment, so a mention of a seat further down a comment never attributes it.
    #[must_use]
    pub fn comment_seat(&self, body: &str, strict: bool) -> String {
        let body = self.lexicon.pyfy(body);
        let body = body.as_str();
        let found = if strict {
            let first = splitlines(body)
                .into_iter()
                .find(|line| !strip(line).is_empty())
                .unwrap_or("");
            self.in_text
                .captures(first)
                .or_else(|| self.seat_word.captures(first))
                .or_else(|| self.declared.captures(body))
        } else {
            self.in_text.captures(body)
        };
        let Some(captures) = found else {
            return String::new();
        };
        let name = captures
            .iter()
            .skip(1)
            .flatten()
            .next()
            .map_or(String::new(), |m| m.as_str().to_lowercase());
        if name == "antigravity" {
            "gemini".to_owned()
        } else {
            name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Attribution;

    fn seat(body: &str, strict: bool) -> String {
        Attribution::new().map_or_else(
            |e| format!("pattern error: {e}"),
            |a| a.comment_seat(body, strict),
        )
    }

    #[test]
    fn a_declaration_anywhere_attributes_a_loose_read() {
        assert_eq!(seat("Seat / agent: Mimo (x)", false), "mimo");
        assert_eq!(
            seat(
                "### Review\n**Seat / Agent**: Antigravity (Gemini)\n",
                false
            ),
            "gemini"
        );
    }

    #[test]
    fn a_strict_read_ignores_a_mention_further_down() {
        assert_eq!(seat("My note.\nMimo reviewed this earlier.\n", true), "");
        assert_eq!(seat("intro\nSeat / agent: Codex\n", true), "codex");
        assert_eq!(seat("Claude seat: done", true), "claude");
    }
}
