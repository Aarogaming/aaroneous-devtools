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
