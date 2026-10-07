//! The ranked list of concrete work items derived from a state snapshot. Port of `next_task.work_items`, `item_key`
//! and `parse_time` (the first stage of the task-card dispatcher; its output is checked against the Python oracle by
//! the differential test, because the committed vectors only pin the finished cards).
//!
//! The snapshot is dynamic JSON and so are the items: both are `serde_json::Value`, read with Python's rules
//! (`dict.get` defaults, truthiness, `str()` in f-strings).

use crate::gate::truthy;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

const STALE_HOURS: f64 = 72.0;
const OBSERVABLE_KINDS: [&str; 4] = [
    "resolve_threads",
    "address_review",
    "identify_implementers",
    "not_mergeable",
];
const NEEDS_KNOWN_IMPLEMENTERS: [&str; 4] = [
    "fix_threads",
    "resolve_threads",
    "address_review",
    "needs_review",
];

/// Days since 1970-01-01 of a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Microseconds since the epoch of `YYYY-MM-DDTHH:MM:SS[.f...](Z|+HH:MM|-HH:MM)`, the forms the steward writes. Anything
/// else (a naive time, whose meaning Python takes from the machine's zone, or another ISO layout) is an error.
///
/// # Errors
/// Returns a message for a string that is not one of those forms.
pub fn parse_time(text: &str) -> Result<i128, String> {
    let bad = || format!("unsupported time {text:?}");
    let bytes: Vec<char> = text.chars().collect();
    let num = |from: usize, len: usize| -> Option<i64> {
        let slice: String = bytes.get(from..from + len)?.iter().collect();
        if slice.chars().all(|c| c.is_ascii_digit()) {
            slice.parse().ok()
        } else {
            None
        }
    };
    let (year, month, day) = (
        num(0, 4).ok_or_else(bad)?,
        num(5, 2).ok_or_else(bad)?,
        num(8, 2).ok_or_else(bad)?,
    );
    if bytes.get(4) != Some(&'-')
        || bytes.get(7) != Some(&'-')
        || bytes.get(10) != Some(&'T')
        || bytes.get(13) != Some(&':')
        || bytes.get(16) != Some(&':')
    {
        return Err(bad());
    }
    let (hour, minute, second) = (
        num(11, 2).ok_or_else(bad)?,
        num(14, 2).ok_or_else(bad)?,
        num(17, 2).ok_or_else(bad)?,
    );
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(bad());
    }
    let mut at = 19;
    let mut micros: i64 = 0;
    if bytes.get(at) == Some(&'.') {
        let mut digits = String::new();
        at += 1;
        while let Some(c) = bytes.get(at).filter(|c| c.is_ascii_digit()) {
            digits.push(*c);
            at += 1;
        }
        if digits.is_empty() || digits.len() > 6 {
            return Err(bad());
        }
        while digits.len() < 6 {
            digits.push('0');
        }
        micros = digits.parse().map_err(|_| bad())?;
    }
    let offset_minutes: i64 = match bytes.get(at) {
        Some('Z') if at + 1 == bytes.len() => 0,
        Some(sign @ ('+' | '-')) if at + 6 == bytes.len() && bytes.get(at + 3) == Some(&':') => {
            let (oh, om) = (
                num(at + 1, 2).ok_or_else(bad)?,
                num(at + 4, 2).ok_or_else(bad)?,
            );
            if oh > 23 || om > 59 {
                return Err(bad());
            }
            let total = oh * 60 + om;
            if *sign == '+' { total } else { -total }
        }
        _ => return Err(bad()),
    };
    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - offset_minutes * 60;
    Ok(i128::from(seconds) * 1_000_000 + i128::from(micros))
}

/// `f"{kind}:{pr}:{head[:8]}"`.
#[must_use]
pub fn item_key(kind: &str, pr: &Value, head: &str) -> String {
    format!(
        "{kind}:{}:{}",
        py_str(pr),
        head.chars().take(8).collect::<String>()
    )
}

/// Python's `str()` of a JSON value.
#[must_use]
pub fn py_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => crate::checklist::py_repr(other),
    }
}

fn get_or<'a>(map: &'a Value, key: &str, default: &'a Value) -> &'a Value {
    map.get(key).unwrap_or(default)
}

/// Ranked work items for the snapshot, most urgent first.
///
/// # Errors
/// Returns a message where the Python version would raise (a missing required key, an unparseable time).
pub fn work_items(state: &Value) -> Result<Vec<Value>, String> {
    let now = parse_time(
        state
            .get("now")
            .and_then(Value::as_str)
            .ok_or("missing now")?,
    )?;
    let done: BTreeSet<String> = state
        .get("done")
        .and_then(Value::as_array)
        .map(|l| {
            l.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let main = json!("main");
    let default_branch = get_or(state, "default_branch", &main).clone();
    let zero = json!(0);
    let empty_str = json!("");
    let mut items: Vec<Map<String, Value>> = Vec::new();
    for pr in state
        .get("prs")
        .and_then(Value::as_array)
        .ok_or("missing prs")?
    {
        let number = pr.get("number").ok_or("pr without number")?;
        let draft = truthy(get_or(pr, "draft", &Value::Bool(false)));
        let clean = pr.get("mergeable_state").and_then(Value::as_str) == Some("CLEAN");
        let threads = get_or(pr, "threads_unresolved", &zero).clone();
        let updated = parse_time(
            pr.get("updated_at")
                .and_then(Value::as_str)
                .ok_or("pr without updated_at")?,
        )?;
        let age_micros = now - updated;
        // timedelta.total_seconds() / 3600, with the same float operations
        let age_h = (age_micros as f64 / 1e6) / 3600.0;
        let seat = pr.get("seat").cloned().unwrap_or(Value::Null);
        let implementers_raw = pr.get("implementers").cloned().unwrap_or(Value::Null);
        let implementers: Vec<Value> = if truthy(&implementers_raw) {
            implementers_raw
                .as_array()
                .cloned()
                .ok_or("implementers is not a list")?
        } else if truthy(&seat) {
            vec![pr.get("seat").cloned().unwrap_or(Value::Null)]
        } else {
            Vec::new()
        };
        let rejected_all: Vec<Value> = pr
            .get("rejected_verdicts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let base_ref = pr
            .get("base")
            .cloned()
            .unwrap_or_else(|| default_branch.clone());
        let mut base = Map::new();
        base.insert("pr".into(), number.clone());
        base.insert("title".into(), get_or(pr, "title", &empty_str).clone());
        base.insert(
            "seat".into(),
            if pr.get("seat").is_some() {
                seat.clone()
            } else {
                json!("")
            },
        );
        base.insert("head".into(), get_or(pr, "head", &empty_str).clone());
        base.insert("implementers".into(), Value::Array(implementers.clone()));
        base.insert("base_ref".into(), base_ref.clone());
        base.insert(
            "base_sha".into(),
            get_or(pr, "base_sha", &empty_str).clone(),
        );
        base.insert("size".into(), get_or(pr, "size", &empty_str).clone());
        base.insert(
            "mergeable_state".into(),
            get_or(pr, "mergeable_state", &empty_str).clone(),
        );
        base.insert(
            "rejected_unattributed".into(),
            Value::Array(
                rejected_all
                    .iter()
                    .filter(|r| !truthy(r.get("seat").unwrap_or(&Value::Null)))
                    .cloned()
                    .collect(),
            ),
        );
        let unanswered = pr
            .get("threads_unanswered")
            .cloned()
            .unwrap_or_else(|| threads.clone());
        let chain_start = items.len();
        let covered = truthy(get_or(pr, "covered", &Value::Bool(false)));
        let item = |kind: &str, rank: i64, extra: Vec<(&str, Value)>| -> Map<String, Value> {
            let mut m = base.clone();
            m.insert("kind".into(), json!(kind));
            m.insert("rank".into(), json!(rank));
            for (k, v) in extra {
                m.insert(k.to_owned(), v);
            }
            m
        };
        if truthy(&threads) && truthy(&unanswered) {
            items.push(item(
                "fix_threads",
                1,
                vec![
                    ("owner_seat", pr.get("seat").cloned().unwrap_or(Value::Null)),
                    (
                        "detail",
                        json!(format!(
                            "{} of {} unresolved review thread(s) not yet answered",
                            py_str(&unanswered),
                            py_str(&threads)
                        )),
                    ),
                ],
            ));
        } else if truthy(&threads) {
            items.push(item(
                "resolve_threads",
                2,
                vec![
                    ("owner_seat", Value::Null),
                    ("detail", json!(format!("{} unresolved review thread(s), all answered; waiting for a reviewer to resolve", py_str(&threads)))),
                ],
            ));
        } else if truthy(pr.get("standing_blocker").unwrap_or(&Value::Null)) {
            let blocker = pr.get("standing_blocker").cloned().unwrap_or(Value::Null);
            let verdict = blocker.get("verdict").ok_or("blocker without verdict")?;
            let by = blocker.get("by").ok_or("blocker without by")?;
            items.push(item(
                "address_review",
                1,
                vec![
                    ("owner_seat", pr.get("seat").cloned().unwrap_or(Value::Null)),
                    ("blocker", blocker.clone()),
                    (
                        "detail",
                        json!(format!(
                            "{} by {} stands at the current head",
                            py_str(verdict),
                            py_str(by)
                        )),
                    ),
                ],
            ));
        } else if !covered {
            let where_ = if draft {
                " (a draft: flipping it ready waits until it is covered)"
            } else {
                ""
            };
            items.push(item(
                "needs_review",
                2,
                vec![(
                    "detail",
                    json!(format!(
                        "no covered whole-diff accept at the current head{where_}"
                    )),
                )],
            ));
        } else if draft {
            items.push(item(
                "draft",
                4,
                vec![(
                    "detail",
                    json!("draft: merge option hidden until ready for review"),
                )],
            ));
        } else if covered && clean && base_ref != default_branch {
            items.push(item(
                "stacked",
                6,
                vec![(
                    "detail",
                    json!(format!(
                        "covered and CLEAN but stacked on {}: lands after its base, not into {}",
                        py_str(pr.get("base").unwrap_or(&Value::Null)),
                        py_str(&default_branch)
                    )),
                )],
            ));
        } else if covered && clean {
            items.push(item(
                "ready_to_merge",
                3,
                vec![("detail", json!("covered, CLEAN, no unresolved threads"))],
            ));
        } else if !clean {
            items.push(item(
                "not_mergeable",
                2,
                vec![(
                    "detail",
                    json!(format!(
                        "mergeable_state={}",
                        py_str(pr.get("mergeable_state").unwrap_or(&Value::Null))
                    )),
                )],
            ));
        }
        if implementers.is_empty() {
            let chain: Vec<Map<String, Value>> = items.split_off(chain_start);
            if chain
                .iter()
                .any(|i| NEEDS_KNOWN_IMPLEMENTERS.contains(&i["kind"].as_str().unwrap_or_default()))
            {
                items.extend(chain.into_iter().filter(|i| {
                    !NEEDS_KNOWN_IMPLEMENTERS.contains(&i["kind"].as_str().unwrap_or_default())
                }));
                items.push(item(
                    "identify_implementers",
                    1,
                    vec![
                        ("owner_seat", Value::Null),
                        (
                            "detail",
                            json!("implementers unknown (no seat trailer, no seat-named branch): author-only and independence-dependent cards are withheld until they are known"),
                        ),
                    ],
                ));
            } else {
                items.extend(chain);
            }
        }
        let mut known: Vec<Value> = implementers.clone();
        known.push(pr.get("seat").cloned().unwrap_or(Value::Null));
        let attributed: BTreeSet<String> = rejected_all
            .iter()
            .filter_map(|r| {
                r.get("seat")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
            })
            .filter(|s| !known.iter().any(|k| k.as_str() == Some(s.as_str())))
            .collect();
        if !attributed.is_empty() && !covered {
            let seat_name = attributed.iter().next().cloned().unwrap_or_default();
            let mine: Vec<Value> = rejected_all
                .iter()
                .filter(|r| r.get("seat").and_then(Value::as_str) == Some(seat_name.as_str()))
                .cloned()
                .collect();
            items.push(item(
                "repair_verdict",
                1,
                vec![
                    ("owner_seat", json!(seat_name)),
                    ("rejected", Value::Array(mine.clone())),
                    (
                        "detail",
                        json!(format!(
                            "{} verdict comment(s) by {} were rejected, so the PR is not covered",
                            mine.len(),
                            seat_name
                        )),
                    ),
                ],
            ));
        }
        if age_h > STALE_HOURS {
            items.push(item(
                "stale",
                5,
                vec![(
                    "detail",
                    json!(format!("no update for {}h", age_h.trunc() as i64)),
                )],
            ));
        }
    }
    let mut kept: Vec<Map<String, Value>> = items
        .into_iter()
        .filter(|i| {
            let kind = i["kind"].as_str().unwrap_or_default();
            OBSERVABLE_KINDS.contains(&kind)
                || !done.contains(&item_key(
                    kind,
                    &i["pr"],
                    i["head"].as_str().unwrap_or_default(),
                ))
        })
        .collect();
    let primary: Vec<Value> = kept
        .iter()
        .filter(|i| {
            matches!(
                i["kind"].as_str(),
                Some("ready_to_merge" | "draft" | "stacked")
            )
        })
        .map(|i| i["pr"].clone())
        .collect();
    kept.retain(|i| !(i["kind"] == "stale" && primary.contains(&i["pr"])));
    kept.sort_by(|a, b| {
        let rank = a["rank"].as_i64().cmp(&b["rank"].as_i64());
        rank.then_with(|| a["pr"].as_i64().cmp(&b["pr"].as_i64()))
    });
    Ok(kept.into_iter().map(Value::Object).collect())
}

#[cfg(test)]
mod tests {
    use super::{item_key, parse_time, work_items};
    use serde_json::json;

    #[test]
    fn times_parse_with_z_offsets_and_fractions() {
        let z = parse_time("2026-10-05T00:00:00Z").unwrap_or_default();
        assert_eq!(
            parse_time("2026-10-05T02:00:00+02:00").unwrap_or_default(),
            z
        );
        assert_eq!(
            parse_time("2026-10-04T22:00:00-02:00").unwrap_or_default(),
            z
        );
        assert_eq!(
            parse_time("2026-10-05T00:00:00.5Z").unwrap_or_default(),
            z + 500_000
        );
        assert!(
            parse_time("2026-10-05T00:00:00").is_err(),
            "naive times are machine-dependent in Python"
        );
        assert!(parse_time("garbage").is_err());
    }

    #[test]
    fn an_uncovered_pr_needs_a_review_and_a_done_key_hides_it() {
        let state = json!({"now": "2026-10-05T00:00:00Z", "default_branch": "main", "done": [],
            "prs": [{"number": 7, "head": "c".repeat(40), "seat": "claude", "updated_at": "2026-10-05T00:00:00Z",
                     "mergeable_state": "CLEAN", "covered": false}]});
        let items = work_items(&state).unwrap_or_default();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["kind"], "needs_review");
        let mut done = state.clone();
        done["done"] = json!([item_key("needs_review", &json!(7), &"c".repeat(40))]);
        assert!(work_items(&done).unwrap_or_default().is_empty());
    }

    #[test]
    fn unknown_implementers_withhold_independence_cards() {
        let state = json!({"now": "2026-10-05T00:00:00Z",
            "prs": [{"number": 7, "head": "c".repeat(40), "updated_at": "2026-10-05T00:00:00Z", "mergeable_state": "CLEAN"}]});
        let items = work_items(&state).unwrap_or_default();
        assert_eq!(
            items
                .iter()
                .map(|i| i["kind"].as_str().unwrap_or_default())
                .collect::<Vec<_>>(),
            ["identify_implementers"]
        );
    }
}
