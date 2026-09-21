use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

#[derive(Serialize)]
struct Report {
    schema_version: u32,
    artifact_kind: String,
    checkout: String,
    operation: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Serialize, Deserialize)]
struct DraftVerificationReport {
    schema_version: u32,
    artifact_kind: String,
    checkout: String,
    target_file: String,
    command: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}
pub(crate) fn metadata(checkout: &Path, output: &Path) -> Result<()> {
    if !checkout.is_absolute() || !checkout.join(".git").exists() {
        bail!("checkout must be an absolute Git checkout");
    }
    if !output.is_absolute()
        || !fs::metadata(output.parent().context("output parent")?)
            .with_context(|| "output parent unavailable")?
            .is_dir()
    {
        bail!("output needs existing absolute parent");
    }
    let result = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version=1"])
        .current_dir(checkout)
        .output()
        .context("failed to start cargo metadata")?;
    let report = Report {
        schema_version: 1,
        artifact_kind: "foundation.dogfood-result".to_owned(),
        checkout: checkout.display().to_string(),
        operation: "cargo metadata --no-deps --format-version=1".to_owned(),
        exit_code: result.status.code(),
        stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
    };
    fs::write(
        output,
        serde_json::to_vec_pretty(&report).context("serialize dogfood report")?,
    )
    .with_context(|| format!("write {}", output.display()))?;
    if !result.status.success() {
        bail!("cargo metadata failed; see report");
    }
    Ok(())
}

/// Writes `draft` to `checkout.join(target_file)` and runs a caller-supplied,
/// caller-scoped `cargo` verification command against `checkout`, capturing the
/// outcome as a versioned report at `output`.
///
/// This never creates, initializes, or otherwise mutates the checkout beyond
/// writing exactly `target_file` — `checkout` must already exist and be a valid
/// Git checkout supplied by the caller (see `FOUNDATION_BOUNDARY.md`'s "no
/// automatic Aaroneous source mutation" rule). A failing verification command is
/// an expected, valid outcome (the draft didn't pass) and is captured in the
/// report rather than surfaced as an error; only genuine tool-usage errors
/// (invalid checkout, path traversal, unwritable target) `bail!`.
pub(crate) fn verify_draft(
    checkout: &Path,
    target_file: &Path,
    draft: &str,
    verify_command: &[String],
    output: &Path,
) -> Result<()> {
    if !checkout.is_absolute() || !checkout.join(".git").exists() {
        bail!("checkout must be an absolute Git checkout");
    }
    require_safe_relative_path("target file", target_file)?;
    if !output.is_absolute()
        || !fs::metadata(output.parent().context("output parent")?)
            .with_context(|| "output parent unavailable")?
            .is_dir()
    {
        bail!("output needs existing absolute parent");
    }
    if verify_command.is_empty() {
        bail!("verify_command must not be empty");
    }
    let destination = checkout.join(target_file);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to prepare {}", parent.display()))?;
    }
    fs::write(&destination, draft)
        .with_context(|| format!("failed to write {}", destination.display()))?;
    let result = Command::new("cargo")
        .args(verify_command)
        .current_dir(checkout)
        .output()
        .context("failed to start cargo verification command")?;
    let report = DraftVerificationReport {
        schema_version: 1,
        artifact_kind: "foundation.draft-verification-result".to_owned(),
        checkout: checkout.display().to_string(),
        target_file: target_file.to_string_lossy().replace('\\', "/"),
        command: format!("cargo {}", verify_command.join(" ")),
        exit_code: result.status.code(),
        stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
    };
    fs::write(
        output,
        serde_json::to_vec_pretty(&report).context("serialize draft verification report")?,
    )
    .with_context(|| format!("write {}", output.display()))?;
    Ok(())
}

/// Constructs the `cargo` verify-command args that run Aaroneous's own
/// `cratify audit` compliance check against `cratify_target` (a path
/// relative to the checkout `verify_draft` writes the draft into, e.g.
/// `crates/some_crate` or a single file). Pulled out as a pure function so
/// the exact argv `cratify_audit` builds is unit-testable without actually
/// invoking `cargo`.
fn cratify_audit_command(cratify_target: &Path) -> Vec<String> {
    vec![
        "run".to_owned(),
        "-p".to_owned(),
        "cratify".to_owned(),
        "--".to_owned(),
        "audit".to_owned(),
        cratify_target.to_string_lossy().replace('\\', "/"),
    ]
}

/// Convenience wrapper around `verify_draft` that runs the drafted file
/// through Aaroneous's own `cratify audit` compliance pipeline instead of
/// requiring every caller to hand-construct the
/// `cargo run -p cratify -- audit <target>` `verify_command` themselves.
///
/// This is deliberately thin: `verify_draft`'s `verify_command` is already
/// generic (`Command::new("cargo").args(verify_command)`), so nothing about
/// the underlying mechanism needed to change for `cratify audit` to work as
/// a verification command — this function only saves callers from repeating
/// the fixed argv. `cratify_target` must be a safe relative path (validated
/// the same way as `target_file`) inside `checkout`; it is typically the
/// same crate/directory `target_file` lives in.
///
/// Note on report fidelity: `cratify audit` (via `ast_auditor`) prints
/// human-readable diagnostics only — it has no machine-parseable/JSON output
/// mode as of this writing. `verify_draft`'s `DraftVerificationReport`
/// (exit_code/stdout/stderr) is therefore not parsed into structured
/// findings here; the exit code is still a real pass/fail compliance
/// signal, and a human/reviewer reads the captured stdout in the report for
/// audit detail.
pub(crate) fn cratify_audit(
    checkout: &Path,
    target_file: &Path,
    draft: &str,
    cratify_target: &Path,
    output: &Path,
) -> Result<()> {
    require_safe_relative_path("cratify target", cratify_target)?;
    let verify_command = cratify_audit_command(cratify_target);
    verify_draft(checkout, target_file, draft, &verify_command, output)
}

/// A named, extensible compliance ruleset selectable for `verify_draft`-based
/// verification (M53). `AaroneousStrict` and `DevtoolsOperatingModel` are
/// deliberately **not** interchangeable — see `governance/
/// REVIEW_CONDITIONS.md`'s devtools conditions table, which states plainly
/// which applies to an Aaroneous-targeted draft vs. a devtools-targeted one.
/// Running `AaroneousStrict` against devtools code would flag devtools' own
/// normal, correct-for-devtools patterns (heap allocation, async I/O) as
/// violations, not because the code is wrong but because it would be checked
/// against the wrong ruleset for that target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ComplianceProfile {
    /// Aaroneous's own compliance ruleset — `cargo run -p cratify -- audit
    /// <cratify_target>`. This is the exact preset `cratify_audit` (M45)
    /// already runs; `cratify_target` is a safe relative path (validated the
    /// same way as `verify_draft`'s `target_file`) inside `checkout`. Only
    /// correct for a checkout of the Aaroneous repository itself.
    AaroneousStrict { cratify_target: PathBuf },
    /// Devtools' own Foundation CI lint check — `cargo clippy
    /// --manifest-path foundation/Cargo.toml -- -D warnings`, matching
    /// `.github/workflows/foundation-ci.yml`'s lint step exactly. This
    /// assumes `checkout` is a devtools repo root containing
    /// `foundation/Cargo.toml`; it is not meaningful against an arbitrary
    /// checkout that lacks that layout.
    DevtoolsOperatingModel,
    /// Pure passthrough: `verify_draft`'s existing fully-generic behavior,
    /// unchanged, for anything not worth a named preset.
    Custom(Vec<String>),
}

/// Resolves a `ComplianceProfile` into the concrete `verify_command`
/// `verify_draft` already accepts. Pure function (no filesystem/subprocess
/// access), so each profile's exact argv is unit-testable without actually
/// invoking `cargo`.
///
/// `AaroneousStrict` is resolved via the same `cratify_audit_command` helper
/// `cratify_audit` (M45) itself calls — not a separately-maintained copy of
/// the same argv — so this profile and `cratify_audit`'s existing behavior
/// stay identical by construction rather than by coincidence.
pub(crate) fn resolve_compliance_profile(profile: &ComplianceProfile) -> Vec<String> {
    match profile {
        ComplianceProfile::AaroneousStrict { cratify_target } => {
            cratify_audit_command(cratify_target)
        }
        ComplianceProfile::DevtoolsOperatingModel => vec![
            "clippy".to_owned(),
            "--manifest-path".to_owned(),
            "foundation/Cargo.toml".to_owned(),
            "--".to_owned(),
            "-D".to_owned(),
            "warnings".to_owned(),
        ],
        ComplianceProfile::Custom(command) => command.clone(),
    }
}

/// Thin wrapper over `verify_draft` that resolves a named `ComplianceProfile`
/// into a concrete `verify_command` first, then delegates unchanged. Adds no
/// verification logic beyond that resolution (and, for `AaroneousStrict`,
/// the same `cratify_target` path-safety check `cratify_audit` performs).
pub(crate) fn verify_draft_with_profile(
    checkout: &Path,
    target_file: &Path,
    draft: &str,
    profile: &ComplianceProfile,
    output: &Path,
) -> Result<()> {
    if let ComplianceProfile::AaroneousStrict { cratify_target } = profile {
        require_safe_relative_path("cratify target", cratify_target)?;
    }
    let verify_command = resolve_compliance_profile(profile);
    verify_draft(checkout, target_file, draft, &verify_command, output)
}

fn require_safe_relative_path(label: &str, path: &Path) -> Result<()> {
    if path.is_absolute() || path.as_os_str().is_empty() {
        bail!(
            "{label} must be a non-empty relative path: {}",
            path.display()
        );
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!(
            "{label} may contain normal path segments only: {}",
            path.display()
        );
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn find_prebuilt_binary(
    binary_name: &str,
    checkout: &Path,
) -> Option<std::path::PathBuf> {
    let candidate_dirs = [
        std::path::PathBuf::from(r"C:\CargoTargetCache\release"),
        std::path::PathBuf::from(r"C:\CargoTargetCache\debug"),
        checkout.join("target").join("release"),
        checkout.join("target").join("debug"),
    ];
    let exe_name = format!("{binary_name}.exe");
    for dir in candidate_dirs {
        let p = dir.join(&exe_name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_prebuilt_binary_locates_local_binary() {
        let temp = tempfile::tempdir().unwrap();
        let target_dir = temp.path().join("target").join("release");
        std::fs::create_dir_all(&target_dir).unwrap();
        let bin_path = target_dir.join("dummy_tool.exe");
        std::fs::write(&bin_path, b"dummy").unwrap();

        let found = find_prebuilt_binary("dummy_tool", temp.path());
        assert_eq!(found, Some(bin_path));
    }

    fn checkout_fixture() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join(".git")).unwrap();
        temp
    }

    #[test]
    fn verify_draft_reports_success_for_a_passing_command() {
        let temp = checkout_fixture();
        let output = temp.path().join("report.json");
        verify_draft(
            temp.path(),
            Path::new("src/draft.rs"),
            "fn reduce() {}",
            &["--version".to_owned()],
            &output,
        )
        .expect("verify_draft should not error on a passing command");
        assert_eq!(
            fs::read_to_string(temp.path().join("src/draft.rs")).unwrap(),
            "fn reduce() {}"
        );
        let report: DraftVerificationReport =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(report.exit_code, Some(0));
        assert_eq!(report.target_file, "src/draft.rs");
    }

    #[test]
    fn verify_draft_captures_failure_without_erroring() {
        let temp = checkout_fixture();
        let output = temp.path().join("report.json");
        verify_draft(
            temp.path(),
            Path::new("src/draft.rs"),
            "fn reduce() {}",
            &["definitely-not-a-real-cargo-subcommand".to_owned()],
            &output,
        )
        .expect("a failing verification command is a valid outcome, not a tool error");
        let report: DraftVerificationReport =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_ne!(report.exit_code, Some(0));
    }

    #[test]
    fn cratify_audit_command_builds_expected_argv() {
        assert_eq!(
            cratify_audit_command(Path::new("crates/some_crate")),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "cratify".to_owned(),
                "--".to_owned(),
                "audit".to_owned(),
                "crates/some_crate".to_owned(),
            ]
        );
    }

    #[test]
    fn cratify_audit_writes_draft_and_runs_verify_draft_with_cratify_command() {
        let temp = checkout_fixture();
        let output = temp.path().join("report.json");
        // `cargo run -p cratify -- audit ...` is not buildable inside this
        // synthetic checkout fixture, so this only exercises that
        // `cratify_audit` delegates to `verify_draft` (writes the draft,
        // captures a failing-but-not-erroring report) rather than actually
        // proving a real cratify audit passes/fails — that is exercised
        // separately against a real Aaroneous checkout (see task report).
        cratify_audit(
            temp.path(),
            Path::new("src/draft.rs"),
            "fn reduce() {}",
            Path::new("src"),
            &output,
        )
        .expect("cratify_audit should not error just because cratify isn't buildable here");
        assert_eq!(
            fs::read_to_string(temp.path().join("src/draft.rs")).unwrap(),
            "fn reduce() {}"
        );
        let report: DraftVerificationReport =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(report.command, "cargo run -p cratify -- audit src");
    }

    #[test]
    fn cratify_audit_rejects_unsafe_cratify_target() {
        let temp = checkout_fixture();
        let output = temp.path().join("report.json");
        let result = cratify_audit(
            temp.path(),
            Path::new("src/draft.rs"),
            "fn reduce() {}",
            Path::new("../escape"),
            &output,
        );
        assert!(result.is_err());
    }

    #[test]
    fn resolve_compliance_profile_aaroneous_strict_matches_cratify_audit_command() {
        // Proves `AaroneousStrict` and `cratify_audit`'s own preset are
        // genuinely the same, not two separately-maintained copies: both
        // resolve through the identical `cratify_audit_command` helper.
        let cratify_target = Path::new("crates/some_crate");
        assert_eq!(
            resolve_compliance_profile(&ComplianceProfile::AaroneousStrict {
                cratify_target: cratify_target.to_path_buf(),
            }),
            cratify_audit_command(cratify_target)
        );
        assert_eq!(
            resolve_compliance_profile(&ComplianceProfile::AaroneousStrict {
                cratify_target: cratify_target.to_path_buf(),
            }),
            vec![
                "run".to_owned(),
                "-p".to_owned(),
                "cratify".to_owned(),
                "--".to_owned(),
                "audit".to_owned(),
                "crates/some_crate".to_owned(),
            ]
        );
    }

    #[test]
    fn resolve_compliance_profile_devtools_operating_model_matches_foundation_ci_lint_step() {
        assert_eq!(
            resolve_compliance_profile(&ComplianceProfile::DevtoolsOperatingModel),
            vec![
                "clippy".to_owned(),
                "--manifest-path".to_owned(),
                "foundation/Cargo.toml".to_owned(),
                "--".to_owned(),
                "-D".to_owned(),
                "warnings".to_owned(),
            ]
        );
    }

    #[test]
    fn resolve_compliance_profile_custom_is_unchanged_passthrough() {
        let command = vec!["check".to_owned(), "-p".to_owned(), "some_crate".to_owned()];
        assert_eq!(
            resolve_compliance_profile(&ComplianceProfile::Custom(command.clone())),
            command
        );
    }

    #[test]
    fn verify_draft_with_profile_custom_matches_verify_draft_directly() {
        // Proves the wrapper is a true passthrough for `Custom`: identical
        // command, identical outcome, modulo the `checkout`/`output` paths
        // themselves (each run needs its own fixture).
        let command = vec!["--version".to_owned()];

        let temp_a = checkout_fixture();
        let output_a = temp_a.path().join("report.json");
        verify_draft(
            temp_a.path(),
            Path::new("src/draft.rs"),
            "fn reduce() {}",
            &command,
            &output_a,
        )
        .expect("verify_draft should succeed");
        let report_a: DraftVerificationReport =
            serde_json::from_str(&fs::read_to_string(&output_a).unwrap()).unwrap();

        let temp_b = checkout_fixture();
        let output_b = temp_b.path().join("report.json");
        verify_draft_with_profile(
            temp_b.path(),
            Path::new("src/draft.rs"),
            "fn reduce() {}",
            &ComplianceProfile::Custom(command),
            &output_b,
        )
        .expect("verify_draft_with_profile should succeed");
        let report_b: DraftVerificationReport =
            serde_json::from_str(&fs::read_to_string(&output_b).unwrap()).unwrap();

        assert_eq!(report_a.command, report_b.command);
        assert_eq!(report_a.exit_code, report_b.exit_code);
        assert_eq!(report_a.target_file, report_b.target_file);
        assert_eq!(
            fs::read_to_string(temp_a.path().join("src/draft.rs")).unwrap(),
            fs::read_to_string(temp_b.path().join("src/draft.rs")).unwrap()
        );
    }

    #[test]
    fn verify_draft_with_profile_aaroneous_strict_writes_draft_and_runs_cratify_command() {
        let temp = checkout_fixture();
        let output = temp.path().join("report.json");
        // Mirrors `cratify_audit_writes_draft_and_runs_verify_draft_with_cratify_command`:
        // `cargo run -p cratify -- audit ...` isn't buildable inside this
        // synthetic fixture, so this exercises delegation (draft written,
        // failing-but-not-erroring report captured), not a real audit
        // pass/fail.
        verify_draft_with_profile(
            temp.path(),
            Path::new("src/draft.rs"),
            "fn reduce() {}",
            &ComplianceProfile::AaroneousStrict {
                cratify_target: Path::new("src").to_path_buf(),
            },
            &output,
        )
        .expect(
            "verify_draft_with_profile should not error just because cratify isn't buildable here",
        );
        assert_eq!(
            fs::read_to_string(temp.path().join("src/draft.rs")).unwrap(),
            "fn reduce() {}"
        );
        let report: DraftVerificationReport =
            serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        assert_eq!(report.command, "cargo run -p cratify -- audit src");
    }

    #[test]
    fn verify_draft_with_profile_rejects_unsafe_cratify_target() {
        let temp = checkout_fixture();
        let output = temp.path().join("report.json");
        let result = verify_draft_with_profile(
            temp.path(),
            Path::new("src/draft.rs"),
            "fn reduce() {}",
            &ComplianceProfile::AaroneousStrict {
                cratify_target: Path::new("../escape").to_path_buf(),
            },
            &output,
        );
        assert!(result.is_err());
    }

    #[test]
    fn verify_draft_rejects_path_traversal() {
        let temp = checkout_fixture();
        let output = temp.path().join("report.json");
        let result = verify_draft(
            temp.path(),
            Path::new("../evil.rs"),
            "fn reduce() {}",
            &["--version".to_owned()],
            &output,
        );
        assert!(result.is_err());
        assert!(!temp.path().parent().unwrap().join("evil.rs").exists());
    }
}
