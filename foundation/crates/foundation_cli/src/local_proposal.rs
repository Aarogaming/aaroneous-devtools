use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, time::Duration};

const KIND: &str = "foundation.local-proposal";
const SYSTEM: &str = "You are a senior Rust systems reviewer. Do not output a thinking trace. Do not request command execution, repository writes, credentials, task claims, or automatic approval. Return only JSON with task, observations, risks, test_plan, recommended_next_step.";

#[derive(Serialize)]
struct Request<'a> {
    model: &'a str,
    stream: bool,
    options: Options,
    messages: [Message<'a>; 2],
}
#[derive(Serialize)]
struct Options {
    temperature: f32,
    num_predict: u32,
}
#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}
#[derive(Deserialize)]
struct Response {
    message: ResponseMessage,
}
#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}
#[derive(Deserialize, Serialize)]
struct Proposal {
    task: String,
    observations: Vec<String>,
    risks: Vec<String>,
    test_plan: Vec<String>,
    recommended_next_step: String,
}
#[derive(Serialize)]
struct Report {
    schema_version: u32,
    artifact_kind: String,
    endpoint: String,
    model: String,
    evidence: String,
    proposal: Proposal,
}

pub(crate) fn run(
    endpoint: &str,
    model: &str,
    evidence: &Path,
    output: &Path,
    num_predict: u32,
) -> Result<()> {
    if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
        bail!("endpoint must be explicit http(s) URL");
    }
    if model.trim().is_empty() || num_predict < 4096 {
        bail!("model must be non-empty and num_predict at least 4096");
    }
    if !evidence.is_absolute()
        || !fs::metadata(evidence)
            .with_context(|| "evidence unavailable")?
            .is_file()
    {
        bail!("evidence must be an available absolute file");
    }
    if !output.is_absolute()
        || !fs::metadata(output.parent().context("output parent missing")?)
            .with_context(|| "output parent unavailable")?
            .is_dir()
    {
        bail!("output needs an existing absolute parent");
    }
    let input = fs::read_to_string(evidence)
        .with_context(|| format!("failed to read {}", evidence.display()))?;
    if input.len() > 65_536 {
        bail!("evidence exceeds 65536 byte limit");
    }
    let prompt = format!(
        "Review only this declared evidence. Produce a proposal, never a patch.\n\n{input}"
    );
    let request = Request {
        model,
        stream: false,
        options: Options {
            temperature: 0.0,
            num_predict,
        },
        messages: [
            Message {
                role: "system",
                content: SYSTEM,
            },
            Message {
                role: "user",
                content: &prompt,
            },
        ],
    };
    let url = format!("{}/api/chat", endpoint.trim_end_matches('/'));
    let response: Response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .context("proposal client")?
        .post(url)
        .json(&request)
        .send()
        .context("local Ollama request")?
        .error_for_status()
        .context("local Ollama status")?
        .json()
        .context("local Ollama response")?;
    let proposal: Proposal = serde_json::from_str(&response.message.content)
        .context("model response is not proposal JSON")?;
    let report = Report {
        schema_version: 1,
        artifact_kind: KIND.to_owned(),
        endpoint: endpoint.to_owned(),
        model: model.to_owned(),
        evidence: evidence.display().to_string(),
        proposal,
    };
    fs::write(
        output,
        serde_json::to_vec_pretty(&report).context("proposal serialization")?,
    )
    .with_context(|| format!("failed to write {}", output.display()))?;
    println!("wrote local proposal: {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_small_budget() {
        let temp = tempfile::tempdir().expect("temp");
        let evidence = temp.path().join("e.json");
        fs::write(&evidence, "{}").expect("evidence");
        assert!(
            run(
                "http://localhost:1",
                "qwen",
                &evidence,
                &temp.path().join("o.json"),
                1
            )
            .is_err()
        );
    }
}
