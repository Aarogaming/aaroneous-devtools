use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, time::Duration};
#[derive(Serialize)]
struct Req<'a> {
    model: &'a str,
    stream: bool,
    options: Opt,
    messages: [Msg<'a>; 2],
}
#[derive(Serialize)]
struct Opt {
    temperature: f32,
    num_predict: u32,
}
#[derive(Serialize)]
struct Msg<'a> {
    role: &'a str,
    content: &'a str,
}
#[derive(Deserialize)]
struct Resp {
    message: Body,
}
#[derive(Deserialize)]
struct Body {
    content: String,
}
#[derive(Deserialize, Serialize)]
struct Intake {
    schema_version: u32,
    id: String,
    source: String,
    priority: String,
    scope: Vec<String>,
    dependencies: Vec<String>,
    verification: Vec<String>,
    decision: String,
}
const SYSTEM: &str = "You are a Rust program-management reviewer. Do not output thinking or markdown. Using only declared evidence, return one JSON object with schema_version=1,id,source,priority(P0/P1/P2/P3),scope array,dependencies array,verification array,decision. Do not propose commands, patches, claims, or repository writes.";
pub(crate) fn run(endpoint: &str, evidence: &Path, output: &Path) -> Result<()> {
    if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
        bail!("endpoint must be http(s)");
    }
    if !evidence.is_absolute() || !output.is_absolute() {
        bail!("evidence and output must be absolute");
    }
    let text = fs::read_to_string(evidence).context("read evidence")?;
    if text.len() > 65536 {
        bail!("evidence too large");
    }
    let req = Req {
        model: "qwen3.5:9b-q6",
        stream: false,
        options: Opt {
            temperature: 0.0,
            num_predict: 4096,
        },
        messages: [
            Msg {
                role: "system",
                content: SYSTEM,
            },
            Msg {
                role: "user",
                content: &text,
            },
        ],
    };
    let url = format!("{}/api/chat", endpoint.trim_end_matches('/'));
    let response: Resp = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()?
        .post(url)
        .json(&req)
        .send()?
        .error_for_status()?
        .json()?;
    let intake: Intake = serde_json::from_str(&response.message.content)
        .context("model did not return intake JSON")?;
    if intake.schema_version != 1
        || intake.id.is_empty()
        || intake.source.is_empty()
        || intake.scope.is_empty()
        || intake.verification.is_empty()
        || intake.decision.is_empty()
    {
        bail!("incomplete intake");
    }
    fs::write(output, serde_json::to_vec_pretty(&intake)?)?;
    Ok(())
}
