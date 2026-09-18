# User-facing maintenance program

## Direction

Devtools becomes a guided maintenance application backed by the existing Rust Foundation. It explains repository practices, runs deterministic checks, schedules bounded local work, and offers budgeted cloud escalation for difficult or urgent jobs. Aaroneous remains independently buildable and runnable. This document is the delivery map; the coordination queue records active ownership.

## Historical Brief Repository Review (2026-09-16 Baseline)

- Aaroneous is the Rust component framework, version 0.3.2. Its canonical verification is `cargo xtask gate`. Existing dirty product files belong to concurrent contributors and were not changed by this review.
- Foundation is already implemented, not a new replacement: `foundation/` contains a Rust workspace and explicit-input maintenance CLI, evidence contracts, inventory, intake, proposal review and local polling worker.
- Historical note (reconciled 2026-09-17): `gui/` (React/TypeScript dashboard) and `agentic_registrar.rs` in `foundation_cli` have since been committed and hardened under tracked repository source (C15, C19). Delivery of these bounded technical slices is distinct from owner acceptance (M34).
- `gui/` uses React, TypeScript, Vite and Tailwind. Read-only dashboard visualization and type-check CI are verified (C15); write integration remains gated.
- The Rust registrar (`agentic_registrar.rs`) has been hardened under C19 with explicit token authentication, Host/Origin checks, request/concurrency limits, and loopback isolation. Unattended auto-promotion/write commands remain strictly gated on owner authorization.
- The existing bounded Rust local worker generates unverified scratch artifacts. Preserve that authority boundary while introducing the application.
- The active Codex audit heartbeat was deleted at the user's request. The older rolling-worklist heartbeat remains paused. Native local review and worker schedules remain installed; this review did not reconfigure them.

This section records the historical 2026-09-16 baseline review for evidence context.

## Guided workflow

1. Connect an explicitly selected repository. Show branch, dirty state, agent rules, required checks and existing task ownership.
2. Choose maintenance goals and a schedule. Explain what each check proves and show local availability, review debt and resource limits.
3. Run deterministic checks first. Convert reproduced findings into bounded proposals with source evidence and acceptance criteria.
4. Offer local analysis and drafts for approved work. Show queued, running, failed, awaiting review and verified states from real evidence.
5. Review a result or request expedited help. Show why escalation is suggested, which evidence leaves the PC, estimated maximum spend and alternatives.
6. Apply accepted changes in an isolated branch/worktree, run repository gates, and present the diff and evidence before integration. Git publication is a separate authorized action.

Offline or unavailable services must display unavailable/stale states. Demonstration data must be explicitly labeled and cannot transition into real job history. A generated result is never a verified change.

## Rust core and interface boundary

Keep scheduling, task transitions, ownership, validation, evidence, provider routing and budgets in reusable Rust modules. The existing TypeScript interface can remain a presentation client in devtools; no executable maintenance policy belongs in it. Decide desktop packaging only after the browser interface and backend contract work. Do not build a second scheduler in the UI or import devtools into Aaroneous.

Use versioned API/artifact contracts with bounded input/output. Local service actions require authorization independent of browser CORS, explicit allowed origins and repository scope. Read-only status is the first slice; model output cannot grant commands, credentials, spending or write permissions.

## Routing and cost policy

Deterministic tools precede inference. Route eligible support jobs to local Qwen first, using the current strict response schema, shared reservation, bounded context and backpressure. Distinguish capability failures from malformed output, outages, missing tools and configuration errors; paying a larger model does not repair a broken environment.

Bound retry attempts and retain idempotency history. Escalation considers required capability, evidence sensitivity, deadline, queue delay and configured price. Select the lowest estimated cost provider that satisfies those constraints, rather than promising local can complete every simple task.

Cloud dispatch defaults to disabled. Enable it only with configured credentials, allowed data scope and per-job/day/month spending caps. Reserve an upper cost bound before dispatch, reconcile actual usage afterward, and handle ambiguous network outcomes without double spending. Unknown pricing or usage blocks automatic spending. Record provider/model, price version, evidence revision, reason, approval, token limits and outcome. Local execution uses hardware and electricity even though it consumes no cloud credits.

## Delivery order and independent lanes

| Phase | Priority | Scope and dependency | Acceptance evidence |
| --- | --- | --- | --- |
| 1. Reconcile prototypes | P0 | Inspect D: checkout registrar, GUI and daemon with their owner; compare with verified Foundation worker | Preserved source, exact-path handoff, explicit retirement/adoption decisions; no duplicate unattended writers |
| 2. Truthful status API | P0 | Rust status contract and service health; depends on phase 1 ownership | Real last-poll/last-success/active-job/error/review counts; stale/offline cases tested; no product writes |
| 3. Guided dashboard | P1 | `gui/` presentation only; depends on phase 2 contract | No simulated production success; connect repository, explain checks, display real queue and review artifacts |
| 4. Durable local operations | P1 | Rust worker/scheduler state | Bounded retry, leases, cancellation, restart recovery, shared resource limits and owner-controlled dispositions tested |
| 5. Cloud escalation | P2 | Rust provider and budget contracts; depends on phases 2 and 4 | Fake-provider tests for caps, reservations, unknown prices, ambiguous retries, data consent; live paid call only when configured and authorized |
| 6. Reviewed patch pipeline | P2 | Isolated worktree runner and verifier; depends on phases 1 and 4 | Scope/dirty-tree protections, full native gates, retained diff/evidence; no automatic shared-branch rollback |
| 7. Installation and onboarding | P2 | Thin OS adapters and user guidance; depends on operational local path | Install/pause/uninstall and restart tests; logged-out limitations shown honestly |

Use at most three primary implementation lanes: Rust core, interface, and independent integration/review. Freeze shared API schemas before parallel edits. Each lane claims exact files in the coordination queue; this portfolio is not permission to overlap C9/C10 review scopes or the uncommitted registrar owner. Qwen remains a bounded support worker under the current agent contract.

## First implementation milestone

Connect the existing dashboard to a read-only Foundation status API and replace simulated success with real unavailable, pending-review and gate-backed states. Review the registrar before adopting any write capability. Paid provider integration follows local correctness, not the other way around.
