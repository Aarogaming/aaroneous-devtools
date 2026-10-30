//! Report-only capability routing. Inputs are supplied by trusted adapters;
//! this module neither authenticates evidence nor grants dispatch authority.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Model,
    Human,
    Tool,
    Other,
}

/// Exact evaluated configuration, including model/tool version and adapter revision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProfileKey {
    pub provider: String,
    pub revision: String,
}

#[derive(Debug, Clone)]
pub struct Evidence {
    pub capability: String,
    pub suite_revision: String,
    pub reference: String,
    /// Same caller-defined time domain as `Job::now`; future evidence is invalid.
    pub observed_at: u64,
    pub passed: u64,
    pub samples: u64,
}

#[derive(Debug, Clone)]
pub struct Provider {
    pub key: ProfileKey,
    pub principal: String,
    pub kind: ProviderKind,
    pub available: bool,
    /// Caller derives this from model_hub::fit or the relevant non-model adapter.
    pub resources_fit: bool,
    pub interface: String,
    /// Upper-bound quote in the job's single declared accounting unit.
    pub cost_bound: Option<u64>,
    pub duration_bound: Option<u64>,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone)]
pub struct Requirement {
    pub capability: String,
    pub suite_revision: String,
    pub minimum_samples: u64,
    pub minimum_pass_bps: u16,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub subject_revision: String,
    pub now: u64,
    pub deadline: u64,
    pub max_evidence_age: u64,
    /// Remaining aggregate operation budget; never a fresh per-retry allowance.
    pub remaining_budget: u64,
    pub interface: String,
    pub requirements: Vec<Requirement>,
    /// Trusted policy input for this exact job, subject and data scope.
    /// A provider's self-description must never populate this list.
    pub authorized: Vec<ProfileKey>,
    /// Implementers or other principals ineligible for this job's role.
    pub excluded_principals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HoldReason {
    Unauthorized,
    PrincipalExcluded,
    Unavailable,
    Resources,
    Interface,
    UnknownQuote,
    Budget,
    Deadline,
    MissingEvidence(String),
    InvalidEvidence(String),
    StaleEvidence(String),
    InsufficientEvidence(String),
    BelowThreshold(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub key: ProfileKey,
    /// Conservative ranking input: worst observed pass rate across requirements.
    /// Not a probability of success or a confidence interval.
    pub score_bps: u16,
    pub cost_bound: u64,
    pub duration_bound: u64,
    pub evidence_references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exclusion {
    pub key: ProfileKey,
    pub reasons: Vec<HoldReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub job_id: String,
    pub subject_revision: String,
    pub evaluated_at: u64,
    /// Empty means hold. Sorted by score descending, cost, duration, then identity.
    pub ranked: Vec<Candidate>,
    pub excluded: Vec<Exclusion>,
}

fn nonempty(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control)
}

// Canonical opaque principal IDs are ASCII lowercase and contain no whitespace or
// format characters. A trusted identity adapter must still resolve distinct aliases.
fn canonical_principal(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.' | b':')
        })
}

/// Deterministic recommendation only. Malformed or ambiguous identities fail the
/// entire report; a lack of eligibility produces explicit per-provider holds.
pub fn assess(job: &Job, providers: &[Provider]) -> Result<Report, &'static str> {
    if !nonempty(&job.id)
        || !nonempty(&job.subject_revision)
        || !nonempty(&job.interface)
        || job.requirements.is_empty()
        || job.requirements.len() > 64
        || providers.len() > 256
        || job.authorized.len() > 256
        || job.excluded_principals.len() > 256
        || job
            .excluded_principals
            .iter()
            .any(|p| !canonical_principal(p))
        || job.deadline < job.now
    {
        return Err("invalid or oversized job/provider set");
    }
    let mut requirements = BTreeSet::new();
    for r in &job.requirements {
        if !nonempty(&r.capability)
            || !nonempty(&r.suite_revision)
            || r.minimum_samples == 0
            || r.minimum_pass_bps > 10_000
            || !requirements.insert(&r.capability)
        {
            return Err("invalid or duplicate requirement");
        }
    }
    let mut identities = BTreeSet::new();
    let mut report = Report {
        job_id: job.id.clone(),
        subject_revision: job.subject_revision.clone(),
        evaluated_at: job.now,
        ranked: Vec::new(),
        excluded: Vec::new(),
    };
    for p in providers {
        if !nonempty(&p.key.provider)
            || !nonempty(&p.key.revision)
            || !canonical_principal(&p.principal)
            || !nonempty(&p.interface)
            || p.evidence.len() > 256
            || !identities.insert(&p.key)
        {
            return Err("invalid or duplicate provider profile");
        }
        let mut reasons = Vec::new();
        for (rejected, reason) in [
            (!job.authorized.contains(&p.key), HoldReason::Unauthorized),
            (
                job.excluded_principals.contains(&p.principal),
                HoldReason::PrincipalExcluded,
            ),
            (!p.available, HoldReason::Unavailable),
            (!p.resources_fit, HoldReason::Resources),
            (p.interface != job.interface, HoldReason::Interface),
        ] {
            if rejected {
                reasons.push(reason);
            }
        }
        match (p.cost_bound, p.duration_bound) {
            (Some(cost), Some(duration)) => {
                if cost > job.remaining_budget {
                    reasons.push(HoldReason::Budget);
                }
                if duration > job.deadline - job.now {
                    reasons.push(HoldReason::Deadline);
                }
            }
            _ => reasons.push(HoldReason::UnknownQuote),
        }
        let mut score = 10_000;
        let mut references = Vec::new();
        for r in &job.requirements {
            let matching: Vec<_> = p
                .evidence
                .iter()
                .filter(|e| e.capability == r.capability && e.suite_revision == r.suite_revision)
                .collect();
            if matching.is_empty() {
                reasons.push(HoldReason::MissingEvidence(r.capability.clone()));
                continue;
            }
            // Never select the most flattering observation from conflicting summaries.
            if matching.len() != 1 {
                reasons.push(HoldReason::InvalidEvidence(r.capability.clone()));
                continue;
            }
            let e = matching[0];
            if !nonempty(&e.reference)
                || e.samples == 0
                || e.passed > e.samples
                || e.observed_at > job.now
            {
                reasons.push(HoldReason::InvalidEvidence(r.capability.clone()));
                continue;
            }
            if job.now - e.observed_at > job.max_evidence_age {
                reasons.push(HoldReason::StaleEvidence(r.capability.clone()));
            }
            if e.samples < r.minimum_samples {
                reasons.push(HoldReason::InsufficientEvidence(r.capability.clone()));
            }
            let rate = (u128::from(e.passed) * 10_000 / u128::from(e.samples)) as u16;
            if rate < r.minimum_pass_bps {
                reasons.push(HoldReason::BelowThreshold(r.capability.clone()));
            }
            score = score.min(rate);
            references.push(e.reference.clone());
        }
        if reasons.is_empty() {
            if let (Some(cost_bound), Some(duration_bound)) = (p.cost_bound, p.duration_bound) {
                report.ranked.push(Candidate {
                    key: p.key.clone(),
                    score_bps: score,
                    cost_bound,
                    duration_bound,
                    evidence_references: references,
                });
            }
        } else {
            report.excluded.push(Exclusion {
                key: p.key.clone(),
                reasons,
            });
        }
    }
    report.ranked.sort_by(|a, b| {
        b.score_bps
            .cmp(&a.score_bps)
            .then(a.cost_bound.cmp(&b.cost_bound))
            .then(a.duration_bound.cmp(&b.duration_bound))
            .then(a.key.cmp(&b.key))
    });
    report.excluded.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(report)
}
