//! The committed golden vectors (`scripts/golden/steward/`) are the contract this crate has to meet.
//!
//! Every `*.in.json` family must be either ported (and then matches its `*.out.json` exactly) or listed in `PENDING`.
//! A family that is neither fails the test, so a vector added to the contract cannot be ignored by accident, and a
//! family listed as pending that no longer exists fails too. Porting a family means moving it from `PENDING` to
//! `check`; the pending list only ever shrinks.

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../scripts/golden/steward")
}

fn load(name: &str) -> Value {
    let path = vectors_dir().join(name);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

/// Vector families whose scenarios are `{tick, state}` -> cards (`next_task.build_cards`); each file is its own family.
const SCENARIO_MARKER: &str = "(scenario)";

/// Not ported yet. Remove an entry when its family is ported and listed in `check`.
const PENDING: [&str; 8] = [
    "audit",
    "chain",
    "gate_breaker",
    "gate_checklist",
    "gate_ci",
    "gate_decide",
    "gate_threads",
    "gate_tier",
];
/// The rule families that have a dedicated function in the Python steward (everything else is a scenario file).
const RULE_FAMILIES: [&str; 15] = [
    "attribution",
    "heads",
    "receipts",
    "rejected",
    "stalls",
    "pushes",
    "seats",
    "coverage",
    "audit",
    "gate_tier",
    "gate_ci",
    "gate_breaker",
    "gate_threads",
    "gate_checklist",
    "gate_decide",
];

fn check(family: &str) -> Option<Result<(), String>> {
    match family {
        "seats" => Some(check_seats()),
        "heads" => Some(check_heads()),
        "attribution" => Some(check_attribution()),
        "rejected" => Some(check_rejected()),
        "receipts" => Some(check_receipts()),
        "stalls" => Some(check_stalls()),
        "coverage" => Some(check_coverage()),
        "pushes" => Some(check_pushes()),
        _ => None,
    }
}

fn check_seats() -> Result<(), String> {
    let inputs = load("seats.in.json");
    let expected = load("seats.out.json");
    let actual: Vec<Value> = inputs
        .as_array()
        .ok_or("seats.in.json is not a list")?
        .iter()
        .map(|s| json!(steward::seats::seat_names(s.as_str().unwrap_or_default())))
        .collect();
    compare(&Value::Array(actual), &expected)
}

fn check_heads() -> Result<(), String> {
    let lexicon = steward::text::Lexicon::new().map_err(|e| e.to_string())?;
    let inputs = load("heads.in.json");
    let expected = load("heads.out.json");
    let actual: Vec<Value> = inputs
        .as_array()
        .ok_or("heads.in.json is not a list")?
        .iter()
        .map(|case| {
            let body = case["body"].as_str().unwrap_or_default();
            let head = case["head"].as_str().unwrap_or_default();
            json!(steward::heads::cites_head(&lexicon, body, head))
        })
        .collect();
    compare(&Value::Array(actual), &expected)
}

fn check_attribution() -> Result<(), String> {
    let attribution = steward::attribution::Attribution::new().map_err(|e| e.to_string())?;
    let inputs = load("attribution.in.json");
    let actual: Vec<Value> = inputs
        .as_array()
        .ok_or("attribution.in.json is not a list")?
        .iter()
        .map(|case| {
            let body = case["body"].as_str().unwrap_or_default();
            json!(attribution.comment_seat(body, case["strict"].as_bool().unwrap_or(false)))
        })
        .collect();
    compare(&Value::Array(actual), &load("attribution.out.json"))
}

fn check_rejected() -> Result<(), String> {
    use steward::rejected::{Comment, Malformed, Rules};
    let rules = Rules::new().map_err(|e| e.to_string())?;
    let inputs = load("rejected.in.json");
    let mut actual = Vec::new();
    for case in inputs.as_array().ok_or("rejected.in.json is not a list")? {
        let comments: Vec<Comment> = case["pr"]["comments"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|c| Comment {
                        id: c["id"].as_i64(),
                        body: c["body"].as_str().unwrap_or_default().to_owned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let head = case["pr"]["head"].as_str().unwrap_or_default();
        let malformed: Vec<Malformed> = case["row"]["malformed"]
            .as_array()
            .map(|list| {
                list.iter()
                    .filter(|p| p.get("comment").is_some())
                    .map(|p| Malformed {
                        comment: p["comment"].as_i64(),
                        problem: p["problem"].as_str().unwrap_or_default().to_owned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let legacy: Vec<Option<i64>> = case["row"]["legacy"]
            .as_array()
            .map(|list| list.iter().map(|p| p["comment"].as_i64()).collect())
            .unwrap_or_default();
        let kept = rules.rejected_verdicts(&comments, head, &malformed, &legacy, None);
        actual.push(Value::Array(
            kept.iter()
                .map(|r| json!({"comment_id": r.comment_id, "seat": r.seat, "reason": r.reason}))
                .collect(),
        ));
    }
    compare(&Value::Array(actual), &load("rejected.out.json"))
}

fn check_coverage() -> Result<(), String> {
    use std::collections::BTreeSet;
    use steward::coverage::{Record, evaluate};
    let mut actual = Vec::new();
    for case in load("coverage.in.json")
        .as_array()
        .ok_or("coverage.in.json is not a list")?
    {
        let originals: Vec<&Value> = case["records"]
            .as_array()
            .map(|l| l.iter().collect())
            .unwrap_or_default();
        let records: Vec<Record> = originals
            .iter()
            .map(|r| Record {
                head: r["head"].as_str().unwrap_or_default().to_owned(),
                base_sha: r["base_sha"].as_str().unwrap_or_default().to_owned(),
                scope: r["scope"].as_str().unwrap_or_default().to_owned(),
                reviewer: r["reviewer"].as_str().unwrap_or_default().to_owned(),
                implementers: string_list(&r["implementers"]),
                verdict: r["verdict"].as_str().unwrap_or_default().to_owned(),
                from: r["from"].as_str().map(str::to_owned),
            })
            .collect();
        let pairs: BTreeSet<(String, String)> = case["ancestry"]
            .as_array()
            .map(|l| {
                l.iter()
                    .map(|p| {
                        (
                            p[0].as_str().unwrap_or_default().to_owned(),
                            p[1].as_str().unwrap_or_default().to_owned(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let is_ancestor =
            |a: &str, b: &str| a == b || pairs.contains(&(a.to_owned(), b.to_owned()));
        let decision = evaluate(
            case["head"].as_str().unwrap_or_default(),
            case["merge_base"].as_str(),
            &records,
            &is_ancestor,
        );
        actual.push(json!({
            "covered": decision.covered,
            "reasons": decision.reasons,
            "accepts_at_head": decision.accepts_at_head,
            "counted": decision.counted.iter().map(|&i| originals[i].clone()).collect::<Vec<_>>(),
            "accept_seats": decision.accept_seats,
        }));
    }
    compare(&Value::Array(actual), &load("coverage.out.json"))
}

fn check_stalls() -> Result<(), String> {
    use std::collections::BTreeMap;
    use steward::engagement::{Event, attribution_weak, stalled_seats};
    let counts = |value: &Value| -> BTreeMap<String, i64> {
        value
            .as_object()
            .map(|map| {
                map.iter()
                    .map(|(k, v)| (k.clone(), v.as_i64().unwrap_or_default()))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut actual = Vec::new();
    for case in load("stalls.in.json")
        .as_array()
        .ok_or("stalls.in.json is not a list")?
    {
        let events: Vec<Event> = case["events"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|e| Event {
                        kind: e["type"].as_str().unwrap_or_default().to_owned(),
                        tick: e["data"]["tick"].as_i64(),
                        agents: string_list(&e["data"]["agents"]),
                        per_seat: counts(&e["data"]["per_seat"]),
                        pushes: counts(&e["data"]["pushes"]),
                        unattributed: e["data"]["unattributed"].as_i64().unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let stalled = stalled_seats(
            &events,
            &string_list(&case["agents"]),
            case["stall_after"].as_i64().unwrap_or_default(),
        );
        let weak = attribution_weak(&events, case["window"].as_i64().unwrap_or_default(), 3);
        actual.push(json!({"stalled": stalled, "attribution_weak": weak}));
    }
    compare(&Value::Array(actual), &load("stalls.out.json"))
}

fn check_pushes() -> Result<(), String> {
    use std::collections::BTreeMap;
    use steward::engagement::{PushPr, pushes};
    let mut actual = Vec::new();
    for case in load("pushes.in.json")
        .as_array()
        .ok_or("pushes.in.json is not a list")?
    {
        let prs: Vec<PushPr> = case["prs"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|p| PushPr {
                        number: p["number"].as_i64().unwrap_or_default(),
                        head: p["head"].as_str().unwrap_or_default().to_owned(),
                        seat: p["seat"].as_str().unwrap_or_default().to_owned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let previous: Option<BTreeMap<String, String>> = case["previous"].as_object().map(|map| {
            map.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                .collect()
        });
        let (heads, credited) = pushes(&prs, previous.as_ref());
        actual.push(json!({"heads": heads, "credited": credited}));
    }
    compare(&Value::Array(actual), &load("pushes.out.json"))
}

fn string_list(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn check_receipts() -> Result<(), String> {
    use std::collections::BTreeMap;
    use steward::receipts::{Pr, Receipts};
    use steward::rejected::Comment;
    let receipts = Receipts::new().map_err(|e| e.to_string())?;
    let by_number = |value: &Value| -> BTreeMap<i64, Vec<String>> {
        value
            .as_object()
            .map(|map| {
                map.iter()
                    .filter_map(|(k, v)| k.parse().ok().map(|n| (n, string_list(v))))
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut actual = Vec::new();
    for case in load("receipts.in.json")
        .as_array()
        .ok_or("receipts.in.json is not a list")?
    {
        let prs: Vec<Pr> = case["snapshot"]["prs"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|p| Pr {
                        number: p["number"].as_i64().unwrap_or_default(),
                        head: p["head"].as_str().unwrap_or_default().to_owned(),
                        comments: p["comments"]
                            .as_array()
                            .map(|c| {
                                c.iter()
                                    .map(|c| Comment {
                                        id: c["id"].as_i64(),
                                        body: c["body"].as_str().unwrap_or_default().to_owned(),
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                        implementers: string_list(&p["implementers"]),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let done = receipts.done_receipts(
            &prs,
            &by_number(&case["repair_owners"]),
            &by_number(&case["implementers_by_pr"]),
        );
        actual.push(json!(done));
    }
    compare(&Value::Array(actual), &load("receipts.out.json"))
}

fn compare(actual: &Value, expected: &Value) -> Result<(), String> {
    if actual == expected {
        return Ok(());
    }
    let (a, e) = (actual.as_array(), expected.as_array());
    if let (Some(a), Some(e)) = (a, e) {
        for (i, (x, y)) in a.iter().zip(e).enumerate() {
            if x != y {
                return Err(format!("case {i}: got {x}, vector says {y}"));
            }
        }
        if a.len() != e.len() {
            return Err(format!("got {} results, vector has {}", a.len(), e.len()));
        }
    }
    Err("results differ".to_owned())
}

fn families_on_disk() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for entry in fs::read_dir(vectors_dir()).expect("vectors directory") {
        let name = entry
            .expect("directory entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        if let Some(stem) = name.strip_suffix(".in.json") {
            found.insert(stem.to_owned());
        }
    }
    found
}

/// A rule family is named in `RULE_FAMILIES` (or is `chain`); a scenario is any other file shaped `{tick, state}`.
/// A file that is neither keeps its own name, so the skip test reports it instead of filing it under scenarios.
fn family_key(stem: &str) -> String {
    if RULE_FAMILIES.contains(&stem) || stem == "chain" {
        return stem.to_owned();
    }
    let input = load(&format!("{stem}.in.json"));
    if input.get("tick").is_some() && input.get("state").is_some() {
        SCENARIO_MARKER.to_owned()
    } else {
        stem.to_owned()
    }
}

#[test]
fn every_ported_family_matches_its_vectors_exactly() {
    let mut failures = Vec::new();
    let mut ported = Vec::new();
    for stem in families_on_disk() {
        if let Some(result) = check(&family_key(&stem)) {
            ported.push(stem.clone());
            if let Err(why) = result {
                failures.push(format!("{stem}: {why}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "vector mismatches:\n{}",
        failures.join("\n")
    );
    assert!(
        !ported.is_empty(),
        "no family is ported: the runner would pass vacuously"
    );
}

#[test]
fn no_vector_family_is_silently_skipped() {
    let pending: BTreeSet<&str> = PENDING.iter().copied().collect();
    let mut keys: BTreeSet<String> = families_on_disk().iter().map(|s| family_key(s)).collect();
    keys.insert(SCENARIO_MARKER.to_owned());
    let mut problems = Vec::new();
    for key in &keys {
        let ported = check(key).is_some();
        let listed = pending.contains(key.as_str()) || key == SCENARIO_MARKER;
        if ported && pending.contains(key.as_str()) {
            problems.push(format!("{key} is ported but still listed as pending"));
        }
        if !ported && !listed {
            problems.push(format!("{key} is neither ported nor listed as pending"));
        }
    }
    for name in &pending {
        if !keys.contains(*name) {
            problems.push(format!(
                "{name} is listed as pending but no such vector family exists"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn report_how_much_of_the_contract_is_ported() {
    let stems = families_on_disk();
    let ported = stems
        .iter()
        .filter(|s| check(&family_key(s)).is_some())
        .count();
    println!(
        "steward port: {ported} of {} vector files ported",
        stems.len()
    );
}
