use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn snapshot() -> Value {
    json!({"v":1,"job":{
        "id":"job-1","subject_revision":"subject-1","now":100,"deadline":120,
        "max_evidence_age":10,"remaining_budget":10,"interface":"task-v1",
        "requirements":[{"capability":"review","suite_revision":"suite-1","minimum_samples":10,"minimum_pass_bps":9000}],
        "authorized":[{"provider":"worker","revision":"config-1"}],"excluded_principals":[]
    },"providers":[{
        "key":{"provider":"worker","revision":"config-1"},"principal":"principal-1","kind":"model",
        "available":true,"resources_fit":true,"interface":"task-v1","cost_bound":10,"duration_bound":20,
        "evidence":[{"capability":"review","suite_revision":"suite-1","reference":"evaluation-1","observed_at":90,"passed":9,"samples":10}]
    }]})
}

fn cli(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_foundation"))
        .args(["guild", "broker-preview", "--input"])
        .arg(path)
        .output()
        .unwrap()
}

fn run(raw: &[u8]) -> Output {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("snapshot.json");
    fs::write(&input, raw).unwrap();
    let out = cli(&input);
    assert_eq!(fs::read(&input).unwrap(), raw);
    assert_eq!(
        fs::read_dir(temp.path()).unwrap().count(),
        1,
        "preview created state"
    );
    out
}

#[test]
fn snapshot_bound_report_is_deterministic_and_never_authorizes_dispatch() {
    let raw = serde_json::to_vec_pretty(&snapshot()).unwrap();
    let a = run(&raw);
    let b = run(&raw);
    assert!(a.status.success(), "{:?}", a);
    assert_eq!(a.stdout, b.stdout);
    let report: Value = serde_json::from_slice(&a.stdout).unwrap();
    assert_eq!(
        report["input_utf8"],
        String::from_utf8(raw.clone()).unwrap()
    );
    assert_eq!(
        report["input_sha256"],
        Sha256::digest(&raw)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    assert_eq!(report["decision"], "candidates");
    assert_eq!(report["ranked"][0]["key"]["provider"], "worker");
    assert_eq!(report["dispatch_authorized"], false);
    assert_eq!(report["policy_verified"], false);
    assert_eq!(report["evidence_verified"], false);
}

#[test]
fn revoked_and_stale_inputs_produce_holds_not_reused_recommendations() {
    let mut input = snapshot();
    input["job"]["authorized"] = json!([]);
    let out = run(&serde_json::to_vec(&input).unwrap());
    assert!(out.status.success());
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["decision"], "hold");
    assert_eq!(report["excluded"][0]["reasons"][0]["code"], "unauthorized");
    let mut input = snapshot();
    input["providers"][0]["evidence"][0]["observed_at"] = json!(89);
    let out = run(&serde_json::to_vec(&input).unwrap());
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        report["excluded"][0]["reasons"][0]["code"],
        "stale_evidence"
    );
}

#[test]
fn schema_rejects_unknown_duplicate_missing_and_wrong_typed_fields() {
    let raw = serde_json::to_string(&snapshot()).unwrap();
    let mut unknown = snapshot();
    unknown["job"]["dispatch_authorized"] = json!(true);
    let mut missing = snapshot();
    missing["job"]
        .as_object_mut()
        .unwrap()
        .remove("subject_revision");
    let mut wrong = snapshot();
    wrong["providers"][0]["available"] = json!("true");
    let mut version = snapshot();
    version["v"] = json!(2);
    for candidate in [
        serde_json::to_string(&unknown).unwrap(),
        serde_json::to_string(&missing).unwrap(),
        serde_json::to_string(&wrong).unwrap(),
        serde_json::to_string(&version).unwrap(),
        raw.replace("\"now\":100", "\"now\":100,\"now\":101"),
        raw.replace(
            "\"provider\":\"worker\"",
            "\"provider\":\"worker\",\"provider\":\"other\"",
        ),
    ] {
        let out = run(candidate.as_bytes());
        assert!(!out.status.success(), "accepted {candidate}");
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn oversized_invalid_utf8_and_relative_input_fail_without_report() {
    for raw in [vec![b' '; 1024 * 1024 + 1], vec![0xff], b"{}".to_vec()] {
        let out = run(&raw);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
    }
    let out = cli(Path::new("relative.json"));
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("absolute"));
}

#[test]
fn empty_inventory_is_an_explicit_hold_and_unknown_quote_never_dispatches() {
    let mut input = snapshot();
    input["providers"] = json!([]);
    let out = run(&serde_json::to_vec(&input).unwrap());
    assert!(out.status.success());
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["decision"], "hold");
    let mut input = snapshot();
    input["providers"][0]
        .as_object_mut()
        .unwrap()
        .remove("cost_bound");
    let out = run(&serde_json::to_vec(&input).unwrap());
    assert!(out.status.success());
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["excluded"][0]["reasons"][0]["code"], "unknown_quote");
    assert_eq!(report["dispatch_authorized"], false);
}
