// Differential test (ignored by default): Rust against the Python oracle on generated adversarial text. See scripts/differential_cases.py.
use serde_json::Value;
use std::collections::BTreeMap;
use steward::receipts::{Pr, Receipts};
use steward::rejected::Comment;

#[test]
#[ignore]
fn differential() {
    let path = std::env::var("STEWARD_DIFF_FILE").expect("STEWARD_DIFF_FILE");
    let cases: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
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
