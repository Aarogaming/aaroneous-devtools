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
