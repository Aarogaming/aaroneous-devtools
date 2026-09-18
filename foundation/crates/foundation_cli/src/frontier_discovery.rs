use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NativeDiscoverySummary {
    pub checkout: String,
    pub branch: String,
    pub tracked_files_count: usize,
    pub remotes: Vec<String>,
    pub submodules_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrateCandidate {
    pub name: String,
    pub description: String,
    pub version: String,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub downloads: u64,
}

pub fn discover_native(checkout: &Path) -> Result<NativeDiscoverySummary> {
    if !checkout.is_absolute() {
        bail!("checkout must be an absolute path: {}", checkout.display());
    }

    let repo = gix::open(checkout)
        .with_context(|| format!("Failed to open repository at {}", checkout.display()))?;

    let branch = repo
        .head_name()
        .ok()
        .flatten()
        .map(|name| name.as_bstr().to_string())
        .unwrap_or_else(|| "HEAD (detached)".into());

    let remotes: Vec<String> = repo
        .remote_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let submodules_count = repo.submodules().ok().flatten().map(|s| s.count()).unwrap_or(0);

    let tracked_files_count = repo
        .index_or_empty()
        .map(|idx| idx.entries().len())
        .unwrap_or(0);

    Ok(NativeDiscoverySummary {
        checkout: checkout.display().to_string(),
        branch,
        tracked_files_count,
        remotes,
        submodules_count,
    })
}

pub fn search_crates_io(query: &str, limit: usize) -> Result<Vec<CrateCandidate>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("AaroneousDevtools/0.1.0 (https://github.com/Aarogaming/aaroneous)")
        .build()
        .context("Failed to construct reqwest client")?;

    let url = format!(
        "https://crates.io/api/v1/crates?q={}&per_page={}",
        urlencoding(query),
        limit.min(20)
    );

    let resp = client
        .get(&url)
        .send()
        .with_context(|| format!("Failed to query crates.io: {url}"))?;

    if !resp.status().is_success() {
        bail!("crates.io returned HTTP status {}", resp.status());
    }

    let json: serde_json::Value = resp.json().context("Failed to parse crates.io response JSON")?;
    let mut candidates = Vec::new();

    if let Some(crates) = json["crates"].as_array() {
        for c in crates {
            let name = c["name"].as_str().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            candidates.push(CrateCandidate {
                name,
                description: c["description"].as_str().unwrap_or("").to_string(),
                version: c["max_version"].as_str().unwrap_or("0.1.0").to_string(),
                repository: c["repository"].as_str().map(|s| s.to_string()),
                documentation: c["documentation"].as_str().map(|s| s.to_string()),
                downloads: c["downloads"].as_u64().unwrap_or(0),
            });
        }
    }

    Ok(candidates)
}

pub fn firecrawl_scrape(url: &str, endpoint: Option<&str>, api_key: Option<&str>) -> Result<String> {
    let target_endpoint = endpoint.unwrap_or("http://localhost:3002/v1/scrape");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to construct reqwest client")?;

    let mut req = client.post(target_endpoint).json(&serde_json::json!({
        "url": url,
        "formats": ["markdown"]
    }));

    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    let resp = req.send().with_context(|| format!("Firecrawl request failed for {url}"))?;
    if !resp.status().is_success() {
        bail!("Firecrawl scrape failed with status: {}", resp.status());
    }

    let val: serde_json::Value = resp.json().context("Failed to parse Firecrawl response")?;
    let markdown = val["data"]["markdown"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if markdown.trim().is_empty() {
        bail!("Firecrawl returned empty markdown content for {url}");
    }

    Ok(markdown)
}

pub fn create_frontier_job(
    candidate: &CrateCandidate,
    output_dir: &Path,
    evidence_rel_path: &str,
) -> Result<PathBuf> {
    if !output_dir.is_absolute() {
        bail!("output_dir must be an absolute path");
    }

    let job_id = format!("FRONTIER-{}", candidate.name.to_uppercase().replace('-', "_"));
    let filename = format!("frontier-{}.toml", candidate.name.to_lowercase().replace('_', "-"));
    let destination = output_dir.join(&filename);

    let instruction = format!(
        "Audit external open-source crate '{}' (v{}) for potential dogfooding or architectural influence in Aaroneous.\nDescription: {}\nRepository: {}\nEvaluate whether this crate provides zero-allocation, zero-ambient authority, or type-safe abstractions that could enhance our hypervisor, IPC bus, compute engine, or devtools.",
        candidate.name,
        candidate.version,
        candidate.description,
        candidate.repository.as_deref().unwrap_or("N/A")
    );

    let manifest = serde_json::json!({
        "schema_version": 1,
        "id": job_id,
        "mode": "analysis",
        "evidence": [evidence_rel_path],
        "instruction": instruction
    });

    let toml_str = toml::to_string_pretty(&manifest)?;
    std::fs::write(&destination, toml_str)?;
    Ok(destination)
}

fn urlencoding(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_discover_native_on_repo() {
        let temp = tempdir().unwrap();
        let _repo = gix::init(temp.path()).unwrap();
        let summary = discover_native(temp.path()).unwrap();
        assert_eq!(summary.tracked_files_count, 0);
        assert!(summary.remotes.is_empty());
    }

    #[test]
    fn test_create_frontier_job() {
        let temp = tempdir().unwrap();
        let candidate = CrateCandidate {
            name: "crossbeam-queue".into(),
            description: "Concurrent multi-producer multi-consumer queues".into(),
            version: "0.3.11".into(),
            repository: Some("https://github.com/crossbeam-rs/crossbeam".into()),
            documentation: None,
            downloads: 50_000_000,
        };

        let job_file = create_frontier_job(&candidate, temp.path(), "docs/ARCHITECTURE.md").unwrap();
        assert!(job_file.exists());
        let text = std::fs::read_to_string(job_file).unwrap();
        assert!(text.contains("FRONTIER-CROSSBEAM_QUEUE"));
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("ring-buffer"), "ring-buffer");
    }
}
