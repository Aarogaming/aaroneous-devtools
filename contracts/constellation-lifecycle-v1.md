# Constellation lifecycle v1 (proposed semantic contract)

Version: 1. Status: specified, NOT IMPLEMENTED or behavior-verified. This describes
a future devtools server contract, not executable JSON/API schemas and not current
repository governance. Unknown versions must be rejected, not guessed. Existing
claim, independent acceptance, spending, deployment and merge rules remain binding.
Architecture: [constellation operations](../architecture/CONSTELLATION_OPERATIONS.md).

## Identity, provenance and projections

Topics have stable IDs, title, optional existing parent, tags, source references,
revision and state. Parent edges are acyclic. Contributions retain their own ID,
topic ID, content, author/identity assurance, evidence anchors and advisory status.
Typed relations retain source/target, vocabulary version, reason, evidence, author
and disposition. Dependency edges are acyclic; dissent/relatedness need not be.
Use the C101 vocabulary only after schema/writer reconciliation; attraction can
be derived from tags/relations without adding an unreviewed relation type.

One authorized writer validates references, bounds, idempotency and expected
revision. Parallel readers propose changes; stale writes are rejected with a
refresh route, never last-writer-wins. Replay rebuilds projections. Layout changes
do not rewrite identity, parentage, evidence or votes. Legacy records are mapped
by explicit adapter events without modifying originals. Source resolution, claim
support, exercised behavior and authorized decisions remain separate fields.

## Roles

- Proposal owner: accountable party refining the suggested change and responding
  to objections; cannot self-accept implementation or dismiss its own blockers.
- Required reviewers: finite declared roster, including independent acceptance;
  implementing seats cannot satisfy that independent role.
- Project authority: human owner or explicitly delegated authority for policy
  changes, permissions and unresolved escalations. Routine recycling is the
  default, not a demand for a human override each time.
- Other contributors: may object and provide evidence; all refusals are heard
  even when the contributor is not in the voting roster.

## Progression and recycling

Discover -> qualify -> experiment proposal -> existing authorization -> retained
result -> refine -> review window -> delivery-eligible -> separately authorized
delivery -> observed outcome. A topic/action card is never an assignment grant.

| Event | Default disposition | Required retained data |
| --- | --- | --- |
| Refusal or unresolved review | recycled-review-needed | disputed claim, impact, response, evidence, disposition and next step |
| Recurring failure | recycled-regression-needed | stable issue ID, prior attempt/fix revision, reproducer, missed acceptance boundary, changed safeguard/test plan |
| No justified delta or budget exhausted | parked | reason, responsible party, review-by UTC, reopening condition and remaining budget |
| New relevant evidence or review-by reached | reassess | deduplicated trigger and bounded reassessment receipt; NOT automatic execution |
| All release conditions satisfied | delivery-eligible | exact candidate revision, complete review record and passing existing gates |

Recycling preserves history, attempts and costs. Require a real delta (new evidence,
changed assumption, narrower scope or different safeguard) before another attempt.
Unknown causes remain unknown. No fresh topic ID to reset rejection/budget history,
no blind retry, and no automatic model calls. A further experiment needs a bounded
authorized budget. Retire explicitly with reason and retained history, never erase.

## Scheduled 24-hour consent and refinement

Freeze candidate revision/scope, evidence, acceptance, reviewer roster and UTC
opening/closing instants. All required reviewers need successful notification
receipts before opening the window. Close is opening + 24 hours; clock acquisition
is explicit, and restart must retain the original instants rather than extend them.
This timer is separate from work claims and operational leases.

Ballots are explicit approve/refuse/abstain, bound to revision and authenticated
reviewer identity through the existing identity route. Silence, missing reviewer
or abstention is not approval. The roster cannot be edited mid-round to remove
dissent. Retain every refusal, acknowledge it, group duplicates without erasing
authors and answer it with evidence or the smallest authorized discriminating check.
Unsupported concerns may request evidence; uncertainty is not invalidity.

An independent reviewer may propose an evidence-backed refutation. The objector
can challenge its disposition; an unresolved challenge still blocks unanimity.
A procedural dismissal cannot turn refusal into approval. Material revisions
invalidate ballots and restart a notified 24-hour round. Clerical non-material
edits require a recorded independent classification, not author discretion.

At scheduled close, delivery is eligible only when every required reviewer has
approved the exact revision, all objections have reviewed dispositions, and
existing acceptance/gates/authority hold. Time alone grants nothing. Late objections
are retained for reassessment; material safety/correctness evidence holds an
unreleased candidate after close. Post-delivery findings enter recurrence review.

Bound refinement initially to two rounds and one budgeted evidence check per
distinct objection; these are proposed starting limits for tuning, not proven
optima. If unresolved, recycle then park or split a smaller reviewable candidate
with its own roster/window. Do not discard objections on timeout or invent an
appeal hierarchy. Project-authority decisions remain separately attributed and
cannot be represented as unanimity or bypass governing gates.

## Parked-item aging

Every parked item has a responsible party, review-by UTC and event-based reopening
condition. Use the existing scheduler/writer, not one loop per topic. A due item
gets one idempotent reassessment event, never an automatic work claim or inference.
Bound each batch by configured count/time budget, retaining a fair oldest-due order
and visible overdue items if capacity is exhausted. New evidence can trigger early
review without duplicate jobs. Unchanged reviews retain the blocker and explicitly
choose a later date; repeated unchanged reviews may lengthen a capped interval or
recommend explicit retirement. No silence-based deletion or unnoticed indefinite
parking. Exact intervals/batch bounds are explicit adoption configuration, not
invented here. Clock jumps/restart cannot lose overdue items or duplicate decisions.

## Useful output and verification obligation

Action records name desired outcome, proposal owner, baseline/NOT MEASURED, measure,
experiment, cost/time budget, acceptance, stop condition and authority route.
Capability candidates name provenance/revision, acquisition permission/license,
inputs/outputs, environment, dependencies, failure/rollback and verification.
Forecasts are not realized benefit; affinity/popularity is not ROI or truth.

Required implementation fixtures (all NOT RUN for this contract): orphan/cyclic
parent, dependency cycle versus preserved dissent, dangling/stale evidence, copied
source lineage, duplicate event, stale concurrent revision, projection replay and
unchanged legacy bytes; silent/abstaining reviewer, refusal at 23:59, notification
failure, revised head, disputed refutation, late safety evidence, exhausted rounds;
recurrence with no delta, budget reset attempt, early/due duplicate wake, overdue
batch fairness, clock jump/restart, retirement preservation; unanimous eligibility
without execution/merge authority. Test authenticated roles, migration/recovery and
Windows/Linux behavior before adoption. Independent review remains required.
