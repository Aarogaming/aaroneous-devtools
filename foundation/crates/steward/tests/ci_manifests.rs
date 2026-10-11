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
