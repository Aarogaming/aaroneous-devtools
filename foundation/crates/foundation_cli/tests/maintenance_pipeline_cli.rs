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
