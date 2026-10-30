use steward::intelligence_broker::*;

#[test]
fn weak_required_skill_limits_rank_and_alias_cannot_evade_exclusion() {
    let mut a = provider("a");
    let mut b = provider("b");
    a.evidence[0].passed = 10;
    let mut secondary = a.evidence[0].clone();
    secondary.capability = "planning".into();
    secondary.passed = 9;
    a.evidence.push(secondary.clone());
    b.evidence.push(secondary);
    let mut j = job(&[a.clone(), b.clone()]);
    let mut requirement = j.requirements[0].clone();
    requirement.capability = "planning".into();
    j.requirements.push(requirement);
    let report = assess(&j, &[b.clone(), a.clone()]).unwrap();
    assert_eq!(report.ranked[0].score_bps, 9_000);
    assert_eq!(report.ranked[1].score_bps, 9_000);
    assert_eq!(report.ranked[0].key.provider, "a");
    b.principal = a.principal.clone();
    j.excluded_principals.push(a.principal);
    hold(&j, b, HoldReason::PrincipalExcluded);
}

fn provider(name: &str) -> Provider {
    Provider {
        key: ProfileKey {
            provider: name.into(),
            revision: "configuration-v1".into(),
        },
        principal: name.into(),
        kind: ProviderKind::Model,
        available: true,
        resources_fit: true,
        interface: "task-v1".into(),
        cost_bound: Some(10),
        duration_bound: Some(20),
        evidence: vec![Evidence {
            capability: "rust-review".into(),
            suite_revision: "suite-v1".into(),
            reference: "retained-evaluation-01".into(),
            observed_at: 90,
            passed: 9,
            samples: 10,
        }],
    }
}
fn job(providers: &[Provider]) -> Job {
    Job {
        id: "job-1".into(),
        subject_revision: "subject-v1".into(),
        now: 100,
        deadline: 120,
        max_evidence_age: 10,
        remaining_budget: 10,
        interface: "task-v1".into(),
        requirements: vec![Requirement {
            capability: "rust-review".into(),
            suite_revision: "suite-v1".into(),
            minimum_samples: 10,
            minimum_pass_bps: 9_000,
        }],
        authorized: providers.iter().map(|p| p.key.clone()).collect(),
        excluded_principals: vec![],
    }
}
fn hold(j: &Job, p: Provider, reason: HoldReason) {
    let report = assess(j, &[p]).unwrap();
    assert!(report.ranked.is_empty());
    assert!(
        report.excluded[0].reasons.contains(&reason),
        "{:?}",
        report.excluded
    );
}

#[test]
fn all_provider_kinds_follow_the_same_contract() {
    for kind in [
        ProviderKind::Model,
        ProviderKind::Human,
        ProviderKind::Tool,
        ProviderKind::Other,
    ] {
        let mut p = provider("worker");
        p.kind = kind;
        let j = job(std::slice::from_ref(&p));
        let r = assess(&j, &[p]).unwrap();
        assert_eq!(r.ranked.len(), 1);
        assert_eq!(r.subject_revision, j.subject_revision);
        assert_eq!(r.evaluated_at, 100);
        assert_eq!(r.ranked[0].evidence_references, ["retained-evaluation-01"]);
    }
}
#[test]
fn high_scores_cannot_override_hard_gates() {
    let original = provider("model");
    let j = job(std::slice::from_ref(&original));
    let mut p = original.clone();
    p.available = false;
    hold(&j, p, HoldReason::Unavailable);
    let mut p = original.clone();
    p.resources_fit = false;
    hold(&j, p, HoldReason::Resources);
    let mut p = original.clone();
    p.interface = "other".into();
    hold(&j, p, HoldReason::Interface);
    let mut p = original.clone();
    p.cost_bound = Some(11);
    hold(&j, p, HoldReason::Budget);
    let mut p = original.clone();
    p.duration_bound = Some(21);
    hold(&j, p, HoldReason::Deadline);
    let mut p = original.clone();
    p.cost_bound = None;
    hold(&j, p, HoldReason::UnknownQuote);
    let mut restricted = j.clone();
    restricted.authorized.clear();
    hold(&restricted, original.clone(), HoldReason::Unauthorized);
    let mut restricted = j;
    restricted.excluded_principals.push("model".into());
    hold(&restricted, original, HoldReason::PrincipalExcluded);
}
#[test]
fn configuration_and_suite_must_match() {
    let mut p = provider("model");
    let j = job(std::slice::from_ref(&p));
    p.key.revision = "configuration-v2".into();
    hold(&j, p.clone(), HoldReason::Unauthorized);
    p.key.revision = "configuration-v1".into();
    p.evidence[0].suite_revision = "obsolete".into();
    hold(&j, p, HoldReason::MissingEvidence("rust-review".into()));
}
#[test]
fn unknown_stale_future_and_conflicting_evidence_hold() {
    let original = provider("model");
    let j = job(std::slice::from_ref(&original));
    let mut p = original.clone();
    p.evidence.clear();
    hold(&j, p, HoldReason::MissingEvidence("rust-review".into()));
    let mut p = original.clone();
    p.evidence[0].observed_at = 89;
    hold(&j, p, HoldReason::StaleEvidence("rust-review".into()));
    let mut p = original.clone();
    p.evidence[0].observed_at = 101;
    hold(&j, p, HoldReason::InvalidEvidence("rust-review".into()));
    let mut p = original.clone();
    p.evidence.push(p.evidence[0].clone());
    hold(&j, p, HoldReason::InvalidEvidence("rust-review".into()));
    let mut p = original;
    p.evidence[0].reference.clear();
    hold(&j, p, HoldReason::InvalidEvidence("rust-review".into()));
}
#[test]
fn sample_quality_and_capability_requirements_are_independent() {
    let mut p = provider("model");
    let mut j = job(std::slice::from_ref(&p));
    p.evidence[0].samples = 9;
    hold(
        &j,
        p.clone(),
        HoldReason::InsufficientEvidence("rust-review".into()),
    );
    p.evidence[0].samples = 10;
    p.evidence[0].passed = 8;
    hold(
        &j,
        p.clone(),
        HoldReason::BelowThreshold("rust-review".into()),
    );
    p.evidence[0].passed = 9;
    let mut second = j.requirements[0].clone();
    second.capability = "security-review".into();
    j.requirements.push(second);
    hold(&j, p, HoldReason::MissingEvidence("security-review".into()));
}
#[test]
fn ranking_is_permutation_invariant_and_quality_precedes_cost() {
    let mut ps: Vec<_> = ["z", "a", "b", "c"].into_iter().map(provider).collect();
    ps[0].evidence[0].passed = 10;
    ps[1].cost_bound = Some(8);
    ps[2].cost_bound = Some(8);
    ps[2].duration_bound = Some(19);
    let j = job(&ps);
    let r = assess(&j, &ps).unwrap();
    assert_eq!(
        r.ranked
            .iter()
            .map(|c| c.key.provider.as_str())
            .collect::<Vec<_>>(),
        ["z", "b", "a", "c"]
    );
    ps.reverse();
    assert_eq!(r, assess(&j, &ps).unwrap());
}
#[test]
fn counters_do_not_overflow_or_accept_impossible_statistics() {
    let mut p = provider("model");
    let j = job(std::slice::from_ref(&p));
    p.evidence[0].samples = u64::MAX;
    p.evidence[0].passed = u64::MAX;
    assert_eq!(
        assess(&j, &[p.clone()]).unwrap().ranked[0].score_bps,
        10_000
    );
    p.evidence[0].samples = 0;
    hold(
        &j,
        p.clone(),
        HoldReason::InvalidEvidence("rust-review".into()),
    );
    p.evidence[0].samples = 10;
    p.evidence[0].passed = 11;
    hold(&j, p, HoldReason::InvalidEvidence("rust-review".into()));
}
#[test]
fn ambiguous_empty_and_oversized_inputs_are_errors() {
    let p = provider("model");
    let j = job(std::slice::from_ref(&p));
    assert!(assess(&j, &[p.clone(), p.clone()]).is_err());
    let mut bad = j.clone();
    bad.requirements.clear();
    assert!(assess(&bad, std::slice::from_ref(&p)).is_err());
    let mut bad = j.clone();
    bad.requirements.push(bad.requirements[0].clone());
    assert!(assess(&bad, std::slice::from_ref(&p)).is_err());
    let mut bad = j.clone();
    bad.deadline = 99;
    assert!(assess(&bad, std::slice::from_ref(&p)).is_err());
    assert!(assess(&j, &vec![p; 257]).is_err());
    assert!(assess(&j, &[]).unwrap().ranked.is_empty());
}
#[test]
fn retry_does_not_restore_budget_or_inherit_eligibility() {
    let p = provider("model");
    let mut j = job(std::slice::from_ref(&p));
    assert_eq!(
        assess(&j, std::slice::from_ref(&p)).unwrap().ranked.len(),
        1
    );
    j.remaining_budget = 0;
    hold(&j, p.clone(), HoldReason::Budget);
    j.remaining_budget = 10;
    j.authorized.clear();
    hold(&j, p, HoldReason::Unauthorized);
}
