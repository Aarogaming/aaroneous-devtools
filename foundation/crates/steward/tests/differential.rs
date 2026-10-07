// Differential test (ignored by default): Rust against the Python oracle on generated adversarial text. See scripts/differential_cases.py.
use serde_json::Value;
use std::collections::BTreeMap;
use steward::receipts::{Pr, Receipts};
use steward::rejected::Comment;

#[test]
#[ignore]
fn differential() {
    let path = std::env::var("STEWARD_DIFF_FILE").expect("STEWARD_DIFF_FILE");
    let all: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases: Vec<Value> = all["text"].as_array().unwrap().clone();
    let attribution = steward::attribution::Attribution::new().unwrap();
    let receipts = Receipts::new().unwrap();
    let lexicon = steward::text::Lexicon::new().unwrap();
    let head = "c".repeat(40);
    let mut bad: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let strings = |v: &Value| -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect()
    };
    for case in &cases {
        let body = case["body"].as_str().unwrap();
        let out = &case["out"];
        let seats: Vec<String> = steward::seats::seat_names(body).into_iter().collect();
        if seats != strings(&out["seats"]) {
            bad.entry("seats")
                .or_default()
                .push(format!("{body:?} rust={seats:?} py={}", out["seats"]));
        }
        let heads = steward::heads::cites_head(&lexicon, body, &head);
        if heads != out["heads"].as_bool().unwrap() {
            bad.entry("heads")
                .or_default()
                .push(format!("{body:?} rust={heads} py={}", out["heads"]));
        }
        for (key, strict) in [("attr_strict", true), ("attr_loose", false)] {
            let got = attribution.comment_seat(body, strict);
            if got != out[key].as_str().unwrap() {
                bad.entry(key)
                    .or_default()
                    .push(format!("{body:?} rust={got:?} py={}", out[key]));
            }
        }
        let pr = Pr {
            number: 7,
            head: head.clone(),
            comments: vec![Comment {
                id: Some(1),
                body: body.to_owned(),
            }],
            implementers: Vec::new(),
        };
        let by_pr = BTreeMap::from([(7, strings(&case["implementers"]))]);
        let owners = BTreeMap::from([(7, strings(&case["owners"]))]);
        let got = receipts.done_receipts(&[pr], &owners, &by_pr);
        let want: BTreeMap<String, String> = out["receipts"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_owned()))
            .collect();
        if got != want {
            bad.entry("receipts")
                .or_default()
                .push(format!("{body:?} rust={got:?} py={want:?}"));
        }
    }
    state_cases(&all, &mut bad);
    for (family, list) in &bad {
        println!("{family}: {} disagreements; first: {}", list.len(), list[0]);
    }
    println!(
        "compared {} cases; families with disagreements: {}",
        cases.len(),
        bad.len()
    );
    assert!(bad.is_empty());
}

fn counts(value: &Value) -> BTreeMap<String, i64> {
    value
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| (k.clone(), v.as_i64().unwrap_or_default()))
                .collect()
        })
        .unwrap_or_default()
}

fn state_cases(all: &Value, bad: &mut BTreeMap<&'static str, Vec<String>>) {
    use steward::engagement::{Event, PushPr, attribution_weak, pushes, stalled_seats};
    let strings = |v: &Value| -> Vec<String> {
        v.as_array()
            .map(|l| {
                l.iter()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    for case in all["stalls"].as_array().unwrap() {
        let events: Vec<Event> = case["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| Event {
                kind: e["type"].as_str().unwrap().to_owned(),
                tick: e["data"]["tick"].as_i64(),
                agents: strings(&e["data"]["agents"]),
                per_seat: counts(&e["data"]["per_seat"]),
                pushes: counts(&e["data"]["pushes"]),
                unattributed: e["data"]["unattributed"].as_i64().unwrap_or_default(),
            })
            .collect();
        let stalled = stalled_seats(
            &events,
            &strings(&case["agents"]),
            case["stall_after"].as_i64().unwrap(),
        );
        if stalled != strings(&case["out"]["stalled"]) {
            bad.entry("stalled")
                .or_default()
                .push(format!("{case} rust={stalled:?}"));
        }
        let weak = attribution_weak(&events, case["window"].as_i64().unwrap(), 3);
        if weak != case["out"]["weak"].as_bool().unwrap() {
            bad.entry("weak")
                .or_default()
                .push(format!("{case} rust={weak}"));
        }
    }
    coverage_cases(all, bad);
    gate_cases(all, bad);
    for case in all["pushes"].as_array().unwrap() {
        let prs: Vec<PushPr> = case["prs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| PushPr {
                number: p["number"].as_i64().unwrap(),
                head: p["head"].as_str().unwrap().to_owned(),
                seat: p["seat"].as_str().unwrap().to_owned(),
            })
            .collect();
        let previous: Option<BTreeMap<String, String>> = case["previous"].as_object().map(|m| {
            m.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_owned()))
                .collect()
        });
        let (heads, credited) = pushes(&prs, previous.as_ref());
        let want_heads: BTreeMap<String, String> = case["out"]["heads"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_owned()))
            .collect();
        if heads != want_heads || credited != counts(&case["out"]["credited"]) {
            bad.entry("pushes")
                .or_default()
                .push(format!("{case} rust={heads:?} {credited:?}"));
        }
    }
}

fn coverage_cases(all: &Value, bad: &mut BTreeMap<&'static str, Vec<String>>) {
    use std::collections::BTreeSet;
    use steward::coverage::{Record, evaluate};
    let strings = |v: &Value| -> Vec<String> {
        v.as_array()
            .map(|l| {
                l.iter()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    for case in all["coverage"].as_array().unwrap() {
        let originals: Vec<&Value> = case["records"].as_array().unwrap().iter().collect();
        let records: Vec<Record> = originals
            .iter()
            .map(|r| Record {
                head: r["head"].as_str().unwrap().to_owned(),
                base_sha: r["base_sha"].as_str().unwrap().to_owned(),
                scope: r["scope"].as_str().unwrap().to_owned(),
                reviewer: r["reviewer"].as_str().unwrap().to_owned(),
                implementers: strings(&r["implementers"]),
                verdict: r["verdict"].as_str().unwrap().to_owned(),
                from: r["from"].as_str().map(str::to_owned),
            })
            .collect();
        let pairs: BTreeSet<(String, String)> = case["ancestry"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p[0].as_str().unwrap().to_owned(),
                    p[1].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let is_ancestor =
            |a: &str, b: &str| a == b || pairs.contains(&(a.to_owned(), b.to_owned()));
        let d = evaluate(
            case["head"].as_str().unwrap(),
            case["merge_base"].as_str(),
            &records,
            &is_ancestor,
        );
        let want = &case["out"];
        let counted: Vec<Value> = d.counted.iter().map(|&i| originals[i].clone()).collect();
        let same = d.covered == want["covered"].as_bool().unwrap()
            && d.reasons == strings(&want["reasons"])
            && d.accepts_at_head as u64 == want["accepts_at_head"].as_u64().unwrap()
            && Value::Array(counted) == want["counted"]
            && d.accept_seats == strings(&want["accept_seats"]);
        if !same {
            bad.entry("coverage")
                .or_default()
                .push(format!("{case} rust={d:?}"));
        }
    }
}

fn gate_cases(all: &Value, bad: &mut BTreeMap<&'static str, Vec<String>>) {
    use steward::gate::{
        CiCheck, breaker_state, category_of, ci_state, gate_plan_level, open_threads, tier_of,
        touches_protected,
    };
    let strings = |v: &Value| -> Vec<String> {
        v.as_array()
            .map(|l| {
                l.iter()
                    .filter_map(|s| s.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    for case in all["gate_tier"].as_array().unwrap() {
        let paths = strings(&case["paths"]);
        let (tier, reason) = tier_of(&paths);
        let want = &case["out"];
        let same = want["tier"][0] == tier.as_str()
            && want["tier"][1] == reason.as_str()
            && strings(&want["protected"]) == touches_protected(&paths)
            && want["plan_level"] == gate_plan_level(&paths)
            && want["category"] == category_of(&paths).as_str();
        if !same {
            bad.entry("gate_tier").or_default().push(format!(
                "{case} rust={tier} {reason} {} {}",
                gate_plan_level(&paths),
                category_of(&paths)
            ));
        }
    }
    for case in all["gate_ci"].as_array().unwrap() {
        let rollup: Vec<CiCheck> = case["rollup"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| CiCheck {
                conclusion: c["conclusion"].as_str().map(str::to_owned),
                status: c["status"].as_str().map(str::to_owned),
            })
            .collect();
        let state = ci_state(&rollup);
        if state.state != case["out"]["state"].as_str().unwrap()
            || state.detail != case["out"]["detail"].as_str().unwrap()
        {
            bad.entry("gate_ci")
                .or_default()
                .push(format!("{case} rust={state:?}"));
        }
    }
    for case in all["gate_breaker"].as_array().unwrap() {
        let got = breaker_state(case["text"].as_str().unwrap());
        let want = case["out"].as_bool();
        if got != want {
            bad.entry("gate_breaker")
                .or_default()
                .push(format!("{} rust={got:?} py={want:?}", case["text"]));
        }
    }
    for case in all["gate_threads"].as_array().unwrap() {
        let got = open_threads(&case["data"]);
        let want = case["out"].as_u64().map(|n| n as usize);
        if got != want {
            bad.entry("gate_threads")
                .or_default()
                .push(format!("{} rust={got:?} py={want:?}", case["data"]));
        }
    }
}
