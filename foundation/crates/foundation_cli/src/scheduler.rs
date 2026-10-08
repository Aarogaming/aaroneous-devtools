use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::fs;
use std::path::Path;

const REPORT_KIND: &str = "foundation.schedule-review";

#[derive(Debug, Serialize)]
struct ScheduleReview {
    schema_version: u32,
    artifact_kind: String,
    queue: String,
    program: String,
    primary_capacity: usize,
    claimed_or_review_lanes: usize,
    ready_candidates: Vec<String>,
    recommended_claims: Vec<String>,
    findings: Vec<String>,
}

pub(crate) fn run(
    queue: &Path,
    program: &Path,
    output: &Path,
    primary_capacity: usize,
) -> Result<()> {
    if primary_capacity == 0 {
        bail!("primary capacity must be at least one");
    }
    require_absolute_file("queue", queue)?;
    require_absolute_file("program", program)?;
    let queue_text =
        fs::read_to_string(queue).with_context(|| format!("failed to read {}", queue.display()))?;
    let program_text = fs::read_to_string(program)
        .with_context(|| format!("failed to read {}", program.display()))?;
    if !program_text.contains("## Capacity model")
        || !program_text.contains("## Priority portfolio")
    {
        bail!("program is missing required multithread sections");
    }
    let rows = queue_rows(&queue_text);
    let ready_candidates = rows
        .iter()
        .filter(|(_, status)| status == "Ready")
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let claimed_or_review_lanes = rows
        .iter()
        .filter(|(_, status)| status == "Claimed" || status == "Review")
        .count();
    let available = primary_capacity.saturating_sub(claimed_or_review_lanes);
    let recommended_claims = ready_candidates
        .iter()
        .take(available)
        .cloned()
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    if claimed_or_review_lanes > primary_capacity {
        findings.push(format!("capacity exceeded: {claimed_or_review_lanes} active lanes for configured limit {primary_capacity}"));
    }
    if ready_candidates.len() > available {
        findings.push(format!(
            "{} ready items remain unassigned after applying capacity",
            ready_candidates.len() - available
        ));
    }
    if recommended_claims.is_empty() && claimed_or_review_lanes == 0 {
        findings.push(
            "no ready work is available; create work only through documented intake".to_owned(),
        );
    }
    validate_output(output)?;
    let report = ScheduleReview {
        schema_version: 1,
        artifact_kind: REPORT_KIND.to_owned(),
        queue: queue.display().to_string(),
        program: program.display().to_string(),
        primary_capacity,
        claimed_or_review_lanes,
        ready_candidates,
        recommended_claims,
        findings,
    };
    fs::write(
        output,
        serde_json::to_vec_pretty(&report).context("failed to serialize schedule review")?,
    )
    .with_context(|| format!("failed to write {}", output.display()))?;
    println!("wrote schedule review: {}", output.display());
    Ok(())
}

fn queue_rows(input: &str) -> Vec<(String, String)> {
    input
        .lines()
        .filter_map(|line| {
            let fields = crate::table::cells(line);
            if fields.len() < 4
                || fields[1] == "ID"
                || fields[1].is_empty()
                || !fields[1].chars().all(|c| c.is_ascii_alphanumeric())
            {
                return None;
            }
            Some((fields[1].to_owned(), fields[3].to_owned()))
        })
        .collect()
}

fn require_absolute_file(label: &str, path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("{label} must be an absolute path: {}", path.display());
    }
    if !fs::metadata(path)
        .with_context(|| format!("{label} is unavailable: {}", path.display()))?
        .is_file()
    {
        bail!("{label} must be a file: {}", path.display());
    }
    Ok(())
}

fn validate_output(output: &Path) -> Result<()> {
    if !output.is_absolute() {
        bail!("output must be an absolute path: {}", output.display());
    }
    let parent = output.parent().context("output must have a parent")?;
    if !fs::metadata(parent)
        .with_context(|| format!("output parent is unavailable: {}", parent.display()))?
        .is_dir()
    {
        bail!("output parent must be a directory: {}", parent.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_only_real_queue_rows() {
        let rows = queue_rows(
            "| ID | Scope | Status |\n| --- | --- | --- |\n| C1 | x | Ready |\n| F7 | x | Done |",
        );
        assert_eq!(
            rows,
            vec![
                ("C1".to_owned(), "Ready".to_owned()),
                ("F7".to_owned(), "Done".to_owned())
            ]
        );
    }

    #[test]
    fn an_escaped_pipe_inside_a_cell_does_not_hide_the_row() {
        // The live C89 row names `kernel\|control\|presentation\|tooling`; splitting on every pipe read its status as
        // `control\` and the row never reached the schedule.
        let rows = queue_rows(
            r#"| C89 | profile = "kernel\|control\|presentation\|tooling" | Ready | Unowned |"#,
        );
        assert_eq!(rows, vec![("C89".to_owned(), "Ready".to_owned())]);
    }
}
