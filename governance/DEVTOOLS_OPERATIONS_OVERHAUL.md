# Devtools operations overhaul

## Objective and evidence

Measure useful Aaroneous outcomes: reproduced defects, reusable components,
passing regression tests, and independently reviewed integrations. Model output
volume is an input cost, not a completion measure.

On 2026-09-23, the retained input in each latest job result was inspected rather
than joining historical results to mutable current manifests. The 731 job IDs
contain 682 analysis results (28 with code), 46 code-draft results (11 with code,
35 without), and three test fixtures with code. These are output-presence counts,
not proof of correctness. The five failed records are included. Earlier claims
that all 693 unreviewed records are usable work, or that reviewer capacity alone
explains throughput, were not supported. Some records require diagnosis before
they are suitable for review. The 108 files in results also include bookkeeping
artifacts; file count must not be called pending work count.

## Responsibility and Aaroneous reuse

Devtools owns the maintenance workflow and may consume Aaroneous capabilities.
Aaroneous never depends on this workflow to build or run. The current Foundation
manifest already imports Aaroneous's ast_auditor, governance, and
llm_gateway_types; priority_scheduler is a local Foundation crate. Importing a
crate does not establish that its desired behavior is exercised correctly.

| Responsibility | Use | Required evidence |
| --- | --- | --- |
| Aaroneous component contracts | Select an existing capability before proposing new code | Real caller, inputs, outputs, state owner, applicable constraints |
| Aaroneous auditor and native gates | Check target-specific invariants in isolated work | Recorded revision, command, exit status, diagnostics, changed files |
| Aaroneous model types | Describe the explicit local provider and request limits | Configuration validated against the actual adapter |
| Devtools scheduling | Admit useful bounded jobs, reserve resources, retain history | Capacity and dependency decisions reproducible without a model |
| Local agent | Analyze evidence, draft tests, or propose a bounded implementation | Requested artifact present, relevant to the acceptance criterion |
| Independent reviewer | Assess claims, correctness, and adoption | Verdict tied to the exact artifact and evidence revision |

Do not substitute a governance import for a tested policy, or a scheduler import
for demonstrated admission/backpressure behavior. Do not apply reducer-only
restrictions to build tooling, bootstrap code, or ordinary Devtools services.

## Outcome-driven workflow

1. Choose one owner-approved outcome and a consumer for the result. Discovery
   can supply candidates but cannot claim implementation work.
2. Inspect current Aaroneous contracts and tests. Package explicit excerpts,
   source revision/digests, baseline failure, target files, and acceptance check.
3. Let Devtools validate scope, ownership, dependencies, evidence budget, and
   output capacity. Do deterministic extraction/checking before a model call.
4. Ask the local agent for one artifact: an analysis with cited claims, a test
   fixture, or a code draft. Record inadequate output as inadequate output.
5. Validate artifact presence and structure. For code, apply to an isolated
   checkout and run the declared checks with explicit limits and configuration.
6. Review the artifact and the actual check result. A plan for a test does not
   count as executing that test. A report path does not imply a passing report.
7. Integrate through a separately authorized product change and native gates.
   Record the adopted revision and consumer. Preserve failed experiments and
   decisions so unchanged work is not repeatedly rediscovered.

## State and throughput contract

A task and an attempt have separate lifetimes. A failed attempt does not close,
delete, or silently supersede the task. Preserve its input, output, diagnostics,
and relationship to previous attempts. Task closure requires accepted delivery
against its criteria or an explicit retirement decision with a reason. Capacity
limits defer dispatch; they never discard important work.

Recovery must distinguish transport/service failure, truncated generation,
invalid output shape, missing evidence, and failure against target standards.
Transport recovery should retry the same request after bounded backoff; missing
evidence should request a narrower or more complete bundle; truncation should
adjust the bounded output budget or split the task; standards failures should
carry concrete diagnostics into a correction attempt. Exhausting an automatic
retry budget leaves the task awaiting intervention with all evidence preserved.
It must not count as a completed task or trigger unlimited identical calls.

Task contracts must allow variety: analysis, Rust fixture, single-file draft,
and eventually multi-file patch or declarative artifact each need their own
validator. The current worker's code-draft/test-fixture contract requires Rust
in rust_draft; that restriction must not become a universal rule for future
artifact types. Tune prompts, evidence selection, and budgets against a small
representative evaluation set, recording delivery rate and standards compliance
separately. A model claim of success cannot change either measured outcome.

Keep separate facts: requested artifact, generated artifact, validation result,
verification result, review verdict, and integrated revision. Historical records
without a fact have unknown status for that fact. Never infer completion from
`pending-owner-review`, archive location, a nonempty field, or an accept verdict
on analysis. Review verdicts should ultimately identify a result digest rather
than job ID alone, because a job may have multiple attempts.

Track unique goals and evidence versions separately from attempts. Report:
artifact delivery rate by mode, verification pass rate, repeated unchanged
attempts, actionable findings, adopted changes, elapsed generation/check/review
time, and reviewer effort per adopted change. Group superseded passes for display
without deleting their evidence or treating them as accepted.

## Implementation sequence and gates

| Increment | Deliverable and acceptance | Status |
| --- | --- | --- |
| C55 output integrity | Code-draft and test-fixture require nonblank rust_draft through initial and correction passes; missing artifacts retained as failed; outbox exposes code presence | Implemented locally; tests and independent review tracked in queue |
| Historical reconciliation | Mode from retained job input; counts distinguish bookkeeping, attempts, goals, inadequate output, and unknown verification; no automatic acceptance or deletion | Read-only counts established; maintained report still to build |
| Per-job verification | Explicit target checkout revision, patch placement, command, limits, and profile; passing/failing/not-run/error states remain separate | Design pending; current optional runtime-wide pre-proof settings are insufficient for heterogeneous jobs |
| Admission and recurrence | One unresolved attempt per goal/evidence version; bounded retries and reviewer capacity; changed evidence or actionable verdict justifies regeneration | Pending implementation and replay tests |
| Review and integration | Digest-bound verdicts, evidence diff, verification diagnostics, minimal patch and resulting revision | Pending contract and consumer updates |

First pilot: one confirmed Aaroneous defect selected after revalidation against
its current checkout. Capture a failing regression, obtain a local proposal,
verify it in isolation, independently review it, and record the integrated change
only if authorized. Then repeat for one test-fixture task and one analysis task.
Compare actual outcome and time at each stage before expanding discovery or
increasing concurrent model calls.

This plan does not require clearing hundreds of records individually before a
controlled useful pilot. Preserve the backlog, classify it, and admit new work
only into a bounded experiment with an explicit result consumer. Continuous
deployment remains separate from implementing these workflow improvements.

## Shared context packages

Store shared context as versioned, task-scoped evidence packages, not an
undifferentiated conversation history. A packet should identify its goal,
acceptance criteria, selected package version, applicable policy profile,
output contract, and verification recipe. Package entries need repository
identity, source revision/content digest, path/range, and provenance. Prior
model claims and reviewer decisions remain separately labelled from source facts.

Devtools resolves and validates the package before dispatch; the model may
request missing context but cannot silently substitute a different source or
treat another model's assertion as authoritative. Missing/stale evidence keeps
the task open for repackaging. Shared storage does not imply shared authority.

The C56 journal now retains exact prompt/input snapshots per dispatch and both
responses through correction. That is traceability, not yet a reusable context
registry: revision/digest validation, package selection, and structured requests
for missing context remain to implement. Compare focused excerpts against
whole-file evidence on existing tasks before adopting retrieval infrastructure.
