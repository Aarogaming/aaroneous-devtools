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
const PENDING: [&str; 0] = [];
/// The rule families that have a dedicated function in the Python steward (everything else is a scenario file).
const RULE_FAMILIES: [&str; 16] = [
    "attribution",
    "heads",
    "receipts",
    "rejected",
    "stalls",
    "pushes",
    "seats",
    "coverage",
    "independence",
    "audit",
    "gate_plan",
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
        "independence" => Some(check_independence()),
        "chain" => Some(check_chain()),
        "gate_checklist" => Some(check_gate_checklist()),
        "gate_decide" => Some(check_gate_decide()),
        "audit" => Some(check_audit()),
        "gate_plan" => Some(check_gate_plan()),
        "gate_ci" => Some(check_gate_ci()),
        "gate_breaker" => Some(check_gate_breaker()),
        "gate_threads" => Some(check_gate_threads()),
        "pushes" => Some(check_pushes()),
        _ => None,
    }
}

/// A scenario file is `{tick, state}` -> the cards `build_cards` issues.
fn check_scenario(stem: &str) -> Result<(), String> {
    let input = load(&format!("{stem}.in.json"));
    let tick = input["tick"].as_i64().ok_or("scenario without tick")?;
    let cards = steward::cards::build_cards(&input["state"], tick)?;
    compare(&Value::Array(cards), &load(&format!("{stem}.out.json")))
}

/// The healthy state with a known newest merge, for each tick: the periodic audit is issued on every 6th tick only.
fn check_audit() -> Result<(), String> {
    let input = load("audit.in.json");
    let mut actual = Vec::new();
    for tick in input["ticks"].as_array().ok_or("no ticks")? {
        let t = tick.as_i64().ok_or("tick is not an integer")?;
        let cards = steward::cards::build_cards(&input["state"], t)?;
        let tasks: Vec<Value> = cards.iter().map(|c| c["task"].clone()).collect();
        actual.push(json!({"tick": t, "tasks": tasks}));
    }
    compare(&Value::Array(actual), &load("audit.out.json"))
}

/// A family file is checked by its family function; a scenario file is checked on its own.
fn check_stem(stem: &str) -> Option<Result<(), String>> {
    let key = family_key(stem);
    if key == SCENARIO_MARKER {
        Some(check_scenario(stem))
    } else {
        check(&key)
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

fn check_chain() -> Result<(), String> {
    use steward::chain::build_log;
    let events: Vec<(String, Value, String)> = load("chain.in.json")
        .as_array()
        .ok_or("chain.in.json is not a list")?
        .iter()
        .map(|e| {
            (
                e["type"].as_str().unwrap_or_default().to_owned(),
                e["data"].clone(),
                e["now"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    let (lines, tip) = build_log(&events);
    compare(
        &json!({"lines": lines, "tip": tip}),
        &load("chain.out.json"),
    )
}

fn check_gate_checklist() -> Result<(), String> {
    use steward::checklist::checklist;
    let input = load("gate_checklist.in.json");
    let mut actual = Vec::new();
    for case in input["cases"].as_array().ok_or("no cases")? {
        let mut facts = input["green"].clone();
        if let (Some(target), Some(over)) = (facts.as_object_mut(), case["override"].as_object()) {
            for (k, v) in over {
                target.insert(k.clone(), v.clone());
            }
        }
        let list = checklist(&facts)?;
        actual.push(json!({"lines": list.lines, "would_merge": list.would_merge, "first_blocker": list.first_blocker}));
    }
    compare(&Value::Array(actual), &load("gate_checklist.out.json"))
}

fn check_gate_decide() -> Result<(), String> {
    use steward::checklist::{decide, reconcile};
    use steward::gate::truthy;
    let green = load("gate_checklist.in.json")["green"].clone();
    let mut actual = Vec::new();
    for case in load("gate_decide.in.json").as_array().ok_or("not a list")? {
        let d = decide(
            truthy(&case["row"]["covered"]),
            &string_list(&case["paths"]),
            case["base"].as_str().unwrap_or_default(),
            truthy(&case["trial"]),
        );
        let (eligible, first) = reconcile(d.eligible, &green)?;
        actual.push(json!({
            "decide": {"eligible": d.eligible, "reasons": d.reasons, "plan": {"level": d.level, "commands": d.commands}},
            "reconciled_with_all_green_checklist": {"eligible": eligible, "first_blocker": first},
        }));
    }
    compare(&Value::Array(actual), &load("gate_decide.out.json"))
}

fn check_gate_plan() -> Result<(), String> {
    use steward::gate::{category_of, gate_plan_level};
    let mut actual = Vec::new();
    for case in load("gate_plan.in.json").as_array().ok_or("not a list")? {
        let paths = string_list(&case["paths"]);
        actual.push(json!({
            "plan_level": gate_plan_level(&paths),
            "category": category_of(&paths),
        }));
    }
    compare(&Value::Array(actual), &load("gate_plan.out.json"))
}

fn check_gate_ci() -> Result<(), String> {
    use steward::gate::{CiCheck, ci_state};
    let mut actual = Vec::new();
    for case in load("gate_ci.in.json").as_array().ok_or("not a list")? {
        let rollup: Vec<CiCheck> = case["rollup"]
            .as_array()
            .map(|list| list.iter().map(CiCheck::from_json).collect())
            .unwrap_or_default();
        let state = ci_state(&rollup);
        actual.push(json!({"state": state.state, "detail": state.detail}));
    }
    compare(&Value::Array(actual), &load("gate_ci.out.json"))
}

fn check_gate_breaker() -> Result<(), String> {
    use steward::gate::breaker_state;
    let mut actual = Vec::new();
    for case in load("gate_breaker.in.json")
        .as_array()
        .ok_or("not a list")?
    {
        actual.push(json!(breaker_state(
            case["text"].as_str().unwrap_or_default()
        )));
    }
    compare(&Value::Array(actual), &load("gate_breaker.out.json"))
}

fn check_gate_threads() -> Result<(), String> {
    use steward::gate::open_threads;
    let mut actual = Vec::new();
    for case in load("gate_threads.in.json")
        .as_array()
        .ok_or("not a list")?
    {
        actual.push(json!(open_threads(&case["data"])));
    }
    compare(&Value::Array(actual), &load("gate_threads.out.json"))
}

fn check_independence() -> Result<(), String> {
    let mut actual = Vec::new();
    for case in load("independence.in.json")
        .as_array()
        .ok_or("independence.in.json is not a list")?
    {
        let counted = case["counted"].as_array().cloned().unwrap_or_default();
        actual.push(steward::independence::independence(&counted, &case["identities"]).to_value());
    }
    compare(&Value::Array(actual), &load("independence.out.json"))
}

fn check_coverage() -> Result<(), String> {
    use std::collections::BTreeSet;
    use steward::coverage::Record;
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
        let registered: BTreeSet<String> = string_list(&case["registered"]).into_iter().collect();
        let decision = steward::coverage::evaluate_in(
            case["head"].as_str().unwrap_or_default(),
            case["merge_base"].as_str(),
            &records,
            &is_ancestor,
            &registered,
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

/// The fewest cases each ported family may hold. A runner that compares two empty lists passes, so emptying or
/// truncating a family's vectors (input and output together) used to leave this test green (skeptic probe: seats,
/// coverage and chain all emptied, 3 tests passed). The floors only rise: add a family here when you port it, and raise
/// a number when vectors are added.
const MIN_CASES: [(&str, usize); 17] = [
    ("seats", 23),
    ("heads", 12),
    ("attribution", 34),
    ("rejected", 7),
    ("receipts", 19),
    ("stalls", 9),
    ("pushes", 7),
    ("coverage", 21),
    ("independence", 20),
    ("gate_plan", 13),
    ("gate_ci", 7),
    ("gate_breaker", 7),
    ("gate_threads", 5),
    // Lowered from 28 and 7 by main's #134 and #106: the tier, owner-path and trust cases were deleted with the
    // features they tested, so the families shrank on purpose; the floor now matches what remains.
    ("gate_checklist", 18),
    ("gate_decide", 6),
    ("chain", 3),
    ("audit", 14),
];

/// The fewest scenario files (`{tick, state}` -> cards) the contract may hold.
const MIN_SCENARIOS: usize = 18;

fn case_count(stem: &str) -> usize {
    let input = load(&format!("{stem}.in.json"));
    input["cases"]
        .as_array()
        .or_else(|| input["ticks"].as_array())
        .or_else(|| input.as_array())
        .map_or(0, Vec::len)
}

#[test]
fn no_ported_family_has_fewer_vectors_than_its_floor_and_every_one_has_a_floor() {
    let floors: std::collections::BTreeMap<&str, usize> = MIN_CASES.iter().copied().collect();
    let mut problems = Vec::new();
    for stem in families_on_disk() {
        if family_key(&stem) == SCENARIO_MARKER || check(&family_key(&stem)).is_none() {
            continue;
        }
        match floors.get(stem.as_str()) {
            None => problems.push(format!("{stem} is ported but has no entry in MIN_CASES")),
            Some(&floor) => {
                let count = case_count(&stem);
                if count < floor {
                    problems.push(format!("{stem} has {count} vector cases, floor is {floor}"));
                }
            }
        }
    }
    for (name, _) in MIN_CASES {
        if check(name).is_none() {
            problems.push(format!("{name} has a floor but is not ported"));
        }
    }
    let scenarios = families_on_disk()
        .iter()
        .filter(|s| family_key(s) == SCENARIO_MARKER)
        .count();
    if scenarios < MIN_SCENARIOS {
        problems.push(format!(
            "{scenarios} scenario files, floor is {MIN_SCENARIOS}"
        ));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_ported_family_matches_its_vectors_exactly() {
    let mut failures = Vec::new();
    let mut ported = Vec::new();
    for stem in families_on_disk() {
        if let Some(result) = check_stem(&stem) {
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
        let ported = key == SCENARIO_MARKER || check(key).is_some();
        let listed = pending.contains(key.as_str());
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
    let ported = stems.iter().filter(|s| check_stem(s).is_some()).count();
    println!(
        "steward port: {ported} of {} vector files ported",
        stems.len()
    );
}
