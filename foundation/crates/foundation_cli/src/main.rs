use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "foundation",
    about = "Explicit-input development foundation tools"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate declared paths and write a versioned input report.
    ValidateInputs {
        /// Absolute path to an Aaroneous checkout.
        #[arg(long)]
        checkout: PathBuf,
        /// Absolute path for the report file; its parent must already exist.
        #[arg(long)]
        output: PathBuf,
        /// Additional absolute support directories, such as models, datasets, or toolchains.
        #[arg(long = "support-dir")]
        support_dirs: Vec<PathBuf>,
    },
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct InputReport {
    schema_version: u32,
    checkout: String,
    output: String,
    support_directories: Vec<String>,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::ValidateInputs {
            checkout,
            output,
            support_dirs,
        } => validate_inputs(&checkout, &output, &support_dirs),
    }
}

fn validate_inputs(checkout: &Path, output: &Path, support_dirs: &[PathBuf]) -> Result<()> {
    require_absolute_directory("checkout", checkout)?;
    if !checkout.join(".git").exists() {
        bail!(
            "checkout must contain a .git file or directory: {}",
            checkout.display()
        );
    }
    if !output.is_absolute() {
        bail!("output must be an absolute path: {}", output.display());
    }
    let parent = output
        .parent()
        .context("output path must have an existing parent directory")?;
    require_absolute_directory("output parent", parent)?;
    for support_dir in support_dirs {
        require_absolute_directory("support directory", support_dir)?;
    }

    let report = InputReport {
        schema_version: 1,
        checkout: checkout.display().to_string(),
        output: output.display().to_string(),
        support_directories: support_dirs
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
    };
    let json = serde_json::to_vec_pretty(&report).context("failed to serialize input report")?;
    fs::write(output, json).with_context(|| format!("failed to write {}", output.display()))?;
    println!("validated explicit foundation inputs: {}", output.display());
    Ok(())
}

fn require_absolute_directory(label: &str, path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("{label} must be an absolute path: {}", path.display());
    }
    let metadata = fs::metadata(path)
        .with_context(|| format!("{label} is unavailable: {}", path.display()))?;
    if !metadata.is_dir() {
        bail!("{label} must be a directory: {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_checkout() {
        let temp = tempfile::tempdir().expect("test directory");
        let error = validate_inputs(Path::new("relative"), &temp.path().join("report.json"), &[])
            .expect_err("relative checkout must be rejected");
        assert!(error.to_string().contains("absolute"));
    }

    #[test]
    fn records_declared_support_directories() {
        let temp = tempfile::tempdir().expect("test directory");
        let checkout = temp.path().join("checkout");
        let support = temp.path().join("models");
        fs::create_dir_all(checkout.join(".git")).expect("checkout metadata");
        fs::create_dir_all(&support).expect("support directory");
        let output = temp.path().join("report.json");

        validate_inputs(&checkout, &output, &[support.clone()]).expect("valid inputs");
        let report: InputReport =
            serde_json::from_slice(&fs::read(output).expect("report")).expect("valid report");
        assert_eq!(report.schema_version, 1);
        assert_eq!(
            report.support_directories,
            vec![support.display().to_string()]
        );
    }
}
