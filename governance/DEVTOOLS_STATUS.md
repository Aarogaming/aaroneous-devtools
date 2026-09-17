# Devtools status

Evidence snapshot: 2026-09-17. Integrated source baseline: `5c2eb53`.
Persistence follow-up: `7b1dcfe` / C21, integrated during this documentation pass.
This is a dated status report, not a release certification. The
[coordination queue](COORDINATION_QUEUE.md) owns live claims and sign-off;
the [maturity plan](../architecture/DEVTOOLS_MATURITY.md) defines proposed next increments.

## Bottom line

Devtools is a usable developer-support foundation with a running local packet
utility. It is not yet a fully installed, restart-safe, end-to-end autonomous
development application. Generating research, accepting it for follow-up,
verifying a framework change and publishing that change remain different actions.

Rust owns durable maintenance policy; TypeScript is presentation. Devtools can
host or invoke Aaroneous through explicit boundaries, but Aaroneous must never
depend on Devtools for build, CI, release, crates or runtime.

## Implemented versus operational

| Area | Implemented evidence | Current limit |
| --- | --- | --- |
| Rust Foundation | Standalone workspace: stack/input validation, component inventory, dependency reports, dogfood evidence, intake and schedule review | Individual tools are not a unified onboarding or development workflow |
| Local support worker | Explicit-input proposals, shared reservation, quiet windows, backpressure and unchanged-input suppression; C10 evidence | Generated drafts remain pending owner review; early quiet/reserved returns do not always refresh liveness evidence |
| Candidate intake | Bounded proposals and semantic validation; C9 independently reviewed and Done | Portfolio/queue reconciliation and review capacity still govern primary work; candidates cannot claim work |
| Opt-in candidate persistence | C21 adds `foundation multithreader-serve` with bounded interval and failure-isolated cycles; no installation, default off | Source integrated; not enabled or deployed by this documentation pass; process presence is the toggle, not boot recovery |
| Packet utility | Readiness, bounded wire research, blueprint/refinement, gated publication, result inspection/disposition; C13 and `eb90623` | One shipped research profile, not arbitrary framework component authoring; independent owner acceptance pending |
| Utility lifecycle | Six-second/2 MiB client bounds, stale disabled actions, exclusive runtime marker, controlled shutdown, Plan restart and retained dispositions | Crash markers need explicit inspection; no boot/login recovery or installed service guarantee |
| React dashboard | Tracked `gui/`, real read-only payloads, validation, stale/error state, serialized refresh and local styling; C15, `0718216` | No reviewed registrar credential setup or same-origin hosting; implemented client is not a connected deployed dashboard |
| Registrar | REST/MCP admission uses explicit token file, exact Host/Origin, bounded framing/deadlines/concurrency; C19, `456c144`, review fix `fecdacb` | Not deployed here; modern MCP transport and client integration are not certified |
| Reviewed patch execution | Registrar dispatch retains proposals without product writes; separate CLI promotion exists | Promotion is not an unattended-safe isolated pipeline and does not establish full Aaroneous gate compliance |
| Recurring contributor work | Existing Codex maintenance task changed to daily 09:00 local time, with at most one small eligible task per run | Requires local execution availability; not a continuously running agent, OS installer or product scheduler |

The packet utility and registrar are separate control planes. A healthy utility
at loopback port 8787 does not prove the registrar is online or the React GUI is
authenticated. Worker ticks that return zero may mean quiet/deferred/idle, not
successful model work or verified code.

## Verification evidence

- Foundation CI [35241817801](https://github.com/Aarogaming/aaroneous-devtools/actions/runs/35241817801)
  passed on the integrated `5c2eb53` baseline. The utility implementation also
  passed Windows/Linux CI [35215134493](https://github.com/Aarogaming/aaroneous-devtools/actions/runs/35215134493).
- The latest local utility pass recorded 68 Rust tests passing with only the
  preserved unrelated `test_find_prebuilt_binary_locates_cache_binary` excluded;
  formatting and strict all-target Clippy passed. This exclusion is a local
  working-tree caveat, not a claim that the uncommitted change is fixed.
- Utility browser-client tests: five passing. Live smoke confirmed duplicate
  rejection without configuration replacement, shutdown lock release, Plan
  restart with review history, and offline actions disabled.
- Dashboard evidence: eight regression tests plus TypeScript/production build;
  Dashboard CI recorded in C15. These are separate from the Rust CI gate.
- Recent scheduled observations: utility reachable, authoring Plan, no active
  operation, no reservation, one utility result awaiting owner review. Last
  worker tick returned zero during its quiet window; stale worker observation
  remains explicitly reported.

Runtime artifacts and tokens stay outside tracked source. This report does not
embed credentials or certify every host/service from one observation.

## Review and operational debt

Owner-directed closeout update: the seven old review lanes now have explicit
Blocked acceptance/correction handoffs and release their broad source scopes.
This is not independent acceptance or Done. The
[closeout plan](LANE_CLOSEOUT_PLAN.md) defines C23 documentation correction,
C24 server lifecycle hardening and M34's per-lane acceptance decisions.
The following historical evidence remains applicable; queue statuses are current.

1. Owner sign-off remains for C10-C13, C15, C16 and C19. Technical pre-review of
   C16/C19 is useful evidence, not a substitute for the reserved owner decision.
2. Preserve unrelated `foundation_cli/src/dogfood.rs` changes and untracked
   `scripts/` until a named owner hands them over and verifies them.
3. Older README, operating-model and application-design passages describe the
   original uncommitted GUI/registrar and historical schedules. Read them as
   dated context, not current deployment evidence. Their correction needs
   ownership-safe claims; this report does not silently rewrite reserved paths.
4. C21 records owner-directed removal of the Windows candidate-review scheduler
   task and replaces it with default-off in-process persistence. Historical
   installation statements are not current service inventory. The new command
   is separate from the daily Codex task and utility polling; do not enable
   overlapping processing or promise boot recovery without deployment evidence.
5. The portfolio still lists some items completed in the queue (for example
   M20/C20 and M29/C18). Never rerun work solely because its portfolio label is stale.

## What maturity means next

First finish acceptance and truthful observability, then connect authenticated
presentation, prove durable local operations, and build a reviewed isolated
patch pipeline. Component-capability dogfooding and budgeted provider routing
follow those foundations. See the maturity plan for bounded acceptance gates.

Maintain this report when evidence materially changes, not after every poll.
Each update identifies the source revision, verification, deployment limits and
pending owner decisions. Do not turn this document into a second ownership queue.
