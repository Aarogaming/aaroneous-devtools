//! Reproducible dependency-vulnerability review (WORKLIST S2).
//!
//! Runs `cargo audit --json` as an external process against an explicitly
//! provided checkout (the "External runner" dogfood level: invokes a
//! documented Aaroneous-side command, never links against it) and writes a
//! versioned report conforming to the schema documented at
//! `contracts/dependency-review/v1/SCHEMA.md`. Never touches Cargo.lock or
//! any other file in the checkout — this command only reads.
//!
//! One thing `cargo-audit` alone gets wrong for this workspace: it matches
//! advisories by package name and version against the *whole* lockfile,
//! including path-dependency workspace members. A local crate that happens
//! to share a name with an unrelated, vulnerable crates.io package (this
//! workspace has one: `crates/scratchpad` collides with a real, vulnerable
//! `scratchpad` published to crates.io) gets flagged even though it's not
//! the same code at all. This module cross-references findings against the
//! checkout's own `cargo metadata` workspace-member list and classifies
//! those separately so they don't get "fixed" by mistake.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub fn run(checkout: &Path, output: &Path) -> Result<()> {
    crate::require_absolute_directory("checkout", checkout)?;
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
    crate::require_absolute_directory("output parent", parent)?;

    let revision = git_rev_parse_head(checkout)?;
    let audit_tool_version = cargo_audit_version(checkout)?;
    let local_members = workspace_member_names(checkout)?;
    let audit_output = run_cargo_audit_json(checkout)?;

    let mut findings = Vec::new();
    for entry in audit_output.vulnerabilities.list {
        findings.push(classify(entry, "vulnerability", &local_members));
    }
    for (kind, entries) in audit_output.warnings {
        for entry in entries {
            findings.push(classify(entry, &kind, &local_members));
        }
    }

    let external_count = findings
        .iter()
        .filter(|f| f.classification == Classification::External)
        .count();
    let collision_count = findings
        .iter()
        .filter(|f| f.classification == Classification::LocalWorkspaceNameCollision)
        .count();

    let report = DependencyReviewReport {
        schema_version: 1,
        checkout: checkout.display().to_string(),
        revision,
        tool: "cargo-audit".to_string(),
        tool_version: audit_tool_version,
        generated_at_unix_ms: unix_ms_now(),
        summary: Summary {
            total_findings: findings.len(),
            external: external_count,
            local_workspace_name_collisions: collision_count,
        },
        findings,
    };

    let json = serde_json::to_vec_pretty(&report)
        .context("failed to serialize dependency review report")?;
    std::fs::write(output, json)
        .with_context(|| format!("failed to write {}", output.display()))?;

    println!(
        "dependency review written: {} ({} external, {} local-name-collision, revision {})",
        output.display(),
        report.summary.external,
        report.summary.local_workspace_name_collisions,
        report.revision
    );
    Ok(())
}

fn classify(
    entry: RawAdvisoryEntry,
    default_kind: &str,
    local_members: &BTreeSet<String>,
) -> Finding {
    let is_local_collision =
        entry.package.source.is_none() && local_members.contains(&entry.package.name);

    let classification = if is_local_collision {
        Classification::LocalWorkspaceNameCollision
    } else {
        Classification::External
    };

    let recommendation = if is_local_collision {
        format!(
            "No action: `{}` {} matches a local workspace crate by name, not the external \
             RustSec-advised package (its Cargo.lock entry has no registry source). Consider \
             renaming the local crate if this collision recurs across advisories.",
            entry.package.name, entry.package.version
        )
    } else if entry.versions.patched.is_empty() {
        format!(
            "No patched version is published for `{}` {} yet ({}: {}). Track the advisory; \
             consider an alternative crate or vendoring a patch if this is reachable code.",
            entry.package.name, entry.package.version, entry.advisory.id, entry.advisory.title
        )
    } else {
        format!(
            "Update `{}` from {} to a version satisfying {:?} ({}: {}). Do not apply \
             automatically — review changelog/breaking changes and re-run the full \
             verification gate after updating.",
            entry.package.name,
            entry.package.version,
            entry.versions.patched,
            entry.advisory.id,
            entry.advisory.title
        )
    };

    Finding {
        kind: entry
            .advisory
            .informational
            .clone()
            .unwrap_or_else(|| default_kind.to_string()),
        advisory_id: entry.advisory.id,
        title: entry.advisory.title,
        url: entry.advisory.url,
        package: entry.package.name,
        installed_version: entry.package.version,
        patched_versions: entry.versions.patched,
        classification,
        recommendation,
    }
}

fn git_rev_parse_head(checkout: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(checkout)
        .output()
        .context("failed to spawn git rev-parse HEAD")?;
    if !output.status.success() {
        bail!(
            "git rev-parse HEAD failed in {}: {}",
            checkout.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn cargo_audit_version(checkout: &Path) -> Result<String> {
    let output = Command::new("cargo")
        .args(["audit", "--version"])
        .current_dir(checkout)
        .output()
        .context(
            "failed to spawn `cargo audit --version` — is cargo-audit installed \
             (`cargo install cargo-audit`)?",
        )?;
    if !output.status.success() {
        bail!(
            "cargo audit --version failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn workspace_member_names(checkout: &Path) -> Result<BTreeSet<String>> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(checkout)
        .output()
        .context("failed to spawn cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata failed in {}: {}",
            checkout.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: CargoMetadata =
        serde_json::from_slice(&output.stdout).context("failed to parse cargo metadata output")?;
    Ok(metadata.packages.into_iter().map(|p| p.name).collect())
}

fn run_cargo_audit_json(checkout: &Path) -> Result<AuditOutput> {
    let output = Command::new("cargo")
        .args(["audit", "--json"])
        .current_dir(checkout)
        .output()
        .context(
            "failed to spawn `cargo audit --json` — is cargo-audit installed \
             (`cargo install cargo-audit`)?",
        )?;
    // cargo-audit exits non-zero when it finds vulnerabilities, by design —
    // that's not a tool failure, so don't gate on status here. Only a missing
    // or unparseable JSON body indicates the run itself broke.
    if output.stdout.is_empty() {
        bail!(
            "cargo audit produced no output (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).context("failed to parse cargo audit JSON output")
}

fn unix_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// --- cargo-audit JSON shapes (only the fields this tool uses) ---

#[derive(Debug, Deserialize)]
struct AuditOutput {
    vulnerabilities: VulnerabilitiesSection,
    #[serde(default)]
    warnings: std::collections::BTreeMap<String, Vec<RawAdvisoryEntry>>,
}

#[derive(Debug, Deserialize)]
struct VulnerabilitiesSection {
    list: Vec<RawAdvisoryEntry>,
}

#[derive(Debug, Deserialize)]
struct RawAdvisoryEntry {
    advisory: RawAdvisory,
    package: RawPackage,
    versions: RawVersions,
}

#[derive(Debug, Deserialize)]
struct RawAdvisory {
    id: String,
    title: String,
    url: Option<String>,
    informational: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawPackage {
    name: String,
    version: String,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawVersions {
    #[serde(default)]
    patched: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
}

#[derive(Debug, Deserialize)]
struct CargoMetadataPackage {
    name: String,
}

// --- Report schema (contracts/dependency-review/v1/SCHEMA.md) ---

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DependencyReviewReport {
    pub schema_version: u32,
    pub checkout: String,
    pub revision: String,
    pub tool: String,
    pub tool_version: String,
    pub generated_at_unix_ms: u64,
    pub summary: Summary,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Summary {
    pub total_findings: usize,
    pub external: usize,
    pub local_workspace_name_collisions: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    /// "vulnerability", "unmaintained", "unsound", or another cargo-audit
    /// informational kind.
    pub kind: String,
    pub advisory_id: String,
    pub title: String,
    pub url: Option<String>,
    pub package: String,
    pub installed_version: String,
    pub patched_versions: Vec<String>,
    pub classification: Classification,
    /// A human-readable next step. Never applied automatically — S2/this
    /// tool's contract is read-only review, not remediation.
    pub recommendation: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    External,
    LocalWorkspaceNameCollision,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(name: &str, source: Option<&str>, patched: Vec<&str>) -> RawAdvisoryEntry {
        RawAdvisoryEntry {
            advisory: RawAdvisory {
                id: "RUSTSEC-0000-0000".to_string(),
                title: "sample".to_string(),
                url: None,
                informational: None,
            },
            package: RawPackage {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                source: source.map(str::to_string),
            },
            versions: RawVersions {
                patched: patched.into_iter().map(str::to_string).collect(),
            },
        }
    }

    #[test]
    fn classifies_sourceless_local_member_as_collision() {
        let mut members = BTreeSet::new();
        members.insert("scratchpad".to_string());
        let entry = sample_entry("scratchpad", None, vec![">=1.3.1"]);

        let finding = classify(entry, "vulnerability", &members);

        assert_eq!(
            finding.classification,
            Classification::LocalWorkspaceNameCollision
        );
        assert!(finding.recommendation.contains("local workspace crate"));
    }

    #[test]
    fn classifies_registry_sourced_package_as_external() {
        let members = BTreeSet::new();
        let entry = sample_entry(
            "rustls",
            Some("registry+https://github.com/rust-lang/crates.io-index"),
            vec![">=0.23.45"],
        );

        let finding = classify(entry, "vulnerability", &members);

        assert_eq!(finding.classification, Classification::External);
        assert!(finding.recommendation.contains("Update `rustls`"));
    }

    #[test]
    fn does_not_misclassify_a_registry_sourced_package_sharing_a_local_name() {
        // A package that DOES have a registry source is never a local
        // workspace member, even if its name happens to also be a member's
        // name — only a null source is evidence of a path dependency.
        let mut members = BTreeSet::new();
        members.insert("scratchpad".to_string());
        let entry = sample_entry(
            "scratchpad",
            Some("registry+https://github.com/rust-lang/crates.io-index"),
            vec![">=1.3.1"],
        );

        let finding = classify(entry, "vulnerability", &members);

        assert_eq!(finding.classification, Classification::External);
    }

    #[test]
    fn notes_when_no_patched_version_exists_yet() {
        let members = BTreeSet::new();
        let entry = sample_entry("hickory-net", Some("registry+https://x"), vec![]);

        let finding = classify(entry, "vulnerability", &members);

        assert!(finding.recommendation.contains("No patched version"));
    }

    #[test]
    fn report_round_trips_through_json() {
        let report = DependencyReviewReport {
            schema_version: 1,
            checkout: "/checkout".to_string(),
            revision: "abc123".to_string(),
            tool: "cargo-audit".to_string(),
            tool_version: "cargo-audit-audit 0.22.1".to_string(),
            generated_at_unix_ms: 0,
            summary: Summary {
                total_findings: 1,
                external: 1,
                local_workspace_name_collisions: 0,
            },
            findings: vec![Finding {
                kind: "vulnerability".to_string(),
                advisory_id: "RUSTSEC-0000-0000".to_string(),
                title: "sample".to_string(),
                url: None,
                package: "rustls".to_string(),
                installed_version: "0.23.44".to_string(),
                patched_versions: vec![">=0.23.45".to_string()],
                classification: Classification::External,
                recommendation: "update".to_string(),
            }],
        };

        let json = serde_json::to_vec(&report).expect("serialize");
        let round_tripped: DependencyReviewReport =
            serde_json::from_slice(&json).expect("deserialize");
        assert_eq!(report, round_tripped);
    }
}
