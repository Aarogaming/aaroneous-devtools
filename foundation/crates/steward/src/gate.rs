//! Facts the merge gate derives from paths, CI, the circuit-breaker file and review threads.
//! Port of `merge_gate.gate_plan`, `category_of`, `ci_state`, `breaker_state` and `open_threads` (vector
//! families `gate_plan`, `gate_ci`, `gate_breaker`, `gate_threads`). The owner-only path tiers were removed in #134.

use serde_json::Value;
use std::collections::BTreeSet;

const DOCS_ALLOWLIST: [&str; 3] = ["governance/", "docs/", "architecture/"];
/// Unicode `Cf` (format) code points, as of the Unicode 15.0 that Python 3.12 ships; `char` has no category API.
const FORMAT_RANGES: [(u32, u32); 21] = [
    (0xAD, 0xAD),
    (0x600, 0x605),
    (0x61C, 0x61C),
    (0x6DD, 0x6DD),
    (0x70F, 0x70F),
    (0x890, 0x891),
    (0x8E2, 0x8E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];

/// Python's `str.isprintable` for one character, except for unassigned (`Cn`) code points other than noncharacters:
/// those depend on the Unicode version of the interpreter, so they print raw here (a documented gap, pinned by a test).
fn is_printable(c: char) -> bool {
    let code = c as u32;
    if c == ' ' {
        return true;
    }
    let private = (0xE000..=0xF8FF).contains(&code) || code >= 0xF_0000;
    let noncharacter = (0xFDD0..=0xFDEF).contains(&code) || code & 0xFFFE == 0xFFFE;
    let format = FORMAT_RANGES
        .iter()
        .any(|&(lo, hi)| (lo..=hi).contains(&code));
    !(c.is_control() || c.is_whitespace() || private || noncharacter || format)
}

/// Python's `repr` of a string: quote choice, backslash and quote escapes, `\n \r \t`, and `\xNN`, `\uNNNN` or
/// `\UNNNNNNNN` for every non-printable character.
pub fn py_repr_str(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if is_printable(c) => out.push(c),
            c if (c as u32) <= 0xFF => out.push_str(&format!("\\x{:02x}", c as u32)),
            c if (c as u32) <= 0xFFFF => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push_str(&format!("\\U{:08x}", c as u32)),
        }
    }
    out.push(quote);
    out
}

/// Python's `repr` of a list of strings (the owner-only reason prints one).
pub fn py_repr_list(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|item| py_repr_str(item)).collect();
    format!("[{}]", quoted.join(", "))
}

/// `"light"` only when every path is a markdown or JSON file under the docs allowlist, otherwise `"full"`.
#[must_use]
pub fn gate_plan_level(paths: &[String]) -> &'static str {
    let docs_only = !paths.is_empty()
        && paths.iter().all(|p| {
            (p.ends_with(".md") || p.ends_with(".json"))
                && DOCS_ALLOWLIST.iter().any(|d| p.starts_with(d))
        });
    if docs_only { "light" } else { "full" }
}

/// `dirname` as `os.path.dirname` reads a forward-slash path: everything before the last `/` (`""` when none).
fn dirname(path: &str) -> &str {
    let head = path.rfind('/').map_or("", |i| &path[..=i]);
    if head.is_empty() || head.chars().all(|c| c == '/') {
        // no directory part, or the root: kept as is
        return head;
    }
    head.trim_end_matches('/')
}

/// `os.path.splitext(path)[1]`: the extension including the dot, or `""` when there is none. A leading dot of the
/// basename does not start an extension (`.gitignore` has none), and neither does a run of leading dots.
fn extension(path: &str) -> &str {
    let base_start = path.rfind('/').map_or(0, |i| i + 1);
    let base = &path[base_start..];
    let lead = base.len() - base.trim_start_matches('.').len();
    match base[lead..].rfind('.') {
        Some(i) => &base[lead + i..],
        None => "",
    }
}

/// A coarse file-pattern category for the trust counter, such as `governance/*.md`.
#[must_use]
pub fn category_of(paths: &[String]) -> String {
    if paths.is_empty() {
        return "?".to_owned();
    }
    let dirs: BTreeSet<&str> = paths.iter().map(|p| dirname(p)).collect();
    let exts: BTreeSet<&str> = paths.iter().map(|p| extension(p)).collect();
    if dirs.len() == 1 && exts.len() == 1 {
        let dir = dirs.into_iter().next().unwrap_or_default();
        let ext = exts.into_iter().next().unwrap_or_default();
        let dir = if dir.is_empty() { "." } else { dir };
        let ext = if ext.is_empty() { "*" } else { ext };
        return format!("{dir}/*{ext}");
    }
    "mixed".to_owned()
}

/// One entry of a `statusCheckRollup` list.
#[derive(Debug, Clone, Default)]
pub struct CiCheck {
    pub conclusion: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiState {
    pub state: String,
    pub detail: String,
}

/// Fail-closed CI state: pending or absent is never success, and neither is skipped.
#[must_use]
pub fn ci_state(rollup: &[CiCheck]) -> CiState {
    if rollup.is_empty() {
        return CiState {
            state: "missing".to_owned(),
            detail: "no CI status found".to_owned(),
        };
    }
    let states: Vec<String> = rollup
        .iter()
        .map(|item| {
            let conclusion = item.conclusion.clone().unwrap_or_default().to_uppercase();
            let status = item.status.clone().unwrap_or_default().to_uppercase();
            if status == "COMPLETED" || !conclusion.is_empty() {
                conclusion
            } else {
                "PENDING".to_owned()
            }
        })
        .collect();
    if states
        .iter()
        .any(|s| matches!(s.as_str(), "" | "PENDING" | "QUEUED" | "IN_PROGRESS"))
    {
        return CiState {
            state: "pending".to_owned(),
            detail: "checks still running".to_owned(),
        };
    }
    let distinct: BTreeSet<&str> = states.iter().map(String::as_str).collect();
    let ok = states.iter().all(|s| s == "SUCCESS");
    CiState {
        state: if ok { "success" } else { "failure" }.to_owned(),
        detail: format!(
            "{} checks: {}",
            states.len(),
            distinct.into_iter().collect::<Vec<_>>().join(", ")
        ),
    }
}

/// `Some(bool)` from a real TOML boolean `tripped` at the top level; `None` (blocked, fail closed) when the file does
/// not parse or the value is missing or not a boolean. A comment such as `# tripped = false` never counts.
#[must_use]
pub fn breaker_state(text: &str) -> Option<bool> {
    let table: toml::Table = text.parse().ok()?;
    table.get("tripped")?.as_bool()
}

/// Python truthiness of a JSON value.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// The unresolved review-thread count from a GraphQL `reviewThreads` response, or `None` when it cannot be trusted:
/// more pages exist, the pagination info is missing, or the response is malformed. A truncated first page is never
/// counted as complete.
#[must_use]
pub fn open_threads(data: &Value) -> Option<usize> {
    let threads = data
        .get("data")?
        .get("repository")?
        .get("pullRequest")?
        .get("reviewThreads")?;
    if threads.get("pageInfo")?.get("hasNextPage")? != &Value::Bool(false) {
        return None;
    }
    match threads.get("nodes")? {
        Value::Array(nodes) => {
            let mut open = 0;
            for node in nodes {
                if !truthy(node.as_object()?.get("isResolved")?) {
                    open += 1;
                }
            }
            Some(open)
        }
        // iterating a dict or a string visits keys or characters, which cannot be indexed: only an empty one is a count
        Value::Object(map) if map.is_empty() => Some(0),
        Value::String(s) if s.is_empty() => Some(0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CiCheck, breaker_state, category_of, ci_state, extension, gate_plan_level, open_threads,
    };
    use serde_json::json;

    fn paths(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    /// Expected values are Python 3.12's `repr`, produced by running it. Skeptic probe on the dispatcher: the first
    /// version escaped only control characters, so a seat name holding U+2028 or U+200B printed raw where Python
    /// prints an escape.
    #[test]
    fn py_repr_str_escapes_every_non_printable_character_the_way_python_does() {
        let cases = [
            ("\u{2028}", "'\\u2028'"),
            ("\u{200b}", "'\\u200b'"),
            ("\u{a0}", "'\\xa0'"),
            ("\u{ad}", "'\\xad'"),
            ("\u{7f}", "'\\x7f'"),
            ("\u{85}", "'\\x85'"),
            ("\u{feff}", "'\\ufeff'"),
            ("\u{e000}", "'\\ue000'"),
            ("\u{e0001}", "'\\U000e0001'"),
            ("\u{10ffff}", "'\\U0010ffff'"),
            ("\u{fdd0}", "'\\ufdd0'"),
            ("\u{fffe}", "'\\ufffe'"),
            ("\u{1fffe}", "'\\U0001fffe'"),
            ("\u{3000}", "'\\u3000'"),
            ("\u{180e}", "'\\u180e'"),
            ("\0", "'\\x00'"),
            ("\u{e9}", "'\u{e9}'"),
            ("\u{1F600}", "'\u{1F600}'"),
            ("it's", "\"it's\""),
            ("say \"hi\"", "'say \"hi\"'"),
            ("it's \"x\"", "'it\\'s \"x\"'"),
            ("tab\there", "'tab\\there'"),
        ];
        for (input, want) in cases {
            assert_eq!(super::py_repr_str(input), want, "{input:?}");
        }
    }

    #[test]
    fn unassigned_code_points_print_raw_which_is_the_documented_gap() {
        // U+0378 is unassigned (Cn): Python 3.12 escapes it as '͸'; the port cannot know without a Unicode table.
        assert_eq!(super::py_repr_str("\u{378}"), "'\u{378}'");
    }

    #[test]
    fn the_light_gate_needs_docs_markdown_or_json_only() {
        assert_eq!(
            gate_plan_level(&paths(&["docs/a.md", "governance/b.json"])),
            "light"
        );
        assert_eq!(gate_plan_level(&paths(&["scripts/a.py"])), "full");
        assert_eq!(gate_plan_level(&[]), "full");
    }

    #[test]
    fn categories_follow_os_path() {
        assert_eq!(
            category_of(&paths(&["governance/a.md", "governance/b.md"])),
            "governance/*.md"
        );
        assert_eq!(category_of(&paths(&["AGENTS.md"])), "./*.md");
        assert_eq!(category_of(&paths(&["Makefile"])), "./**");
        assert_eq!(category_of(&paths(&["a/.gitignore"])), "a/**");
        assert_eq!(category_of(&paths(&["a/x.md", "b/y.md"])), "mixed");
        assert_eq!(extension("dir/archive.tar.gz"), ".gz");
        assert_eq!(extension("..hidden"), "");
    }

    #[test]
    fn ci_is_green_only_when_every_check_succeeded() {
        let done = |c: &str| CiCheck {
            conclusion: Some(c.into()),
            status: Some("COMPLETED".into()),
        };
        assert_eq!(ci_state(&[]).state, "missing");
        assert_eq!(
            ci_state(&[done("SUCCESS"), done("success")]).detail,
            "2 checks: SUCCESS"
        );
        assert_eq!(ci_state(&[done("SKIPPED")]).state, "failure");
        assert_eq!(
            ci_state(&[CiCheck {
                conclusion: None,
                status: Some("QUEUED".into())
            }])
            .state,
            "pending"
        );
    }

    #[test]
    fn the_breaker_needs_a_real_toml_boolean() {
        assert_eq!(breaker_state("tripped = false\n"), Some(false));
        assert_eq!(
            breaker_state("# tripped = false\ntripped = true\n"),
            Some(true)
        );
        assert_eq!(breaker_state("# tripped = false\n"), None);
        assert_eq!(breaker_state("tripped = \"false\"\n"), None);
        assert_eq!(breaker_state("tripped = = false"), None);
    }

    #[test]
    fn threads_are_counted_only_from_a_complete_page() {
        let page = |next: serde_json::Value, nodes: serde_json::Value| json!({"data": {"repository": {"pullRequest": {"reviewThreads": {"nodes": nodes, "pageInfo": {"hasNextPage": next}}}}}});
        assert_eq!(
            open_threads(&page(
                json!(false),
                json!([{"isResolved": true}, {"isResolved": false}])
            )),
            Some(1)
        );
        assert_eq!(open_threads(&page(json!(true), json!([]))), None);
        assert_eq!(open_threads(&page(json!(null), json!([]))), None);
        assert_eq!(
            open_threads(&page(json!(false), json!([{"isResolved": 0}]))),
            Some(1),
            "falsy counts as open"
        );
        assert_eq!(open_threads(&json!({"data": {}})), None);
    }
}
