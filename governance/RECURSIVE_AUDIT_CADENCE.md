# Recursive Audit Cadence

Owner-directed, 2026-10-07. Replaces the consecutive-clean-merge
trust-building counter that an earlier draft of
[`AUTOMATED_MERGE_POLICY.md`](AUTOMATED_MERGE_POLICY.md) proposed with a
qualitative, standing practice instead: the same five audit types this
repository already runs ad hoc, applied recursively at every scale the
system has, on a schedule that does not depend on merge volume. Quality
isn't a number to cross; it's upkeep that never stops.

## Scope

This cadence governs `Aarogaming/aaroneous-devtools` end to end,
autonomously, under `AUTOMATED_MERGE_POLICY.md`'s Tier B. It also covers
`Aarogaming/aaroneous` for audit *purposes* — reading it, reporting drift,
proposing a fix — but auditing Aaroneous never implies authority to merge
there. Aaroneous stays entirely owner-gated until something graduates into
it via the component-graduation process in
`LOCAL_MODEL_AUTONOMY_POLICY.md`; that graduation merge is the owner's
checkpoint, not this cadence's. An audit finding something wrong in
Aaroneous produces a report and, if warranted, a PR proposing a fix —
never a direct push, never an autonomous merge, regardless of how clean
the finding is.

## The five audit types, at every scale

1. **State review** — does the current state match what's recorded?
   (queue rows vs. real PR/branch state, a crate's declared profile vs.
   its actual dependencies, a doc's "status: live" vs. what's actually
   installed)
2. **Roadmap/plan check** — is the stated direction still the direction,
   and has anything shipped that the roadmap doesn't know about yet (or
   vice versa)?
3. **Documentation audit** — canonical-doc collisions, stale
   cross-references, drift between what a doc claims and what the code
   does (the method `AUDIT_2026-09-26_DOCUMENTATION_AND_PROCESS.md` already
   used once)
4. **Code audit** — correctness, reuse, simplification: what `code-review`
   and `simplify` already do per PR, re-run at a wider lens
5. **Compliance audit** — does this scope still meet the floor rules
   (`AGENTS.md`, `CRATIFY_SPEC.md`, the evidence-retention rule, the
   identity/independence rules), not just "did it meet them the day it
   merged"

Every scope below runs some subset of these five, widening as the scope
widens.

## Scopes and cadence

| Scope | Audit types | Channel | Trigger / frequency | Rolls up to | Runs via |
|---|---|---|---|---|---|
| Script / function | Code, Compliance | Variable (event) | Every PR that touches it | Crate | Existing PR review + `check_coordination_evidence.py` |
| Crate / module | Code, Compliance, State | Variable (event + floor) | 10 merged PRs touching it, or 14 days, whichever comes first | Program | New steward `audit_card` (see below) |
| Program / binary | Code, State, Roadmap | Fixed | Weekly | Repository | Existing weekly manual-review slot, steward-issued |
| Repository | State, Roadmap, Documentation, Code | Fixed | Monthly | Project | Existing monthly architecture-review slot, cloud Routine |
| Project (devtools + Aaroneous together) | All five, plus graduation-readiness | Fixed | Quarterly | Platform | Cloud Routine, dedicated quarterly firing |
| Platform (schedulers, triggers, agent infra, this cadence itself) | All five, plus a meta-audit of the audit system | Fixed | Semiannual | — (top) | Cloud Routine, dedicated semiannual firing |

**Escalation override**, independent of the table above: a circuit-breaker
trip, a revert, or the same defect class found twice forces an immediate
audit at that scope and the one directly above it, regardless of where its
clock or counter stands. The schedule is a floor, not a ceiling — it
guarantees a review happens even in a quiet period; it never delays one
that's already warranted.

## Fixed vs. variable channels

- **Fixed channels** are clock-driven and run whether or not there's
  "enough" change — this is what stops a quiet scope from silently
  decaying. They are the existing cadence tiers in
  `SCHEDULE.md`/`SCHEDULER_MAINTAINER.md` (daily/weekly/monthly/quarterly),
  extended upward with the new semiannual platform tier and tied
  explicitly to the audit *types* above, not just "review CI and claim a
  lane."
- **Variable channels** are event-driven — a count of merged PRs, a count
  of new scripts, a repeated finding — and exist at the smaller,
  faster-changing scopes where a fixed clock would either fire on nothing
  or miss a burst. They route through the steward's existing tick
  mechanism, not a separate cron.

## Steward/Guild implementation

This is design, not yet built — tracked as its own lane, same convention
as every other proposal in this repository:

- **Variable channel → `steward_run.py`.** A new event type, `audit_due`,
  computed each tick from two counters already derivable from the
  persistent log: merged-PR count per path-scope since that scope's last
  audit, and elapsed wall time since then (the log already carries ticks
  and timestamps; this reads it, it does not restructure it). When a scope
  crosses its threshold, the steward issues a new card kind, `audit_card`,
  through the existing per-seat outbox — the same delivery mechanism `#56`
  already built, a new card kind rather than a new channel.
- **Fixed channel → the cloud Routine.** The existing daily/weekly/monthly
  primary-lane routine (`#63`'s lens rotation) gains two more firing
  tiers — quarterly and semiannual — and its lens selection at each tier
  maps to this table's audit types for that tier's scope, instead of (or
  alongside) today's generic checklist.
- **Guild (Rust port) parity.** `#57`'s golden vectors already pin the
  dispatcher and log-chain contract a Rust port must reproduce byte for
  byte. Before this cadence logic ships, it needs the same treatment:
  golden vectors for the counter math (when does a scope cross its
  threshold) and the escalation override, so a future Guild implementation
  reproduces audit scheduling exactly, not just approximately. No cadence
  logic ships without its golden vectors, same rule as everything else in
  that family.
- **Reporting, not a new gate.** An audit finding something wrong produces
  a PR (fix, doc update, or a proposal if it's a design question) through
  the exact same Tier A/B path everything else uses — this cadence is a
  *source* of work for the existing pipeline, not a new approval layer on
  top of it.

## Relationship to `AUTOMATED_MERGE_POLICY.md`

That document's consecutive-clean-merge counter is removed by owner
decision, 2026-10-07: Tier B auto-merge is live immediately for devtools,
no per-category waiting period required. This cadence is the replacement
trust mechanism — not a gate in front of merging, but a standing practice
that runs alongside it and catches drift after the fact, at every scale,
forever. The difference in kind: the old mechanism asked "has this
category proven itself enough times to be trusted" before acting; this one
assumes trust and keeps checking anyway, on a schedule that doesn't depend
on volume.

## Open items

- The crate-level threshold (10 PRs / 14 days) is a starting guess, not
  measured — expect to retune after a few cycles report "too noisy" or
  "too quiet."
- Who actually writes the `audit_due` counter and `audit_card` kind in
  `steward_run.py`, and on what timeline, is a queue item, not committed
  here.
- The quarterly project-level audit is where graduation-readiness gets
  assessed (is a devtools capability solid enough to refactor into
  Aaroneous-compliant crates) — the exact graduation criteria beyond "the
  audit found nothing blocking" still needs its own writeup.
- Platform-level audits (the cadence auditing itself) start simple: did
  every lower tier's audit actually fire on schedule, and did its findings
  get acted on. Deeper self-audit (is the schedule itself still the right
  shape) is deferred until there is a few cycles of history to look at.
