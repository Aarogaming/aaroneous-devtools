use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path},
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
