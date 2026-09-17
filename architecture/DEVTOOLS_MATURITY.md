# Devtools maturity, development and enhancement

Planning baseline: [dated status](../governance/DEVTOOLS_STATUS.md), 2026-09-17.
This document refines the existing [application delivery map](MAINTENANCE_PROGRAM.md);
it does not add Ready tasks or replace the
[multithread portfolio](../governance/MULTITHREAD_PROGRAM.md) and
[ownership queue](../governance/COORDINATION_QUEUE.md).

## Target

A practical Rust-owned development companion that makes building Aaroneous
systematic: select explicit evidence, find an available capability, author a
bounded job, review its output, verify an isolated change and separately
authorize integration. Models assist; deterministic software owns policy,
bookkeeping and verification.

Devtools may launch/host Aaroneous and develop new Aaroneous components. A
component delivered to the product must meet the product constitution and native
gates without importing Devtools. No new always-on writer, paid provider or
credential is authorized by this plan.

## Ordered maturity gates

| Order | Increment and source of need | Acceptance before moving on |
| --- | --- | --- |
| 1 - acceptance | Review existing C10-C13/C15/C16/C19 work; reconcile stale documents, schedules and portfolio labels | Independent decisions with revision/gate evidence; preserved unfinished work handed off; one truthful current deployment inventory |
| 2 - connected application | Registrar/dashboard integration gap in REGISTRAR.md | Explicit protected credential inputs; reviewed same-origin hosting; no CORS bypass; real status/tasks/logs; auth expiry, offline, malformed and stale cases tested |
| 3 - durable local operations | Current foreground runtime and incomplete early-return liveness | Observable idle/quiet/reserved/running/failed states; restart recovery, lock ownership, cancellation and bounded retry tests; documented install/pause/uninstall and logged-out limitations |
| 4 - reviewed development loop | CLI promotion lacks isolated full-gate workflow | Declared exact scope; clean isolated branch/worktree; no shared-index capture; retained proposal/diff; complete Aaroneous gate; separate review and publication authority; rollback cannot erase another contributor's work |
| 5 - component-capability dogfood | Current utility has only a fixed research profile | Versioned capability descriptors and broker eligibility tests; one useful Aaroneous component developed with product-native evidence; independent product build/run without Devtools demonstrated |
| 6 - enhancement | Provider/experiment/performance aspirations in the delivery map and portfolio | Benefit measured against baseline; data/budget limits and fake-provider ambiguous-outcome tests before paid calls; isolated experiments promoted only with support/retirement criteria |

These are proposed increments, not six simultaneous implementation lanes. Claim
one narrowly useful slice only after checking dependency and reviewer capacity.

## Development priorities

### Near-term reliability and usability

- Make each maintained UI report which service and revision it observed. Never
  turn a quiet tick or generated result into a green product-verification badge.
- Add the utility's browser-client regressions to a claimed automated CI gate;
  its Node tests currently run separately from Foundation Rust CI.
- Reconcile old planning language through its owners rather than duplicate it.
  A small documentation correction can be valuable morning work.
- Finish onboarding with explicit repository, binary, storage and endpoint
  selections, actionable readiness errors and a clear offline mode.
- Add reviewed lifecycle integration tests covering an occupied port, crash
  recovery, pending-operation shutdown and authenticated client reconnection.

### Maintenance-core maturity

- Centralize observable scheduling policy in reusable Rust, with thin platform
  adapters. Distinguish the daily contributor task from the native support
  worker and from utility polling; avoid duplicate inference loops.
- C21's opt-in `multithreader-serve` is an implemented starting point, not a new
  task to repeat. Evaluate its deployment, stop/restart and liveness evidence
  before enabling it; the removed OS candidate task must not be recreated here.
- Preserve idempotency/disposition history while introducing bounded retention
  or compaction. Provide a preview and explicit approval for destructive cleanup.
- Use versioned contracts and regression fixtures across the packet utility,
  registrar and GUI. Share policy deliberately, not by adding another scheduler
  or copying route semantics into browser code.
- Treat local outputs as proposals requiring source checks. Rejected research
  already demonstrated why confident model assertions are not sufficient evidence.

### Capability integration and enhancement

Begin with a descriptor for one proven capability: owner, version, explicit
inputs/outputs, resource limits, side effects, required approval, prerequisites
and verification evidence. The broker selects only capabilities satisfying the
declared authority and resource constraints, and returns a useful unavailable
reason otherwise. It does not infer permission from model-generated instructions.

Dogfood one bounded workflow with existing Aaroneous interfaces before building a
general self-development layer. If a needed capability is absent, author a
separately reviewed product component with focused tests and native gates;
Devtools-specific orchestration stays here. Track component adoption and
retirement without silently modifying product release/runtime dependencies.

Later enhancements can include measured performance baselines (M13/M27),
explicit stack evaluations (M15), broader reviewed research/job profiles,
accessible task-history presentation and opt-in budgeted provider routing.
Do not add scaffolding or providers merely to fill out a feature list.

## Slow-churn background policy

The existing Codex task runs once each morning at 09:00 local time. It advances
at most one small eligible portfolio item per run, or collects a useful narrow
review-evidence bundle when implementation capacity is unavailable. It does not
spawn extra primary agents, change the cadence, or bypass ownership to look busy.

1. Inspect rules, Git/dirty state, queue capacity, recent CI and pending results.
2. Reconcile portfolio status against merged evidence and the queue. Prioritize
   review debt and reproduced failures before expanding features.
3. Select an unowned Ready item (or record a bounded eligible Ready item), claim
   exact paths, then implement and run the applicable gates.
4. Retain compact revision/evidence and a handoff; move completed implementation
   to Review, not self-certified Done. Product work uses isolated product scope.
5. Notify only meaningful completion, failure or required user action. Remain
   quiet on unchanged/non-actionable state and defer unavailable tools/limits
   rather than retrying indefinitely.

Local scheduled work depends on the computer and app being available, as described
in the [official scheduled-task documentation](https://learn.chatgpt.com/docs/automations?surface=app).
This is paced background contribution, not a guarantee of continuous execution.

## Release readiness and progress measures

A useful maintained release needs clean-checkout Windows/Linux verification,
reviewed authority boundaries, documented support requirements, secure explicit
setup, restart/failure tests, and installation/pause/removal evidence. End-to-end
claims additionally need the isolated full-gate product workflow above.

Measure reviewed items closed, oldest review age, successful deterministic runs,
repeated/ambiguous failures, restart recovery, retained evidence and actual
resource/cost usage. Do not report tokens generated, queue size or model confidence
as maturity. A quiet, correct, inspectable system is progress.
