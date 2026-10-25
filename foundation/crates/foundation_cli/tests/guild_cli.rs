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

struct CheckpointFixture {
    _temp: tempfile::TempDir,
    log: PathBuf,
    tip: PathBuf,
    outbox: PathBuf,
}

impl CheckpointFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("fixture directory");
        let fixture = Self {
            log: temp.path().join("events.jsonl"),
            tip: temp.path().join("events.jsonl.tip"),
            outbox: temp.path().join("outbox"),
            _temp: temp,
        };
        let output = fixture.tick();
        assert!(output.status.success(), "first tick: {output:?}");
        fixture
    }

    fn tick(&self) -> Output {
        let snapshot = vectors_dir().join("covered_clean_pr_gets_the_merge_brief.in.json");
        cli(&[
            "guild",
            "tick",
            "--snapshot",
            snapshot.to_str().unwrap(),
            "--log",
            self.log.to_str().unwrap(),
            "--outbox",
            self.outbox.to_str().unwrap(),
        ])
    }

    fn cards(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut cards: Vec<_> = fs::read_dir(&self.outbox)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect();
        cards.sort_by(|a, b| a.0.cmp(&b.0));
        cards
    }

    fn assert_rejected_without_writes(&self) {
        let log_before = fs::read(&self.log).ok();
        let tip_before = fs::read(&self.tip).unwrap();
        let cards_before = self.cards();
        let output = self.tick();
        assert!(
            !output.status.success(),
            "damaged checkpoint must stop the tick: {output:?}"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("checkpoint") || error.contains("sidecar"),
            "must explain the recovery hold: {output:?}"
        );
        assert_eq!(fs::read(&self.log).ok(), log_before);
        assert_eq!(fs::read(&self.tip).unwrap(), tip_before);
        assert_eq!(self.cards(), cards_before);
        for action in ["status", "verify-log"] {
            let output = cli(&["guild", action, "--log", self.log.to_str().unwrap()]);
            assert!(
                !output.status.success(),
                "{action} must reject damaged evidence: {output:?}"
            );
            assert!(!String::from_utf8_lossy(&output.stdout).contains("verified"));
        }
        assert_eq!(fs::read(&self.log).ok(), log_before);
        assert_eq!(fs::read(&self.tip).unwrap(), tip_before);
        assert_eq!(self.cards(), cards_before);
    }
}

#[test]
fn checkpoint_deleted_log_does_not_start_a_new_chain() {
    let fixture = CheckpointFixture::new();
    fs::remove_file(&fixture.log).unwrap();
    fixture.assert_rejected_without_writes();
}

#[test]
fn checkpoint_empty_log_does_not_start_a_new_chain() {
    let fixture = CheckpointFixture::new();
    fs::write(&fixture.log, b"").unwrap();
    fixture.assert_rejected_without_writes();
}

#[test]
fn checkpoint_malformed_json_is_not_overwritten() {
    let fixture = CheckpointFixture::new();
    fs::write(&fixture.tip, b"{\"count\":1,\"tip\":").unwrap();
    fixture.assert_rejected_without_writes();
}

#[test]
fn checkpoint_missing_digest_is_not_overwritten() {
    let fixture = CheckpointFixture::new();
    fs::write(&fixture.tip, br#"{"count":1}"#).unwrap();
    fixture.assert_rejected_without_writes();
}

#[test]
fn checkpoint_wrong_count_is_not_overwritten() {
    let fixture = CheckpointFixture::new();
    let mut tip: Value = serde_json::from_slice(&fs::read(&fixture.tip).unwrap()).unwrap();
    tip["count"] = Value::from(7);
    fs::write(&fixture.tip, serde_json::to_vec(&tip).unwrap()).unwrap();
    fixture.assert_rejected_without_writes();
}

#[test]
fn checkpoint_invalid_count_is_not_overwritten() {
    let fixture = CheckpointFixture::new();
    let mut tip: Value = serde_json::from_slice(&fs::read(&fixture.tip).unwrap()).unwrap();
    tip["count"] = Value::from("1");
    fs::write(&fixture.tip, serde_json::to_vec(&tip).unwrap()).unwrap();
    fixture.assert_rejected_without_writes();
}

#[test]
fn checkpoint_stale_after_append_is_not_repaired_implicitly() {
    let fixture = CheckpointFixture::new();
    let first_tip = fs::read(&fixture.tip).unwrap();
    assert!(fixture.tick().status.success());
    fs::write(&fixture.tip, first_tip).unwrap();
    fixture.assert_rejected_without_writes();
}

#[test]
fn checkpoint_legacy_digest_can_continue_without_losing_history() {
    let fixture = CheckpointFixture::new();
    let tip: Value = serde_json::from_slice(&fs::read(&fixture.tip).unwrap()).unwrap();
    fs::write(&fixture.tip, tip["tip"].as_str().unwrap()).unwrap();
    assert!(fixture.tick().status.success());
    let continued: Value = serde_json::from_slice(&fs::read(&fixture.tip).unwrap()).unwrap();
    assert_eq!(continued["count"], 2);
}

#[test]
fn checkpoint_absent_legacy_log_can_continue() {
    let fixture = CheckpointFixture::new();
    fs::remove_file(&fixture.tip).unwrap();
    assert!(fixture.tick().status.success());
    let continued: Value = serde_json::from_slice(&fs::read(&fixture.tip).unwrap()).unwrap();
    assert_eq!(continued["count"], 2);
}

#[test]
fn checkpoint_json_requires_count_and_rejects_ambiguous_fields() {
    for malformed in [
        r#"{"tip":"DIGEST"}"#,
        r#"{"count":1,"count":1,"tip":"DIGEST"}"#,
        r#"{"count":1,"tip":"DIGEST","tip":"DIGEST"}"#,
        "null",
        "[]",
    ] {
        let fixture = CheckpointFixture::new();
        let tip: Value = serde_json::from_slice(&fs::read(&fixture.tip).unwrap()).unwrap();
        let malformed = malformed.replace("DIGEST", tip["tip"].as_str().unwrap());
        fs::write(&fixture.tip, malformed).unwrap();
        fixture.assert_rejected_without_writes();
    }
}

#[test]
fn checkpoint_external_digest_cannot_override_damaged_sidecar() {
    let fixture = CheckpointFixture::new();
    let tip: Value = serde_json::from_slice(&fs::read(&fixture.tip).unwrap()).unwrap();
    fs::write(&fixture.tip, b"{broken").unwrap();
    let output = cli(&[
        "guild",
        "verify-log",
        "--log",
        fixture.log.to_str().unwrap(),
        "--expect-tip",
        tip["tip"].as_str().unwrap(),
    ]);
    assert!(
        !output.status.success(),
        "sidecar must still be validated: {output:?}"
    );
    assert_eq!(fs::read(&fixture.tip).unwrap(), b"{broken");
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

#[test]
fn test_guild_tick_refuses_non_clone_repo_dir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_path = temp.path().join("steward-events.jsonl");

    // Init a dummy git repo with a different origin remote
    Command::new("git")
        .args(["init", temp.path().to_str().unwrap()])
        .output()
        .expect("git init");
    Command::new("git")
        .args([
            "-C",
            temp.path().to_str().unwrap(),
            "remote",
            "add",
            "origin",
            "git@github.com:OtherOrg/unrelated-repo.git",
        ])
        .output()
        .expect("git remote add");

    let out = cli(&[
        "guild",
        "tick",
        "--repo-dir",
        temp.path().to_str().unwrap(),
        "--repo",
        "Aarogaming/aaroneous-devtools",
        "--log",
        log_path.to_str().unwrap(),
    ]);

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("is not a clone of Aarogaming/aaroneous-devtools"),
        "unexpected stderr: {stderr}"
    );
}
