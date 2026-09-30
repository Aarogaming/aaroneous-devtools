//! Deterministic conformance checks for local-model drafts (lane C76).
//!
//! Pure functions over text: no I/O, no clock, no environment. Nothing here
//! compiles or runs a draft (that is `dogfood::verify_draft`, M44); this
//! module supplies the parts that need no toolchain:
//!
//! * [`RULE_PACK`] - Aaroneous floor rules, phrased for a small model's system prompt.
//! * [`scan_draft`] - heuristic static scan for the floor's banned patterns.
//! * [`unanchored_observations`] - rejects model claims that do not quote a
//!   line that actually occurs in the supplied evidence.
//!
//! The scan is a line/brace heuristic, not a parser. It ignores comments and
//! braces inside string/char literals, but still reports banned patterns that
//! appear inside literals, and it does not track raw strings or literals that span
//! lines. Tier 0 (`xtask gate`, `ast_auditor`) stays the authority. A clean scan is
//! never evidence of conformance.

/// Rules for the worker system prompt. Source: AGENTS.md sections 2 and 5.
pub(crate) const RULE_PACK: &str = "\
Aaroneous floor rules (a draft that breaks any is rejected):
1. No std::env::var/var_os/set_var/remove_var/temp_dir/current_dir and no .canonicalize(). Take paths and config through constructor arguments.
2. No SystemTime::now() or Instant::now() in non-test code. Take time as a parameter (tick or injected clock).
3. Never spawn threads or async tasks (std::thread::spawn, tokio::spawn).
4. No todo!() or unimplemented!(). No unsafe impl of Pod or Zeroable (derive only).
5. Outside #[cfg(test)]: no .unwrap(), .expect(), panic!() or assert!(). Return Result. The only exception is a call directly preceded by a `// INFALLIBLE: <reason>` comment.
6. Tests build their own fixtures with tempfile::tempdir(); never read host environment variables or use ambient temp folders.
7. Every observation must quote the exact source line it refers to in `backticks`. Do not claim something is missing unless you searched the supplied source for it.";

/// One rule breach found by [`scan_draft`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Violation {
    pub rule: &'static str,
    /// 1-based line number within the scanned source.
    pub line: usize,
}

/// Patterns banned everywhere in a draft, including tests.
const BANNED_EVERYWHERE: &[(&str, &str)] = &[
    ("ambient-env", "env::var"),
    ("ambient-env", "env::set_var"),
    ("ambient-env", "env::remove_var"),
    ("ambient-env", "env::temp_dir"),
    ("ambient-env", "env::current_dir"),
    ("ambient-path", ".canonicalize("),
    ("self-started-thread", "thread::spawn"),
    ("self-started-task", "tokio::spawn"),
    ("stub", "todo!("),
    ("stub", "unimplemented!("),
    ("unsafe-pod", "unsafe impl Pod"),
    ("unsafe-pod", "unsafe impl Zeroable"),
];

/// Patterns banned outside test regions.
const BANNED_OUTSIDE_TESTS: &[(&str, &str)] = &[
    ("ambient-clock", "SystemTime::now"),
    ("ambient-clock", "Instant::now"),
    ("panic-on-input", ".unwrap()"),
    ("panic-on-input", ".expect("),
    ("panic-on-input", "panic!("),
    ("panic-on-input", "assert!("),
    ("panic-on-input", "assert_eq!("),
    ("panic-on-input", "assert_ne!("),
];

/// Scans `source` for the floor's banned patterns.
///
/// Test regions (a `#[cfg(test)]` item or a `#[test]` function body, found by
/// brace counting) are exempt from the panic and clock rules only. Comment-only
/// lines are ignored. A panic-rule hit is waived when the nearest preceding
/// non-blank line is a `// INFALLIBLE:` comment.
pub(crate) fn scan_draft(source: &str) -> Vec<Violation> {
    let mut found = Vec::new();
    let mut depth: i64 = 0;
    // Depth at which the current test region opened; `None` outside one.
    let mut test_region_from: Option<i64> = None;
    let mut test_attr_pending = false;
    let mut previous_code_line: Option<&str> = None;

    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        let comment_only = line.starts_with("//");
        if !comment_only {
            if line.contains("#[cfg(test)]") || line.starts_with("#[test]") {
                test_attr_pending = true;
            }
            let in_test = test_region_from.is_some() || test_attr_pending;
            let (code, structure) = split_code(line);
            for (rule, needle) in BANNED_EVERYWHERE {
                if code.contains(needle) {
                    found.push(Violation {
                        rule,
                        line: index + 1,
                    });
                }
            }
            if !in_test {
                let waived = previous_code_line
                    .is_some_and(|p| p.trim_start().starts_with("// INFALLIBLE:"));
                for (rule, needle) in BANNED_OUTSIDE_TESTS {
                    if code.contains(needle) && !(waived && *rule == "panic-on-input") {
                        found.push(Violation {
                            rule,
                            line: index + 1,
                        });
                    }
                }
            }
            for ch in structure.chars() {
                match ch {
                    '{' => {
                        if test_attr_pending && test_region_from.is_none() {
                            test_region_from = Some(depth);
                            test_attr_pending = false;
                        }
                        depth += 1;
                    }
                    '}' => {
                        depth -= 1;
                        if test_region_from == Some(depth) {
                            test_region_from = None;
                        }
                    }
                    _ => {}
                }
            }
        }
        if !line.is_empty() {
            previous_code_line = Some(raw);
        }
    }
    found
}

/// Splits one line into (`code`, `structure`).
///
/// `code` is the line with any trailing `//` comment removed, where a `//` inside a
/// string literal does not start a comment; string contents are kept, so patterns
/// inside literals are still reported (over-report). `structure` is the same text
/// with string and char literal contents removed, so braces inside literals do not
/// move the test-region depth. Raw strings and literals spanning lines are not
/// tracked; that is a documented limit of this heuristic.
fn split_code(line: &str) -> (String, String) {
    let chars: Vec<char> = line.chars().collect();
    let mut code = String::new();
    let mut structure = String::new();
    let mut in_str = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            code.push(c);
            if c == '\\' && i + 1 < chars.len() {
                code.push(chars[i + 1]);
                i += 1;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            code.push(c);
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            break;
        } else if c == '\'' && chars.get(i + 2) == Some(&'\'') {
            // char literal such as '{'
            code.extend(&chars[i..i + 3]);
            i += 2;
        } else if c == '\'' && chars.get(i + 1) == Some(&'\\') && chars.get(i + 3) == Some(&'\'') {
            // escaped char literal such as '\n'
            code.extend(&chars[i..i + 4]);
            i += 3;
        } else {
            code.push(c);
            structure.push(c);
        }
        i += 1;
    }
    (code, structure)
}

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Quoted segments shorter than this carry too little information to anchor a claim.
const MIN_ANCHOR_LEN: usize = 8;

/// Lines of slack either side of a cited range before a quote counts as out of range.
/// A live run of the worker showed near misses (cited 104, real line 98) that are citation
/// noise, not wrong claims; larger distances are the ones worth a reviewer's attention.
const LINE_SLACK: usize = 10;

/// Returns the indices of observations that are not anchored in `evidence`.
///
/// An observation is anchored when it contains at least one backtick-quoted
/// excerpt of `MIN_ANCHOR_LEN`+ characters and every such excerpt occurs in
/// `evidence` (compared with whitespace normalised). This catches invented or
/// paraphrased "findings"; it cannot tell whether a real quote supports the
/// claim, so it is a floor filter, not review.
pub(crate) fn unanchored_observations(observations: &[String], evidence: &str) -> Vec<usize> {
    let haystack = normalize(evidence);
    observations
        .iter()
        .enumerate()
        .filter(|(_, observation)| {
            let quotes = backtick_quotes(observation);
            quotes.is_empty() || quotes.iter().any(|q| !haystack.contains(&normalize(q)))
        })
        .map(|(index, _)| index)
        .collect()
}

fn backtick_quotes(text: &str) -> Vec<&str> {
    text.split('`')
        .enumerate()
        .filter(|(i, part)| i % 2 == 1 && part.trim().len() >= MIN_ANCHOR_LEN)
        .map(|(_, part)| part)
        .collect()
}

/// Parses the first `line N` / `lines A-B` citation in `text` as an inclusive 1-based range.
fn cited_line_range(text: &str) -> Option<(usize, usize)> {
    let lower = text.to_ascii_lowercase();
    let at = lower.find("line")?;
    let rest = &lower[at + 4..];
    let rest = rest.strip_prefix('s').unwrap_or(rest).trim_start();
    let digits = |s: &str| -> (usize, usize) {
        let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        (s[..end].parse().unwrap_or(0), end)
    };
    let (start, used) = digits(rest);
    if used == 0 || start == 0 {
        return None;
    }
    let tail = rest[used..].trim_start();
    let tail = tail
        .strip_prefix('-')
        .or_else(|| tail.strip_prefix('\u{2013}'));
    let end = match tail {
        Some(t) => {
            let (n, used_end) = digits(t.trim_start());
            if used_end == 0 { start } else { n.max(start) }
        }
        None => start,
    };
    Some((start, end))
}

/// Returns indices of observations whose cited line range does not contain their quoted text.
///
/// An observation that cites `line N` or `lines A-B` and quotes `backticked` text (8+ chars)
/// is rejected when any quote is absent from those lines of `evidence`, widened by
/// `LINE_SLACK` lines either side (whitespace normalised). Observations without a citation or without a quote are not reported here:
/// they are unverifiable, not wrong (use [`unanchored_observations`] for quote presence).
pub(crate) fn out_of_range_observations(observations: &[String], evidence: &str) -> Vec<usize> {
    let lines: Vec<&str> = evidence.lines().collect();
    observations
        .iter()
        .enumerate()
        .filter(|(_, observation)| {
            let Some((a, b)) = cited_line_range(observation) else {
                return false;
            };
            let quotes = backtick_quotes(observation);
            if quotes.is_empty() {
                return false;
            }
            let start = a.saturating_sub(LINE_SLACK).max(1);
            let end = b.saturating_add(LINE_SLACK).min(lines.len());
            let window = if start > end {
                String::new()
            } else {
                normalize(&lines[start - 1..end].join("\n"))
            };
            quotes.iter().any(|q| !window.contains(&normalize(q)))
        })
        .map(|(index, _)| index)
        .collect()
}

/// First 1-based line of `evidence` whose normalised text contains the normalised `quote`.
fn first_line_of(evidence: &str, quote: &str) -> Option<usize> {
    let needle = normalize(quote);
    evidence
        .lines()
        .position(|line| normalize(line).contains(&needle))
        .map(|index| index + 1)
}

/// Maximum number of draft flags recorded per result, to keep records bounded.
const MAX_DRAFT_FLAGS: usize = 20;

/// Shadow-mode flags for the draft source itself: one entry per scan violation, for example
/// `draft line 23: panic-on-input`. Empty means "nothing detected", never "conforms".
pub(crate) fn draft_flags(rust_draft: &str) -> Vec<String> {
    scan_draft(rust_draft)
        .into_iter()
        .take(MAX_DRAFT_FLAGS)
        .map(|violation| format!("draft line {}: {}", violation.line, violation.rule))
        .collect()
}

/// Shadow-mode claim checks for a worker draft against every declared evidence file.
///
/// Returns one human-readable flag per suspicious observation; an empty vector means
/// "nothing detected", never "correct". An observation is flagged
/// - `not anchored` when a backtick quote of 8+ characters appears in none of the files,
///   or it has no such quote at all (see [`unanchored_observations`]);
/// - `line range` when it cites `line N` / `lines A-B` and, in every file where its quotes
///   do occur, they are outside the cited lines (see [`out_of_range_observations`]).
///
/// Observations without a citation are never range-flagged: they are unverifiable.
pub(crate) fn claim_flags(observations: &[String], evidence_files: &[String]) -> Vec<String> {
    let mut flags = Vec::new();
    let per_file_unanchored: Vec<Vec<usize>> = evidence_files
        .iter()
        .map(|file| unanchored_observations(observations, file))
        .collect();
    let per_file_out_of_range: Vec<Vec<usize>> = evidence_files
        .iter()
        .map(|file| out_of_range_observations(observations, file))
        .collect();
    for (index, observation) in observations.iter().enumerate() {
        let unanchored_everywhere = !evidence_files.is_empty()
            && per_file_unanchored
                .iter()
                .all(|missing| missing.contains(&index));
        if unanchored_everywhere {
            flags.push(format!(
                "observation {index}: not anchored in any declared evidence file"
            ));
            continue;
        }
        let anchored_files: Vec<usize> = (0..evidence_files.len())
            .filter(|file| !per_file_unanchored[*file].contains(&index))
            .collect();
        if !anchored_files.is_empty()
            && anchored_files
                .iter()
                .all(|file| per_file_out_of_range[*file].contains(&index))
        {
            let found = backtick_quotes(observation).first().and_then(|quote| {
                anchored_files
                    .iter()
                    .find_map(|file| first_line_of(&evidence_files[*file], quote))
            });
            let found = found.map_or(String::new(), |line| {
                format!("; first found at line {line}")
            });
            flags.push(format!(
                "observation {index}: cited line range (+/-{LINE_SLACK} lines) does not contain its quoted text{found}"
            ));
        }
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(source: &str) -> Vec<&'static str> {
        scan_draft(source).into_iter().map(|v| v.rule).collect()
    }

    #[test]
    fn clean_reducer_passes() {
        let src =
            "pub fn reduce(state: u32, input: u32) -> u32 {\n    state.saturating_add(input)\n}\n";
        assert!(scan_draft(src).is_empty());
    }

    #[test]
    fn ambient_authority_is_flagged_even_in_tests() {
        let src = "#[test]\nfn t() {\n    let d = std::env::temp_dir();\n}\n";
        assert_eq!(rules(src), vec!["ambient-env"]);
    }

    #[test]
    fn stubs_threads_and_pod_impls_are_flagged() {
        let src =
            "fn a() { todo!() }\nfn b() { std::thread::spawn(|| {}); }\nunsafe impl Pod for X {}\n";
        assert_eq!(
            rules(src),
            vec!["stub", "self-started-thread", "unsafe-pod"]
        );
    }

    #[test]
    fn unwrap_outside_tests_is_flagged_with_line_number() {
        let src = "fn f(x: Option<u8>) -> u8 {\n    x.unwrap()\n}\n";
        assert_eq!(
            scan_draft(src),
            vec![Violation {
                rule: "panic-on-input",
                line: 2
            }]
        );
    }

    #[test]
    fn unwrap_inside_cfg_test_module_and_test_fn_is_exempt() {
        let src = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n        Some(1).unwrap();\n        assert_eq!(1, 1);\n    }\n}\n";
        assert!(scan_draft(src).is_empty());
        let single = "#[test]\nfn t() {\n    assert!(true);\n}\nfn after(x: Option<u8>) -> u8 { x.unwrap() }\n";
        assert_eq!(
            scan_draft(single).len(),
            1,
            "region must close before `after`"
        );
    }

    #[test]
    fn infallible_comment_waives_only_the_immediately_following_call() {
        let src = "fn f(x: Option<u8>) -> u8 {\n    // INFALLIBLE: checked by caller\n    x.unwrap()\n}\nfn g(x: Option<u8>) -> u8 {\n    x.unwrap()\n}\n";
        let hits = scan_draft(src);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 6);
    }

    #[test]
    fn ambient_clock_is_flagged_outside_tests_only() {
        assert_eq!(
            rules("fn now() { let _ = std::time::Instant::now(); }\n"),
            vec!["ambient-clock"]
        );
        assert!(scan_draft("#[test]\nfn t() { let _ = std::time::Instant::now(); }\n").is_empty());
    }

    #[test]
    fn comments_do_not_trigger_rules() {
        let src = "// never call .unwrap() or env::var here\nfn f() {} // todo!( later\n";
        assert!(scan_draft(src).is_empty());
    }

    #[test]
    fn rule_pack_names_every_banned_family() {
        for needle in [
            "env::var",
            "SystemTime::now",
            "thread::spawn",
            "todo!()",
            ".unwrap()",
            "tempfile",
            "INFALLIBLE",
            "backticks",
        ] {
            assert!(RULE_PACK.contains(needle), "rule pack missing {needle}");
        }
    }

    #[test]
    fn claims_quoting_real_source_are_anchored() {
        let evidence = "fn walk(depth: u32) {\n    if max_depth  == 0 { return; }\n}\n";
        let obs = vec!["Guard exists: `if max_depth == 0 { return; }`".to_string()];
        assert!(unanchored_observations(&obs, evidence).is_empty());
    }

    #[test]
    fn invented_or_unquoted_claims_are_rejected() {
        let evidence = "fn walk(depth: u32) { if max_depth == 0 { return; } }";
        let obs = vec![
            "No depth guard exists".to_string(),
            "Missing: `depth.checked_sub(1).ok_or(Overflow)?`".to_string(),
            "Short quote `x` is too small to anchor".to_string(),
            "Real: `if max_depth == 0`".to_string(),
        ];
        assert_eq!(unanchored_observations(&obs, evidence), vec![0, 1, 2]);
    }

    #[test]
    fn c56_style_contradicted_claim_is_not_anchored_by_a_paraphrase() {
        let evidence = "Component::CurDir => continue,";
        let obs = vec!["CurDir handling is missing: `Component::CurDir => Err(())`".to_string()];
        assert_eq!(unanchored_observations(&obs, evidence), vec![0]);
    }

    #[test]
    fn double_slash_inside_a_string_does_not_hide_a_later_violation() {
        let src = "fn f(x: Option<u8>) -> u8 { let _u = \"https://example\"; x.unwrap() }\n";
        assert_eq!(rules(src), vec!["panic-on-input"]);
    }

    #[test]
    fn braces_inside_literals_do_not_keep_a_test_region_open() {
        let src = "#[test]\nfn t() { let _s = \"}\"; let _c = '{'; }\nfn after(x: Option<u8>) -> u8 { x.unwrap() }\n";
        let hits = scan_draft(src);
        assert_eq!(
            hits.len(),
            1,
            "region must close at the real brace: {hits:?}"
        );
        assert_eq!(hits[0].line, 3);
    }

    #[test]
    fn a_real_trailing_comment_is_still_stripped() {
        assert!(scan_draft("fn f() {} // x.unwrap() here is only a comment\n").is_empty());
    }

    #[test]
    fn range_check_accepts_a_quote_inside_the_cited_lines() {
        let evidence = "fn a() {}\nfn walk(depth: u32) {\n    if max_depth == 0 { return; }\n}\n";
        let obs = vec!["Guard at lines 2-3: `if max_depth == 0 { return; }`".to_string()];
        assert!(out_of_range_observations(&obs, evidence).is_empty());
    }

    #[test]
    fn range_check_rejects_a_real_quote_at_the_wrong_lines() {
        let mut evidence = String::new();
        for _ in 0..30 {
            evidence.push_str("fn filler() {}\n");
        }
        evidence.push_str("fn walk(depth: u32) { if max_depth == 0 { return; } }\n");
        let obs = vec!["See lines 1-2: `if max_depth == 0 { return; }`".to_string()];
        assert_eq!(out_of_range_observations(&obs, &evidence), vec![0]);
        let near = vec!["See line 25: `if max_depth == 0 { return; }`".to_string()];
        assert!(
            out_of_range_observations(&near, &evidence).is_empty(),
            "within slack"
        );
    }

    #[test]
    fn range_check_ignores_unverifiable_observations() {
        let evidence = "fn a() {}\n";
        let obs = vec![
            "No citation but a quote: `fn a() {}` here".to_string(),
            "Cites lines 1-1 but quotes nothing".to_string(),
        ];
        assert!(out_of_range_observations(&obs, evidence).is_empty());
    }

    #[test]
    fn range_check_handles_single_line_and_out_of_file_ranges() {
        let evidence = "one\ntwo_two_two\nthree\n";
        let ok = vec!["At line 2: `two_two_two`".to_string()];
        let past_end = vec!["At lines 40-50: `two_two_two`".to_string()];
        assert!(out_of_range_observations(&ok, evidence).is_empty());
        assert_eq!(out_of_range_observations(&past_end, evidence), vec![0]);
    }

    #[test]
    fn claim_flags_reports_unanchored_and_wrong_range_but_not_good_claims() {
        let mut file = "one\nfn walk() {}\nlet guard = `x`;\n".to_string();
        for _ in 0..20 {
            file.push_str("fn filler() {}\n");
        }
        file.push_str("fn normalize_path() {}\n");
        let obs = vec![
            "Good, line 24: `fn normalize_path() {}`".to_string(),
            "Invented: `fn invented_thing() {}` exists".to_string(),
            "Wrong place, lines 1-2: `fn normalize_path() {}`".to_string(),
            "No citation, quote `fn walk() {}` only".to_string(),
        ];
        let flags = claim_flags(&obs, &[file]);
        assert_eq!(flags.len(), 2, "{flags:?}");
        assert!(flags[0].starts_with("observation 1:"), "{flags:?}");
        assert!(flags[0].contains("not anchored"), "{flags:?}");
        assert!(flags[1].starts_with("observation 2:"), "{flags:?}");
        assert!(flags[1].contains("line range"), "{flags:?}");
        assert!(flags[1].contains("first found at line 24"), "{flags:?}");
    }

    #[test]
    fn claim_flags_accepts_a_quote_that_is_right_in_at_least_one_file() {
        let a = "alpha\nbeta\n".to_string();
        let b = "gamma\nfn target_fn() {}\n".to_string();
        let obs = vec!["See line 2: `fn target_fn() {}`".to_string()];
        assert!(claim_flags(&obs, &[a, b]).is_empty());
    }

    #[test]
    fn claim_flags_with_no_evidence_flags_nothing() {
        let obs = vec!["anything `quoted text`".to_string()];
        assert!(claim_flags(&obs, &[]).is_empty());
    }

    #[test]
    fn draft_flags_reports_scan_hits_with_lines_and_is_bounded() {
        let src = "pub fn a(x: Option<u8>) -> u8 {\n    x.unwrap()\n}\n";
        assert_eq!(
            draft_flags(src),
            vec!["draft line 2: panic-on-input".to_string()]
        );
        assert!(draft_flags("pub fn ok() {}\n").is_empty());
        let many = "fn f(x: Option<u8>) { x.unwrap(); }\n".repeat(50);
        assert_eq!(draft_flags(&many).len(), MAX_DRAFT_FLAGS);
    }
}
