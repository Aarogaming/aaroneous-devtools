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

#[test]
fn atomic_replacement_leaves_no_temporary_sidecars_and_updates_content() {
    let dir = tempfile::tempdir().unwrap();
    let control_file = dir.path().join("loop-control.json");

    // Control writes loop-control.json via save_json_atomic / write_atomic
    let out = Command::new(env!("CARGO_BIN_EXE_foundation"))
        .args(["maintenance", "control", "--runtime-root"])
        .arg(dir.path())
        .args(["--state", "run"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "control failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(control_file.is_file());

    // Update state to pause atomically
    let out2 = Command::new(env!("CARGO_BIN_EXE_foundation"))
        .args(["maintenance", "control", "--runtime-root"])
        .arg(dir.path())
        .args(["--state", "pause"])
        .output()
        .unwrap();
    assert!(
        out2.status.success(),
        "control failed: {}",
        String::from_utf8_lossy(&out2.stderr)
    );

    let read_back: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&control_file).unwrap()).unwrap();
    assert_eq!(read_back["state"], "pause");

    // Ensure no .tmp files exist anywhere in runtime root
    let entries: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "tmp"))
        .collect();
    assert!(
        entries.is_empty(),
        "temporary files were not cleaned up: {:?}",
        entries
    );
}

#[test]
fn daemon_accepts_explicit_checkout_flag() {
    let dir = tempfile::tempdir().unwrap();
    let fake_checkout = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("worker.toml"), "unused test sentinel").unwrap();
    fs::write(
        dir.path().join("loop-control.json"),
        json!({"state":"pause"}).to_string(),
    )
    .unwrap();

    // Verify passing --checkout to daemon succeeds without complaining about D:\Aaroneous
    let output = Command::new(env!("CARGO_BIN_EXE_foundation"))
        .args(["maintenance", "daemon", "--runtime-root"])
        .arg(dir.path())
        .args(["--checkout"])
        .arg(fake_checkout.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "daemon failed with explicit --checkout: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("daemon exiting on control state: pause"));
}
