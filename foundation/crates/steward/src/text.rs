//! Python string semantics the steward's rules rely on, so a port reads text the way the oracle does.

/// `str.isspace()`: Unicode White_Space plus the four ASCII separator controls Python also counts (`\x1c`-`\x1f`).
#[must_use]
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{1c}'..='\u{1f}')
}

/// `str.strip()` for whitespace: trims both ends with [`is_space`].
#[must_use]
pub fn strip(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// `str.splitlines()`: breaks at `\n`, `\r`, `\r\n` and the separators `\x0b`, `\x0c`, `\x1c`-`\x1e`, `\x85`,
/// U+2028 and U+2029, without keeping them. A trailing break does not add an empty last line.
#[must_use]
pub fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        let is_break = matches!(
            c,
            '\n' | '\r'
                | '\u{0b}'
                | '\u{0c}'
                | '\u{1c}'
                | '\u{1d}'
                | '\u{1e}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if !is_break {
            continue;
        }
        lines.push(&text[start..at]);
        start = at + c.len_utf8();
        if c == '\r' {
            if let Some(&(next_at, '\n')) = chars.peek() {
                chars.next();
                start = next_at + 1;
            }
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{is_space, splitlines, strip};

    #[test]
    fn splitlines_matches_python() {
        assert_eq!(splitlines("a\nb\r\nc\rd"), ["a", "b", "c", "d"]);
        assert_eq!(splitlines("a\n"), ["a"]);
        assert_eq!(splitlines("a\n\nb"), ["a", "", "b"]);
        assert_eq!(splitlines("a\u{2028}b\u{0b}c\u{85}d"), ["a", "b", "c", "d"]);
        assert!(splitlines("").is_empty());
    }

    #[test]
    fn strip_counts_the_ascii_separators_python_counts() {
        assert!(is_space('\u{1f}'));
        assert_eq!(strip("\u{1f} x \u{a0}"), "x");
    }
}
