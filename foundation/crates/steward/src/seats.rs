//! Which seats a reviewer or implementer string names. Port of `review_coverage.seat_names` (vector family `seats`).

use std::collections::BTreeSet;

/// A seat is recognised by a token that STARTS with its name ("claude-sonnet-5-5", "gemini-3.8-flash").
const KNOWN_SEATS: [&str; 6] = ["claude", "codex", "gemini", "antigravity", "mimo", "qwen"];

/// Seats that are another seat under a different name.
const ALIASES: [(&str, &str); 2] = [("antigravity", "gemini"), ("qwen", "mimo")];

fn canonical(seat: &str) -> &str {
    ALIASES
        .iter()
        .find(|(from, _)| *from == seat)
        .map_or(seat, |(_, to)| to)
}

/// The set of seats `name` names. A string may name several ("Claude, Mimo", "claude+mimo", "Gemini/Antigravity");
/// a string that names none keeps its first alphanumeric token, so an unknown seat still compares with itself.
/// Tokens are the lower-cased runs of ASCII letters and digits, so any other character, a zero-width space
/// included, separates them.
#[must_use]
pub fn seat_names(name: &str) -> BTreeSet<String> {
    let lowered = name.to_lowercase();
    let tokens: Vec<&str> = lowered
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();
    let mut found = BTreeSet::new();
    for token in &tokens {
        for seat in KNOWN_SEATS {
            if token.starts_with(seat) {
                found.insert(canonical(seat).to_owned());
            }
        }
    }
    if found.is_empty() {
        if let Some(first) = tokens.first() {
            found.insert((*first).to_owned());
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::seat_names;

    fn names(input: &str) -> Vec<String> {
        seat_names(input).into_iter().collect()
    }

    #[test]
    fn spellings_of_one_seat_agree() {
        assert_eq!(names("Gemini"), ["gemini"]);
        assert_eq!(names("Antigravity"), ["gemini"]);
        assert_eq!(names("claude-sonnet-5-5"), ["claude"]);
        assert_eq!(names("Local Qwen"), ["mimo"]);
    }

    #[test]
    fn one_string_may_name_several_seats() {
        assert_eq!(names("claude+mimo"), ["claude", "mimo"]);
    }

    #[test]
    fn an_unknown_string_keeps_its_first_token_and_nothing_names_nothing() {
        assert_eq!(names("someone else"), ["someone"]);
        assert!(seat_names("!!!").is_empty());
        assert!(seat_names("").is_empty());
    }
}
