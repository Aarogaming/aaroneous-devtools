//! Does a comment cite a pull request's head? Port of `collect_state.cites_head` (vector family `heads`).

use crate::text::Lexicon;

/// A word character once the text has been through [`Lexicon::pyfy`], where Rust's notion equals Python's.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_hex(c: char) -> bool {
    matches!(c, '0'..='9' | 'a'..='f')
}

/// The 8 to 40 character hex tokens of `text`, the way `\b[0-9a-f]{8,40}\b` finds them: a maximal run of hex
/// characters with a word boundary on both sides. A run inside a longer word ("deadbeefcafe_x", a 42-character run)
/// is not a token.
fn hex_tokens(text: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut run_start: Option<usize> = None;
    let mut before_is_word = false;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for (i, &(at, c)) in chars.iter().enumerate() {
        if is_hex(c) {
            if run_start.is_none() {
                run_start = Some(at);
                before_is_word = i > 0 && is_word(chars[i - 1].1);
            }
            continue;
        }
        if let Some(start) = run_start.take() {
            push_if_token(text, start, at, before_is_word || is_word(c), &mut tokens);
        }
    }
    if let Some(start) = run_start {
        push_if_token(text, start, text.len(), before_is_word, &mut tokens);
    }
    tokens
}

fn push_if_token<'a>(
    text: &'a str,
    start: usize,
    end: usize,
    touches_word: bool,
    tokens: &mut Vec<&'a str>,
) {
    let length = end - start;
    if !touches_word && (8..=40).contains(&length) {
        tokens.push(&text[start..end]);
    }
}

/// True when `body` cites `head`: a hex token of 8 to 40 characters that is a prefix of the full head. A 40-character
/// token must equal the head, so a different head that shares the first eight characters is not mistaken for this one.
#[must_use]
pub fn cites_head(lexicon: &Lexicon, body: &str, head: &str) -> bool {
    let head = head.to_lowercase();
    if head.is_empty() {
        return false;
    }
    let body = lexicon.pyfy(&body.to_lowercase());
    hex_tokens(&body)
        .into_iter()
        .any(|token| head.starts_with(token))
}

#[cfg(test)]
mod tests {
    use super::cites_head as cites;
    use crate::text::Lexicon;

    fn cites_head(body: &str, head: &str) -> bool {
        Lexicon::new().is_ok_and(|lexicon| cites(&lexicon, body, head))
    }

    const HEAD: &str = "cccccccccccccccccccccccccccccccccccccccc";

    #[test]
    fn a_prefix_of_eight_or_more_cites_the_head() {
        assert!(cites_head("head cccccccc", HEAD));
        assert!(cites_head(&format!("head {HEAD}"), HEAD));
        assert!(cites_head(&format!("HEAD {}", HEAD.to_uppercase()), HEAD));
    }

    #[test]
    fn short_different_or_embedded_tokens_do_not() {
        assert!(!cites_head("head ccccccc", HEAD));
        assert!(!cites_head(
            "head ccccccccdddddddddddddddddddddddddddddddd",
            HEAD
        ));
        assert!(!cites_head("no hash here", HEAD));
        assert!(!cites_head(&format!("head {HEAD}"), ""));
        assert!(
            !cites_head("cccccccc_suffix", HEAD),
            "an underscore is a word character"
        );
        assert!(
            !cites_head(&"c".repeat(42), HEAD),
            "a 42-character run is one over-long token"
        );
        // Python's \w: a superscript digit is a word character, a combining mark is not.
        assert!(!cites_head("cccccccc\u{b2}", HEAD));
        assert!(cites_head("cccccccc\u{301}", HEAD));
        assert!(cites_head("cccccccc\u{e31}", HEAD));
    }
}
