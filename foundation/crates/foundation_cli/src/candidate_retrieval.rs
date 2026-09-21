use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::candidate_metadata::CandidateRecord;

/// Extracted discrete feature set for explainable similarity calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateFeatures {
    pub candidate_id: String,
    pub kind: String,
    pub tokens: BTreeSet<String>,
    pub dependencies: BTreeSet<String>,
    pub total_lines: Option<usize>,
    pub file_count: Option<usize>,
    pub passed_g1: Option<bool>,
    pub passed_g5: Option<bool>,
    pub license: Option<String>,
    pub last_updated_secs: u64,
}

/// A ranked candidate similarity match with an explainable breakdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateMatch {
    pub candidate_id: String,
    pub kind: String,
    pub score: f64,
    pub token_similarity: f64,
    pub dep_similarity: f64,
    pub size_similarity: Option<f64>,
    pub shared_tokens: Vec<String>,
    pub shared_dependencies: Vec<String>,
    pub explanation: String,
}

/// Tokenizes a raw string by splitting on punctuation, whitespace, and camelCase boundaries.
pub fn tokenize(raw: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut current = String::new();

    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase()
                && !current.is_empty()
                && current.chars().any(|c| c.is_ascii_lowercase())
            {
                if current.len() >= 2 {
                    tokens.insert(current.to_ascii_lowercase());
                }
                current.clear();
            }
            current.push(ch);
        } else {
            if current.len() >= 2 {
                tokens.insert(current.to_ascii_lowercase());
            }
            current.clear();
        }
    }
    if current.len() >= 2 {
        tokens.insert(current.to_ascii_lowercase());
    }

    tokens
}

/// Computes the Jaccard similarity between two sets: |A ∩ B| / |A ∪ B|.
pub fn jaccard_similarity<T: Ord>(a: &BTreeSet<T>, b: &BTreeSet<T>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Computes line count sizing ratio: min(a, b) / max(a, b).
pub fn size_similarity(a: Option<usize>, b: Option<usize>) -> Option<f64> {
    match (a, b) {
        (Some(lines_a), Some(lines_b)) => {
            if lines_a == 0 && lines_b == 0 {
                Some(1.0)
            } else {
                let min = lines_a.min(lines_b) as f64;
                let max = lines_a.max(lines_b) as f64;
                if max > 0.0 {
                    Some(min / max)
                } else {
                    Some(0.0)
                }
            }
        }
        _ => None,
    }
}

/// Extracts discrete features from an accumulated CandidateRecord.
pub fn extract_features(record: &CandidateRecord) -> CandidateFeatures {
    let tokens = tokenize(&record.candidate_id);

    let mut dependencies = BTreeSet::new();
    let mut total_lines = None;
    let mut file_count = None;
    let mut passed_g1 = None;
    let mut passed_g5 = None;
    let mut license = None;

    for pass in &record.passes {
        if let Some(obj) = pass.findings.as_object() {
            if let Some(deps) = obj.get("dependencies").and_then(|v| v.as_array()) {
                for dep in deps {
                    if let Some(cid) = dep.get("crate_id").and_then(|v| v.as_str()) {
                        dependencies.insert(cid.to_string());
                    }
                }
            }
            if let Some(lines) = obj.get("total_lines").and_then(|v| v.as_u64()) {
                total_lines = Some(lines as usize);
            }
            if let Some(files) = obj.get("file_count").and_then(|v| v.as_u64()) {
                file_count = Some(files as usize);
            }
            if let Some(g1) = obj.get("passed_g1_license").and_then(|v| v.as_bool()) {
                passed_g1 = Some(g1);
            }
            if let Some(g5) = obj.get("passed_g5").and_then(|v| v.as_bool()) {
                passed_g5 = Some(g5);
            }
            if let Some(lic) = obj.get("spdx_license").and_then(|v| v.as_str()) {
                license = Some(lic.to_string());
            }
        }
    }

    CandidateFeatures {
        candidate_id: record.candidate_id.clone(),
        kind: record.kind.clone(),
        tokens,
        dependencies,
        total_lines,
        file_count,
        passed_g1,
        passed_g5,
        license,
        last_updated_secs: record.updated_at_secs,
    }
}

/// Calculates similarity between two feature sets with full explainability.
pub fn score_similarity(query: &CandidateFeatures, target: &CandidateFeatures) -> CandidateMatch {
    let token_sim = jaccard_similarity(&query.tokens, &target.tokens);
    let dep_sim = jaccard_similarity(&query.dependencies, &target.dependencies);
    let size_sim = size_similarity(query.total_lines, target.total_lines);

    let shared_tokens: Vec<String> = query.tokens.intersection(&target.tokens).cloned().collect();

    let shared_dependencies: Vec<String> = query
        .dependencies
        .intersection(&target.dependencies)
        .cloned()
        .collect();

    let has_deps = !query.dependencies.is_empty() || !target.dependencies.is_empty();
    let (overall_score, explanation) = if token_sim == 0.0 && dep_sim == 0.0 {
        (
            0.0,
            "Score 0.00: 0% token overlap, 0% dependency overlap".to_string(),
        )
    } else if has_deps {
        let (score, size_str) = match size_sim {
            Some(ratio) => (
                (token_sim * 0.45) + (dep_sim * 0.45) + (ratio * 0.10),
                format!(", {:.0}% size ratio", ratio * 100.0),
            ),
            None => ((token_sim * 0.50) + (dep_sim * 0.50), String::new()),
        };
        let expl = format!(
            "Score {:.2}: {:.0}% token overlap ({} shared: {:?}), {:.0}% dependency overlap ({} shared: {:?}){}",
            score,
            token_sim * 100.0,
            shared_tokens.len(),
            shared_tokens,
            dep_sim * 100.0,
            shared_dependencies.len(),
            shared_dependencies,
            size_str
        );
        (score, expl)
    } else {
        let (score, size_str) = match size_sim {
            Some(ratio) => (
                (token_sim * 0.85) + (ratio * 0.15),
                format!(", {:.0}% size ratio", ratio * 100.0),
            ),
            None => (token_sim, String::new()),
        };
        let expl = format!(
            "Score {:.2}: {:.0}% token overlap ({} shared: {:?}){}",
            score,
            token_sim * 100.0,
            shared_tokens.len(),
            shared_tokens,
            size_str
        );
        (score, expl)
    };

    CandidateMatch {
        candidate_id: target.candidate_id.clone(),
        kind: target.kind.clone(),
        score: overall_score,
        token_similarity: token_sim,
        dep_similarity: dep_sim,
        size_similarity: size_sim,
        shared_tokens,
        shared_dependencies,
        explanation,
    }
}

/// Recursively discovers all JSON candidate records in `metadata_dir`.
pub fn load_all_candidates(metadata_dir: &Path) -> Result<Vec<CandidateFeatures>> {
    if !metadata_dir.is_absolute() {
        bail!(
            "metadata_dir must be an absolute path: {}",
            metadata_dir.display()
        );
    }

    let mut records = Vec::new();
    if !metadata_dir.exists() {
        return Ok(records);
    }

    fn visit_dir(dir: &Path, acc: &mut Vec<PathBuf>) -> Result<()> {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    visit_dir(&p, acc)?;
                } else if p.extension().and_then(|s| s.to_str()) == Some("json") {
                    acc.push(p);
                }
            }
        }
        Ok(())
    }

    let mut json_paths = Vec::new();
    visit_dir(metadata_dir, &mut json_paths)?;

    for path in json_paths {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(rec) = serde_json::from_str::<CandidateRecord>(&content) {
                records.push(extract_features(&rec));
            }
        }
    }

    Ok(records)
}

/// Query indexed candidate records for the top-k most similar items to `query_id`.
pub fn query_candidates(
    metadata_dir: &Path,
    query_id: &str,
    kind_filter: Option<&str>,
    min_score: f64,
    limit: usize,
) -> Result<Vec<CandidateMatch>> {
    let all = load_all_candidates(metadata_dir)?;

    let query_features = all
        .iter()
        .find(|c| c.candidate_id.eq_ignore_ascii_case(query_id))
        .cloned()
        .unwrap_or_else(|| CandidateFeatures {
            candidate_id: query_id.to_string(),
            kind: kind_filter.unwrap_or("unknown").to_string(),
            tokens: tokenize(query_id),
            dependencies: BTreeSet::new(),
            total_lines: None,
            file_count: None,
            passed_g1: None,
            passed_g5: None,
            license: None,
            last_updated_secs: 0,
        });

    let mut matches = Vec::new();

    for candidate in &all {
        if candidate
            .candidate_id
            .eq_ignore_ascii_case(&query_features.candidate_id)
        {
            continue;
        }
        if let Some(k) = kind_filter {
            if !candidate.kind.eq_ignore_ascii_case(k) {
                continue;
            }
        }

        let scored = score_similarity(&query_features, candidate);
        if scored.score >= min_score {
            matches.push(scored);
        }
    }

    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.candidate_id.cmp(&b.candidate_id))
    });

    matches.truncate(limit);
    Ok(matches)
}

/// CLI runner for `foundation query-candidates`.
pub fn run(
    metadata_dir: &Path,
    like: &str,
    kind: Option<&str>,
    min_score: f64,
    limit: usize,
    json: bool,
) -> Result<()> {
    let matches = query_candidates(metadata_dir, like, kind, min_score, limit)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&matches)?);
        return Ok(());
    }

    println!("=== CROSS-CANDIDATE RETRIEVAL (M59) ===");
    println!("Query Target:     {}", like);
    println!("Metadata Store:   {}", metadata_dir.display());
    println!("Matches Found:    {}", matches.len());
    println!("--------------------------------------------------");

    if matches.is_empty() {
        println!(
            "No similar candidates found clearing min_score = {:.2}.",
            min_score
        );
        return Ok(());
    }

    for (idx, m) in matches.iter().enumerate() {
        println!(
            "[{}] {} ({}) — Score: {:.2}",
            idx + 1,
            m.candidate_id,
            m.kind,
            m.score
        );
        println!("    {}", m.explanation);
    }
    println!("==================================================");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate_metadata::PassEntry;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("bevy_ecs/query_state-v2");
        assert!(tokens.contains("bevy"));
        assert!(tokens.contains("ecs"));
        assert!(tokens.contains("query"));
        assert!(tokens.contains("state"));
        assert!(tokens.contains("v2"));
    }

    #[test]
    fn test_jaccard_similarity() {
        let mut a = BTreeSet::new();
        a.insert("serde".to_string());
        a.insert("bytemuck".to_string());

        let mut b = BTreeSet::new();
        b.insert("serde".to_string());
        b.insert("tokio".to_string());

        let sim = jaccard_similarity(&a, &b);
        // Intersection = 1 ("serde"), Union = 3 ("serde", "bytemuck", "tokio") -> 1/3 = 0.333...
        assert!((sim - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_size_similarity() {
        assert_eq!(size_similarity(Some(100), Some(100)), Some(1.0));
        assert_eq!(size_similarity(Some(50), Some(100)), Some(0.5));
        assert_eq!(size_similarity(None, Some(100)), None);
    }

    #[test]
    fn test_extract_and_score_similarity() {
        let rec1 = CandidateRecord {
            schema_version: 1,
            candidate_id: "ring_buffer_core".into(),
            kind: "crate".into(),
            created_at_secs: 100,
            updated_at_secs: 100,
            passes: vec![PassEntry {
                pass_name: "mechanical_prefilter".into(),
                recorded_at_secs: 100,
                findings: serde_json::json!({
                    "dependencies": [
                        { "crate_id": "bytemuck" },
                        { "crate_id": "serde" }
                    ],
                    "spdx_license": "Apache-2.0 OR MIT"
                }),
            }],
        };

        let rec2 = CandidateRecord {
            schema_version: 1,
            candidate_id: "swmr_ring_buffer".into(),
            kind: "crate".into(),
            created_at_secs: 105,
            updated_at_secs: 105,
            passes: vec![PassEntry {
                pass_name: "mechanical_prefilter".into(),
                recorded_at_secs: 105,
                findings: serde_json::json!({
                    "dependencies": [
                        { "crate_id": "bytemuck" },
                        { "crate_id": "rkyv" }
                    ],
                    "spdx_license": "MIT"
                }),
            }],
        };

        let feat1 = extract_features(&rec1);
        let feat2 = extract_features(&rec2);

        let match_result = score_similarity(&feat1, &feat2);
        assert!(match_result.score > 0.0);
        assert!(match_result.shared_tokens.contains(&"ring".to_string()));
        assert!(match_result.shared_tokens.contains(&"buffer".to_string()));
        assert!(
            match_result
                .shared_dependencies
                .contains(&"bytemuck".to_string())
        );
    }

    #[test]
    fn test_query_candidates_on_disk() {
        let temp = tempfile::tempdir().expect("tempdir");
        let metadata_dir = temp.path().join("metadata");
        fs::create_dir_all(metadata_dir.join("crate")).expect("create crate dir");

        let rec1 = CandidateRecord {
            schema_version: 1,
            candidate_id: "bevy_ecs".into(),
            kind: "crate".into(),
            created_at_secs: 100,
            updated_at_secs: 100,
            passes: vec![PassEntry {
                pass_name: "mechanical_prefilter".into(),
                recorded_at_secs: 100,
                findings: serde_json::json!({
                    "dependencies": [{ "crate_id": "bitflags" }],
                    "spdx_license": "MIT OR Apache-2.0"
                }),
            }],
        };

        let rec2 = CandidateRecord {
            schema_version: 1,
            candidate_id: "bevy_app".into(),
            kind: "crate".into(),
            created_at_secs: 105,
            updated_at_secs: 105,
            passes: vec![PassEntry {
                pass_name: "mechanical_prefilter".into(),
                recorded_at_secs: 105,
                findings: serde_json::json!({
                    "dependencies": [{ "crate_id": "bitflags" }],
                    "spdx_license": "MIT"
                }),
            }],
        };

        let rec3 = CandidateRecord {
            schema_version: 1,
            candidate_id: "unrelated_tool".into(),
            kind: "crate".into(),
            created_at_secs: 110,
            updated_at_secs: 110,
            passes: vec![PassEntry {
                pass_name: "mechanical_prefilter".into(),
                recorded_at_secs: 110,
                findings: serde_json::json!({
                    "dependencies": [{ "crate_id": "clap" }],
                    "spdx_license": "GPL-3.0"
                }),
            }],
        };

        fs::write(
            metadata_dir.join("crate").join("bevy_ecs.json"),
            serde_json::to_string(&rec1).unwrap(),
        )
        .unwrap();
        fs::write(
            metadata_dir.join("crate").join("bevy_app.json"),
            serde_json::to_string(&rec2).unwrap(),
        )
        .unwrap();
        fs::write(
            metadata_dir.join("crate").join("unrelated_tool.json"),
            serde_json::to_string(&rec3).unwrap(),
        )
        .unwrap();

        let matches = query_candidates(&metadata_dir, "bevy_ecs", None, 0.1, 5).expect("query");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].candidate_id, "bevy_app");
        assert!(matches[0].shared_tokens.contains(&"bevy".to_string()));
        assert!(
            matches[0]
                .shared_dependencies
                .contains(&"bitflags".to_string())
        );
    }
}
