//! M48 — Multi-perspective draft-seeding producer.
//!
//! The producer M46 actually needs and doesn't have: given one `code-draft`
//! goal, emit several packets attempting it from different angles, sharing
//! one `exploration_group`, so M46's comparison-summary and M44's
//! pre-proofing have something real to compare once results come back.
//!
//! Modeled directly on `frontier_discovery::create_frontier_job` and
//! `cratification_intake::create_cratification_job`: build a
//! `serde_json::json!({...})` manifest and write it via
//! `toml::to_string_pretty`, not the typed `local_worker::Job` struct (that
//! struct is private to `local_worker` and this module has no reason to
//! depend on it). **No new dispatch, pre-proofing, or scoring logic is
//! needed** — each emitted packet is an ordinary `code-draft` job; M44's
//! existing dispatch-loop hook and pre-proofing already handle it once
//! picked up, and M46's existing grouping/comparison-summary already
//! handles multiple drafts sharing an `exploration_group` once results
//! land. This milestone is purely packet generation.

use crate::intake::read_bounded;
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The starter perspective vocabulary named when M46 was first scoped.
/// Used whenever the caller passes an empty `perspectives` list.
pub const DEFAULT_PERSPECTIVES: [&str; 3] =
    ["correctness-first", "performance-first", "minimal-diff"];

/// Slugify a goal label into an id-safe key: lowercase, non-alphanumeric
/// runs collapsed to a single `_`, trimmed of leading/trailing `_`.
///
/// Duplicated from (not imported from) `frontier_discovery::slugify_query`
/// and `cratification_intake::slugify_crate_name`: both are private to
/// their own modules, and M43's precedent (repeated again at M49) is to
/// duplicate a small helper like this rather than widen visibility across
/// unrelated modules for a ~15-line call site.
fn slugify(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut last_was_sep = true; // suppress a leading separator
    for ch in text.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_sep = false;
        } else if !last_was_sep {
            slug.push('_');
            last_was_sep = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    slug
}

/// Sanitizes free text for safe use as a packet id component: uppercase,
/// non-alphanumeric runs replaced with `_`. Mirrors `frontier_discovery`'s
/// and `cratification_intake`'s id-sanitization style.
fn sanitize_for_id(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_uppercase()
}

/// Returns the fixed directive appended to the base instruction for a given
/// perspective. The three named perspectives (`correctness-first`,
/// `performance-first`, `minimal-diff`) get the exact canned directive text
/// from the M48 plan section. Any other perspective string (a caller-
/// supplied custom angle) falls back to a generic "approach this from the
/// following angle" directive built from the perspective text itself,
/// rather than erroring or panicking -- a custom perspective is a valid
/// input, just one this module has no canned wording for.
fn directive_for(perspective: &str) -> String {
    match perspective {
        "correctness-first" => {
            "Optimize primarily for correctness and edge-case coverage over performance or diff size."
                .to_string()
        }
        "performance-first" => {
            "Optimize primarily for runtime performance, even if the diff is larger or less minimal."
                .to_string()
        }
        "minimal-diff" => {
            "Optimize primarily for the smallest diff that solves the problem.".to_string()
        }
        other => format!("Approach this from the following angle: {other}."),
    }
}

/// Generates one shared `exploration_group` id for a seeding round: a slug
/// of `goal` plus a short time-based suffix (seconds since epoch), so
/// repeated seedings of a similar goal don't collide.
fn generate_exploration_group(goal: &str, now_secs: u64) -> String {
    format!("{}-{now_secs}", slugify(goal))
}

/// Emits one `code-draft` `worker-jobs/*.toml` packet per perspective.
///
/// `instruction_file` is an explicit absolute path to a text file holding
/// the base drafting goal/instruction, read via `intake::read_bounded`
/// matching this crate's convention for larger text input.  `evidence` is
/// the caller-supplied evidence list (unchanged, shared verbatim across
/// every emitted packet). `goal` is a short caller-supplied label -- not
/// the full instruction -- matching `frontier_discovery`'s use of the
/// query as `goal`. `perspectives` defaults to `DEFAULT_PERSPECTIVES` when
/// empty. `output_dir` must be an explicit absolute path; this function
/// never infers or scans for one.
pub fn run(
    instruction_file: &Path,
    evidence: Vec<String>,
    goal: &str,
    perspectives: Vec<String>,
    output_dir: &Path,
) -> Result<Vec<PathBuf>> {
    if !instruction_file.is_absolute() {
        bail!(
            "instruction_file must be an absolute path: {}",
            instruction_file.display()
        );
    }
    if !output_dir.is_absolute() {
        bail!(
            "output_dir must be an absolute path: {}",
            output_dir.display()
        );
    }
    if goal.trim().is_empty() {
        bail!("goal must not be empty");
    }
    if evidence.is_empty() {
        bail!("evidence must not be empty");
    }

    let base_instruction = read_bounded(instruction_file, 65536)?;
    if base_instruction.trim().is_empty() {
        bail!(
            "instruction_file must contain a non-empty instruction: {}",
            instruction_file.display()
        );
    }

    let perspectives = if perspectives.is_empty() {
        DEFAULT_PERSPECTIVES.iter().map(|s| s.to_string()).collect()
    } else {
        perspectives
    };

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let exploration_group = generate_exploration_group(goal, now_secs);

    let mut written = Vec::with_capacity(perspectives.len());
    for perspective in &perspectives {
        let path = create_exploration_job(
            instruction_file,
            base_instruction.trim(),
            &evidence,
            goal,
            perspective,
            &exploration_group,
            output_dir,
        )?;
        written.push(path);
    }
    Ok(written)
}

/// Emits a single perspective's packet. Split out from `run` so the
/// per-packet shape is directly testable without going through file I/O
/// for the instruction, and so `run` stays a thin read-then-loop wrapper.
#[allow(clippy::too_many_arguments)]
fn create_exploration_job(
    instruction_file: &Path,
    base_instruction: &str,
    evidence: &[String],
    goal: &str,
    perspective: &str,
    exploration_group: &str,
    output_dir: &Path,
) -> Result<PathBuf> {
    let goal_slug = slugify(goal);
    let perspective_slug = slugify(perspective);
    let job_id = format!(
        "EXPLORE-{}-{}",
        sanitize_for_id(&goal_slug),
        sanitize_for_id(&perspective_slug)
    );
    let filename = format!("explore-{goal_slug}-{perspective_slug}.toml");
    let destination = output_dir.join(&filename);

    let instruction = format!("{base_instruction}\n\n{}", directive_for(perspective));

    let manifest = serde_json::json!({
        "schema_version": 1,
        "id": job_id,
        "mode": "code-draft",
        "evidence": evidence,
        "instruction": instruction,
        "perspective": perspective,
        "goal": goal,
        "exploration_group": exploration_group,
    });

    let toml_str = toml::to_string_pretty(&manifest)?;
    std::fs::write(&destination, toml_str).map_err(|e| {
        anyhow::anyhow!(
            "failed to write exploration packet {} (from instruction file {}): {e}",
            destination.display(),
            instruction_file.display()
        )
    })?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_instruction(dir: &Path, text: &str) -> PathBuf {
        let path = dir.join("instruction.txt");
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn default_perspectives_produce_three_packets_with_named_directives() {
        let temp = tempdir().unwrap();
        let instruction_file = write_instruction(temp.path(), "Draft a faster ring buffer.");
        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();

        let written = run(
            &instruction_file,
            vec!["src/lib.rs".into()],
            "faster-ring-buffer",
            vec![],
            &output_dir,
        )
        .unwrap();

        assert_eq!(written.len(), 3);

        let mut found_correctness = false;
        let mut found_performance = false;
        let mut found_minimal_diff = false;
        for path in &written {
            assert!(path.exists());
            let text = std::fs::read_to_string(path).unwrap();
            let parsed: toml::Value = toml::from_str(&text).unwrap();
            assert_eq!(parsed["schema_version"].as_integer(), Some(1));
            assert_eq!(parsed["mode"].as_str(), Some("code-draft"));
            assert_eq!(parsed["goal"].as_str(), Some("faster-ring-buffer"));

            let instruction = parsed["instruction"].as_str().unwrap();
            assert!(instruction.contains("Draft a faster ring buffer."));
            match parsed["perspective"].as_str().unwrap() {
                "correctness-first" => {
                    assert!(instruction.contains(
                        "Optimize primarily for correctness and edge-case coverage over performance or diff size."
                    ));
                    found_correctness = true;
                }
                "performance-first" => {
                    assert!(instruction.contains(
                        "Optimize primarily for runtime performance, even if the diff is larger or less minimal."
                    ));
                    found_performance = true;
                }
                "minimal-diff" => {
                    assert!(instruction.contains(
                        "Optimize primarily for the smallest diff that solves the problem."
                    ));
                    found_minimal_diff = true;
                }
                other => panic!("unexpected perspective: {other}"),
            }
        }
        assert!(found_correctness && found_performance && found_minimal_diff);
    }

    #[test]
    fn packets_share_exploration_group_but_have_distinct_ids_and_perspectives() {
        let temp = tempdir().unwrap();
        let instruction_file = write_instruction(temp.path(), "Draft a parser.");
        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();

        let written = run(
            &instruction_file,
            vec!["src/parser.rs".into()],
            "faster-parser",
            vec![],
            &output_dir,
        )
        .unwrap();

        let mut groups = std::collections::HashSet::new();
        let mut ids = std::collections::HashSet::new();
        let mut perspectives = std::collections::HashSet::new();
        for path in &written {
            let text = std::fs::read_to_string(path).unwrap();
            let parsed: toml::Value = toml::from_str(&text).unwrap();
            groups.insert(parsed["exploration_group"].as_str().unwrap().to_string());
            ids.insert(parsed["id"].as_str().unwrap().to_string());
            perspectives.insert(parsed["perspective"].as_str().unwrap().to_string());
        }
        assert_eq!(
            groups.len(),
            1,
            "all packets must share one exploration_group"
        );
        assert_eq!(ids.len(), 3, "each packet must have a distinct id");
        assert_eq!(
            perspectives.len(),
            3,
            "each packet must have a distinct perspective"
        );
    }

    #[test]
    fn custom_perspective_list_produces_exactly_that_many_packets() {
        let temp = tempdir().unwrap();
        let instruction_file = write_instruction(temp.path(), "Draft a scheduler.");
        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();

        let written = run(
            &instruction_file,
            vec!["src/scheduler.rs".into()],
            "custom-scheduler",
            vec!["low-latency".into(), "throughput-first".into()],
            &output_dir,
        )
        .unwrap();

        assert_eq!(written.len(), 2);
    }

    #[test]
    fn unrecognized_perspective_gets_fallback_directive_not_panic() {
        let temp = tempdir().unwrap();
        let instruction_file = write_instruction(temp.path(), "Draft a cache.");
        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();

        let written = run(
            &instruction_file,
            vec!["src/cache.rs".into()],
            "custom-cache",
            vec!["security-first".into()],
            &output_dir,
        )
        .unwrap();

        assert_eq!(written.len(), 1);
        let text = std::fs::read_to_string(&written[0]).unwrap();
        let parsed: toml::Value = toml::from_str(&text).unwrap();
        let instruction = parsed["instruction"].as_str().unwrap();
        assert!(instruction.contains("Approach this from the following angle: security-first."));
    }

    #[test]
    fn rejects_relative_instruction_file() {
        let temp = tempdir().unwrap();
        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();
        let err = run(
            Path::new("relative/instruction.txt"),
            vec!["src/lib.rs".into()],
            "goal",
            vec![],
            &output_dir,
        )
        .unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn rejects_relative_output_dir() {
        let temp = tempdir().unwrap();
        let instruction_file = write_instruction(temp.path(), "Draft something.");
        let err = run(
            &instruction_file,
            vec!["src/lib.rs".into()],
            "goal",
            vec![],
            Path::new("relative/dir"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn rejects_empty_evidence() {
        let temp = tempdir().unwrap();
        let instruction_file = write_instruction(temp.path(), "Draft something.");
        let output_dir = temp.path().join("worker-jobs");
        std::fs::create_dir_all(&output_dir).unwrap();
        let err = run(&instruction_file, vec![], "goal", vec![], &output_dir).unwrap_err();
        assert!(err.to_string().contains("evidence"));
    }

    #[test]
    fn slugify_matches_sibling_producer_style() {
        assert_eq!(slugify("faster ring buffer"), "faster_ring_buffer");
        assert_eq!(slugify("  Weird__Goal!! "), "weird_goal");
        assert_eq!(slugify("zero-copy"), "zero_copy");
    }
}
