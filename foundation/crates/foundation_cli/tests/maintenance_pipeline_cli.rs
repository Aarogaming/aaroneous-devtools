use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_foundation"))
        .args(args)
        .output()
        .expect("foundation binary execution")
}

fn setup_mock_checkout(root: &Path) -> PathBuf {
    let checkout = root.join("checkout");
    let crate_path = checkout.join("crates/test_component");
    fs::create_dir_all(crate_path.join("src")).unwrap();
    fs::create_dir_all(checkout.join(".git")).unwrap();

    let manifest = r#"
[package]
name = "test_component"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = "1.0"
"#;
    fs::write(crate_path.join("Cargo.toml"), manifest).unwrap();

    let src = r#"
pub struct Service;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}

    #[tokio::test]
    async fn async_works() {}
}
"#;
    fs::write(crate_path.join("src/lib.rs"), src).unwrap();
    checkout
}

#[test]
fn test_maintenance_refresh_and_facts_generation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let checkout = setup_mock_checkout(temp.path());
    let runtime_root = temp.path().join("maintenance");
    fs::create_dir_all(&runtime_root).unwrap();

    let out = cli(&[
        "maintenance",
        "refresh",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
        "--checkout",
        checkout.to_str().unwrap(),
        "--component",
        "crates/test_component",
    ]);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("evidence_digest="));
    assert!(stdout.contains("new_capture=true"));

    // Verify files created
    let ev_dir = runtime_root.join("evidence");
    let facts_path = ev_dir.join("inventory-facts.json");
    let index_path = ev_dir.join("source-index.json");
    let last_refresh_path = runtime_root.join("last-refresh.json");

    assert!(facts_path.is_file());
    assert!(index_path.is_file());
    assert!(last_refresh_path.is_file());

    let facts_val: Value = serde_json::from_str(&fs::read_to_string(&facts_path).unwrap()).unwrap();
    assert_eq!(facts_val["total_inline_test_markers"], 3);
    assert_eq!(facts_val["components"][0]["rust_source_files"], 1);
    assert_eq!(facts_val["components"][0]["inline_test_marker_count"], 3);
}

#[test]
fn test_maintenance_check_stage_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    let res_path = temp.path().join("proposal.json");

    let proposal = serde_json::json!({
        "schema_version": 1,
        "artifact_kind": "foundation.local-worker-result",
        "job_id": "TEST-JOB-42",
        "submitted_at_secs": 12345,
        "outcome": "pending-owner-review",
        "input": "...",
        "proposal": {
            "task": "TEST-JOB-42",
            "observations": ["obs 1"],
            "risks": ["risk 1"],
            "test_plan": ["NOT RUN fixture check"],
            "recommended_next_step": "Draft brief",
            "rust_draft": null
        }
    });
    fs::write(&res_path, serde_json::to_string(&proposal).unwrap()).unwrap();

    let out = cli(&[
        "maintenance",
        "check-stage",
        "--result-path",
        res_path.to_str().unwrap(),
        "--stage",
        "plan",
        "--task-kind",
        "inventory-integrity",
    ]);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let val: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["mechanical_pass"], true);
    assert_eq!(val["job_id"], "TEST-JOB-42");
}

#[test]
fn test_maintenance_housekeeping_and_status_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    let runtime_root = temp.path().join("runtime");
    fs::create_dir_all(runtime_root.join("results")).unwrap();
    fs::create_dir_all(runtime_root.join("archive")).unwrap();
    fs::create_dir_all(runtime_root.join("pipeline")).unwrap();

    fs::write(runtime_root.join("results/r1.json"), "{}").unwrap();
    fs::write(runtime_root.join("archive/a1.json"), "{}").unwrap();

    // Housekeeping CLI
    let hk_out = cli(&[
        "maintenance",
        "housekeeping",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
    ]);
    assert!(hk_out.status.success());
    let hk_stdout = String::from_utf8_lossy(&hk_out.stdout);
    assert!(hk_stdout.contains("pending=1; deferred=0; deletion=false"));

    let hk_file = runtime_root.join("pipeline/housekeeping.json");
    assert!(hk_file.is_file());

    // Status CLI
    let st_out = cli(&[
        "maintenance",
        "status",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
    ]);
    assert!(st_out.status.success());
    let st_val: Value = serde_json::from_slice(&st_out.stdout).unwrap();
    assert_eq!(st_val["route"], "Status");
    assert_eq!(st_val["pending"], 1);
    assert_eq!(st_val["archived"], 1);
}

#[test]
fn test_maintenance_control_cli() {
    let temp = tempfile::tempdir().expect("tempdir");
    let runtime_root = temp.path().join("runtime");
    fs::create_dir_all(&runtime_root).unwrap();

    // Set pause
    let pause_out = cli(&[
        "maintenance",
        "control",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
        "--state",
        "pause",
    ]);
    assert!(pause_out.status.success());

    let ctrl_file = runtime_root.join("loop-control.json");
    let ctrl_val: Value = serde_json::from_str(&fs::read_to_string(&ctrl_file).unwrap()).unwrap();
    assert_eq!(ctrl_val["state"], "pause");

    // Invalid state should fail
    let invalid_out = cli(&[
        "maintenance",
        "control",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
        "--state",
        "unknown_state",
    ]);
    assert!(!invalid_out.status.success());
}

#[test]
fn test_maintenance_advance_initializes_plan_job() {
    let temp = tempfile::tempdir().expect("tempdir");
    let checkout = setup_mock_checkout(temp.path());
    let runtime_root = temp.path().join("maintenance");
    fs::create_dir_all(&runtime_root).unwrap();

    // Step 1: Refresh first
    let ref_out = cli(&[
        "maintenance",
        "refresh",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
        "--checkout",
        checkout.to_str().unwrap(),
        "--component",
        "crates/test_component",
    ]);
    assert!(ref_out.status.success());

    // Step 2: Advance pipeline
    let adv_out = cli(&[
        "maintenance",
        "advance",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
    ]);
    assert!(
        adv_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&adv_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&adv_out.stdout);
    assert!(stdout.contains("pipeline stage=plan-ready"));
    assert!(stdout.contains("attempt=0"));

    // Verify state.json and route-plan.toml
    let state_file = runtime_root.join("pipeline/state.json");
    assert!(state_file.is_file());
    let state_val: Value = serde_json::from_str(&fs::read_to_string(&state_file).unwrap()).unwrap();
    assert_eq!(state_val["stage"], "plan-ready");
    assert_eq!(state_val["attempt"], 0);

    let plan_job = runtime_root.join("jobs/route-plan.toml");
    assert!(plan_job.is_file());
    let plan_toml = fs::read_to_string(&plan_job).unwrap();
    assert!(plan_toml.contains("mode = \"analysis\"") || plan_toml.contains("mode = 'analysis'"));
    assert!(plan_toml.contains("MAINTENANCE-PLAN-"));
}

#[test]
fn test_maintenance_advance_accepts_model_independent_decision() {
    let temp = tempfile::tempdir().expect("tempdir");
    let runtime_root = temp.path().join("maintenance");
    let checkout = setup_mock_checkout(temp.path());
    let ref_out = cli(&[
        "maintenance",
        "refresh",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
        "--checkout",
        checkout.to_str().unwrap(),
        "--component",
        "crates/test_component",
    ]);
    assert!(ref_out.status.success());

    let init_adv = cli(&[
        "maintenance",
        "advance",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
    ]);
    assert!(init_adv.status.success());

    let pipe_dir = runtime_root.join("pipeline");
    let decisions_dir = pipe_dir.join("decisions");
    let reviews_dir = runtime_root.join("reviews");
    fs::create_dir_all(&decisions_dir).unwrap();
    fs::create_dir_all(&reviews_dir).unwrap();

    let initial_state: Value =
        serde_json::from_str(&fs::read_to_string(pipe_dir.join("state.json")).unwrap()).unwrap();
    let cycle_id = initial_state["cycle_id"].as_str().unwrap().to_string();
    let evidence_key = initial_state["evidence_key"].as_str().unwrap().to_string();

    let ticket = serde_json::json!({
        "schema_version": 1,
        "cycle_id": &cycle_id,
        "responsibility": "test",
        "authority": "test",
        "non_overlap": "test",
        "evidence_key": &evidence_key,
        "mechanical_counts": [],
        "limits": [],
        "independent_review": {},
        "review_sha256": "rev-sha",
        "feedback": null
    });
    fs::write(
        runtime_root.join("evidence").join("pipeline-ticket.json"),
        serde_json::to_string(&ticket).unwrap(),
    )
    .unwrap();

    let review_rel = "reviews/rev1.txt";
    let review_full = runtime_root.join(review_rel);
    fs::write(&review_full, "independent review passed").unwrap();

    let mut hasher = sha2::Sha256::default();
    use sha2::Digest;
    hasher.update(b"independent review passed");
    let review_sha256 = format!("{:x}", hasher.finalize());

    let state = serde_json::json!({
        "schema_version": 1,
        "task_kind": "inventory-integrity",
        "cycle_id": &cycle_id,
        "evidence_key": &evidence_key,
        "stage": "awaiting-independent-review",
        "attempt": 0,
        "plan_job": "",
        "brief_job": "",
        "plan_result": "",
        "brief_result": "",
        "plan_sha256": "plan-hash",
        "brief_sha256": "brief-hash",
        "decision_sha256": "",
        "next_action": "",
        "whole_job_accepted": false
    });
    fs::write(
        pipe_dir.join("state.json"),
        serde_json::to_string(&state).unwrap(),
    )
    .unwrap();

    // Model-independent decision: actor is gemini, independent_of is ubuntu-worker-01
    let decision = serde_json::json!({
        "actor": "gemini",
        "independent_of": "ubuntu-worker-01",
        "cycle_id": &cycle_id,
        "evidence_key": &evidence_key,
        "plan_sha256": "plan-hash",
        "brief_sha256": "brief-hash",
        "verdict": "revise",
        "review_path": review_rel,
        "review_sha256": review_sha256,
        "notes": "clean review"
    });
    fs::write(
        decisions_dir.join(format!("{cycle_id}-R0.json")),
        serde_json::to_string(&decision).unwrap(),
    )
    .unwrap();

    let adv_out = cli(&[
        "maintenance",
        "advance",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
    ]);
    assert!(
        adv_out.status.success(),
        "advance failed: {}",
        String::from_utf8_lossy(&adv_out.stderr)
    );

    let state_after: Value =
        serde_json::from_str(&fs::read_to_string(pipe_dir.join("state.json")).unwrap()).unwrap();
    assert_eq!(state_after["stage"], "plan-ready");
    assert_eq!(state_after["attempt"], 1);
}

#[test]
fn test_maintenance_advance_rejects_self_review() {
    let temp = tempfile::tempdir().expect("tempdir");
    let runtime_root = temp.path().join("maintenance");
    let checkout = setup_mock_checkout(temp.path());
    let ref_out = cli(&[
        "maintenance",
        "refresh",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
        "--checkout",
        checkout.to_str().unwrap(),
        "--component",
        "crates/test_component",
    ]);
    assert!(ref_out.status.success());

    let init_adv = cli(&[
        "maintenance",
        "advance",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
    ]);
    assert!(init_adv.status.success());

    let pipe_dir = runtime_root.join("pipeline");
    let decisions_dir = pipe_dir.join("decisions");
    let reviews_dir = runtime_root.join("reviews");
    fs::create_dir_all(&decisions_dir).unwrap();
    fs::create_dir_all(&reviews_dir).unwrap();

    let initial_state: Value =
        serde_json::from_str(&fs::read_to_string(pipe_dir.join("state.json")).unwrap()).unwrap();
    let cycle_id = initial_state["cycle_id"].as_str().unwrap().to_string();
    let evidence_key = initial_state["evidence_key"].as_str().unwrap().to_string();

    let ticket = serde_json::json!({
        "schema_version": 1,
        "cycle_id": &cycle_id,
        "responsibility": "test",
        "authority": "test",
        "non_overlap": "test",
        "evidence_key": &evidence_key,
        "mechanical_counts": [],
        "limits": [],
        "independent_review": {},
        "review_sha256": "rev-sha",
        "feedback": null
    });
    fs::write(
        runtime_root.join("evidence").join("pipeline-ticket.json"),
        serde_json::to_string(&ticket).unwrap(),
    )
    .unwrap();

    let review_rel = "reviews/rev2.txt";
    let review_full = runtime_root.join(review_rel);
    fs::write(&review_full, "self review attempt").unwrap();

    use sha2::Digest;
    let mut hasher = sha2::Sha256::default();
    hasher.update(b"self review attempt");
    let review_sha256 = format!("{:x}", hasher.finalize());

    let state = serde_json::json!({
        "schema_version": 1,
        "task_kind": "inventory-integrity",
        "cycle_id": &cycle_id,
        "evidence_key": &evidence_key,
        "stage": "awaiting-independent-review",
        "attempt": 0,
        "plan_job": "",
        "brief_job": "",
        "plan_result": "",
        "brief_result": "",
        "plan_sha256": "plan-hash",
        "brief_sha256": "brief-hash",
        "decision_sha256": "",
        "next_action": "",
        "whole_job_accepted": false
    });
    fs::write(
        pipe_dir.join("state.json"),
        serde_json::to_string(&state).unwrap(),
    )
    .unwrap();

    // Self-review violation: actor == independent_of
    let decision = serde_json::json!({
        "actor": "gemini",
        "independent_of": "gemini",
        "cycle_id": &cycle_id,
        "evidence_key": &evidence_key,
        "plan_sha256": "plan-hash",
        "brief_sha256": "brief-hash",
        "verdict": "revise",
        "review_path": review_rel,
        "review_sha256": review_sha256,
        "notes": "invalid self review"
    });
    fs::write(
        decisions_dir.join(format!("{cycle_id}-R0.json")),
        serde_json::to_string(&decision).unwrap(),
    )
    .unwrap();

    let adv_out = cli(&[
        "maintenance",
        "advance",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
    ]);
    assert!(!adv_out.status.success());
    let stderr = String::from_utf8_lossy(&adv_out.stderr);
    assert!(
        stderr.contains("Unbound or invalid independent decision"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn test_maintenance_advance_rejects_unregistered_seats() {
    let temp = tempfile::tempdir().expect("tempdir");
    let runtime_root = temp.path().join("maintenance");
    let checkout = setup_mock_checkout(temp.path());
    let ref_out = cli(&[
        "maintenance",
        "refresh",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
        "--checkout",
        checkout.to_str().unwrap(),
        "--component",
        "crates/test_component",
    ]);
    assert!(ref_out.status.success());

    let init_adv = cli(&[
        "maintenance",
        "advance",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
    ]);
    assert!(init_adv.status.success());

    let pipe_dir = runtime_root.join("pipeline");
    let decisions_dir = pipe_dir.join("decisions");
    let reviews_dir = runtime_root.join("reviews");
    fs::create_dir_all(&decisions_dir).unwrap();
    fs::create_dir_all(&reviews_dir).unwrap();

    let initial_state: Value =
        serde_json::from_str(&fs::read_to_string(pipe_dir.join("state.json")).unwrap()).unwrap();
    let cycle_id = initial_state["cycle_id"].as_str().unwrap().to_string();
    let evidence_key = initial_state["evidence_key"].as_str().unwrap().to_string();

    let ticket = serde_json::json!({
        "schema_version": 1,
        "cycle_id": &cycle_id,
        "responsibility": "test",
        "authority": "test",
        "non_overlap": "test",
        "evidence_key": &evidence_key,
        "mechanical_counts": [],
        "limits": [],
        "independent_review": {},
        "review_sha256": "rev-sha",
        "feedback": null
    });
    fs::write(
        runtime_root.join("evidence").join("pipeline-ticket.json"),
        serde_json::to_string(&ticket).unwrap(),
    )
    .unwrap();

    let review_rel = "reviews/rev3.txt";
    let review_full = runtime_root.join(review_rel);
    fs::write(&review_full, "unregistered seat review attempt").unwrap();

    use sha2::Digest;
    let mut hasher = sha2::Sha256::default();
    hasher.update(b"unregistered seat review attempt");
    let review_sha256 = format!("{:x}", hasher.finalize());

    let state = serde_json::json!({
        "schema_version": 1,
        "task_kind": "inventory-integrity",
        "cycle_id": &cycle_id,
        "evidence_key": &evidence_key,
        "stage": "awaiting-independent-review",
        "attempt": 0,
        "plan_job": "",
        "brief_job": "",
        "plan_result": "",
        "brief_result": "",
        "plan_sha256": "plan-hash",
        "brief_sha256": "brief-hash",
        "decision_sha256": "",
        "next_action": "",
        "whole_job_accepted": false
    });
    fs::write(
        pipe_dir.join("state.json"),
        serde_json::to_string(&state).unwrap(),
    )
    .unwrap();

    // Unregistered seats: untrusted-a and untrusted-b
    let decision = serde_json::json!({
        "actor": "untrusted-a",
        "independent_of": "untrusted-b",
        "cycle_id": &cycle_id,
        "evidence_key": &evidence_key,
        "plan_sha256": "plan-hash",
        "brief_sha256": "brief-hash",
        "verdict": "revise",
        "review_path": review_rel,
        "review_sha256": review_sha256,
        "notes": "unregistered seat review"
    });
    fs::write(
        decisions_dir.join(format!("{cycle_id}-R0.json")),
        serde_json::to_string(&decision).unwrap(),
    )
    .unwrap();

    let adv_out = cli(&[
        "maintenance",
        "advance",
        "--runtime-root",
        runtime_root.to_str().unwrap(),
    ]);
    assert!(!adv_out.status.success());
    let stderr = String::from_utf8_lossy(&adv_out.stderr);
    assert!(
        stderr.contains("Unbound or invalid independent decision"),
        "unexpected stderr: {stderr}"
    );
}
