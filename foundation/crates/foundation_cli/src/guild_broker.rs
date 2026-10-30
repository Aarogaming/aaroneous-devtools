//! Untrusted snapshot preview, deliberately separate from live Guild cards.
use anyhow::{Context, Result, bail};
use clap::Parser;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs::File, io::Read, path::PathBuf};
use steward::intelligence_broker as broker;

const MAX_INPUT: u64 = 1024 * 1024;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    /// Absolute UTF-8 JSON snapshot path, at most 1 MiB; output goes to stdout
    #[arg(long)]
    pub input: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    v: u32,
    job: JobInput,
    providers: Vec<ProviderInput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Key {
    provider: String,
    revision: String,
}
impl From<Key> for broker::ProfileKey {
    fn from(k: Key) -> Self {
        Self {
            provider: k.provider,
            revision: k.revision,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Requirement {
    capability: String,
    suite_revision: String,
    minimum_samples: u64,
    minimum_pass_bps: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JobInput {
    id: String,
    subject_revision: String,
    now: u64,
    deadline: u64,
    max_evidence_age: u64,
    remaining_budget: u64,
    interface: String,
    requirements: Vec<Requirement>,
    authorized: Vec<Key>,
    excluded_principals: Vec<String>,
}
impl From<JobInput> for broker::Job {
    fn from(j: JobInput) -> Self {
        Self {
            id: j.id,
            subject_revision: j.subject_revision,
            now: j.now,
            deadline: j.deadline,
            max_evidence_age: j.max_evidence_age,
            remaining_budget: j.remaining_budget,
            interface: j.interface,
            requirements: j
                .requirements
                .into_iter()
                .map(|r| broker::Requirement {
                    capability: r.capability,
                    suite_revision: r.suite_revision,
                    minimum_samples: r.minimum_samples,
                    minimum_pass_bps: r.minimum_pass_bps,
                })
                .collect(),
            authorized: j.authorized.into_iter().map(Into::into).collect(),
            excluded_principals: j.excluded_principals,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Model,
    Human,
    Tool,
    Other,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    capability: String,
    suite_revision: String,
    reference: String,
    observed_at: u64,
    passed: u64,
    samples: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderInput {
    key: Key,
    principal: String,
    kind: Kind,
    available: bool,
    resources_fit: bool,
    interface: String,
    cost_bound: Option<u64>,
    duration_bound: Option<u64>,
    evidence: Vec<Evidence>,
}
impl From<ProviderInput> for broker::Provider {
    fn from(p: ProviderInput) -> Self {
        Self {
            key: p.key.into(),
            principal: p.principal,
            kind: match p.kind {
                Kind::Model => broker::ProviderKind::Model,
                Kind::Human => broker::ProviderKind::Human,
                Kind::Tool => broker::ProviderKind::Tool,
                Kind::Other => broker::ProviderKind::Other,
            },
            available: p.available,
            resources_fit: p.resources_fit,
            interface: p.interface,
            cost_bound: p.cost_bound,
            duration_bound: p.duration_bound,
            evidence: p
                .evidence
                .into_iter()
                .map(|e| broker::Evidence {
                    capability: e.capability,
                    suite_revision: e.suite_revision,
                    reference: e.reference,
                    observed_at: e.observed_at,
                    passed: e.passed,
                    samples: e.samples,
                })
                .collect(),
        }
    }
}

fn key(k: &broker::ProfileKey) -> Value {
    json!({"provider": k.provider, "revision": k.revision})
}

fn reason(r: &broker::HoldReason) -> Value {
    use broker::HoldReason::*;
    let (code, capability) = match r {
        Unauthorized => ("unauthorized", None),
        PrincipalExcluded => ("principal_excluded", None),
        Unavailable => ("unavailable", None),
        Resources => ("resources", None),
        Interface => ("interface", None),
        UnknownQuote => ("unknown_quote", None),
        Budget => ("budget", None),
        Deadline => ("deadline", None),
        MissingEvidence(c) => ("missing_evidence", Some(c)),
        InvalidEvidence(c) => ("invalid_evidence", Some(c)),
        StaleEvidence(c) => ("stale_evidence", Some(c)),
        InsufficientEvidence(c) => ("insufficient_evidence", Some(c)),
        BelowThreshold(c) => ("below_threshold", Some(c)),
    };
    json!({"code":code,"capability":capability})
}

pub fn run(args: &Args) -> Result<()> {
    if !args.input.is_absolute() {
        bail!("input must be an explicit absolute path");
    }
    let file = File::open(&args.input).context("open broker snapshot")?;
    if !file.metadata()?.is_file() {
        bail!("snapshot must be a regular file");
    }
    let mut bytes = Vec::new();
    file.take(MAX_INPUT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_INPUT {
        bail!("snapshot exceeds 1 MiB");
    }
    let raw = String::from_utf8(bytes).context("snapshot must be UTF-8")?;
    // Deserialize from original bytes: duplicate struct fields cannot disappear
    // through an intermediate Value map. Serde's recursion limit remains enabled.
    let snapshot: Snapshot = serde_json::from_str(&raw).context("invalid broker snapshot")?;
    if snapshot.v != 1 {
        bail!("unsupported broker snapshot version");
    }
    let job = snapshot.job.into();
    let providers: Vec<_> = snapshot.providers.into_iter().map(Into::into).collect();
    let report = broker::assess(&job, &providers).map_err(anyhow::Error::msg)?;
    let ranked: Vec<_> = report
        .ranked
        .iter()
        .map(|c| {
            json!({
                "key":key(&c.key), "score_bps":c.score_bps, "cost_bound":c.cost_bound,
                "duration_bound":c.duration_bound, "evidence_references":c.evidence_references,
            })
        })
        .collect();
    let excluded: Vec<_> = report
        .excluded
        .iter()
        .map(|c| {
            json!({
                "key":key(&c.key),"reasons":c.reasons.iter().map(reason).collect::<Vec<_>>(),
            })
        })
        .collect();
    let output = json!({
        "v":1, "kind":"guild.broker-preview", "dispatch_authorized":false,
        "policy_verified":false, "evidence_verified":false,
        "decision":if ranked.is_empty() { "hold" } else { "candidates" },
        "job_id":report.job_id, "subject_revision":report.subject_revision,
        "evaluated_at":report.evaluated_at,
        "input_sha256":steward::chain::digest(&raw), "input_utf8":raw,
        "ranked":ranked, "excluded":excluded,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
