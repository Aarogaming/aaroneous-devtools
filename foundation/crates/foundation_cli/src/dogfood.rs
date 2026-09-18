use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{fs, path::Path, process::Command};

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
}
