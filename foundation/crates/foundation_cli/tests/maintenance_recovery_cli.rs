use serde_json::json;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn cli(root: &Path, action: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_foundation"))
        .args(["maintenance", action, "--runtime-root"])
        .arg(root)
        .output()
        .unwrap()
}

#[test]
fn corrupt_state_is_retained_and_never_treated_as_a_new_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let pipeline = dir.path().join("pipeline");
    fs::create_dir(&pipeline).unwrap();
    let path = pipeline.join("state.json");
    let original = b"{\"schema_version\":1,";
    fs::write(&path, original).unwrap();
    for action in ["status", "advance", "housekeeping"] {
        let result = cli(dir.path(), action);
        assert!(
            !result.status.success(),
            "{action} silently accepted corrupt state"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("state.json"),
            "{action}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    assert!(!dir.path().join("jobs").exists());
    assert!(!pipeline.join("router.lock").exists());
}

#[test]
fn invalid_control_is_reported_instead_of_hidden() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("loop-control.json");
    for bytes in [
        b"{\"state\":\"pause\"".as_slice(),
        b"{\"state\":\"typo\"}",
        b"{\"route\":\"Control\"}",
    ] {
        fs::write(&path, bytes).unwrap();
        let result = cli(dir.path(), "status");
        assert!(!result.status.success(), "invalid control was hidden");
        assert!(String::from_utf8_lossy(&result.stderr).contains("loop-control.json"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn absent_state_is_a_fresh_runtime_and_valid_controls_remain_readable() {
    let dir = tempfile::tempdir().unwrap();
    let result = cli(dir.path(), "status");
    assert!(result.status.success());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(report["pipeline"].is_null());
    assert!(report["control"].is_null());
    for state in ["run", "pause", "restart"] {
        fs::write(
            dir.path().join("loop-control.json"),
            serde_json::to_vec(&json!({"state":state})).unwrap(),
        )
        .unwrap();
        let result = cli(dir.path(), "status");
        assert!(result.status.success());
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["control"]["state"], state);
    }
}

#[test]
fn a_directory_cannot_masquerade_as_absent_state_or_control() {
    for relative in ["pipeline/state.json", "loop-control.json"] {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(relative)).unwrap();
        let result = cli(dir.path(), "status");
        assert!(!result.status.success(), "{relative} treated as absent");
        assert!(String::from_utf8_lossy(&result.stderr).contains(".json"));
        assert!(dir.path().join(relative).is_dir());
    }
}

fn valid_state() -> serde_json::Value {
    json!({"schema_version":1,"task_kind":"inventory-integrity","cycle_id":"cycle-1",
        "evidence_key":"evidence-1","stage":"plan-ready","attempt":0,
        "plan_job":"plan-1","brief_job":"","plan_result":"","brief_result":"",
        "plan_sha256":"","brief_sha256":"","decision_sha256":"",
        "next_action":"await plan","whole_job_accepted":false})
}

#[test]
fn persisted_stage_version_and_retry_budget_are_not_silently_accepted() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("pipeline")).unwrap();
    let path = dir.path().join("pipeline/state.json");
    for (key, value) in [
        ("schema_version", json!(2)),
        ("stage", json!("invented")),
        ("attempt", json!(2)),
    ] {
        let mut state = valid_state();
        state[key] = value;
        let bytes = serde_json::to_vec(&state).unwrap();
        fs::write(&path, &bytes).unwrap();
        let result = cli(dir.path(), "advance");
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("unsupported pipeline state"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    fs::write(&path, serde_json::to_vec(&valid_state()).unwrap()).unwrap();
    assert!(cli(dir.path(), "status").status.success());
}

#[test]
fn daemon_refuses_corrupt_control_before_refresh_or_worker_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("worker.toml"), "unused test sentinel").unwrap();
    fs::write(dir.path().join("loop-control.json"), "{").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_foundation"))
        .args(["maintenance", "daemon", "--runtime-root"])
        .arg(dir.path())
        .arg("--serve")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("daemon did not stop on corrupt control");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let result = child.wait_with_output().unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("loop-control.json"));
    assert!(!dir.path().join("last-refresh.json").exists());
    assert!(!dir.path().join("last-cycle.json").exists());
    assert!(!dir.path().join("maintenance-loop.lock").exists());
}

#[test]
fn oversized_persistent_inputs_are_retained_and_rejected() {
    for relative in ["pipeline/state.json", "loop-control.json"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bytes = vec![b' '; 65537];
        fs::write(&path, &bytes).unwrap();
        let result = cli(dir.path(), "status");
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("exceeds 65536 bytes"));
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
}
