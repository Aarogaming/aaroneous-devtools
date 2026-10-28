use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};

// Both queue consumers must see the complete queue, including active claims near
// its end. Keep a finite limit so an oversized file fails closed.
pub(crate) const MAX_COORDINATION_QUEUE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Proposal {
    pub schema_version: u32,
    pub id: String,
    pub source: String,
    pub priority: String,
    pub scope: Vec<String>,
    pub dependencies: Vec<String>,
    pub verification: Vec<String>,
    pub decision: String,
}

pub(crate) fn relative_file(value: &str) -> bool {
    !value.trim().is_empty()
        && value == value.trim()
        && !value.contains(['\\', ':', '|', '\n', '\r'])
        && !value.chars().any(char::is_whitespace)
        && Path::new(value)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && value.contains('.')
}

impl Proposal {
    pub(crate) fn check(&self) -> Result<()> {
        if self.schema_version != 1
            || self.id.is_empty()
            || !self
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || !relative_file(&self.source)
            || !matches!(self.priority.as_str(), "P0" | "P1" | "P2" | "P3")
            || self.scope.is_empty()
            || self.scope.len() > 16
            || self.scope.iter().any(|s| !relative_file(s))
            || self.verification.is_empty()
            || self.verification.len() > 16
            || self.verification.iter().any(|s| s.trim().is_empty())
            || self.dependencies.iter().any(|s| s.trim().is_empty())
            || self.decision.trim().is_empty()
        {
            bail!(
                "proposal requires a valid ID, priority, exact relative file paths and nonempty verification/decision"
            );
        }
        Ok(())
    }
}

pub(crate) fn read_bounded(path: &Path, limit: u64) -> Result<String> {
    let mut data = String::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_string(&mut data)?;
    if data.len() as u64 > limit {
        bail!("input exceeds {limit} bytes: {}", path.display());
    }
    Ok(data)
}

pub(crate) fn validate(input: &Path, output: &Path) -> Result<()> {
    if !input.is_absolute() || !output.is_absolute() {
        bail!("proposal and output must be absolute");
    }
    crate::require_absolute_directory("output parent", output.parent().context("output parent")?)?;
    let proposal: Proposal = serde_json::from_str(&read_bounded(input, 65536)?)?;
    proposal.check()?;
    fs::write(
        output,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version":1,"artifact_kind":"foundation.intake-validation",
            "structurally_valid":true,"requires_owner_review":true,"proposal":proposal
        }))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_prose_and_traversal_scopes() {
        for bad in [
            "Documentation claims regarding performance metrics",
            "../README.md",
            "C:/README.md",
            "/README.md",
            "crates/../README.md",
        ] {
            assert!(!relative_file(bad), "{bad}");
        }
        assert!(relative_file("crates/ipc_bus/src/lib.rs"));
    }
    #[test]
    fn bounded_read_rejects_large_input() {
        let dir = tempfile::tempdir().expect("fixture");
        let file = dir.path().join("evidence.txt");
        fs::write(&file, "12345").expect("write");
        assert!(read_bounded(&file, 4).is_err());
    }
}
