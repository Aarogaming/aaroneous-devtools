//! The merge gate's nine-box checklist and default decision.
//! Port of `merge_gate.checklist`, `reconcile` and `decide` (vector families `gate_checklist` and `gate_decide`), as
//! they are with PR #122 applied: every fact is read strictly, so a missing or mistyped one blocks its box.

use crate::gate::{gate_plan_level, py_repr_str, truthy};
use serde_json::Value;

const FULL_GATE: [&[&str]; 5] = [
    &["git", "diff", "--check", "{base}...HEAD"],
    &[
        "python",
        "scripts/check_coordination_evidence.py",
        "{base}",
        "HEAD",
    ],
    &["cargo", "fmt", "--all", "--", "--check"],
    &[
        "cargo",
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ],
    &["cargo", "test", "-p", "foundation_cli"],
];
const LIGHT_GATE: [&[&str]; 2] = [
    &["git", "diff", "--check", "{base}...HEAD"],
    &[
        "python",
        "scripts/check_coordination_evidence.py",
        "{base}",
        "HEAD",
    ],
];

/// A checklist box: checked, unchecked, or blocked (the checker could not evaluate it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    Checked,
    Unchecked,
    Blocked,
}

fn mark_of(ok: Option<bool>) -> Mark {
    match ok {
        Some(true) => Mark::Checked,
        Some(false) => Mark::Unchecked,
        None => Mark::Blocked,
    }
}

/// `box()`: `[x]`, `[ ]` or `[blocked: reason]`, then the label and, unless blocked, the detail in parentheses.
fn render(mark: Mark, label: &str, detail: &str) -> String {
    match mark {
        Mark::Checked => format!("[x] {label}{}", paren(detail)),
        Mark::Unchecked => format!("[ ] {label}{}", paren(detail)),
        Mark::Blocked => format!("[blocked: {detail}] {label}"),
    }
}

fn paren(detail: &str) -> String {
    if detail.is_empty() {
        String::new()
    } else {
        format!(" ({detail})")
    }
}

/// A JSON boolean as a three-state mark: anything that is not literally true or false is blocked.
fn json_mark(value: Option<&Value>) -> Option<bool> {
    value.and_then(Value::as_bool)
}

/// Whether a raw fact is present but neither a boolean nor null: such a box is blocked and, as in the Python version
/// (which picks the blocked reason only when the value `is None`), shows its detail instead of its reason.
fn is_other(value: Option<&Value>) -> bool {
    matches!(value, Some(v) if !v.is_null() && !v.is_boolean())
}

/// Python's `str()` of a JSON value as it appears in the checklist's f-strings.
fn py_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => py_repr(other),
    }
}

/// Python's `repr()` of a JSON value.
pub fn py_repr(value: &Value) -> String {
    match value {
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => py_repr_str(s),
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(py_repr).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(k, v)| format!("{}: {}", py_repr(&Value::String(k.clone())), py_repr(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn is_count(value: Option<&Value>) -> Option<i64> {
    value.and_then(Value::as_i64)
}

fn first_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

/// `str(value[:12])` for a value Python can slice: a string gives its first characters, a list gives the repr of its
/// first items; anything else (None, a number, a dict) is the `TypeError` the Python version raises.
fn slice12(value: &Value, what: &str) -> Result<String, String> {
    match value {
        Value::String(text) => Ok(first_chars(text, 12)),
        Value::Array(items) => Ok(py_repr(&Value::Array(
            items.iter().take(12).cloned().collect(),
        ))),
        _ => Err(format!("{what} cannot be sliced")),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checklist {
    pub lines: Vec<String>,
    pub would_merge: bool,
    pub first_blocker: Option<String>,
}

/// The policy's literal checklist. Report-only. A box the checker cannot evaluate is blocked and fails the gate like
/// an unchecked box; nothing is rounded up to green. `Err` is a fact the checklist cannot even read (a key every
/// caller must supply), which the Python version reports as a crash.
///
/// # Errors
/// Returns the name of a required fact (`head`, `base_ref`, `default_branch`, `trial_merge_ok`) that is missing.
pub fn checklist(facts: &Value) -> Result<Checklist, String> {
    let need = |key: &str| {
        facts
            .get(key)
            .ok_or_else(|| format!("missing required fact: {key}"))
    };
    let head = slice12(need("head")?, "head")?;
    let base_ref = need("base_ref")?;
    let default_branch = need("default_branch")?;
    let trial = need("trial_merge_ok")?;
    let ci_default = serde_json::json!({"state": "missing", "detail": "no CI status found"});
    let ci = match facts.get("ci") {
        Some(v) if truthy(v) => v,
        _ => &ci_default,
    };
    let ci_state = ci.get("state").ok_or("missing required fact: ci.state")?;
    let ci_detail_raw = ci.get("detail");
    let ci_detail = ci_detail_raw.map(py_str).unwrap_or_default();
    let seats_value = facts.get("accept_seats");
    let seats: Option<Vec<String>> = seats_value.and_then(Value::as_array).and_then(|items| {
        items
            .iter()
            .map(|s| s.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
    });
    let distinct = seats
        .as_ref()
        .map(|list| {
            list.iter()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        })
        .unwrap_or(0);
    let covered = json_mark(facts.get("covered"));
    let missing_evidence = facts
        .get("verdicts_missing_evidence")
        .and_then(Value::as_array);
    let counted = is_count(facts.get("counted_verdicts"));
    let threads_open = is_count(facts.get("threads_open"));
    let standing = json_mark(facts.get("standing_changes_requested"));
    let tripped = json_mark(facts.get("breaker_tripped"));
    let base_tip = match facts.get("base_tip") {
        None => "?".to_owned(),
        Some(v) => slice12(v, "base_tip")?,
    };

    let mut checks: Vec<(&str, Option<bool>, String, String, String)> = Vec::new();
    checks.push((
        "1",
        if ci_state.as_str() == Some("missing") {
            None
        } else {
            Some(ci_state.as_str() == Some("success"))
        },
        format!("CI green on head {head}"),
        ci_detail.clone(),
        if truthy(ci_detail_raw.unwrap_or(&Value::Null)) {
            ci_detail.clone()
        } else {
            "no CI status found".to_owned()
        },
    ));
    checks.push((
        "2",
        json_mark(Some(trial)),
        format!(
            "No merge conflict vs. {} tip {}",
            py_str(base_ref),
            base_tip
        ),
        String::new(),
        String::new(),
    ));
    checks.push((
        "2a",
        Some(base_ref == default_branch),
        "Base branch is the repository default branch".to_owned(),
        format!("base={}", py_str(base_ref)),
        String::new(),
    ));
    checks.push((
        "2b",
        match facts.get("is_draft") {
            None | Some(Value::Null) => None,
            Some(value) => Some(!truthy(value)),
        },
        "PR is not a draft".to_owned(),
        String::new(),
        "draft status unavailable (fail closed)".to_owned(),
    ));
    // Owner decision 2026-10-11 (DECISIONS.md), mirrored from scripts/merge_gate.py: only an exact boolean true grants the exemption.
    let identity_exempt = matches!(
        facts.get("identity_registration_exempt"),
        Some(Value::Bool(true))
    );
    let exempt_suffix = if identity_exempt {
        " — exempt: additive-only identities.json registration (owner decision 2026-10-11)"
    } else {
        ""
    };
    let seat_label = match &seats {
        Some(list) if !list.is_empty() => list.join(", "),
        _ => "none".to_owned(),
    };
    checks.push((
        "3",
        if identity_exempt {
            Some(true)
        } else {
            match (&seats, covered) {
                (Some(_), Some(c)) => Some(distinct >= 2 && c),
                _ => None,
            }
        },
        format!(">=2 independent accept verdicts at {head}, seats: {seat_label}{exempt_suffix}"),
        String::new(),
        "accepting seats or coverage unavailable".to_owned(),
    ));
    checks.push((
        "4",
        match (threads_open, standing) {
            (Some(n), Some(s)) => Some(n == 0 && !s),
            _ => None,
        },
        "Zero open blocking threads".to_owned(),
        format!(
            "count checked: {}",
            threads_open.map(|n| n.to_string()).unwrap_or_default()
        ),
        if threads_open.is_none() {
            "thread count unavailable"
        } else {
            "standing-rejection status unavailable"
        }
        .to_owned(),
    ));
    let evidence_detail = match missing_evidence {
        Some(list) if !list.is_empty() => format!(
            "verdicts missing evidence: {}",
            py_repr(&Value::Array(list.clone()))
        ),
        _ => format!(
            "counted: {}",
            counted.map(|n| n.to_string()).unwrap_or_default()
        ),
    };
    checks.push((
        "5",
        if identity_exempt {
            Some(true)
        } else {
            match (counted, missing_evidence) {
                (Some(n), Some(list)) => Some(n > 0 && list.is_empty()),
                _ => None,
            }
        },
        format!("Each counted verdict cites command + exit code{exempt_suffix}"),
        evidence_detail,
        "verdict evidence facts unavailable".to_owned(),
    ));
    checks.push((
        "6",
        json_mark(facts.get("revertible")),
        "Revertible via plain `git revert` (no migration/external side effect)".to_owned(),
        String::new(),
        "needs a human classification".to_owned(),
    ));
    checks.push((
        "7",
        tripped.map(|t| !t),
        "circuit_breaker_override.toml: tripped = false".to_owned(),
        String::new(),
        "circuit breaker file not found or unparseable (fail closed)".to_owned(),
    ));
    let mut lines: Vec<String> = Vec::new();
    let mut first: Option<String> = None;
    let other_boxes: Vec<&str> = [
        ("2", is_other(Some(trial))),
        ("6", is_other(facts.get("revertible"))),
    ]
    .iter()
    .filter(|(_, other)| *other)
    .map(|(number, _)| *number)
    .collect();
    for (number, ok, label, detail, blocked_reason) in checks {
        let mark = mark_of(ok);
        let shown = if mark == Mark::Blocked && !other_boxes.contains(&number) {
            &blocked_reason
        } else {
            &detail
        };
        lines.push(render(mark, &format!("{number}. {label}"), shown));
        if mark != Mark::Checked && first.is_none() {
            first = Some(number.to_owned());
        }
    }
    Ok(Checklist {
        lines,
        would_merge: first.is_none(),
        first_blocker: first,
    })
}

/// One verdict: `decide`'s reasons are necessary, the checklist is sufficient; both must pass. Returns
/// `(eligible, first_blocker)`.
///
/// # Errors
/// As [`checklist`].
pub fn reconcile(decide_ok: bool, facts: &Value) -> Result<(bool, Option<String>), String> {
    let list = checklist(facts)?;
    Ok((decide_ok && list.would_merge, list.first_blocker))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub eligible: bool,
    pub reasons: Vec<String>,
    pub level: &'static str,
    pub commands: Vec<Vec<String>>,
}

/// The default decision, by condition letter: a (covered), b (trial merge), d (base is main).
#[must_use]
pub fn decide(covered: bool, paths: &[String], base_ref: &str, trial_merge_ok: bool) -> Decision {
    let mut reasons = Vec::new();
    if !covered {
        reasons.push(
            "a: not covered by a signed independent whole-diff accept at the current head"
                .to_owned(),
        );
    }
    if !trial_merge_ok {
        reasons.push("b: trial merge into the base conflicts".to_owned());
    }
    if base_ref != "main" {
        reasons.push(format!("d: base is {base_ref}, not main"));
    }
    let level = gate_plan_level(paths);
    let commands: Vec<Vec<String>> = if level == "light" {
        LIGHT_GATE
            .iter()
            .map(|c| c.iter().map(|s| (*s).to_owned()).collect())
            .collect()
    } else {
        FULL_GATE
            .iter()
            .map(|c| c.iter().map(|s| (*s).to_owned()).collect())
            .collect()
    };
    Decision {
        eligible: reasons.is_empty(),
        reasons,
        level,
        commands,
    }
}

#[cfg(test)]
mod tests {
    use super::{checklist, decide, reconcile};
    use serde_json::json;

    fn green() -> serde_json::Value {
        json!({"accept_seats": ["codex", "gemini"], "base_ref": "main", "base_tip": "t".repeat(40), "breaker_tripped": false,
               "category": "scripts/*.py", "ci": {"detail": "3 checks: SUCCESS", "state": "success"}, "counted_verdicts": 2,
               "covered": true, "default_branch": "main", "head": "h".repeat(40), "revertible": true,
               "is_draft": false, "standing_changes_requested": false, "threads_open": 0, "trial_merge_ok": true,
               "verdicts_missing_evidence": []})
    }

    #[test]
    fn all_green_merges_and_a_missing_fact_blocks_its_box() {
        let all = checklist(&green()).unwrap_or_else(|e| panic!("{e}"));
        assert!(all.would_merge);
        assert_eq!(all.lines.len(), 9);
        let mut facts = green();
        facts.as_object_mut().map(|m| m.remove("is_draft"));
        let blocked = checklist(&facts).unwrap_or_else(|e| panic!("{e}"));
        assert!(!blocked.would_merge);
        assert_eq!(blocked.first_blocker.as_deref(), Some("2b"));
    }

    #[test]
    fn mistyped_facts_block() {
        for (key, value) in [
            ("threads_open", json!(false)),
            ("breaker_tripped", json!(0)),
            ("accept_seats", json!("ab")),
            ("counted_verdicts", json!(true)),
            ("covered", json!(1)),
        ] {
            let mut facts = green();
            facts[key] = value;
            assert!(
                !checklist(&facts)
                    .unwrap_or_else(|e| panic!("{e}"))
                    .would_merge,
                "{key}"
            );
        }
    }

    #[test]
    fn a_missing_required_key_is_reported() {
        let mut facts = green();
        facts.as_object_mut().map(|m| m.remove("head"));
        assert_eq!(
            checklist(&facts),
            Err("missing required fact: head".to_owned())
        );
    }

    #[test]
    fn decide_reports_every_blocking_reason_and_reconcile_needs_both() {
        let paths = vec![".github/workflows/x.yml".to_owned()];
        let d = decide(false, &paths, "proposal/x", false);
        assert_eq!(d.reasons.len(), 3);
        assert!(!d.eligible);
        let ok = decide(true, &["docs/a.md".to_owned()], "main", true);
        assert!(ok.eligible && ok.level == "light" && ok.commands.len() == 2);
        assert_eq!(reconcile(true, &green()), Ok((true, None)));
        assert_eq!(reconcile(false, &green()), Ok((false, None)));
    }
}
