use std::fs;
use std::process::Command;

fn run_foundation(args: &[&str]) -> std::process::Output {
    let bin_path = env!("CARGO_BIN_EXE_foundation");
    Command::new(bin_path)
        .args(args)
        .output()
        .expect("failed to execute foundation binary")
}

#[test]
fn test_contribution_envelope_verify_cli_valid() {
    let temp = tempfile::tempdir().unwrap();
    let env_path = temp.path().join("envelope.json");

    let cid = "C117/gemini-contribution-lifecycle-v1";
    let origin_rev = "47360c238b722d3e0b4931a7f058097b830d97fe";
    let evidence_key = "e".repeat(64);
    let review_sha = "f".repeat(64);

    let envelope_json = serde_json::json!({
        "schema_version": 1,
        "artifact_kind": "foundation.contribution-envelope",
        "contribution_id": cid,
        "origin_revision": origin_rev,
        "lane": "C117",
        "producer": "gemini",
        "slug": "contribution-lifecycle",
        "version": 1,
        "target_crate": "foundation_cli",
        "target_file": "src/contribution.rs",
        "instruction": "connect contribution lifecycle",
        "evidence_key": evidence_key,
        "verification_commands": [
            { "command": "cargo check", "exit_code": 0 },
            { "command": "cargo test", "exit_code": 0 }
        ],
        "independent_review": {
            "actor": "claude",
            "independent_of": "gemini",
            "review_sha256": review_sha,
            "verdict": "accept"
        },
        "trailers": [
            format!("Contribution-ID: {cid}"),
            "Co-Authored-By: Gemini <gemini@aarogaming.local>"
        ],
        "created_at_secs": 1728470000
    });

    fs::write(
        &env_path,
        serde_json::to_string_pretty(&envelope_json).unwrap(),
    )
    .unwrap();

    let out = run_foundation(&[
        "verify-contribution",
        "--envelope",
        env_path.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stdout: {}, stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("is valid"));
    assert!(stdout.contains("Reviewer:          claude (independent of gemini)"));
}

#[test]
fn test_contribution_envelope_verify_cli_rejects_self_review() {
    let temp = tempfile::tempdir().unwrap();
    let env_path = temp.path().join("envelope.json");

    let cid = "C117/gemini-contribution-lifecycle-v1";
    let origin_rev = "47360c238b722d3e0b4931a7f058097b830d97fe";
    let evidence_key = "e".repeat(64);
    let review_sha = "f".repeat(64);

    // Self-review: actor is "gemini" which matches producer "gemini"
    let envelope_json = serde_json::json!({
        "schema_version": 1,
        "artifact_kind": "foundation.contribution-envelope",
        "contribution_id": cid,
        "origin_revision": origin_rev,
        "lane": "C117",
        "producer": "gemini",
        "slug": "contribution-lifecycle",
        "version": 1,
        "target_crate": "foundation_cli",
        "target_file": "src/contribution.rs",
        "instruction": "connect contribution lifecycle",
        "evidence_key": evidence_key,
        "verification_commands": [],
        "independent_review": {
            "actor": "gemini",
            "independent_of": "claude",
            "review_sha256": review_sha,
            "verdict": "accept"
        },
        "trailers": [
            format!("Contribution-ID: {cid}"),
            "Co-Authored-By: Gemini <gemini@aarogaming.local>"
        ],
        "created_at_secs": 1728470000
    });

    fs::write(
        &env_path,
        serde_json::to_string_pretty(&envelope_json).unwrap(),
    )
    .unwrap();

    let out = run_foundation(&[
        "verify-contribution",
        "--envelope",
        env_path.to_str().unwrap(),
    ]);
    assert!(!out.status.success(), "self-review must fail validation");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("independent reviewer cannot be producer")
            || stderr.contains("failed validation")
    );
}

#[test]
fn test_contribution_envelope_inspect_cli() {
    let temp = tempfile::tempdir().unwrap();
    let env_path = temp.path().join("envelope.json");

    let cid = "C117/gemini-contribution-lifecycle-v1";
    let origin_rev = "47360c238b722d3e0b4931a7f058097b830d97fe";
    let evidence_key = "a".repeat(64);

    let envelope_json = serde_json::json!({
        "schema_version": 1,
        "artifact_kind": "foundation.contribution-envelope",
        "contribution_id": cid,
        "origin_revision": origin_rev,
        "lane": "C117",
        "producer": "gemini",
        "slug": "contribution-lifecycle",
        "version": 1,
        "target_crate": "foundation_cli",
        "target_file": "src/contribution.rs",
        "instruction": "connect contribution lifecycle",
        "evidence_key": evidence_key,
        "verification_commands": [],
        "trailers": [
            format!("Contribution-ID: {cid}"),
            "Co-Authored-By: Gemini <gemini@aarogaming.local>"
        ],
        "created_at_secs": 1728470000
    });

    fs::write(
        &env_path,
        serde_json::to_string_pretty(&envelope_json).unwrap(),
    )
    .unwrap();

    let out = run_foundation(&[
        "inspect-contribution",
        "--envelope",
        env_path.to_str().unwrap(),
    ]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("foundation.contribution-envelope"));
    assert!(stdout.contains(cid));
}

#[test]
fn test_contribution_envelope_with_virtual_patch_binding() {
    let temp = tempfile::tempdir().unwrap();
    let env_path = temp.path().join("envelope.json");

    let cid = "C117/gemini-contribution-lifecycle-v1";
    let origin_rev = "47360c238b722d3e0b4931a7f058097b830d97fe";
    let evidence_key = "b".repeat(64);
    let patch_digest = "c".repeat(64);

    let envelope_json = serde_json::json!({
        "schema_version": 1,
        "artifact_kind": "foundation.contribution-envelope",
        "contribution_id": cid,
        "origin_revision": origin_rev,
        "lane": "C117",
        "producer": "gemini",
        "slug": "contribution-lifecycle",
        "version": 1,
        "target_crate": "foundation_cli",
        "target_file": "src/contribution.rs",
        "instruction": "connect contribution lifecycle",
        "evidence_key": evidence_key,
        "patch_sha256": patch_digest,
        "virtual_patch_set_id": "VPS-C117-001",
        "verification_commands": [
            { "command": "cargo test -p foundation_cli", "exit_code": 0 }
        ],
        "trailers": [
            format!("Contribution-ID: {cid}"),
            "Co-Authored-By: Gemini <gemini@aarogaming.local>"
        ],
        "created_at_secs": 1728470000
    });

    fs::write(
        &env_path,
        serde_json::to_string_pretty(&envelope_json).unwrap(),
    )
    .unwrap();

    let out = run_foundation(&[
        "verify-contribution",
        "--envelope",
        env_path.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
