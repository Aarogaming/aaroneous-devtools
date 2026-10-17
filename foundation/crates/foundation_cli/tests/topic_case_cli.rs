use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/topic-case-v1.json")).unwrap()
}

fn save(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_foundation"))
        .args(args)
        .output()
        .unwrap()
}

fn review(value: &Value) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let case = dir.path().join("case.json");
    save(&case, value);
    cli(&["topic-case", "review", "--case", case.to_str().unwrap()])
}

fn receipt() -> Value {
    json!({"id":"event-5", "event":{"type":"outcome", "outcome":{
        "experiment":"experiment-1", "work_item":"C109", "reviewer":"independent-reviewer",
        "verdict":"needs_changes", "review":{"reference":"fixture/review", "revision":"head-1"},
        "actual_cost":{"value":0.0, "unit":"USD"}, "actual_hours":0.5,
        "measured_value":1.0, "measurement_evidence":[{"reference":"fixture/count", "revision":"1"}],
        "unresolved":["baseline missing"], "lesson":"Collect a baseline before calculating improvement."
    }}})
}

#[test]
fn review_preserves_dissent_and_unknown_baseline() {
    let mut value = fixture();
    value["events"].as_array_mut().unwrap().push(receipt());
    let result = review(&value);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["contributions"]["dissent-1"]["kind"], "dissent");
    assert_eq!(report["experiments"][0]["threshold_met"], true);
    assert!(report["experiments"][0]["measured_delta"].is_null());
    assert_eq!(report["references_verified"], false);
    assert_eq!(report["requires_owner_review"], true);
}

#[test]
fn export_round_trips_existing_intake_and_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let case = dir.path().join("case.json");
    let output = dir.path().join("intake.json");
    let validation = dir.path().join("validated.json");
    save(&case, &fixture());
    let args = [
        "topic-case",
        "export-intake",
        "--case",
        case.to_str().unwrap(),
        "--experiment",
        "experiment-1",
        "--output",
        output.to_str().unwrap(),
    ];
    assert!(cli(&args).status.success());
    let original = fs::read(&output).unwrap();
    let exported: Value = serde_json::from_slice(&original).unwrap();
    assert!(
        exported["decision"]
            .as_str()
            .unwrap()
            .contains("case-sha256:")
    );
    assert!(
        exported["decision"]
            .as_str()
            .unwrap()
            .contains("owner review and canonical queue claim required")
    );
    assert!(!cli(&args).status.success());
    assert_eq!(fs::read(&output).unwrap(), original);
    assert!(
        cli(&[
            "validate-intake",
            "--proposal",
            output.to_str().unwrap(),
            "--output",
            validation.to_str().unwrap()
        ])
        .status
        .success()
    );
}

#[test]
fn revise_preserves_history_binds_parent_and_rejects_duplicate_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let case = dir.path().join("case.json");
    let event = dir.path().join("event.json");
    let revised = dir.path().join("revised.json");
    let bad = dir.path().join("bad.json");
    save(&case, &fixture());
    save(&event, &receipt());
    let original = fs::read(&case).unwrap();
    assert!(
        cli(&[
            "topic-case",
            "revise",
            "--case",
            case.to_str().unwrap(),
            "--event",
            event.to_str().unwrap(),
            "--output",
            revised.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert_eq!(fs::read(&case).unwrap(), original);
    let next: Value = serde_json::from_slice(&fs::read(&revised).unwrap()).unwrap();
    assert_eq!(next["parent_digest"].as_str().unwrap().len(), 64);
    assert!(
        cli(&[
            "topic-case",
            "verify-parent",
            "--case",
            revised.to_str().unwrap(),
            "--parent",
            case.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert_eq!(
        &next["events"].as_array().unwrap()[..4],
        fixture()["events"].as_array().unwrap()
    );
    let mut duplicate = receipt();
    duplicate["id"] = json!("event-6");
    save(&event, &duplicate);
    assert!(
        !cli(&[
            "topic-case",
            "revise",
            "--case",
            revised.to_str().unwrap(),
            "--event",
            event.to_str().unwrap(),
            "--output",
            bad.to_str().unwrap()
        ])
        .status
        .success()
    );
    assert!(!bad.exists());
    assert!(
        !cli(&[
            "topic-case",
            "export-intake",
            "--case",
            revised.to_str().unwrap(),
            "--experiment",
            "experiment-1",
            "--output",
            bad.to_str().unwrap()
        ])
        .status
        .success()
    );
}

#[test]
fn rejects_unknown_versions_fields_and_dangling_or_duplicate_ids() {
    let mut cases = Vec::new();
    let mut v = fixture();
    v["schema_version"] = json!(2);
    cases.push(v);
    let mut v = fixture();
    v["invented"] = json!(true);
    cases.push(v);
    let mut v = fixture();
    v["events"][0]["event"]["topic"]["parent"] = json!("missing");
    cases.push(v);
    let mut v = fixture();
    v["events"][1]["event"]["contribution"]["topic"] = json!("missing");
    cases.push(v);
    let mut v = fixture();
    v["events"][1]["id"] = json!("event-1");
    cases.push(v);
    let mut v = fixture();
    v["events"][3]["event"]["experiment"]["contributions"] = json!(["missing"]);
    cases.push(v);
    let mut v = fixture();
    v["events"][3]["event"]["experiment"]["intake"]["scope"] = json!(["../escape.rs"]);
    cases.push(v);
    let mut v = fixture();
    v["events"][3]["event"]["experiment"]["time_budget_hours"] = json!(-1);
    cases.push(v);
    for (i, value) in cases.iter().enumerate() {
        assert!(!review(value).status.success(), "invalid case {i}");
    }
}

fn relationship(id: &str, from: &str, to: &str, kind: &str) -> Value {
    json!({"id":id, "event":{"type":"relationship", "relationship":{
        "id":id, "from":from, "to":to, "kind":kind, "reason":"explicit relation",
        "evidence":[{"reference":"fixture/relation", "revision":"1"}]
    }}})
}

#[test]
fn dependency_cycles_rejected_but_dissent_cycles_and_tags_do_not_change_hierarchy() {
    let mut value = fixture();
    let mut child = value["events"][0].clone();
    child["id"] = json!("child-event");
    child["event"]["topic"]["id"] = json!("child");
    child["event"]["topic"]["parent"] = json!("topics");
    let events = value["events"].as_array_mut().unwrap();
    events.push(child);
    events.push(relationship("rel-1", "topics", "child", "contradicts"));
    events.push(relationship("rel-2", "child", "topics", "contradicts"));
    assert!(review(&value).status.success());
    value["events"][5]["event"]["relationship"]["kind"] = json!("depends_on");
    value["events"][6]["event"]["relationship"]["kind"] = json!("depends_on");
    assert!(!review(&value).status.success());
}

#[test]
fn retry_requires_terminal_receipt_and_declared_revision_delta() {
    let mut value = fixture();
    let mut retry = value["events"][3].clone();
    retry["id"] = json!("retry-event");
    retry["event"]["experiment"]["id"] = json!("experiment-2");
    retry["event"]["experiment"]["intake"]["id"] = json!("TOPIC-EXPERIMENT-2");
    retry["event"]["experiment"]["previous_attempt"] = json!("experiment-1");
    retry["event"]["experiment"]["revision_delta"] =
        json!("Collect baseline before a new bounded test.");
    value["events"].as_array_mut().unwrap().push(retry.clone());
    assert!(!review(&value).status.success());
    value["events"].as_array_mut().unwrap().pop();
    value["events"].as_array_mut().unwrap().push(receipt());
    value["events"].as_array_mut().unwrap().push(retry);
    assert!(review(&value).status.success());
    value["events"][5]["event"]["experiment"]["revision_delta"] = Value::Null;
    assert!(!review(&value).status.success());
}

#[test]
fn measured_delta_and_budget_are_observations_separate_from_review_verdict() {
    let mut case = fixture();
    case["events"][3]["event"]["experiment"]["baseline"] = json!({"status":"measured", "value":3.0, "evidence":[{"reference":"fixture/baseline","revision":"1"}]});
    case["events"][3]["event"]["experiment"]["direction"] = json!("at_most");
    let mut outcome = receipt();
    outcome["event"]["outcome"]["actual_hours"] = json!(2.0);
    case["events"].as_array_mut().unwrap().push(outcome);
    let result = review(&case);
    assert!(result.status.success());
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["experiments"][0]["measured_delta"], -2.0);
    assert_eq!(report["experiments"][0]["threshold_met"], true);
    assert_eq!(report["experiments"][0]["budget_exceeded"], true);
    assert_eq!(
        report["experiments"][0]["outcome"]["verdict"],
        "needs_changes"
    );
}

#[test]
fn receipts_require_matching_work_cost_units_and_measurement_evidence() {
    for (field, value) in [
        ("work_item", json!("other")),
        ("measurement_evidence", json!([])),
        ("actual_hours", json!(-1.0)),
        ("actual_cost", json!({"value":1.0,"unit":"hours"})),
    ] {
        let mut case = fixture();
        let mut event = receipt();
        event["event"]["outcome"][field] = value;
        case["events"].as_array_mut().unwrap().push(event);
        assert!(!review(&case).status.success(), "{field}");
    }
    let mut case = fixture();
    case["events"][3]["event"]["experiment"]["baseline"] = json!({"status":"measured","value":-1.0e308,"evidence":[{"reference":"fixture/baseline","revision":"1"}]});
    let mut event = receipt();
    event["event"]["outcome"]["measured_value"] = json!(1.0e308);
    case["events"].as_array_mut().unwrap().push(event);
    assert!(!review(&case).status.success());
}

#[test]
fn parent_verification_rejects_valid_but_rewritten_history_and_wrong_digest() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("parent.json");
    let event = dir.path().join("event.json");
    let next = dir.path().join("next.json");
    save(&parent, &fixture());
    save(&event, &receipt());
    assert!(
        cli(&[
            "topic-case",
            "revise",
            "--case",
            parent.to_str().unwrap(),
            "--event",
            event.to_str().unwrap(),
            "--output",
            next.to_str().unwrap()
        ])
        .status
        .success()
    );
    let original: Value = serde_json::from_slice(&fs::read(&next).unwrap()).unwrap();
    for digest_only in [false, true] {
        let mut changed = original.clone();
        if digest_only {
            changed["parent_digest"] = json!("0".repeat(64));
        } else {
            changed["events"][0]["event"]["topic"]["title"] = json!("rewritten");
        }
        save(&next, &changed);
        assert!(
            !cli(&[
                "topic-case",
                "verify-parent",
                "--case",
                next.to_str().unwrap(),
                "--parent",
                parent.to_str().unwrap()
            ])
            .status
            .success()
        );
    }
}

#[test]
fn competing_exports_publish_one_complete_file_and_clean_staging_files() {
    let dir = tempfile::tempdir().unwrap();
    let case = dir.path().join("case.json");
    let output = dir.path().join("output.json");
    save(&case, &fixture());
    let spawn = || {
        Command::new(env!("CARGO_BIN_EXE_foundation"))
            .args([
                "topic-case",
                "export-intake",
                "--case",
                case.to_str().unwrap(),
                "--experiment",
                "experiment-1",
                "--output",
                output.to_str().unwrap(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut first = spawn();
    let mut second = spawn();
    let won = [
        first.wait().unwrap().success(),
        second.wait().unwrap().success(),
    ];
    assert_eq!(won.iter().filter(|success| **success).count(), 1);
    let exported: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(exported["id"], "TOPIC-EXPERIMENT-1");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn rejects_relative_paths_oversized_input_and_nested_unknown_fields() {
    assert!(
        !cli(&["topic-case", "review", "--case", "relative.json"])
            .status
            .success()
    );
    let dir = tempfile::tempdir().unwrap();
    let big = dir.path().join("big.json");
    fs::write(&big, vec![b' '; 4 * 1024 * 1024 + 1]).unwrap();
    let result = cli(&["topic-case", "review", "--case", big.to_str().unwrap()]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("input exceeds"));
    let mut case = fixture();
    case["events"][0]["event"]["topic"]["extra"] = json!(true);
    assert!(!review(&case).status.success());
    let mut case = fixture();
    case["events"][0]["event"]["extra"] = json!(true);
    assert!(!review(&case).status.success());
    let mut case = fixture();
    let duplicate = case["events"][0].clone();
    case["events"] = json!(vec![duplicate; 4097]);
    assert!(!review(&case).status.success());
}

#[test]
fn export_size_limit_is_checked_after_provenance_before_publication() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("case.json");
    let output = dir.path().join("intake.json");
    let mut case = fixture();
    let proposal = &mut case["events"][3]["event"]["experiment"]["intake"];
    proposal["decision"] = json!("");
    let overhead = serde_json::to_vec_pretty(proposal).unwrap().len();
    proposal["decision"] = json!("x".repeat(65536 - overhead - 32));
    assert!(serde_json::to_vec_pretty(proposal).unwrap().len() < 65536);
    save(&input, &case);
    let result = cli(&[
        "topic-case",
        "export-intake",
        "--case",
        input.to_str().unwrap(),
        "--experiment",
        "experiment-1",
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("output exceeds 65536 bytes"));
    assert!(!output.exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn measured_baseline_requires_dedicated_evidence_and_retry_branches_are_visible() {
    let mut case = fixture();
    case["events"][3]["event"]["experiment"]["baseline"] =
        json!({"status":"measured","value":0.0,"evidence":[]});
    assert!(!review(&case).status.success());
    let mut case = fixture();
    let mut first = case["events"][3].clone();
    first["id"] = json!("retry-2");
    first["event"]["experiment"]["id"] = json!("experiment-2");
    first["event"]["experiment"]["intake"]["id"] = json!("INTAKE-2");
    first["event"]["experiment"]["previous_attempt"] = json!("experiment-1");
    first["event"]["experiment"]["revision_delta"] = json!("bounded alternative A");
    let mut second = first.clone();
    second["id"] = json!("retry-3");
    second["event"]["experiment"]["id"] = json!("experiment-3");
    second["event"]["experiment"]["intake"]["id"] = json!("INTAKE-3");
    second["event"]["experiment"]["revision_delta"] = json!("bounded alternative B");
    case["events"]
        .as_array_mut()
        .unwrap()
        .extend([receipt(), first, second]);
    let result = review(&case);
    assert!(result.status.success());
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        report["retry_children"]["experiment-1"],
        json!(["experiment-2", "experiment-3"])
    );
}

#[test]
fn relationship_retirement_preserves_history_and_removes_the_active_dependency() {
    let mut case = fixture();
    let mut child = case["events"][0].clone();
    child["id"] = json!("child-event");
    child["event"]["topic"]["id"] = json!("child");
    child["event"]["topic"]["parent"] = json!("topics");
    let events = case["events"].as_array_mut().unwrap();
    events.push(child);
    events.push(relationship("rel-1", "topics", "child", "depends_on"));
    let retire = json!({"id":"retire-1","event":{"type":"relationship_retired","retirement":{"relationship":"rel-1","actor":"reviewer","reason":"disproved by inspection","evidence":[{"reference":"fixture/review","revision":"1"}]}}});
    events.push(retire.clone());
    events.push(relationship("rel-2", "child", "topics", "depends_on"));
    let result = review(&case);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["active_relationships"], json!(["rel-2"]));
    assert_eq!(report["relationships"]["rel-1"]["from"], "topics");
    assert_eq!(report["topics"]["child"]["parent"], "topics");
    let mut duplicate = retire;
    duplicate["id"] = json!("retire-2");
    case["events"].as_array_mut().unwrap().push(duplicate);
    assert!(!review(&case).status.success());
}
