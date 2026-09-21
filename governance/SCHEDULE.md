# Maintenance Schedule

## Daily maintainer cadence (09:00 local)

Review Aaroneous CI and `governance/WORKLIST.md` at 09:00 local maintainer cadence. Update only when evidence changes: CI result,
blocked item, completed item, or newly reproduced defect. Do not issue routine status noise. This maintainer cadence is distinct from passive packet utility polling and default-off C21 in-process persistence (`foundation multithreader-serve`).

## Weekly manual review

- Reconcile completed queue claims and expired ownership.
- Run the dependency/advisory review when its lane is active.
- Review the hardening matrix for evidence gaps introduced by recent changes.

## Monthly architecture review

- Reassess the execution-host boundary and public APIs.
- Classify one bounded legacy/interface/terminology area.
- Retire obsolete foundation artifacts and verify no product dependency points at devtools.
## Per-claim lifecycle

At claim time, record priority, dependency, exact paths, and expected evidence. At review time,
verify the required gate and release paths. At weekly review, keep no more than three primary
implementation claims open unless an additional scope has independent files and a named reviewer.

## Native candidate review (installed 2026-09-16; updated 2026-09-17)

The historical Windows task `Aaroneous-Devtools-Multithreader` reviewed declared evidence hourly at :00 through the Rust foundation executable. On 2026-09-17, this OS task was deliberately removed because the repository was not ready for unattended all-day execution. It was replaced by `foundation multithreader-serve` (an in-process persistent review loop that defaults to off). Suggestions and failures go to an external devtools inbox for owner review. See [MULTITHREADER_SERVICE.md](MULTITHREADER_SERVICE.md) for inputs, limits, verification and operating commands. This service does not check CI or automatically promote/implement suggestions. The retired rolling-worklist heartbeat remains paused. The Codex audit and Rust reuse heartbeat was deleted at the user's request.

## Local support processing

`Aaroneous-Devtools-LocalWorker` continuously polls approved `worker-jobs/` and yields to the shared review reservation. It operates as a continuous background persistence layer with no time-based dispatch windows. It generates scratch Rust drafts and bounded analysis, suppresses unchanged inputs and pauses at eight pending results. See [LOCAL_WORKER_SERVICE.md](LOCAL_WORKER_SERVICE.md). Windows supports the installed task while logged in; S4U logged-out registration was denied.

## Cloud primary-lane routine (installed 2026-09-17)

A Claude Code cloud Routine wakes a persistent session daily at 12:00 UTC and, on Mondays and the
1st of the month, runs additional weekly/monthly scope in the same wake. Unlike the native
candidate reviewer and local worker above (suggestion-only, no CI check, no promotion), this
routine can actually claim a `COORDINATION_QUEUE.md` lane, implement it, run Aaroneous's full
native gate, and push/open a pull request — it is the primary-lane capacity this schedule assumed
before a maintained local or GUI equivalent existed (see `README.md`'s "What works today").

- **Daily:** review Aaroneous CI and the coordination queue; if capacity allows (fewer than three
  primary lanes claimed system-wide) and a Ready/P0/P1 item has clear scope, claim exactly one lane,
  make one small mergeable increment, and update the queue. Otherwise do nothing and report nothing
  — an idle day is a valid outcome, per this schedule's own no-status-noise rule.
- **Weekly (Monday):** additionally reconcile expired (>24h) or stale queue claims, and work through
  the oldest unattended Dependabot pull request on `Aarogaming/aaroneous`.
- **Monthly (1st):** additionally check for roadmap/documentation drift against actual code and CI
  evidence (the `M28`/`M30`–`M33` reconciliation lane) and flag anything in devtools that
  `FOUNDATION_BOUNDARY.md` says should be retired.

Owned by the user's Claude Code session; see that session for run history. Capacity, claim, and gate
rules are unchanged — this routine is one more thread under `MULTITHREAD_PROGRAM.md`'s existing
limits, not an exception to them.
