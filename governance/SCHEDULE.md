# Maintenance Schedule

## Daily automated check

Review Aaroneous CI and `governance/WORKLIST.md`. Update only when evidence changes: CI result,
blocked item, completed item, or newly reproduced defect. Do not issue routine status noise.

## Weekly manual review

- Reconcile completed queue claims and expired ownership.
- Run the dependency/advisory review when its lane is active.
- Review the hardening matrix for evidence gaps introduced by recent changes.

## Monthly architecture review

- Reassess the execution-host boundary and public APIs.
- Classify one bounded legacy/interface/terminology area.
- Retire obsolete foundation artifacts and verify no product dependency points at devtools.
## Quarterly documentation & process audit

A full repeat of the 2026-09-26/27 documentation-and-process audit
(`governance/AUDIT_2026-09-26_DOCUMENTATION_AND_PROCESS.md`), not just the
cloud routine's monthly (1st) roadmap/documentation-drift check further
below in this file — that audit found the same self-certification pattern
recurring five times and real doc sprawl/canonical-doc collisions that a
drift-only check wouldn't catch. Without a standing cadence this stays a
one-off, and the queue's own sprawl (37 `governance/` entries as of this
writing — the exact count will drift; "well past what fits in one head" is
the durable point — six of them separately claiming to be "one home for X"
in `OPERATING_MODEL.md`) is exactly the kind of thing cheap to catch early
and expensive to catch late.

- **Cadence:** every three months, independent of and in addition to the
  cloud routine's monthly roadmap/drift check (below) and this file's own
  "Monthly architecture review" section (above) — this is a deliberately
  lower-frequency layer, the same way the native candidate review (hourly),
  local worker (every two minutes, gated to specific minute windows), and
  cloud routine (daily/weekly/monthly) already run at different frequencies
  for different scopes rather than forcing everything onto one clock.
- **Trigger, redundantly:** the cloud primary-lane routine's monthly (1st)
  wake checks whether a quarterly audit is due (three months since the last
  one logged in `DECISIONS.md` or this file; if neither has a logged
  quarterly audit yet, use the original 2026-09-26/27
  `AUDIT_2026-09-26_DOCUMENTATION_AND_PROCESS.md` as the reference date for
  that first computation) and runs it in that wake if so — but this does
  not depend on that routine's continued existence; any active agent
  session doing a monthly review should make the same check as an
  interlock, not a single point of failure.
- **Scope:** re-run the original audit's method — self-certification
  pattern check across recent queue entries, gate/CI coverage drift (does
  `cargo xtask gate` still actually cover every CI workflow, not just
  `check-and-test`?), canonical-doc collisions, and whether
  `OPERATING_MODEL.md`'s "one home for each planning fact" list still
  matches reality or has drifted again.
- **Evidence:** a dated findings doc under `governance/` (follow the
  `AUDIT_2026-09-26_DOCUMENTATION_AND_PROCESS.md` naming convention), a
  remediation-status line logged in `DECISIONS.md`, and — per this
  repo's own standing evidence rule — no claim of a finding being fixed
  without the command/exit-code/log-path that proves it.

## Per-claim lifecycle

At claim time, record priority, dependency, exact paths, and expected evidence. At review time,
verify the required gate and release paths. At weekly review, keep no more than three primary
implementation claims open unless an additional scope has independent files and a named reviewer.

## Native candidate review (installed 2026-09-16; task removed 2026-09-17, see [PERSISTENT_OPERATIONS.md](PERSISTENT_OPERATIONS.md))

Windows task `Aaroneous-Devtools-Multithreader` reviews the declared evidence hourly at :00 through the Rust foundation executable. Suggestions and failures go to an external devtools inbox for owner review. See [MULTITHREADER_SERVICE.md](MULTITHREADER_SERVICE.md) for inputs, limits, verification and operating commands. This service does not check CI or automatically promote/implement suggestions. The retired rolling-worklist heartbeat remains paused. The Codex audit and Rust reuse heartbeat was deleted at the user's request. No active Codex maintenance schedule remains.

## Local support processing

`Aaroneous-Devtools-LocalWorker` polls approved `worker-jobs/` every two minutes, dispatching only in minutes 06–20 and 41–50 and yielding to the shared review reservation. It generates scratch Rust drafts and bounded analysis, suppresses unchanged inputs and pauses at eight pending results. See [LOCAL_WORKER_SERVICE.md](LOCAL_WORKER_SERVICE.md). Windows supports the installed task while logged in; S4U logged-out registration was denied. Only native candidate review remains hourly at :00. Existing worker quiet windows remain unchanged, including the former :30 slot; scheduling consolidation belongs to the native maintenance program.

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
