use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

/// Result of ground-truth claim verification against a real repository checkout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FactCheckReport {
    pub passed_paths: Vec<String>,
    pub failed_paths: Vec<String>,
    pub passed_crates: Vec<String>,
    pub failed_crates: Vec<String>,
    pub is_valid: bool,
}

/// Known repository root directories that unambiguously identify repo-relative paths.
const KNOWN_ROOT_DIRS: &[&str] = &[
    "crates/",
    "core/",
    "dev/",
    "architecture/",
    "governance/",
    "contracts/",
    "foundation/",
    "xtask/",
    "docs/",
    "scripts/",
    "worker-jobs/",
    "artifacts/",
    "src/",
    "tests/",
];

/// Known file extensions that denote source and configuration artifacts.
const KNOWN_EXTENSIONS: &[&str] = &[".rs", ".toml", ".md", ".json", ".wit", ".yml", ".yaml"];

/// Extracts candidate file paths from arbitrary text (markdown, code comments, prose).
pub fn extract_cited_paths(text: &str) -> Vec<String> {
    let mut found = BTreeSet::new();

    for raw_word in text.split_whitespace() {
        let trimmed = raw_word
            .trim_matches(|c: char| {
                c == '`'
                    || c == '"'
                    || c == '\''
                    || c == '('
                    || c == ')'
                    || c == '['
                    || c == ']'
                    || c == '{'
                    || c == '}'
                    || c == '<'
                    || c == '>'
                    || c == ','
                    || c == ';'
                    || c == '.'
            })
            .replace('\\', "/");

        // Strip trailing line numbers like `file.rs:123` or `file.rs:123:45`
        let path_str = if let Some(idx) = trimmed.find(".rs:") {
            &trimmed[..idx + 3]
        } else if let Some(idx) = trimmed.find(".toml:") {
            &trimmed[..idx + 5]
        } else if let Some(idx) = trimmed.find(".md:") {
            &trimmed[..idx + 3]
        } else {
            &trimmed
        };

        if path_str.is_empty() || path_str.contains("://") || path_str.starts_with("http") {
            continue;
        }

        let is_known_root = KNOWN_ROOT_DIRS.iter().any(|dir| path_str.starts_with(dir));
        let is_known_ext = KNOWN_EXTENSIONS.iter().any(|ext| path_str.ends_with(ext));
        let has_ext = path_str
            .rsplit('/')
            .next()
            .is_some_and(|leaf| leaf.contains('.'));
        let is_dir = path_str.ends_with('/');

        if (is_known_ext || (is_known_root && (has_ext || is_dir))) && path_str.contains('/') {
            // Must not contain parent traversal segments
            if !path_str.contains("..") {
                found.insert(path_str.to_string());
            }
        }
    }

    found.into_iter().collect()
}

/// Reads the workspace member crate names from the root `Cargo.toml`.
pub fn get_workspace_crates(checkout: &Path) -> Result<BTreeSet<String>> {
    let cargo_toml = checkout.join("Cargo.toml");
    if !cargo_toml.exists() {
        bail!("Root Cargo.toml not found at {}", cargo_toml.display());
    }

    let content = fs::read_to_string(&cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

    let mut members = BTreeSet::new();
    let value: toml::Value = toml::from_str(&content)
        .with_context(|| format!("Failed to parse TOML in {}", cargo_toml.display()))?;

    if let Some(workspace) = value.get("workspace").and_then(|w| w.as_table()) {
        if let Some(arr) = workspace.get("members").and_then(|m| m.as_array()) {
            for item in arr.iter().filter_map(|i| i.as_str()) {
                // Extract crate directory name, e.g. "crates/hypervisor" -> "hypervisor"
                if let Some(name) = item.rsplit('/').next() {
                    members.insert(name.to_string());
                }
            }
        }
    }

    Ok(members)
}

/// Extracts potential crate names cited in backticks or words matching `[a-z0-9_-]+`.
pub fn extract_cited_crates(text: &str, known_crates: &BTreeSet<String>) -> Vec<String> {
    let mut cited = BTreeSet::new();

    for raw_word in text.split_whitespace() {
        let cleaned = raw_word.trim_matches(|c: char| {
            c == '`'
                || c == '"'
                || c == '\''
                || c == '('
                || c == ')'
                || c == '['
                || c == ']'
                || c == ','
                || c == ';'
                || c == '.'
                || c == ':'
        });

        if known_crates.contains(cleaned) {
            cited.insert(cleaned.to_string());
        }
    }

    cited.into_iter().collect()
}

/// Verifies factual path claims and crate references against a repository checkout.
pub fn verify_claims(checkout: &Path, text: &str) -> Result<FactCheckReport> {
    if !checkout.is_absolute() {
        bail!("checkout must be an absolute path: {}", checkout.display());
    }
    if !checkout.exists() {
        bail!("checkout path does not exist: {}", checkout.display());
    }

    let cited_paths = extract_cited_paths(text);
    let mut passed_paths = Vec::new();
    let mut failed_paths = Vec::new();

    for rel in cited_paths {
        let candidate = checkout.join(&rel);
        if candidate.exists() {
            passed_paths.push(rel);
        } else {
            failed_paths.push(rel);
        }
    }

    let mut passed_crates = Vec::new();
    let mut failed_crates = Vec::new();

    if let Ok(known_crates) = get_workspace_crates(checkout) {
        let cited = extract_cited_crates(text, &known_crates);
        for c in cited {
            if known_crates.contains(&c) {
                passed_crates.push(c);
            } else {
                failed_crates.push(c);
            }
        }
    }

    let is_valid = failed_paths.is_empty() && failed_crates.is_empty();

    Ok(FactCheckReport {
        passed_paths,
        failed_paths,
        passed_crates,
        failed_crates,
        is_valid,
    })
}

/// CLI runner for verifying ground-truth claims in a target file or raw string.
pub fn run_cli(checkout: &Path, input: &Path, json_output: bool) -> Result<()> {
    let content = if input.is_file() {
        fs::read_to_string(input).with_context(|| format!("Failed to read {}", input.display()))?
    } else {
        input.to_string_lossy().into_owned()
    };

    let report = verify_claims(checkout, &content)?;

    if json_output {
        let json = serde_json::to_string_pretty(&report)?;
        println!("{json}");
    } else {
        println!("=== GROUND-TRUTH FACT CHECK REPORT (M56) ===");
        println!("Checkout:      {}", checkout.display());
        println!(
            "Overall Status: {}",
            if report.is_valid { "VALID" } else { "FAILED" }
        );
        println!("--------------------------------------------------");
        println!(
            "Paths Verified ({}/{}):",
            report.passed_paths.len(),
            report.passed_paths.len() + report.failed_paths.len()
        );
        for p in &report.passed_paths {
            println!("  [PASS] {p}");
        }
        for f in &report.failed_paths {
            println!("  [FAIL] {f} (file does not exist on disk)");
        }
        if !report.passed_crates.is_empty() || !report.failed_crates.is_empty() {
            println!("Workspace Crates Verified:");
            for c in &report.passed_crates {
                println!("  [PASS] crate '{c}'");
            }
            for f in &report.failed_crates {
                println!("  [FAIL] crate '{f}' (not in workspace members)");
            }
        }
        println!("==================================================");
    }

    if !report.is_valid {
        bail!(
            "Ground-truth verification failed: {} missing paths, {} invalid crates",
            report.failed_paths.len(),
            report.failed_crates.len()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_extract_cited_paths() {
        let sample = r#"
            We reviewed `crates/compute/src/engine.rs:42` and found issues.
            Also see `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`.
            Check `foundation/Cargo.toml`.
            Ignore https://github.com/rust-lang/cargo and `not_a_path`.
        "#;

        let paths = extract_cited_paths(sample);
        assert!(paths.contains(&"crates/compute/src/engine.rs".to_string()));
        assert!(paths.contains(&"architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md".to_string()));
        assert!(paths.contains(&"foundation/Cargo.toml".to_string()));
        assert_eq!(paths.len(), 3);
    }

    #[test]
    fn test_verify_claims_against_tempdir() -> Result<()> {
        let dir = tempdir()?;
        let root = dir.path();

        let cargo_toml = r#"
            [workspace]
            members = ["crates/wire", "crates/paths"]
        "#;
        fs::write(root.join("Cargo.toml"), cargo_toml)?;

        let crates_wire = root.join("crates/wire");
        fs::create_dir_all(&crates_wire)?;
        fs::write(crates_wire.join("lib.rs"), "// wire lib")?;

        let sample = r#"
            This task touches `crates/wire/lib.rs` and `crates/nonexistent/foo.rs`.
            It also cites `wire` and `unknown_crate`.
        "#;

        let report = verify_claims(root, sample)?;
        assert!(!report.is_valid);
        assert_eq!(report.passed_paths, vec!["crates/wire/lib.rs"]);
        assert_eq!(report.failed_paths, vec!["crates/nonexistent/foo.rs"]);
        assert_eq!(report.passed_crates, vec!["wire"]);

        Ok(())
    }
}
