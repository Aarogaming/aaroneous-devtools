# Constellation operations: devtools proving ground

Status: owner-directed target architecture, documentation only. No deployment,
source implementation, schema adoption, scheduling change or new authority.
Source: owner discussion on 2026-10-05 EDT, culminating in the request to publish
this direction as a PR. [Boundary](../FOUNDATION_BOUNDARY.md),
[operating model](../governance/OPERATING_MODEL.md) and repository rules remain binding.

## Direction

Devtools is the gatekeeper and scratchpad of Aaroneous: experiment, collect and
qualify evidence, coordinate work, and operate approved capabilities. The devtools
server consumes explicitly selected, version-pinned stable Aaroneous interfaces.
Aaroneous remains buildable, testable, releasable and runnable without devtools.
Do not guess the latest stable version or silently upgrade an installed system.
Independent devtools tests may use its adapters; product CI/runtime may not require them.

Brainstorm, guild/steward and maintenance are modular devtools responsibilities,
not permission to combine their processes, state writers or execution authority.
Guild/steward remain here until stable responsibilities can be reduced into narrow
Aaroneous components. Then devtools uses those components as a thinner operator/
transformer layer. Extraction follows stability evidence, not premature abstraction.

| Responsibility | Boundary |
| --- | --- |
| Brainstorm | topic case files, evidence, dissent, experiments and refinement |
| Guild/steward | current ownership, assignment proposals, review and delivery coordination |
| Devtools operations | explicitly authorized checks, maintenance and bounded execution adapters |
| Stable Aaroneous | independently usable product components behind native contracts/gates |
| Presentation | expandable topic bubbles and explainable connections; no executable policy |

Use one existing canonical writer/event route for each responsibility. A graph,
search index or visual layout is a rebuildable projection, not a competing ledger.
The server does not inherit every component's permissions or create a second
dispatcher. Authenticate actions separately from self-reported discussion labels.

## Topic interaction and useful intelligence

The playbook is one topic, not the interaction system. Stable topic identities
expand into subtopics; relation edges connect neighboring ideas without merging
their context. Tags and evidence explain attraction. Visual proximity does not
prove correctness, consensus or execution eligibility. Preserve contradictory
branches, source revisions, author attribution and distinct evidence lineages.

Scavenge authorized data, intelligence and capability candidates, then qualify
them against actual needs. A capability has explicit inputs/outputs, dependencies,
permissions, environment, failure/rollback behavior and retained verification.
Do not confuse raw collection, model interpretation and tested reusable behavior.
Reuse existing capabilities before building new infrastructure.

Each actionable idea needs an accountable proposal owner, desired outcome,
baseline (or NOT MEASURED), success measure, smallest experiment, bounded cost/time,
acceptance, stop condition and existing authority route. Compare alternatives and
refine outputs before delivery. Refused/unresolved issues recycle; recurring
failures require a regression case and a changed safeguard. Park when evidence
or budget is insufficient, with a review-by date and explicit reopening condition.
The normative proposed transitions live only in [lifecycle v1](../contracts/constellation-lifecycle-v1.md).

ROI means independently reviewed useful outcomes per total effort: qualified
discoveries, verified capabilities reused, integration time, regressions reduced,
and observed benefit versus acquisition/maintenance/model cost. Distinguish
forecasts from realized value. Node count, comments and tokens are not outcomes.

## Reconcile existing work, do not fork it

- C101 [PR #59](https://github.com/Aarogaming/aaroneous-devtools/pull/59), reviewed
  as a source at `340ed3585999faabe0e17831bc07f6ac395a5923`, proposes neutral notation,
  revision-bound references, relations and rebuildable graph/retrieval projections.
  Do not introduce a second relation vocabulary. Map topic links into its accepted
  vocabulary or propose a reviewed version extension. Resolver success is not claim support.
- C102 [PR #60](https://github.com/Aarogaming/aaroneous-devtools/pull/60), inspected
  at `17d223dc24444c8129e8872c826b638528c99632`, is Proposed/Unowned server intake,
  not a Ready implementation grant. Reconcile schema and writer ownership there.
- C104 [PR #66](https://github.com/Aarogaming/aaroneous-devtools/pull/66) proposes
  leases, not in-force rules. The 24-hour delivery-review window below is NOT a
  claim-expiry timer and does not adopt that proposal.

These are pinned review inputs, not perpetual current-head assertions. Recheck
claims, heads and dispositions before implementation. Current runtime discussion
transport and isolated synthetic models are experiments, not the supported
cross-platform server. Their successful checks do not establish this architecture.

## Adoption and stability sequence

1. Merge reviewed documentation/pointers only; keep implementation status explicit.
2. Reconcile existing C101/C102 contract/writer decisions and exact Ready scope.
3. First maintained slice: topic ID/parent, contribution topic reference, typed
   relation events and read-only projection; legacy entries retain IDs/bytes/hashes.
4. Add expandable views, then refinement/objection/ballot state and bounded aging
   through existing authorization and scheduling, with language-neutral negative fixtures.
5. Tune on fixed-bundle flat-versus-topic tasks: factual coverage, time to reviewed
   decision and total cost. Baseline and target must be set before claiming benefit.
6. Stabilize crash/restart, concurrent stale writes, outages, review continuity,
   storage bounds, migration and rollback on Windows/Linux. Retain independent evidence.
7. Only then propose product extraction of a narrow reusable responsibility with
   Aaroneous-owned contracts/native gates and no devtools dependency.

No graduation date or numeric performance target is invented here. The scoped
implementation owner/reviewer defines a bounded tuning window and thresholds.
Rollback disables the new projection/client or adapter without deleting retained
events or restoring old history over newer data. Installed schedules, product
entrypoints and runtime settings require separate exact-path adoption work.
