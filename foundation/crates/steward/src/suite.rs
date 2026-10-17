//! What a verification node runs and how it judges the run (lane C129, phase 2).
//!
//! A suite is a manifest in the repository (`ci/suites/<name>.toml`), read at the request's `base_sha` so a pull request cannot
//! change what verifies it. It is a list of **argument vectors**, never a shell string. The platform policy
//! (`ci/platform-policy.toml`) says which platforms a change must be verified on.
//!
//! Pure functions: no I/O, no clock, no environment. The node reads the manifest text from git and runs the processes; this
//! module parses and validates the text, checks that a request matches it and the node, and turns what happened into a verdict.

use crate::chain::digest;
use crate::verification::{Request, Verdict, required_platforms};
use std::collections::BTreeSet;
use toml::Value;

const MAX_STEPS: usize = 32;
const MAX_ARGS: usize = 64;
const MAX_ARG_LEN: usize = 4096;
const MAX_TIMEOUT_S: i64 = 7200;
const DEFAULT_TIMEOUT_S: u32 = 1800;

/// Programs that interpret a string as code; running one with an inline-script flag is a shell string in disguise.
const INTERPRETERS: [&str; 8] = [
    "sh",
    "bash",
    "zsh",
    "dash",
    "cmd",
    "cmd.exe",
    "powershell",
    "pwsh",
];
const INLINE_FLAGS: [&str; 6] = ["-c", "/c", "-command", "-encodedcommand", "-e", "/k"];

fn is_label(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some('a'..='z'))
        && text.chars().count() <= 48
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
}

fn plain(text: &str) -> bool {
    !text.chars().any(|c| c == '\0' || c == '\n' || c == '\r')
}

/// A relative path that stays inside the repository: no root, no drive, no `..`, no backslash, no empty segment.
fn relative_inside(path: &str) -> bool {
    if path.is_empty() {
        return true;
    }
    path.len() <= 200
        && plain(path)
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    pub argv: Vec<String>,
    /// Working directory inside the repository; empty means the repository root.
    pub cwd: String,
    pub timeout_s: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suite {
    pub name: String,
    pub platforms: Vec<String>,
    pub steps: Vec<Step>,
    /// SHA-256 of the manifest text exactly as stored in git. A request pins this value.
    pub digest: String,
}

fn only_keys(table: &toml::Table, allowed: &[&str], what: &str) -> Result<(), String> {
    match table.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(extra) => Err(format!("{what}: unknown key {extra:?}")),
        None => Ok(()),
    }
}

fn string_of<'a>(table: &'a toml::Table, key: &str, what: &str) -> Result<&'a str, String> {
    table
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{what}: {key} must be a string"))
}

fn labels_of(table: &toml::Table, key: &str, what: &str) -> Result<Vec<String>, String> {
    let list = table
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{what}: {key} must be a list"))?;
    let mut out = Vec::new();
    for item in list {
        let label = item
            .as_str()
            .filter(|s| is_label(s))
            .ok_or_else(|| format!("{what}: {key} entries must be short lower-case labels"))?;
        if out.iter().any(|seen| seen == label) {
            return Err(format!("{what}: {key} lists {label:?} twice"));
        }
        out.push(label.to_owned());
    }
    Ok(out)
}

fn parse_step(value: &Value, seen: &mut BTreeSet<String>) -> Result<Step, String> {
    let table = value.as_table().ok_or("step must be a table")?;
    only_keys(table, &["name", "argv", "cwd", "timeout_s"], "step")?;
    let name = string_of(table, "name", "step")?;
    if !is_label(name) {
        return Err(format!(
            "step name {name:?} must be a short lower-case label"
        ));
    }
    if !seen.insert(name.to_owned()) {
        return Err(format!("step {name:?} appears twice"));
    }
    let what = format!("step {name}");
    let raw = table.get("argv").and_then(Value::as_array).ok_or_else(|| {
        format!("{what}: argv must be a list of strings (a shell string is not accepted)")
    })?;
    if raw.is_empty() || raw.len() > MAX_ARGS {
        return Err(format!("{what}: argv must have 1 to {MAX_ARGS} entries"));
    }
    let mut argv = Vec::new();
    for item in raw {
        let arg = item
            .as_str()
            .ok_or_else(|| format!("{what}: every argv entry must be a string"))?;
        if arg.len() > MAX_ARG_LEN || !plain(arg) {
            return Err(format!(
                "{what}: an argv entry is too long or holds a newline or NUL"
            ));
        }
        argv.push(arg.to_owned());
    }
    let program = argv[0].as_str();
    if program.is_empty() {
        return Err(format!("{what}: argv[0] must name a program"));
    }
    if program.contains('\\')
        || program.contains(':')
        || program.starts_with('/')
        || program.contains("..")
    {
        return Err(format!(
            "{what}: argv[0] must be a bare program name or a path inside the repository"
        ));
    }
    let base = program
        .rsplit('/')
        .next()
        .unwrap_or(program)
        .to_ascii_lowercase();
    if INTERPRETERS.contains(&base.as_str())
        && argv[1..]
            .iter()
            .any(|a| INLINE_FLAGS.contains(&a.to_ascii_lowercase().as_str()))
    {
        return Err(format!(
            "{what}: {program} with an inline script flag is a shell string; put the script in a file"
        ));
    }
    let cwd = match table.get("cwd") {
        None => String::new(),
        Some(v) => v
            .as_str()
            .filter(|s| relative_inside(s))
            .ok_or_else(|| format!("{what}: cwd must be a relative path inside the repository"))?
            .to_owned(),
    };
    let timeout_s = match table.get("timeout_s") {
        None => DEFAULT_TIMEOUT_S,
        Some(v) => v
            .as_integer()
            .filter(|n| (1..=MAX_TIMEOUT_S).contains(n))
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| format!("{what}: timeout_s must be 1 to {MAX_TIMEOUT_S}"))?,
    };
    Ok(Step {
        name: name.to_owned(),
        argv,
        cwd,
        timeout_s,
    })
}

/// Parse and validate a suite manifest. Strict: an unknown key, a shell string, a path that leaves the repository, or an
/// argument vector that smuggles an inline script is an error, not a warning.
///
/// # Errors
/// A sentence naming the first thing wrong.
pub fn parse_suite(text: &str) -> Result<Suite, String> {
    let table: toml::Table = text
        .parse()
        .map_err(|e| format!("manifest is not valid TOML: {e}"))?;
    only_keys(&table, &["name", "platforms", "step"], "manifest")?;
    let name = string_of(&table, "name", "manifest")?;
    if !is_label(name) {
        return Err(format!(
            "suite name {name:?} must be a short lower-case label"
        ));
    }
    let platforms = labels_of(&table, "platforms", "manifest")?;
    if platforms.is_empty() {
        return Err("manifest: platforms must not be empty".to_owned());
    }
    let raw_steps = table
        .get("step")
        .and_then(Value::as_array)
        .ok_or("manifest: at least one [[step]] is required")?;
    if raw_steps.is_empty() || raw_steps.len() > MAX_STEPS {
        return Err(format!("manifest: 1 to {MAX_STEPS} steps are allowed"));
    }
    let mut seen = BTreeSet::new();
    let steps = raw_steps
        .iter()
        .map(|s| parse_step(s, &mut seen))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Suite {
        name: name.to_owned(),
        platforms,
        steps,
        digest: digest(text),
    })
}

/// Which platforms a change must be verified on, as read from `ci/platform-policy.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub default: Vec<String>,
    pub rules: Vec<(String, String)>,
}

impl Policy {
    /// # Errors
    /// A sentence naming the first thing wrong.
    pub fn parse(text: &str) -> Result<Self, String> {
        let table: toml::Table = text
            .parse()
            .map_err(|e| format!("policy is not valid TOML: {e}"))?;
        only_keys(&table, &["default", "rule"], "policy")?;
        let default = labels_of(&table, "default", "policy")?;
        if default.is_empty() {
            return Err("policy: default must name at least one platform".to_owned());
        }
        let mut rules = Vec::new();
        if let Some(raw) = table.get("rule") {
            for item in raw
                .as_array()
                .ok_or("policy: rule must be a list of tables")?
            {
                let rule = item.as_table().ok_or("policy: rule must be a table")?;
                only_keys(rule, &["prefix", "platform"], "policy rule")?;
                let prefix = string_of(rule, "prefix", "policy rule")?;
                let platform = string_of(rule, "platform", "policy rule")?;
                if prefix.is_empty()
                    || !relative_inside(prefix.strip_suffix('/').unwrap_or(prefix))
                    || !is_label(platform)
                {
                    return Err(format!(
                        "policy rule {prefix:?} -> {platform:?}: the prefix must be a relative path inside the repository and the platform a label"
                    ));
                }
                rules.push((prefix.to_owned(), platform.to_owned()));
            }
        }
        Ok(Self { default, rules })
    }

    /// The platforms a change touching `changed_paths` must pass on.
    #[must_use]
    pub fn required(&self, changed_paths: &[String]) -> Vec<String> {
        required_platforms(&self.default, &self.rules, changed_paths)
    }
}

/// What a node will run for one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub steps: Vec<Step>,
}

/// Decide whether this node should run `request` with `suite`. The suite is the manifest the node itself read from git at
/// the request's `base_sha`; a request that disagrees with it, names another platform than this node's, or asks for a suite
/// that does not support the platform is refused, never adapted.
///
/// # Errors
/// A sentence naming why the request is refused.
pub fn plan(request: &Request, suite: &Suite, node_platform: &str) -> Result<Plan, String> {
    if let Some(problem) = request.problem() {
        return Err(format!("malformed request: {problem}"));
    }
    if request.platform != node_platform {
        return Err(format!(
            "misrouted: request is for {} and this node is {node_platform}",
            request.platform
        ));
    }
    if request.suite != suite.name {
        return Err(format!(
            "request names suite {:?} but the manifest is {:?}",
            request.suite, suite.name
        ));
    }
    if request.suite_digest != suite.digest {
        return Err(
            "the manifest at base_sha does not match the request's suite_digest".to_owned(),
        );
    }
    if !suite.platforms.iter().any(|p| p == node_platform) {
        return Err(format!(
            "suite {} does not support {node_platform}",
            suite.name
        ));
    }
    Ok(Plan {
        steps: suite.steps.clone(),
    })
}

/// How one step went, as the node observed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepResult {
    pub name: String,
    /// False when the process could not be started at all.
    pub started: bool,
    /// The exit code, or `None` when the process was ended by a signal.
    pub exit: Option<i32>,
    pub timed_out: bool,
}

/// The verdict for a run. Steps run in order and stop at the first that does not pass.
///
/// - Every planned step started, finished inside its timeout and exited 0: `Pass`.
/// - A step ran and exited non-zero, or exceeded its timeout: `Fail` (a hung test is a finding about the code).
/// - A step could not start, was ended by a signal (an out-of-memory kill is not a verdict), the results do not line up with
///   the plan, or steps are missing without a failure to explain it: `Error`. Never a pass.
#[must_use]
pub fn summarize(plan: &Plan, results: &[StepResult]) -> Verdict {
    if plan.steps.is_empty() {
        return Verdict::Error;
    }
    for (index, step) in plan.steps.iter().enumerate() {
        let Some(result) = results.get(index) else {
            return Verdict::Error;
        };
        if result.name != step.name {
            return Verdict::Error;
        }
        if !result.started {
            return Verdict::Error;
        }
        if result.timed_out {
            return Verdict::Fail;
        }
        match result.exit {
            Some(0) => {}
            Some(_) => return Verdict::Fail,
            None => return Verdict::Error,
        }
    }
    if results.len() > plan.steps.len() {
        return Verdict::Error;
    }
    Verdict::Pass
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUITE: &str = r#"
name = "foundation-ci"
platforms = ["linux-x86_64", "windows-x86_64"]

[[step]]
name = "fmt"
argv = ["cargo", "fmt", "--manifest-path", "foundation/Cargo.toml", "--all", "--", "--check"]
timeout_s = 600

[[step]]
name = "test"
argv = ["cargo", "test", "--manifest-path", "foundation/Cargo.toml"]
"#;

    fn with_step(body: &str) -> String {
        format!("name = \"s\"\nplatforms = [\"linux-x86_64\"]\n[[step]]\nname = \"a\"\n{body}\n")
    }

    #[test]
    fn a_manifest_parses_and_its_digest_is_the_hash_of_its_exact_text() {
        let suite = parse_suite(SUITE).expect("parses");
        assert_eq!(suite.name, "foundation-ci");
        assert_eq!(suite.steps.len(), 2);
        assert_eq!(suite.steps[0].timeout_s, 600);
        assert_eq!(suite.steps[1].timeout_s, DEFAULT_TIMEOUT_S);
        assert_eq!(suite.digest, digest(SUITE));
        let reordered = SUITE.replace("timeout_s = 600", "timeout_s = 601");
        assert_ne!(
            parse_suite(&reordered).unwrap().digest,
            suite.digest,
            "any byte changes the digest"
        );
    }

    #[test]
    fn a_shell_string_an_inline_script_or_a_path_that_escapes_is_refused() {
        for (why, body) in [
            ("shell string", "argv = \"cargo test\""),
            ("empty argv", "argv = []"),
            ("non-string entry", "argv = [\"cargo\", 7]"),
            ("empty program", "argv = [\"\"]"),
            ("absolute program", "argv = [\"/usr/bin/cargo\"]"),
            (
                "windows path program",
                "argv = [\"C:\\\\tools\\\\cargo.exe\"]",
            ),
            ("program escapes", "argv = [\"../cargo\"]"),
            (
                "bash -c",
                "argv = [\"bash\", \"-c\", \"cargo test && curl evil\"]",
            ),
            ("sh -c", "argv = [\"sh\", \"-c\", \"x\"]"),
            ("cmd /c", "argv = [\"cmd.exe\", \"/C\", \"x\"]"),
            ("pwsh -Command", "argv = [\"pwsh\", \"-Command\", \"x\"]"),
            ("newline in an argument", "argv = [\"cargo\", \"a\\nb\"]"),
            ("cwd escapes", "argv = [\"cargo\"]\ncwd = \"../x\""),
            ("absolute cwd", "argv = [\"cargo\"]\ncwd = \"/etc\""),
            ("drive cwd", "argv = [\"cargo\"]\ncwd = \"C:/x\""),
            ("backslash cwd", "argv = [\"cargo\"]\ncwd = \"a\\\\b\""),
            ("zero timeout", "argv = [\"cargo\"]\ntimeout_s = 0"),
            ("huge timeout", "argv = [\"cargo\"]\ntimeout_s = 999999"),
            ("unknown key", "argv = [\"cargo\"]\nenv = \"X=1\""),
        ] {
            assert!(
                parse_suite(&with_step(body)).is_err(),
                "{why} must be refused"
            );
        }
        assert!(
            parse_suite(&with_step("argv = [\"bash\", \"scripts/check.sh\"]")).is_ok(),
            "a script file is fine"
        );
        assert!(
            parse_suite(&with_step(
                "argv = [\"cargo\", \"test\"]\ncwd = \"foundation\""
            ))
            .is_ok()
        );
    }

    #[test]
    fn manifest_level_mistakes_are_refused() {
        for (why, text) in [
            ("no steps", "name = \"s\"\nplatforms = [\"linux-x86_64\"]\n"),
            (
                "no platforms",
                "name = \"s\"\nplatforms = []\n[[step]]\nname=\"a\"\nargv=[\"cargo\"]\n",
            ),
            (
                "duplicate platform",
                "name = \"s\"\nplatforms = [\"a\", \"a\"]\n[[step]]\nname=\"a\"\nargv=[\"cargo\"]\n",
            ),
            (
                "bad name",
                "name = \"S\"\nplatforms = [\"a\"]\n[[step]]\nname=\"a\"\nargv=[\"cargo\"]\n",
            ),
            (
                "unknown top-level key",
                "name = \"s\"\nplatforms = [\"a\"]\nrun = \"x\"\n[[step]]\nname=\"a\"\nargv=[\"cargo\"]\n",
            ),
            (
                "duplicate step",
                "name = \"s\"\nplatforms = [\"a\"]\n[[step]]\nname=\"a\"\nargv=[\"x\"]\n[[step]]\nname=\"a\"\nargv=[\"y\"]\n",
            ),
            ("not toml", "name = = ="),
        ] {
            assert!(parse_suite(text).is_err(), "{why}");
        }
        let many: String = (0..=MAX_STEPS)
            .map(|i| format!("[[step]]\nname=\"s{i}\"\nargv=[\"x\"]\n"))
            .collect();
        assert!(
            parse_suite(&format!("name=\"s\"\nplatforms=[\"a\"]\n{many}")).is_err(),
            "too many steps"
        );
    }

    const POLICY: &str = r#"
default = ["linux-x86_64"]

[[rule]]
prefix = "foundation/crates/platform_bridge/"
platform = "windows-x86_64"
"#;

    #[test]
    fn the_policy_adds_platforms_by_path_and_refuses_nonsense() {
        let policy = Policy::parse(POLICY).expect("parses");
        assert_eq!(
            policy.required(&["governance/x.md".into()]),
            ["linux-x86_64"]
        );
        assert_eq!(
            policy.required(&["foundation/crates/platform_bridge/src/lib.rs".into()]),
            ["linux-x86_64", "windows-x86_64"]
        );
        for bad in [
            "default = []",
            "default = [\"linux-x86_64\"]\n[[rule]]\nprefix = \"\"\nplatform = \"windows-x86_64\"",
            "default = [\"linux-x86_64\"]\n[[rule]]\nprefix = \"../x\"\nplatform = \"windows-x86_64\"",
            "default = [\"linux-x86_64\"]\n[[rule]]\nprefix = \"a/\"\nplatform = \"Windows\"",
            "default = [\"linux-x86_64\"]\n[[rule]]\nprefix = \"a/\"\nplatform = \"w\"\nextra = 1",
            "default = [\"linux-x86_64\"]\nmode = \"x\"",
        ] {
            assert!(Policy::parse(bad).is_err(), "{bad}");
        }
    }

    fn request_for(suite: &Suite, platform: &str) -> Request {
        Request {
            repo: "Aarogaming/aaroneous-devtools".into(),
            head: "a".repeat(40),
            base_sha: "b".repeat(40),
            platform: platform.into(),
            suite: suite.name.clone(),
            suite_digest: suite.digest.clone(),
        }
    }

    #[test]
    fn a_node_runs_only_what_matches_its_platform_and_the_manifest_it_read_itself() {
        let suite = parse_suite(SUITE).unwrap();
        assert_eq!(
            plan(&request_for(&suite, "linux-x86_64"), &suite, "linux-x86_64")
                .unwrap()
                .steps
                .len(),
            2
        );
        let misrouted = plan(
            &request_for(&suite, "windows-x86_64"),
            &suite,
            "linux-x86_64",
        );
        assert!(misrouted.unwrap_err().contains("misrouted"));
        let mut wrong_digest = request_for(&suite, "linux-x86_64");
        wrong_digest.suite_digest = "d".repeat(64);
        assert!(
            plan(&wrong_digest, &suite, "linux-x86_64")
                .unwrap_err()
                .contains("suite_digest")
        );
        let mut wrong_name = request_for(&suite, "linux-x86_64");
        wrong_name.suite = "other".into();
        assert!(plan(&wrong_name, &suite, "linux-x86_64").is_err());
        let mut malformed = request_for(&suite, "linux-x86_64");
        malformed.head = "x".into();
        assert!(
            plan(&malformed, &suite, "linux-x86_64")
                .unwrap_err()
                .contains("malformed")
        );
        let linux_only =
            parse_suite(&SUITE.replace("\"linux-x86_64\", \"windows-x86_64\"", "\"linux-x86_64\""))
                .unwrap();
        assert!(
            plan(
                &request_for(&linux_only, "windows-x86_64"),
                &linux_only,
                "windows-x86_64"
            )
            .unwrap_err()
            .contains("does not support")
        );
    }

    fn two_step_plan() -> Plan {
        Plan {
            steps: parse_suite(SUITE).unwrap().steps,
        }
    }

    fn done(name: &str, exit: Option<i32>) -> StepResult {
        StepResult {
            name: name.into(),
            started: true,
            exit,
            timed_out: false,
        }
    }

    #[test]
    fn the_verdict_follows_the_steps_and_never_rounds_up() {
        let plan = two_step_plan();
        assert_eq!(
            summarize(&plan, &[done("fmt", Some(0)), done("test", Some(0))]),
            Verdict::Pass
        );
        assert_eq!(
            summarize(&plan, &[done("fmt", Some(0)), done("test", Some(101))]),
            Verdict::Fail
        );
        assert_eq!(
            summarize(&plan, &[done("fmt", Some(1))]),
            Verdict::Fail,
            "stops at the first failure"
        );
        assert_eq!(
            summarize(
                &plan,
                &[StepResult {
                    timed_out: true,
                    exit: None,
                    ..done("fmt", None)
                }]
            ),
            Verdict::Fail
        );
        assert_eq!(
            summarize(
                &plan,
                &[StepResult {
                    started: false,
                    exit: None,
                    ..done("fmt", None)
                }]
            ),
            Verdict::Error
        );
        assert_eq!(
            summarize(&plan, &[done("fmt", None)]),
            Verdict::Error,
            "a signal kill is not a verdict"
        );
        assert_eq!(
            summarize(&plan, &[done("fmt", Some(0)), done("test", None)]),
            Verdict::Error,
            "a kill on the last step is not a pass"
        );
        assert_eq!(
            summarize(&plan, &[done("fmt", Some(0))]),
            Verdict::Error,
            "a missing step is not a pass"
        );
        assert_eq!(summarize(&plan, &[]), Verdict::Error);
        assert_eq!(
            summarize(&plan, &[done("test", Some(0)), done("fmt", Some(0))]),
            Verdict::Error,
            "out of order"
        );
        assert_eq!(
            summarize(
                &plan,
                &[
                    done("fmt", Some(0)),
                    done("test", Some(0)),
                    done("extra", Some(0))
                ]
            ),
            Verdict::Error
        );
        assert_eq!(
            summarize(&Plan { steps: vec![] }, &[]),
            Verdict::Error,
            "an empty plan verifies nothing"
        );
    }
}
