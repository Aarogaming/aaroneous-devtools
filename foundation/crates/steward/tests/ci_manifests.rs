//! The committed suite manifests and platform policy must stay valid, agree with each other, and be what the nodes will run.
//! Reads the real files in `ci/` (lane C129).

use std::fs;
use std::path::PathBuf;
use steward::suite::{Policy, parse_suite};

fn ci_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../ci")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

#[test]
fn every_committed_suite_is_valid_and_named_for_its_file() {
    let mut seen = 0;
    for entry in fs::read_dir(ci_dir().join("suites")).expect("ci/suites exists") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let suite =
            parse_suite(&read(path.clone())).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(
            path.file_stem().and_then(|s| s.to_str()),
            Some(suite.name.as_str()),
            "the suite name must match its file name"
        );
        seen += 1;
    }
    assert!(seen >= 1, "at least one suite is committed");
}

#[test]
fn the_policy_is_valid_and_every_platform_it_can_require_has_a_suite_that_runs_there() {
    let policy =
        Policy::parse(&read(ci_dir().join("platform-policy.toml"))).expect("policy parses");
    let suite =
        parse_suite(&read(ci_dir().join("suites/foundation-ci.toml"))).expect("suite parses");
    let mut platforms: Vec<String> = policy.default.clone();
    platforms.extend(policy.rules.iter().map(|(_, p)| p.clone()));
    for platform in platforms {
        assert!(
            suite.platforms.contains(&platform),
            "the policy can require {platform}, but foundation-ci does not list it"
        );
    }
}

#[test]
fn windows_specific_code_is_verified_on_windows_before_merge() {
    // Codex review of #199: the Windows-only path containment in foundation_cli was not covered by the first policy.
    let policy =
        Policy::parse(&read(ci_dir().join("platform-policy.toml"))).expect("policy parses");
    for path in [
        "foundation/crates/foundation_cli/src/agentic_registrar.rs",
        "foundation/crates/foundation_cli/src/local_worker.rs",
        "foundation/crates/maintenance_ui/src/main.rs",
        "foundation/Cargo.lock",
        "stacks/local-rust.toml",
        "ci/suites/foundation-ci.toml",
        ".cargo/config.toml",
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain",
        "rust-toolchain.toml",
        ".github/workflows/foundation-ci.yml",
        ".github/workflows/foundation-ci-windows.yml",
    ] {
        let required = policy.required(&[path.to_owned()]);
        assert!(
            required.contains(&"windows-x86_64".to_owned()),
            "{path} must require Windows, got {required:?}"
        );
        assert!(
            required.contains(&"linux-x86_64".to_owned()),
            "{path} must require Linux, got {required:?}"
        );
    }
    // a pure document change stays Linux-only
    assert_eq!(
        policy.required(&["governance/x.md".to_owned()]),
        ["linux-x86_64"]
    );
}

#[test]
fn foundation_ci_runs_the_same_commands_as_the_workflow_it_replaces() {
    let suite =
        parse_suite(&read(ci_dir().join("suites/foundation-ci.toml"))).expect("suite parses");
    // The live workflow (whichever runners it uses); its workspace expression is the repository root for a node.
    let workflow = read(ci_dir().join("../.github/workflows/foundation-ci.yml"))
        .replace("${{ github.workspace }}/", "");
    let commands: Vec<&str> = workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("run: "))
        .map(str::trim)
        .collect();
    for step in &suite.steps {
        let joined = step.argv.join(" ");
        assert!(
            commands.contains(&joined.as_str()),
            "step {} ({joined}) is not exactly a `run:` command in .github/workflows/foundation-ci.yml",
            step.name
        );
    }
}
