use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_foundation"))
        .args(args)
        .output()
        .expect("foundation binary execution")
}

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../scripts/golden/steward")
}

#[test]
fn test_guild_help() {
    let out = cli(&["guild", "--help"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("tick"));
    assert!(stdout.contains("verify-log"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("daemon"));
}

#[test]
fn test_guild_tick_with_snapshot_and_outbox() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_path = temp.path().join("steward-events.jsonl");
    let outbox_path = temp.path().join("outbox");
    let scenario_path = vectors_dir().join("covered_clean_pr_gets_the_merge_brief.in.json");

    assert!(scenario_path.exists(), "golden vector scenario must exist");

    let out = cli(&[
        "guild",
        "tick",
        "--snapshot",
        scenario_path.to_str().unwrap(),
        "--log",
        log_path.to_str().unwrap(),
        "--outbox",
        outbox_path.to_str().unwrap(),
        "--json",
    ]);

    assert!(
        out.status.success(),
        "guild tick failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let cards: Value = serde_json::from_str(&stdout).expect("valid cards JSON output");
    assert!(cards.is_array());
    let card_list = cards.as_array().unwrap();
    assert!(!card_list.is_empty(), "must generate cards");

    // Verify outbox files were written atomically
    assert!(outbox_path.exists());
    let mut outbox_files = Vec::new();
    for entry in fs::read_dir(&outbox_path).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().to_string();
        if name.ends_with(".md") {
            outbox_files.push(name);
        }
    }
    assert!(!outbox_files.is_empty(), "outbox markdown files must exist");

    // Verify log file and tip sidecar exist
    assert!(log_path.exists(), "event log must exist");
    let tip_path = temp.path().join("steward-events.jsonl.tip");
    assert!(
        tip_path.exists(),
        "tip sidecar must exist at steward-events.jsonl.tip"
    );
    let tip_content = fs::read_to_string(&tip_path).unwrap();
    let tip_json: Value = serde_json::from_str(tip_content.trim()).expect("tip sidecar is JSON");
    assert_eq!(tip_json.get("count").and_then(Value::as_u64), Some(1));
    let tip_hex = tip_json
        .get("tip")
        .and_then(Value::as_str)
        .expect("tip hex");
    assert_eq!(tip_hex.len(), 64, "tip must be 64-hex SHA-256");

    // Verify log verification command passes against sidecar automatically
    let verify_sidecar = cli(&["guild", "verify-log", "--log", log_path.to_str().unwrap()]);
    assert!(verify_sidecar.status.success());

    // Verify log verification command passes with explicit --expect-tip
    let verify_out = cli(&[
        "guild",
        "verify-log",
        "--log",
        log_path.to_str().unwrap(),
        "--expect-tip",
        tip_hex,
    ]);
    assert!(verify_out.status.success());
    let verify_stdout = String::from_utf8_lossy(&verify_out.stdout);
    assert!(verify_stdout.contains("verified 1 event(s)"));

    // Verify status command
    let status_out = cli(&[
        "guild",
        "status",
        "--log",
        log_path.to_str().unwrap(),
        "--outbox",
        outbox_path.to_str().unwrap(),
    ]);
    assert!(status_out.status.success());
    let status_stdout = String::from_utf8_lossy(&status_out.stdout);
    assert!(status_stdout.contains("Guild Status:"));
    assert!(status_stdout.contains("Total events:   1"));
}

#[test]
fn test_guild_tampered_log_fails() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_path = temp.path().join("steward-events.jsonl");
    let outbox_path = temp.path().join("outbox");
    let scenario_path = vectors_dir().join("covered_clean_pr_gets_the_merge_brief.in.json");

    // Run tick 1
    let out1 = cli(&[
        "guild",
        "tick",
        "--snapshot",
        scenario_path.to_str().unwrap(),
        "--log",
        log_path.to_str().unwrap(),
        "--outbox",
        outbox_path.to_str().unwrap(),
    ]);
    assert!(out1.status.success());

    // Run tick 2 (appends second event to the chain)
    let out2 = cli(&[
        "guild",
        "tick",
        "--snapshot",
        scenario_path.to_str().unwrap(),
        "--log",
        log_path.to_str().unwrap(),
        "--outbox",
        outbox_path.to_str().unwrap(),
    ]);
    assert!(out2.status.success());

    // Tamper with the first event in the log
    let original = fs::read_to_string(&log_path).unwrap();
    let tampered = original.replacen("cards", "tampered_cards", 1);
    fs::write(&log_path, tampered).unwrap();

    // Verify-log must now fail with chain broken on event 2
    let verify_out = cli(&["guild", "verify-log", "--log", log_path.to_str().unwrap()]);
    assert!(!verify_out.status.success());
    let stderr = String::from_utf8_lossy(&verify_out.stderr);
    assert!(stderr.contains("chain broken"));
}
