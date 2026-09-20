use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    concurrence: Option<ConcurrenceAdvisory>,
}

/// Advisory-only summary of an Aaroneous hypervisor concurrence report. This is
/// never load-bearing: it must not affect `ready_candidates` or
/// `recommended_claims`. See M40 in
/// `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ConcurrenceAdvisory {
    rolling_concurrence: f32,
    graduated: bool,
}

/// Best-effort read of an optional concurrence report. A missing, unreadable,
/// or unparseable file is never an error here — it simply yields `None`, since
/// the report is advisory-only and the Aaroneous hypervisor that produces it
/// may not be running.
fn read_concurrence_report(path: &Path) -> Option<ConcurrenceAdvisory> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub(crate) fn run(
    queue: &Path,
    program: &Path,
    output: &Path,
    primary_capacity: usize,
    concurrence_report: Option<&Path>,
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
    let concurrence = concurrence_report.and_then(read_concurrence_report);
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
        concurrence,
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
            let fields = line.split('|').map(str::trim).collect::<Vec<_>>();
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

    fn write_fixture_inputs(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let queue = dir.join("queue.md");
        let program = dir.join("program.md");
        fs::write(
            &queue,
            "| ID | Scope | Status |\n| --- | --- | --- |\n| C1 | x | Ready |\n",
        )
        .expect("write queue fixture");
        fs::write(&program, "## Capacity model\n\n## Priority portfolio\n")
            .expect("write program fixture");
        (queue, program)
    }

    #[test]
    fn run_without_concurrence_report_is_unaffected() {
        let temp = tempfile::tempdir().expect("test directory");
        let (queue, program) = write_fixture_inputs(temp.path());
        let output = temp.path().join("report.json");

        run(&queue, &program, &output, 3, None).expect("scheduler run");

        let written = fs::read_to_string(&output).expect("read report");
        let value: serde_json::Value = serde_json::from_str(&written).expect("valid json");
        assert!(value.get("concurrence").is_none());
        assert_eq!(value["ready_candidates"], serde_json::json!(["C1"]));
    }

    #[test]
    fn run_attaches_valid_concurrence_report() {
        let temp = tempfile::tempdir().expect("test directory");
        let (queue, program) = write_fixture_inputs(temp.path());
        let output = temp.path().join("report.json");
        let concurrence_path = temp.path().join("concurrence_report.json");
        fs::write(
            &concurrence_path,
            r#"{
                "rolling_concurrence": 0.97,
                "total_ticks": 1234,
                "total_agreements": 1198,
                "avg_confidence": 0.91,
                "avg_reward": 0.5,
                "divergence_mse": 0.02,
                "graduated": true,
                "ticks_since_graduation": 40
            }"#,
        )
        .expect("write concurrence fixture");

        run(&queue, &program, &output, 3, Some(&concurrence_path)).expect("scheduler run");

        let written = fs::read_to_string(&output).expect("read report");
        let value: serde_json::Value = serde_json::from_str(&written).expect("valid json");
        assert_eq!(value["concurrence"]["rolling_concurrence"], 0.97);
        assert_eq!(value["concurrence"]["graduated"], true);
    }

    #[test]
    fn run_ignores_missing_or_malformed_concurrence_report() {
        let temp = tempfile::tempdir().expect("test directory");
        let (queue, program) = write_fixture_inputs(temp.path());

        // Missing file: never an error, and no field in the output.
        let output = temp.path().join("report_missing.json");
        let missing_path = temp.path().join("does_not_exist.json");
        run(&queue, &program, &output, 3, Some(&missing_path)).expect("scheduler run");
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&output).expect("read report"))
                .expect("valid json");
        assert!(value.get("concurrence").is_none());

        // Malformed file: also never an error.
        let output = temp.path().join("report_malformed.json");
        let malformed_path = temp.path().join("malformed.json");
        fs::write(&malformed_path, "not json").expect("write malformed fixture");
        run(&queue, &program, &output, 3, Some(&malformed_path)).expect("scheduler run");
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&output).expect("read report"))
                .expect("valid json");
        assert!(value.get("concurrence").is_none());
    }
}
