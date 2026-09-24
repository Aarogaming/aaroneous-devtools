# Local Model Autonomy Policy

Owner-directed, 2026-09-18. This formalizes how much autonomy the local-agent
pipeline (`local_worker.rs`, `multithreader.rs`, `agentic_registrar.rs`) has,
exactly what forces it to stop and hand off, and how cloud agents fit in as a
scalable review layer rather than a synchronous bottleneck. It does not
relax [`LOCAL_AGENT_CONTROL_PLANE.md`](LOCAL_AGENT_CONTROL_PLANE.md)'s
existing constraints — it says precisely where those constraints' edges are
and what happens at them.

## Why this exists

A local model auditing its own output in the same context that generated it
is a weak signal everywhere, cloud or local — C14's incident (a prototype
that wrote straight into a checkout and self-certified via a bare
`cargo check`) is the concrete proof, and this session independently
rebuilt the same anti-pattern once before catching it. The fix is not a
smarter self-audit; it's more independent, deterministic verification, and
routing anything deterministic tools can't cover to a genuinely separate
process — never the same context grading its own work.

Cloud agents are also the right shape for the judgment calls deterministic
tools can't make: reviewing a batch of local-model drafts is cheaper than
authoring that same batch would be, so the efficient pattern is local churn
(cheap, always-available, run continuously) feeding a cloud agent's batched
review (expensive per call, so it should review in bulk on a schedule, not
be invoked synchronously per draft).

## The four tiers

**Tier 0 — deterministic, non-negotiable, no model involved.** Compiler,
`cargo clippy -D warnings`, the test suite, Miri, `cargo-audit`/`cargo-deny`,
`cargo-semver-checks`, `ast_auditor`, `compliance_auditor`. This is what
`xtask gate` and `compliance_auditor`'s CI job already are — the real
authority. Nothing local-model-originated lands without passing the exact
same gate a human's PR would, via the real GitHub CI a pushed branch's PR
triggers, never a hand-rolled local approximation of it.

**Tier 1 — local model, cheap and always-on.** `worker-jobs/*.toml` →
`local_worker.rs` drafts; `agentic_registrar` dispatch. Bounded, narrow work:
new test files, fixture drafts, evidence classification. Selection is
oldest-last-attempted-first (not alphabetical — see the fairness fix landed
alongside this doc), so no single job can permanently occupy the queue.

**Tier 2 — automatic escalation, no human yet.** `agentic_registrar::promote`
enforces these without asking anyone:

- **High-risk allowlist** (`is_high_risk`): a change touching a
  safety/security-critical crate (`ipc_bus`, `wire`, `hypervisor`,
  `governance`, `ast_auditor`, `compliance_auditor`, `platform_bridge`,
  `core_contracts`, `sdk`) or containing generated `unsafe` code is blocked
  from local promotion outright, regardless of how clean the draft looks.
  That class of change needs a cloud agent to implement directly, not a
  local model writing straight to the checkout.
- **Circuit breaker**: two consecutive verification failures at promotion
  time trips it; further promotion is refused until explicitly cleared via
  `reset-circuit-breaker --reason "..."`. A push/network failure after a
  successful local commit does not count — that's not evidence the model's
  output is bad, and the branch/commit is left in place for a manual retry
  instead of being discarded. A cloud agent can also trip it **remotely**
  after the fact (a later-discovered revert, a real problem found in
  review) via the tracked `circuit_breaker_override.toml` — see "What
  still needs a human" below for how that channel works.
- **Mod-reachability gate** (`ensure_new_file_is_reachable`): a new
  `src/`-rooted file Cargo would never actually compile (no `mod`
  declaration anywhere) is refused, not silently "verified" against nothing.

Anything past these gates gets a cheap local pre-flight (`cargo check` +
`cargo clippy` scoped to the touched crate — fast, catches the obvious
cases) and, on success, is pushed to an isolated `local-agent/{task_id}`
branch. `promote` stops there. It does **not** open a pull request itself —
adding GitHub API calls and a credential to this binary would duplicate
what a cloud agent already has (real GitHub access, real push access) for
no benefit, and would be exactly the kind of "new always-on writer, paid
provider or credential" `DEVTOOLS_MATURITY.md` says this plan doesn't
authorize.

**Tier 3 — the human.** Only for what Tier 2's cloud-agent escalation
couldn't close itself: a security-relevant finding a cloud agent isn't
confident resolving alone, or a decision this policy explicitly reserves
(see "What still needs a human" below).

## The escalation channel: push a branch, let a cloud agent take it from there

A recurring cloud-agent Routine (not a synchronous per-draft trigger) scans
`Aarogaming/aaroneous` for open `local-agent/*` branches with no PR yet, and
for local-model-originated PRs already open, on a schedule — reviewing in a
batch is what makes "100 reviews easier than 100 reviews worth of
content-writing" actually true; a synchronous trigger per draft would not
get that benefit. For each one found:

1. Open the PR (if not already open), clearly marked as local-model-drafted
   in its body, referencing the originating task id.
2. Let Aaroneous's real CI run — the actual authority, not a duplicate.
3. Review the diff. Confident and in scope → drive it to green and sign off.
   Ambiguous, or the diff doesn't match its stated intent → close with a
   comment explaining why, or escalate to the owner if genuinely stuck.
4. **Never auto-merge**, even on green CI plus a clean review, for now. See
   below for why and when that might change.

## Why no auto-merge yet, and what would justify it

Dependabot-style auto-merge is safe because a version bump is narrow and
semver-checked. Free-form generated code in a framework with this much
`unsafe`/memory-layout surface (the header-size bug and the `swmr_shm`
crash-recovery bug this session found were both in exactly that surface) is
a different risk class — green CI proves it compiles and passes today's
tests, not that it's the right change. A cloud agent's explicit sign-off is
the standing gate before merge, not necessarily the owner's.

Auto-merge could be justified later, but only for a narrow, proven-safe
category (e.g. new test-only files under `tests/`, matching Tier 1's own
scope) after a trust-building period — a specific number of consecutive
clean cloud-agent-reviewed merges in that category with zero reverts, not a
general policy change. Track this as a follow-up decision, not a default.

## Adaptive pacing, not fixed-interval retriggering

Owner's explicit design instruction, verbatim: "trigger the agent, and
monitor its activity, output, byproducts, and use that to determine when to
fire the next agent trigger... don't re-engage every second, persistence,
we're playing the long game, efficiently." `local-worker-serve` implements
this directly: tick, read what the tick actually did from `status.txt`
(a real dispatch vs. quiet — nothing eligible, backpressure, or the
shared lock already held), and only then decide the
next delay — doubling on quiet up to a 60-second ceiling, resetting to the
5-second base the moment real work happens. The same principle applies to
any other scheduling this pipeline grows: never fire on a blind interval
when the previous outcome tells you something about the next one.

## What still needs a human (not delegated to any tier above)

- **M34 acceptance.** Nothing above is "accepted" in the queue's own terms
  until the C10-C19 review backlog gets an actual independent verdict — see
  `LANE_CLOSEOUT_PLAN.md`. This policy governs how the pipeline behaves; it
  doesn't retroactively accept its own prior lanes.
- ~~**Revert-triggered circuit breaker trips.**~~ **Closed (C38, renumbered
  2026-09-20 from C27 to resolve a coordination-queue ID collision — see
  `COORDINATION_QUEUE.md`'s C35 row).** The
  breaker still only tracks local Tier-0 verification failures at
  promotion time automatically, but a cloud agent that finds a real
  problem after the fact — a merged local-agent PR reverted, a pattern of
  failures — can now stop the pipeline remotely: set `tripped = true` and
  a `reason` in the tracked
  [`circuit_breaker_override.toml`](circuit_breaker_override.toml) and
  push. `promote` fast-forward-pulls this repo's `main` before every
  promotion when the registrar config sets `devtools_repo_root`, and
  fails **closed** — refuses to promote — if that pull itself fails,
  rather than silently acting on stale state. Clearing a trip is the same
  action in reverse (`tripped = false`, push); there's deliberately no CLI
  command for it, it's a plain reviewable file. A human is still the one
  who decides whether the underlying problem is actually fixed before
  clearing it — this only closes the *reporting* gap, not the judgment
  call.
- **Raising the auto-merge bar** described above.
- **Anything a cloud agent's own review explicitly flags as needing owner
  judgment** — that escalation path already exists in the PR-babysitting
  protocol those agents run under; this policy doesn't change it.

## Write-path inventory (verified 2026-09-23)

Every mechanism that can write to a Git checkout, and the fence around it. A mechanism not listed
here has no write authority. Add a row before adding a write path.

| Mechanism | Writes to | Fence | Status |
| --- | --- | --- | --- |
| `agentic_registrar::promote` | Isolated `local-agent/{task_id}` branch | High-risk allowlist, circuit breaker, mod-reachability, scoped pre-flight; no PR, no merge | The sanctioned write path |
| `local_staging` (M62) | Linked worktrees on local disk | No `git push` in source; never the owner's checkout | Allowed |
| `foundation actuate` (`actuator.rs`) | Configured `checkout` | Refuses unless `checkout` is a linked worktree with a clean status (commit `60bee27`); rejects absolute/`..` paths | **Violates the guardrail below** (a second write path that skips the allowlist and circuit breaker and trusts a local gate). Must not run until it routes through `promote`'s gates or the owner amends this policy |
| `scripts/auto_actuator.ps1.disabled` | Would `git add .`/`commit`/`reset --hard` the Aaroneous checkout | Disabled (renamed); script is syntactically broken | **Do not re-enable** |
| `dispatch` / `local_worker` / `multithreader-review` | Result JSON and artifacts only | No checkout writes | Allowed |

Before the fence landed, the live worker config (`artifacts/local-worker-service/worker.toml`) pointed
`actuate` at the owner's primary Aaroneous checkout, where a red gate would have triggered
`git reset --hard HEAD` over uncommitted work. The scheduled task `Aaroneous-Devtools-LocalWorker`
referenced below was **not installed** on the owner's machine as of this date.

## Guardrails this policy does not relax

- `promote` stays the only write path. `dispatch`/`local_worker`/
  `multithreader-review` never gain write authority "to move faster" —
  every real incident this repo's history has (C14, and the Aaroneous-side
  daemon retired earlier this session) came from exactly that shortcut.
- No new OS-level always-on scheduled task beyond the already-installed
  `Aaroneous-Devtools-LocalWorker` poll. C21's decision to make broader
  persistence opt-in rather than installed was deliberate and still holds;
  `local-worker-serve` is opt-in for the same reason.
- Review debt is a bottleneck lane, not free capacity — per
  `MULTITHREAD_PROGRAM.md`'s own capacity model, don't let it grow while
  spending capacity on new autonomy features.
