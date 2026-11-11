//! Reproducible dependency-vulnerability review (WORKLIST S2).
//!
//! Runs `cargo audit --json` as an external process against an explicitly
//! provided checkout (the "External runner" dogfood level: invokes a
//! documented Aaroneous-side command, never links against it) and writes a
//! versioned report conforming to the schema documented at
//! `contracts/dependency-review/v1/SCHEMA.md`. Never touches Cargo.lock or
//! any other file in the checkout — this command only reads. That is enforced,
//! not just stated: before any external command runs, the report destination
//! must resolve (symlinks, junctions and letter case included) to a directory
//! outside the checkout, and the report is published as a brand-new file that
//! never replaces or writes through an existing one, hard links included. The
//! directory is resolved once, before any external command runs, and the
//! report is later published into that resolved directory, not through the
//! spelling the caller gave, so retargeting a junction or symlink alias while
//! the audit runs cannot redirect it. This is the trusted-directory model: it
//! does not claim to defeat someone replacing the resolved directory itself.
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
use std::path::{Path, PathBuf};
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
    let destination = resolve_output_outside_checkout(checkout, output)?;

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
    crate::intake::publish_new_bytes(&destination, &json)
        .with_context(|| format!("failed to write {}", destination.display()))?;

    println!(
        "dependency review written: {} ({} external, {} local-name-collision, revision {})",
        destination.display(),
        report.summary.external,
        report.summary.local_workspace_name_collisions,
        report.revision
    );
    Ok(())
}

/// Resolve where the report will be written and refuse a destination inside
/// the checkout. The output directory is canonicalized once (symlinks, Windows
/// junctions and letter case included) and compared with the canonical
/// checkout; the returned path is that resolved directory joined with the
/// requested file name. The caller must publish to the returned path, not to
/// `output`, so an alias retargeted after this check (for example while the
/// audit subprocess runs) cannot redirect the write.
fn resolve_output_outside_checkout(checkout: &Path, output: &Path) -> Result<PathBuf> {
    let output_parent = output
        .parent()
        .context("output path must have an existing parent directory")?;
    let file_name = output
        .file_name()
        .with_context(|| format!("output must name a file: {}", output.display()))?;
    let checkout_real = std::fs::canonicalize(checkout)
        .with_context(|| format!("cannot resolve checkout {}", checkout.display()))?;
    let parent_real = std::fs::canonicalize(output_parent).with_context(|| {
        format!(
            "cannot resolve output directory {}",
            output_parent.display()
        )
    })?;
    if parent_real.starts_with(&checkout_real) {
        bail!(
            "output must be outside the read-only checkout: {} resolves inside {}",
            output_parent.display(),
            checkout.display()
        );
    }
    Ok(parent_real.join(file_name))
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

    fn fixture_checkout() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".git"), "gitdir: nowhere\n").expect(".git");
        std::fs::write(dir.path().join("Cargo.lock"), b"original lock bytes").expect("lock");
        std::fs::create_dir(dir.path().join("sub")).expect("sub");
        dir
    }

    fn assert_rejected_and_untouched(checkout: &Path, output: &Path) {
        let err = run(checkout, output).expect_err("output inside the checkout must be refused");
        assert!(
            err.to_string().contains("outside the read-only checkout"),
            "unexpected error: {err:#}"
        );
        assert_eq!(
            std::fs::read(checkout.join("Cargo.lock")).expect("lock"),
            b"original lock bytes"
        );
    }

    #[test]
    fn refuses_to_overwrite_a_checkout_file_before_running_any_tool() {
        let dir = fixture_checkout();
        assert_rejected_and_untouched(dir.path(), &dir.path().join("Cargo.lock"));
        assert_rejected_and_untouched(dir.path(), &dir.path().join("sub").join("report.json"));
        assert!(!dir.path().join("sub").join("report.json").exists());
    }

    #[cfg(windows)]
    #[test]
    fn refuses_a_differently_cased_spelling_of_the_checkout() {
        let dir = fixture_checkout();
        let alias = std::path::PathBuf::from(dir.path().display().to_string().to_uppercase());
        assert_ne!(alias.as_os_str(), dir.path().as_os_str());
        assert_rejected_and_untouched(dir.path(), &alias.join("Cargo.lock"));
    }

    #[cfg(windows)]
    #[test]
    fn refuses_a_junction_that_points_into_the_checkout() {
        let dir = fixture_checkout();
        let outside = tempfile::tempdir().expect("outside");
        let junction = outside.path().join("alias");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(dir.path())
            .output()
            .expect("run mklink");
        assert!(status.status.success(), "mklink /J failed: {status:?}");
        assert_rejected_and_untouched(dir.path(), &junction.join("Cargo.lock"));
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_symlink_that_points_into_the_checkout() {
        let dir = fixture_checkout();
        let outside = tempfile::tempdir().expect("outside");
        let link = outside.path().join("alias");
        std::os::unix::fs::symlink(dir.path(), &link).expect("symlink");
        assert_rejected_and_untouched(dir.path(), &link.join("Cargo.lock"));
    }

    #[test]
    fn resolved_destination_is_a_new_path_outside_and_keeps_the_file_name() {
        let dir = fixture_checkout();
        let outside = tempfile::tempdir().expect("outside");
        let wanted = outside.path().join("report.json");
        let resolved = resolve_output_outside_checkout(dir.path(), &wanted).expect("outside ok");
        assert_eq!(resolved.file_name(), wanted.file_name());
        assert!(!resolved.starts_with(std::fs::canonicalize(dir.path()).expect("canon")));
    }

    #[cfg(windows)]
    fn link_dir(link: &Path, target: &Path) {
        let out = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .expect("run mklink");
        assert!(out.status.success(), "mklink /J failed: {out:?}");
    }

    #[cfg(unix)]
    fn link_dir(link: &Path, target: &Path) {
        std::os::unix::fs::symlink(target, link).expect("symlink");
    }

    #[cfg(any(windows, unix))]
    #[test]
    fn retargeting_the_alias_after_resolution_cannot_redirect_the_report() {
        let checkout = fixture_checkout();
        let outside = tempfile::tempdir().expect("outside");
        let holder = tempfile::tempdir().expect("holder");
        let alias = holder.path().join("alias");
        link_dir(&alias, outside.path());
        let resolved = resolve_output_outside_checkout(checkout.path(), &alias.join("report.json"))
            .expect("alias to an outside directory is accepted");
        // The alias is swapped to point into the checkout while the audit runs.
        std::fs::remove_dir(&alias)
            .or_else(|_| std::fs::remove_file(&alias))
            .expect("drop alias");
        link_dir(&alias, checkout.path());
        crate::intake::publish_new_bytes(&resolved, b"report").expect("publish to resolved path");
        assert_eq!(
            std::fs::read(outside.path().join("report.json")).expect("outside report"),
            b"report"
        );
        assert!(!checkout.path().join("report.json").exists());
        assert_eq!(
            std::fs::read(checkout.path().join("Cargo.lock")).expect("lock"),
            b"original lock bytes"
        );
        // And the exclusive publisher still refuses to replace it.
        crate::intake::publish_new_bytes(&resolved, b"other").expect_err("no replace");
        assert_eq!(
            std::fs::read(outside.path().join("report.json")).expect("outside report"),
            b"report"
        );
    }
}
