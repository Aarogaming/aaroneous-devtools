# Discovery/Research/Filter/Translate/Recreate/Review/Repeat Loop

## Purpose

Every stage below already has a concrete mechanism — a real milestone, a real
governance document, or real existing code. What this document adds is the thing
none of them state individually: what triggers the next stage, and what closes the
loop back to the first one. Without this, the seven stages are independent
stations that each require a manual invocation; with it, they are one running
belt. This document names no new mechanism that isn't already specified
elsewhere — it is entirely a map of triggers between existing pieces.

## The stages and their mechanism

| Stage | Mechanism | Where |
| --- | --- | --- |
| **Discovery** (external) | `frontier_discovery::serve` — recurring crates.io/ecosystem search | M41, `foundation/crates/foundation_cli/src/frontier_discovery.rs` |
| **Discovery** (internal) | Capability composition explorer over `capability_broker.rs`'s registry | M47 |
| **Research** | Structural self-inventory before touching anything | `component_inventory.rs`, `dependency_review.rs` |
| **Filter** (external candidates) | Hard gates + quadratic scoring | `contracts/FRONTIER_FILTRATION_STANDARD_V1.md` |
| **Filter** (internal ideas / roadmap admission) | Hard gates + quadratic scoring, same method | `governance/BUILD_WORTHINESS_STANDARD.md` |
| **Translate** (candidate → compliant component) | `cratify`/`adaptation_engine`/`ast_auditor` pipeline | M45 |
| **Recreate** (draft new code) | Local-worker `code-draft` mode + local pre-proofing | M44 |
| **Review** (is a specific change mergeable) | Tiered evidence requirements + fixed condition table | `governance/CHANGE_VERIFICATION_TIERS.md`, `governance/REVIEW_CONDITIONS.md` |
| **Repeat** | This document | below |

## Triggers between stages

This is the part that was previously unwritten. Each arrow names what output of
the left stage becomes the input that starts the right stage, and under what
condition it fires automatically versus needing a person to kick it off.

1. **Discovery → Filter.** `frontier_discovery::serve` (M41) and the composition
   explorer (M47) both emit candidates directly into their respective filter's
   scoring input — a discovered crate goes to `FRONTIER_FILTRATION_STANDARD_V1.md`,
   a discovered internal composition goes to `BUILD_WORTHINESS_STANDARD.md`. This
   is automatic once M41/M47 are implemented: discovery output *is* filter input,
   not a separate handoff step.
2. **Filter → Translate or Recreate.** A `FRONTIER_FILTRATION_STANDARD_V1.md`
   score in the `DIRECT_ADOPT_WITH_WRAPPER`/`CRATIFY_KERNEL_EXTRACTION` range
   routes to Translate (M45's `cratify` pipeline takes an external candidate as
   input). A `BUILD_WORTHINESS_STANDARD.md` `ADMIT_TO_READY`/`ADMIT_WITH_SCOPING`
   outcome for an internally-generated idea routes to Recreate (M44's local-worker
   drafting — there is no existing artifact to translate, it has to be written).
   `BORROW_SPEC_AND_SYNTHESIZE` (Frontier Filtration's middle outcome) is the one
   crossover case: an external candidate too coupled to import directly still
   routes to Recreate, using the candidate's design as reference rather than its
   source.
3. **Translate/Recreate → Review.** Both M45's cratified output and M44's
   pre-proofed draft enter `governance/REVIEW_CONDITIONS.md`'s condition table the
   same way any other change does — the table does not care which stage produced
   the change, only what it touches. This is already stage-agnostic by
   construction; no new trigger logic is needed here, which is worth stating
   explicitly so a future session doesn't build one unnecessarily.
4. **Review → Repeat (the loop-closing step that was missing).** Every Review
   outcome — accepted or rejected — writes back to the stage that originated the
   candidate, not just to a merge/discard log:
   - An **accepted** change updates `governance/MULTITHREAD_PROGRAM.md`'s status
     for its milestone/row to `Done`, per `EVIDENCE_STANDARD.md`'s existing
     evidence requirement for that label.
   - A **rejected** change's reason is archived against its originating
     `exploration_group` (M46) or Frontier Filtration/Build-Worthiness score
     record, not just discarded. This is the actual repeat mechanism: the next
     Discovery pass for a similar candidate can check whether something
     structurally similar was already tried and rejected, and why, before
     re-proposing it blind.
5. **Rubric drift check (the second loop-closing step, periodic rather than
   per-change).** `governance/SCHEDULE.md` already defines a monthly architecture
   review cadence. That review should include: sampling recent
   `REJECT_LOG_REASON`/`ARCHIVE_AS_ROADMAPPED` outcomes from
   `BUILD_WORTHINESS_STANDARD.md` and `REFACTOR_COST_PROHIBITIVE` outcomes from
   `FRONTIER_FILTRATION_STANDARD_V1.md`, and asking whether the rejection reasons
   cluster around something the rubric itself should weight differently — not
   revising the rubric on every rejection (that would make it unstable and
   unpredictable), but on a fixed periodic cadence with accumulated evidence,
   matching this repo's existing evidence-before-status-change discipline.

## Rejection quality and boundary firmness

Discussed 2026-09-20: whether to treat this loop's accumulated outcomes like a
machine learning system. The useful half of that framing is already implicit in
what's built; the risky half is worth naming and explicitly declining.

**Useful: treat the evidence trail like a dataset, not a log to discard.** Every
`REJECT_LOG_REASON`/`ARCHIVE_AS_ROADMAPPED` outcome from
`BUILD_WORTHINESS_STANDARD.md`, every `REFACTOR_COST_PROHIBITIVE` outcome from
`FRONTIER_FILTRATION_STANDARD_V1.md`, every `Rejected`/`NeedsRevision` verdict
from `cross_verifier.rs`, and (once M50 lands) every entry in
`REVERT_LOG.md` is a labeled record: a candidate, a score or verdict, and
increasingly a *reason*. The `Rubric drift check` above already asks someone to
sample these periodically — that only works if the samples are worth reading.

**Rejections are first-class output, not a lesser outcome of the pipeline — same
standard as a Rust compiler error, not a discarded default.** A `rustc` error is
not just "no"; it names the exact rule violated, the exact span, and often what
would make it compile. A rejection anywhere in this loop should meet the same
bar: which specific gate or quadrant failed (not just a composite score), what
evidence was weighed, and — where it's knowable — what would need to be true for
a different outcome. A terse "score 32.1, rejected" is worse than useless for the
rubric-drift review it's supposed to feed; it can't be clustered by cause, so it
can't tell anyone whether the rubric itself needs to change. This is a real
authoring bar on every rejection this loop produces, not just a preference.

**Don't reward the yes over the no.** A pipeline that only takes acceptances
seriously (celebrated, merged, visible) and treats rejections as noise to be
cleared out will drift toward whatever maximizes acceptance rate, not whatever is
actually correct — the review, scoring, and archival machinery in this repo
should give a well-evidenced "no" the same weight and permanence as a "yes": both
get recorded, both get reasons, both are inputs to the next discovery pass and the
periodic drift review, neither is treated as the pipeline's real product with the
other as its byproduct.

**Stand firm on boundaries; don't relax them under pressure, only under new
evidence.** A rejection produced by a fixed rubric (`BUILD_WORTHINESS_STANDARD.md`,
`FRONTIER_FILTRATION_STANDARD_V1.md`, `CHANGE_VERIFICATION_TIERS.md`) should not
soften because a resubmission is rephrased, repeated, or insisted on — only
because the actual evidence changed. This mirrors `governance/
CHANGE_VERIFICATION_TIERS.md`'s existing "ambiguity defaults to the higher tier"
rule and this session's own refusal to relax review requirements under direct
pressure (see `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md`'s decisions log,
2026-09-20, on the rejected self-approval draft flagged as instruction-poisoning-
shaped). A rubric that bends to repetition
instead of evidence isn't a boundary, it's a suggestion — and the whole point of
writing these standards down was to make them hold the same way every time,
regardless of who's asking or how many times.

**Explicitly declined: training something to predict or make the accept/reject
call.** A model trained on this repo's own accept/reject history would reproduce
whatever bias already exists in that history with no way to distinguish "the
rubric is right" from "the model learned the reviewer's blind spot," and it would
replace an actual written reason with a number nobody can meaningfully argue
with — undoing the entire point of the rejection-quality bar above. The dataset
discipline is for periodic human/capable-agent review of drift (already
`SCHEDULE.md`'s monthly cadence), not for training a scorer. If this is
revisited later, it needs its own explicit decision, not a drift into it via
"we already have the data."

## What "closing the loop" does not mean

It does not mean unattended end-to-end automation from Discovery to a merged
change with no review — `governance/CHANGE_VERIFICATION_TIERS.md` and
`governance/REVIEW_CONDITIONS.md` still gate every change at the Review stage
regardless of which upstream stage produced it, and `LOCAL_AGENT_CONTROL_PLANE.md`'s
constraints on the local worker are unaffected by anything in this document. The
loop that closes is *information* — a rejected idea's reason reaching the next
discovery pass, an accepted change's evidence reaching the roadmap's status record
— not authority bypassing review.

## Relationship to other documents

This document adds no new mechanism of its own; every stage and every artifact it
references already exists elsewhere. It exists only to state the connective
tissue between them, since without it the milestones above are independently
useful but not a "running belt" the way the loop was originally described.
