use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
#[derive(Deserialize, Serialize)]
struct Proposal {
    schema_version: u32,
    id: String,
    source: String,
    priority: String,
    scope: Vec<String>,
    dependencies: Vec<String>,
    verification: Vec<String>,
    decision: String,
}
#[derive(Serialize)]
struct Report {
    schema_version: u32,
    artifact_kind: String,
    accepted: bool,
    proposal: Proposal,
}
pub(crate) fn validate(input: &Path, output: &Path) -> Result<()> {
    if !input.is_absolute()
        || !fs::metadata(input)
            .with_context(|| "proposal unavailable")?
            .is_file()
    {
        bail!("proposal must be an available absolute file");
    }
    if !output.is_absolute()
        || !fs::metadata(output.parent().context("output parent")?)
            .with_context(|| "output parent unavailable")?
            .is_dir()
    {
        bail!("output needs existing absolute parent");
    }
    let proposal: Proposal = serde_json::from_slice(&fs::read(input).context("read proposal")?)
        .context("invalid intake JSON")?;
    if proposal.schema_version != 1
        || proposal.id.trim().is_empty()
        || proposal.source.trim().is_empty()
        || !matches!(proposal.priority.as_str(), "P0" | "P1" | "P2" | "P3")
        || proposal.scope.is_empty()
        || proposal.verification.is_empty()
        || proposal.decision.trim().is_empty()
    {
        bail!("proposal lacks required intake fields");
    }
    let report = Report {
        schema_version: 1,
        artifact_kind: "foundation.intake-validation".to_owned(),
        accepted: true,
        proposal,
    };
    fs::write(
        output,
        serde_json::to_vec_pretty(&report).context("serialize report")?,
    )
    .context("write report")?;
    Ok(())
}
