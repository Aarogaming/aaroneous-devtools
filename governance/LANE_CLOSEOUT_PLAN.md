# Review-lane closeout and successor work

Owner-directed decomposition, 2026-09-17. This plan releases broad source
reservations, not acceptance requirements. The queue remains authoritative.
Old lanes are Blocked with retained commits/gates and explicit handoffs; none
is relabeled Done merely to create capacity.

## Delivered scope and remaining decisions

| Prior lane | Retained delivery | Closeout requirement / successor |
| --- | --- | --- |
| C10 | Bounded support worker; proposals never automatically applied | M34 independent acceptance; C23 correct service claims; M37 improve liveness |
| C11 | Delivery design and dated status/maturity report | M34 independent acceptance; C23 separate historical/current claims |
| C12 | Companion standards and planning navigation | M34 independent acceptance; C23 correct stale adoption/navigation language |
| C13 | Packet/job/review utility and client/runtime lifecycle tests | M34 independent acceptance; C24 server lifecycle; M38 automate client tests |
| C15 | Truthful read-only React dashboard and eight regressions | M34 independent acceptance of that slice; M36 authenticated integration is separate |
| C16 | Read-only Git-triggered GUI CI; existing maintainer task | Independent technical pre-review retained; M34 owner acceptance; C23 daily cadence |
| C19 | Authenticated bounded registrar admission | Independent technical pre-review retained; M34 owner acceptance; M36 deployment is separate |

The latest re-review passed 70 selected Rust tests, five utility-client tests,
eight dashboard tests, dashboard type/build, formatting and strict all-target
Clippy. The preserved unrelated cache-binary test remains excluded locally.
This re-review of Codex work is not independent acceptance.

## Immediate independent tasks

### C23 / M35: truthful current documentation

Exact files: README.md; governance/OPERATING_MODEL.md,
PERSISTENT_OPERATIONS.md, LOCAL_WORKER_SERVICE.md, SCHEDULE.md;
architecture/MAINTENANCE_PROGRAM.md. No Rust/UI/runtime edits.

Subtasks:

1. Replace current-tense uncommitted-GUI/registrar claims with tracked-source
   status and actual integration limits; link dated status and maturity reports.
2. Record the daily 09:00 local maintainer cadence. Preserve dated historical
   schedules; distinguish it from utility polling and default-off C21 persistence.
3. Correct installed-service promises using explicit current evidence. Do not
   infer deployment from source or recreate removed OS tasks.
4. Label historical prototype audit sections and distinguish delivery from
   owner acceptance. Verify links, strict UTF-8/no BOM/LF and diff checks.

Reviewer: an independent seat. Retirement: remove stale current-tense assertions,
retain historical evidence. No new scheduling behavior is authorized.

### C24 / M39: packet utility server lifecycle

Exact initial files: foundation/crates/maintenance_ui/src/main.rs;
new foundation/crates/maintenance_ui/src/http.rs;
foundation/crates/maintenance_ui/tests/http_lifecycle.rs;
contracts/maintenance-utility-v1.md; governance/UTILITY.md.
A claimant may narrow this scope when claiming, but must record any expansion.

Subtasks:

1. Reproduce stalled authenticated `Expect: 100-continue` request behavior in
   an isolated test server, never by wedging the user's running utility.
2. Bound server-side header/body/write acquisition and admission; client
   AbortController deadlines are not server deadlines. Handle unsupported
   Expect/upgrade/framing explicitly and ensure cleanup cannot block the app loop.
3. Ensure client disconnects do not exit the whole utility. Preserve serialized
   durable app mutation, token/Host/Origin checks and controlled idle shutdown.
4. Add socket regression tests proving other status requests, worker completion
   and shutdown remain responsive. Run utility/workspace tests, strict Clippy,
   formatting and lifecycle smoke; update the contract. Do not deploy until gates pass.

Reviewer: an independent seat; independent technical review required before closure.
Retirement: replace unbounded lifecycle paths without changing product authority.

## Follow-on portfolio briefs (not Ready claims)

- M34: one independent reviewer records a verdict per old lane, cited
  revision and gate evidence. Accept a delivered bounded slice only where its
  acceptance criteria pass; explicitly identify blocking corrections. Mark
  that old row Done only after this decision. Do not equate the request to
  organize work with seven acceptance verdicts. Queue metadata only; no source edits.
- M36: same-origin authenticated registrar/dashboard integration. Depends on
  C19/C15 acceptance. Claim GUI and registrar integration files separately;
  explicit protected credential inputs, no wildcard CORS, negative auth/stale/
  offline tests, and no HTTP promotion endpoint. No credential or service setup
  occurs until separately authorized/configured.
- M37: worker liveness and persistent-mode failure policy. Depends on ownership
  reconciliation for local_worker.rs/multithreader.rs. Distinguish quiet/reserved
  from missing/stale; bound retries for permanent configuration/lock failures;
  document crash-marker recovery and retain failure evidence. No new inference loop.
- M38: utility client CI gate. Claim a new .github/workflows/utility-client-ci.yml
  only; run existing five Node tests on Windows/Linux with read-only permission,
  locked tool version, bounded timeout and path filters. No existing GUI gate overlap.

These briefs enter the queue only once scope, dependencies and reviewer capacity
are checked. Future isolated product-patch and capability-broker work stays in
the existing maturity plan, not another simultaneous implementation lane.

## Sustainable execution

C22 is coordination documentation only. C23 and C24 are disjoint Ready
corrections, not active edits. With C22 in Review they fit the default capacity
of three if claimed one at a time; do not expand more work until review catches up.
Daily slow-churn can claim one eligible correction, run its gates and hand off.
Source reservations released by old rows do not authorize overwriting uncommitted
dogfood.rs or scripts/ work. The framework remains independently buildable/runnable.
