//! Markdown table rows, split the way GitHub renders them.
//!
//! A cell may hold a pipe written `\|` (GitHub shows it as `|`, also inside a code span). Splitting on every `|`, which
//! several readers of `COORDINATION_QUEUE.md` did, cut such a cell in two: the C89 row names the profiles
//! `kernel\|control\|presentation\|tooling`, so its status read as `control\` and the scheduler never saw the row.

/// The raw cells of one table row, between the unescaped pipes, with `\|` turned into `|`. The empty strings before the
/// first pipe and after the last one are kept, so indexes match a plain `split('|')` of a row with no escapes.
pub(crate) fn raw_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = row.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('\\', Some('|')) => {
                current.push('|');
                chars.next();
            }
            ('\\', Some('\\')) => {
                // an escaped backslash: keep both, and the next character is not escaped by either of them
                current.push_str("\\\\");
                chars.next();
            }
            ('|', _) => cells.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    cells.push(current);
    cells
}

/// [`raw_cells`], each cell trimmed.
pub(crate) fn cells(row: &str) -> Vec<String> {
    raw_cells(row)
        .into_iter()
        .map(|c| c.trim().to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{cells, raw_cells};

    #[test]
    fn a_row_without_escapes_splits_like_a_plain_split() {
        let row = "| C1 | scope | Ready | owner |";
        assert_eq!(
            raw_cells(row),
            row.split('|').map(str::to_owned).collect::<Vec<_>>()
        );
        assert_eq!(cells(row), ["", "C1", "scope", "Ready", "owner", ""]);
    }

    #[test]
    fn an_escaped_pipe_stays_inside_its_cell() {
        let row = r#"| C89 | profile = "kernel\|control\|tooling" | Ready | x |"#;
        let found = cells(row);
        assert_eq!(found.len(), 6);
        assert_eq!(found[2], r#"profile = "kernel|control|tooling""#);
        assert_eq!(found[3], "Ready");
    }

    #[test]
    fn an_escaped_backslash_does_not_escape_the_pipe_after_it() {
        // `\\|` is a backslash and then a real separator
        assert_eq!(cells(r"| a\\| b |"), ["", r"a\\", "b", ""]);
    }
}
