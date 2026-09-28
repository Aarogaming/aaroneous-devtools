# Decision Log

Short decisions prevent rediscovering old tradeoffs. Add a dated entry when a choice affects a
contract, dependency direction, security boundary, or long-term maintenance cost.

## 2026-09-28 — Wrote the cross-agent coordination protocol AGENTS.md already pointed to

`AGENTS.md` (Product repo) has referenced `governance/CROSS_AGENT_COORDINATION_PROTOCOL.md` and
`governance/ROUND_TABLE_INTRO_PROMPT.md` since at least 2026-09-23; neither existed until the owner
asked directly what tasks are safe to hand Antigravity and how to prompt it, after the 2026-09-26/27
audit found the same unevidenced-claim pattern recurring five times. Both files now exist, with
dated, cited per-agent notes (not personality judgments) and a concrete "scope as tripwire, not
description" prompting rule. Branch protection (enabled 2026-09-27, see `GIT_WORKFLOW.md`) is named
in both as the actual mechanical fix; the prompting guidance reduces friction, it doesn't replace it.

## 2026-09-16 — Rust-first foundation with explicit external inputs

Foundation logic is Rust. External support directories are allowed only as explicit CLI or manifest
inputs and are recorded in versioned reports. Aaroneous does not depend on the foundation workspace.

## 2026-09-16 — Practical companion, independent framework

Devtools becomes a user-facing local maintenance companion. It keeps Rust-first reusable logic but permits ordinary services, allocation, async work and a TypeScript UI. Aaroneous's stricter execution-plane rules apply to product changes, not all companion code. Existing Git, OS scheduling and maintained tooling are reused; deterministic software precedes model inference. Cloud escalation is disabled until its data and spending policy is configured.

The operating model owns organization; the multithread portfolio owns priorities; the coordination queue owns live claims; the application design owns delivery milestones. Old worklist briefs retain context without competing as current status. No prototype relocation or service-path change is authorized by a documentation cleanup alone.

## 2026-09-17 — Cloud routine as interim primary-lane capacity

Until a maintained local or GUI equivalent exists (`gui/` is uncommitted prototype work; see
`README.md`), a Claude Code cloud Routine fills the primary-lane role the native services
deliberately do not: it can claim a queue lane, implement it, run Aaroneous's full native gate, and
push/open a pull request unattended. It is bound to the user's own persistent cloud session rather
than spawning a fresh session per run, so run-to-run context (what was claimed, what is blocked)
survives without re-reading the whole queue from scratch. Cadence is daily, with wider weekly/monthly
scope folded into the Monday and 1st-of-month wakes — see `SCHEDULE.md`. It obeys the same capacity,
claim-expiry, and native-gate rules as any other thread in `MULTITHREAD_PROGRAM.md`; it does not get
a reserved lane, and it stands down when three primary lanes are already claimed.
