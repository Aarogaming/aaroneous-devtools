//! Explicit, immutable topic case revisions. These artifacts confer no queue or review authority.
use anyhow::{Context, Result, ensure};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Subcommand)]
pub(crate) enum Action {
    /// Validate and summarize an explicit case file (prints JSON, writes nothing).
    Review {
        #[arg(long)]
        case: PathBuf,
    },
    /// Verify that a revision is exactly one appended event after an explicit parent.
    VerifyParent {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        parent: PathBuf,
    },
    /// Append one event into a NEW immutable revision; never overwrites the input or output.
    Revise {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        event: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Export one qualified experiment in the existing Foundation intake v1 format.
    ExportIntake {
        #[arg(long)]
        case: PathBuf,
        #[arg(long)]
        experiment: String,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Case {
    schema_version: u32,
    artifact_kind: String,
    parent_digest: Option<String>,
    events: Vec<Entry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    event: Event,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Event {
    Topic { topic: Topic },
    Contribution { contribution: Contribution },
    Relationship { relationship: Relationship },
    RelationshipRetired { retirement: Retirement },
    Experiment { experiment: Box<Experiment> },
    Outcome { outcome: Outcome },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    reference: String,
    revision: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Topic {
    id: String,
    parent: Option<String>,
    title: String,
    question: String,
    owner: String,
    tags: Vec<String>,
    evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ContributionKind {
    Idea,
    Alternative,
    Dissent,
    Evidence,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Contribution {
    id: String,
    topic: String,
    actor: String,
    kind: ContributionKind,
    text: String,
    lineage: String,
    evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RelationKind {
    DependsOn,
    Related,
    Contradicts,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Relationship {
    id: String,
    from: String,
    to: String,
    kind: RelationKind,
    reason: String,
    evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Amount {
    value: f64,
    unit: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Retirement {
    relationship: String,
    actor: String,
    reason: String,
    evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
enum Baseline {
    Measured { value: f64, evidence: Vec<Evidence> },
    NotMeasured { reason: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Direction {
    AtLeast,
    AtMost,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Experiment {
    id: String,
    topic: String,
    contributions: Vec<String>,
    owner: String,
    intended_outcome: String,
    metric: String,
    unit: String,
    baseline: Baseline,
    method: String,
    direction: Direction,
    threshold: f64,
    budget: Amount,
    time_budget_hours: f64,
    stop_condition: String,
    work_item: String,
    review_destination: String,
    previous_attempt: Option<String>,
    revision_delta: Option<String>,
    intake: Intake,
}

/// Same representation as intake::Proposal; converted and checked by the existing validator.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Verdict {
    Accepted,
    Rejected,
    NeedsChanges,
    Inconclusive,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Outcome {
    experiment: String,
    work_item: String,
    reviewer: String,
    verdict: Verdict,
    review: Evidence,
    actual_cost: Amount,
    actual_hours: f64,
    measured_value: Option<f64>,
    measurement_evidence: Vec<Evidence>,
    unresolved: Vec<String>,
    lesson: String,
}

#[derive(Default)]
struct Projection {
    topics: BTreeMap<String, Topic>,
    contributions: BTreeMap<String, Contribution>,
    relationships: BTreeMap<String, Relationship>,
    retirements: BTreeMap<String, Retirement>,
    experiments: BTreeMap<String, Experiment>,
    outcomes: BTreeMap<String, Outcome>,
}

fn text(value: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && !value.chars().any(char::is_control),
        "text must be nonempty and contain no control characters"
    );
    Ok(())
}

fn id(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "invalid stable ID: {value}"
    );
    Ok(())
}

fn evidence(values: &[Evidence]) -> Result<()> {
    ensure!(
        !values.is_empty(),
        "at least one revision-pinned evidence reference is required"
    );
    for v in values {
        text(&v.reference)?;
        text(&v.revision)?;
    }
    Ok(())
}

fn finite(value: f64) -> Result<()> {
    ensure!(value.is_finite(), "measurement must be finite");
    Ok(())
}
fn nonnegative(value: f64) -> Result<()> {
    finite(value)?;
    ensure!(value >= 0.0, "cost/time must be nonnegative");
    Ok(())
}
fn amount(value: &Amount) -> Result<()> {
    nonnegative(value.value)?;
    text(&value.unit)
}

fn unique(values: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for v in values {
        text(v)?;
        ensure!(seen.insert(v), "duplicate value: {v}");
    }
    Ok(())
}

fn intake(value: &Intake) -> Result<crate::intake::Proposal> {
    let proposal: crate::intake::Proposal = serde_json::from_value(serde_json::to_value(value)?)?;
    proposal.check()?;
    Ok(proposal)
}

impl Projection {
    fn has_topic(&self, topic: &str) -> Result<()> {
        ensure!(self.topics.contains_key(topic), "unknown topic: {topic}");
        Ok(())
    }

    fn apply(&mut self, event: &Event) -> Result<()> {
        match event {
            Event::Topic { topic: v } => {
                id(&v.id)?;
                text(&v.title)?;
                text(&v.question)?;
                text(&v.owner)?;
                evidence(&v.evidence)?;
                unique(&v.tags)?;
                ensure!(
                    !self.topics.contains_key(&v.id),
                    "duplicate topic: {}",
                    v.id
                );
                if let Some(parent) = &v.parent {
                    self.has_topic(parent)?;
                }
                // Parent must already exist and identity cannot be replaced: hierarchy stays acyclic.
                self.topics.insert(v.id.clone(), v.clone());
            }
            Event::Contribution { contribution: v } => {
                id(&v.id)?;
                self.has_topic(&v.topic)?;
                text(&v.actor)?;
                text(&v.text)?;
                text(&v.lineage)?;
                evidence(&v.evidence)?;
                ensure!(
                    !self.contributions.contains_key(&v.id),
                    "duplicate contribution: {}",
                    v.id
                );
                self.contributions.insert(v.id.clone(), v.clone());
            }
            Event::Relationship { relationship: v } => {
                id(&v.id)?;
                self.has_topic(&v.from)?;
                self.has_topic(&v.to)?;
                text(&v.reason)?;
                evidence(&v.evidence)?;
                ensure!(
                    v.from != v.to && !self.relationships.contains_key(&v.id),
                    "self or duplicate relationship"
                );
                ensure!(
                    self.relationships
                        .values()
                        .filter(|r| !self.retirements.contains_key(&r.id))
                        .all(|r| r.from != v.from || r.to != v.to || r.kind != v.kind),
                    "duplicate relationship endpoints/kind"
                );
                if v.kind == RelationKind::DependsOn {
                    let mut pending = vec![v.to.as_str()];
                    let mut visited = BTreeSet::new();
                    while let Some(node) = pending.pop() {
                        ensure!(node != v.from, "dependency cycle");
                        if visited.insert(node) {
                            pending.extend(
                                self.relationships
                                    .values()
                                    .filter(|r| {
                                        r.kind == RelationKind::DependsOn
                                            && r.from == node
                                            && !self.retirements.contains_key(&r.id)
                                    })
                                    .map(|r| r.to.as_str()),
                            );
                        }
                    }
                }
                self.relationships.insert(v.id.clone(), v.clone());
            }
            Event::RelationshipRetired { retirement: v } => {
                ensure!(
                    self.relationships.contains_key(&v.relationship),
                    "unknown relationship to retire"
                );
                ensure!(
                    !self.retirements.contains_key(&v.relationship),
                    "relationship already retired"
                );
                text(&v.actor)?;
                text(&v.reason)?;
                evidence(&v.evidence)?;
                self.retirements.insert(v.relationship.clone(), v.clone());
            }
            Event::Experiment { experiment: v } => {
                id(&v.id)?;
                self.has_topic(&v.topic)?;
                ensure!(
                    !self.experiments.contains_key(&v.id),
                    "duplicate experiment: {}",
                    v.id
                );
                ensure!(!v.contributions.is_empty(), "promotion needs contributions");
                unique(&v.contributions)?;
                for c in &v.contributions {
                    ensure!(
                        self.contributions
                            .get(c)
                            .is_some_and(|c| c.topic == v.topic),
                        "unknown or cross-topic contribution: {c}"
                    );
                }
                for field in [
                    &v.owner,
                    &v.intended_outcome,
                    &v.metric,
                    &v.unit,
                    &v.method,
                    &v.stop_condition,
                    &v.work_item,
                    &v.review_destination,
                ] {
                    text(field)?;
                }
                match &v.baseline {
                    Baseline::Measured {
                        value,
                        evidence: refs,
                    } => {
                        finite(*value)?;
                        evidence(refs)?;
                    }
                    Baseline::NotMeasured { reason } => text(reason)?,
                }
                finite(v.threshold)?;
                amount(&v.budget)?;
                nonnegative(v.time_budget_hours)?;
                ensure!(v.time_budget_hours > 0.0, "time budget must be positive");
                intake(&v.intake)?;
                ensure!(
                    self.experiments
                        .values()
                        .all(|e| e.intake.id != v.intake.id),
                    "intake ID already used by another experiment"
                );
                if let Some(previous) = &v.previous_attempt {
                    let previous = self
                        .experiments
                        .get(previous)
                        .context("unknown previous attempt")?;
                    ensure!(previous.topic == v.topic, "retry must belong to same topic");
                    let receipt = self
                        .outcomes
                        .get(&previous.id)
                        .context("retry requires previous outcome receipt")?;
                    ensure!(
                        !matches!(receipt.verdict, Verdict::Accepted),
                        "accepted attempt cannot be retried"
                    );
                    text(
                        v.revision_delta
                            .as_deref()
                            .context("retry requires meaningful revision_delta")?,
                    )?;
                } else {
                    ensure!(
                        v.revision_delta.is_none(),
                        "revision_delta requires previous_attempt"
                    );
                }
                self.experiments.insert(v.id.clone(), v.as_ref().clone());
            }
            Event::Outcome { outcome: v } => {
                let experiment = self
                    .experiments
                    .get(&v.experiment)
                    .context("unknown experiment")?;
                ensure!(
                    !self.outcomes.contains_key(&v.experiment),
                    "outcome already recorded; retain history"
                );
                ensure!(
                    v.work_item == experiment.work_item,
                    "receipt work item mismatch"
                );
                text(&v.reviewer)?;
                evidence(std::slice::from_ref(&v.review))?;
                amount(&v.actual_cost)?;
                ensure!(
                    v.actual_cost.unit == experiment.budget.unit,
                    "cost unit mismatch"
                );
                nonnegative(v.actual_hours)?;
                unique(&v.unresolved)?;
                text(&v.lesson)?;
                if let Some(value) = v.measured_value {
                    finite(value)?;
                    evidence(&v.measurement_evidence)?;
                    if let Baseline::Measured {
                        value: baseline, ..
                    } = experiment.baseline
                    {
                        finite(value - baseline)?;
                    }
                } else {
                    ensure!(
                        v.measurement_evidence.is_empty(),
                        "unmeasured outcome cannot claim measurement evidence"
                    );
                }
                self.outcomes.insert(v.experiment.clone(), v.clone());
            }
        }
        Ok(())
    }
}

fn project(case: &Case) -> Result<Projection> {
    ensure!(
        case.schema_version == 1 && case.artifact_kind == "foundation.topic-case",
        "unsupported topic case schema/kind"
    );
    ensure!(case.events.len() <= 4096, "too many events");
    if let Some(digest) = &case.parent_digest {
        ensure!(
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "invalid parent digest"
        );
    }
    let mut projection = Projection::default();
    let mut ids = BTreeSet::new();
    for entry in &case.events {
        id(&entry.id)?;
        ensure!(ids.insert(&entry.id), "duplicate event: {}", entry.id);
        projection
            .apply(&entry.event)
            .with_context(|| format!("event {}", entry.id))?;
    }
    ensure!(!projection.topics.is_empty(), "case requires a topic");
    Ok(projection)
}

fn read(path: &Path) -> Result<String> {
    ensure!(path.is_absolute(), "input must be absolute");
    crate::intake::read_bounded(path, 4 * 1024 * 1024)
}

fn publish(path: &Path, value: &impl Serialize, limit: usize) -> Result<()> {
    ensure!(path.is_absolute(), "output must be absolute");
    crate::require_absolute_directory("output parent", path.parent().context("output parent")?)?;
    // Publish a synced staging inode through an exclusive hard link; no partial final file is exposed.
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    ensure!(bytes.len() <= limit, "output exceeds {limit} bytes");
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    struct Staging(PathBuf);
    impl Drop for Staging {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let mut stage = None;
    for _ in 0..16 {
        let candidate = path.parent().context("output parent")?.join(format!(
            ".topic-case-{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                stage = Some((Staging(candidate), file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    let (stage, mut file) = stage.context("could not reserve staging file")?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::hard_link(&stage.0, path).with_context(|| {
        format!(
            "output must be new and filesystem must support hard links: {}",
            path.display()
        )
    })?;
    Ok(())
}

pub(crate) fn run(action: Action) -> Result<()> {
    let case_path = match &action {
        Action::Review { case }
        | Action::VerifyParent { case, .. }
        | Action::Revise { case, .. }
        | Action::ExportIntake { case, .. } => case,
    };
    let raw = read(case_path)?;
    let mut case: Case = serde_json::from_str(&raw).context("invalid topic case JSON")?;
    let projection = project(&case)?;
    match action {
        Action::VerifyParent { parent, .. } => {
            let parent_raw = read(&parent)?;
            let previous: Case = serde_json::from_str(&parent_raw)?;
            project(&previous)?;
            ensure!(
                case.parent_digest.as_deref()
                    == Some(crate::virtual_patch::sha256_digest(parent_raw.as_bytes()).as_str()),
                "parent digest mismatch"
            );
            ensure!(
                case.events.len() == previous.events.len() + 1,
                "revision must append exactly one event"
            );
            ensure!(
                serde_json::to_value(&case.events[..previous.events.len()])?
                    == serde_json::to_value(&previous.events)?,
                "revision rewrote parent history"
            );
            println!("parent verified; artifact references and review authority remain unverified");
            Ok(())
        }
        Action::Review { .. } => {
            let experiments: Vec<_> = projection.experiments.values().map(|e| {
                let outcome = projection.outcomes.get(&e.id);
                let threshold_met = outcome.and_then(|o| o.measured_value).map(|value| match e.direction { Direction::AtLeast => value >= e.threshold, Direction::AtMost => value <= e.threshold });
                let delta = match (&e.baseline, outcome.and_then(|o| o.measured_value)) { (Baseline::Measured { value, .. }, Some(actual)) => Some(actual - value), _ => None };
                serde_json::json!({"experiment":e, "outcome":outcome, "threshold_met":threshold_met, "measured_delta":delta, "budget_exceeded":outcome.map(|o| o.actual_cost.value > e.budget.value || o.actual_hours > e.time_budget_hours)})
            }).collect();
            let active_relationships: Vec<_> = projection
                .relationships
                .keys()
                .filter(|id| !projection.retirements.contains_key(*id))
                .collect();
            let mut retry_children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
            for experiment in projection.experiments.values() {
                if let Some(previous) = &experiment.previous_attempt {
                    retry_children
                        .entry(previous)
                        .or_default()
                        .push(&experiment.id);
                }
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "schema_version":1, "artifact_kind":"foundation.topic-review", "case_digest":crate::virtual_patch::sha256_digest(raw.as_bytes()),
                    "requires_owner_review":true, "references_verified":false, "topics":projection.topics, "contributions":projection.contributions,
                    "relationships":projection.relationships, "relationship_retirements":projection.retirements, "active_relationships":active_relationships, "experiments":experiments, "retry_children":retry_children
                }))?
            );
            Ok(())
        }
        Action::Revise { event, output, .. } => {
            let entry: Entry =
                serde_json::from_str(&read(&event)?).context("invalid event JSON")?;
            case.parent_digest = Some(crate::virtual_patch::sha256_digest(raw.as_bytes()));
            case.events.push(entry);
            project(&case)?;
            publish(&output, &case, 4 * 1024 * 1024)
        }
        Action::ExportIntake {
            experiment, output, ..
        } => {
            let experiment = projection
                .experiments
                .get(&experiment)
                .context("unknown experiment")?;
            ensure!(
                !projection.outcomes.contains_key(&experiment.id),
                "experiment already closed"
            );
            let mut proposal = intake(&experiment.intake)?;
            proposal.decision.push_str(&format!(" Topic: {}; experiment: {}; case-sha256: {}; declared work item: {}; review destination: {}. Structural export only; owner review and canonical queue claim required.", experiment.topic, experiment.id, crate::virtual_patch::sha256_digest(raw.as_bytes()), experiment.work_item, experiment.review_destination));
            publish(&output, &proposal, 65536)
        }
    }
}
