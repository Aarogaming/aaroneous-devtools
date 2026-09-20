# Local/cloud orchestration plan

**Status:** Dormant/Roadmapped (per `governance/EVIDENCE_STANDARD.md`) — no code has
been written against this plan yet. This document records scope, sequencing, and
governance boundaries for milestones M40–M45 in `governance/MULTITHREAD_PROGRAM.md`.

## Goal

Increase the share of bounded engineering work (analysis, drafting, inventory,
frontier discovery) that the local `qwen3.5:9b-q6` worker performs unattended,
while keeping cloud-model (Codex/Claude Code) sessions focused on claims, review,
verification, and anything the local worker is not authorized to do. This reduces
cloud token spend without changing who is allowed to commit product changes.

## Governing constraint

This plan does not relax `governance/LOCAL_AGENT_CONTROL_PLANE.md`. That document is
authoritative: the scheduler is deterministic, the local worker never claims queue
rows, edits either repository, invokes a shell, or decides verification passed — it
only returns a reviewed proposal artifact. Every milestone below is scoped to either
(a) expanding what the local worker analyzes or drafts as a proposal, or (b) making
the deterministic Rust scheduler smarter about which proposals to request and when —
never to giving the local worker new authority.

The `ConcurrenceSnapshot` / graduation signal in Aaroneous's
`crates/compute/src/concurrence_engine.rs` (rolling local/cloud agreement, graduation
at ≥95%) is an **advisory input to the integration owner's review**, not an
auto-approval mechanism. M40 surfaces that score; it does not let a high score skip
human review. Aaroneous itself gets no new dependency on devtools from this plan —
all scheduler/queue work stays in `aaroneous-devtools/foundation`.

## Milestones

### M40 — Surface the concurrence score in the deterministic scheduler

Read `crates/compute/src/concurrence_engine.rs` and
`foundation/crates/foundation_cli/src/scheduler.rs`. Draft (as a reviewed proposal,
not a committed change) a way for the scheduler to read the current
`ConcurrenceSnapshot` for a component and attach it as an advisory confidence field
on eligible-work recommendations, so the integration owner sees "local/cloud
agreement: 97%" next to a claim candidate instead of having to check the Shadow
Model HUD separately. No change to claim authority.

- **Dependency:** none
- **Preferred capability:** Rust systems review, read-only cross-repo analysis
- **Evidence:** a proposal artifact plus a short design note in this file's
  changelog once reviewed; `Complete` requires committed source, passing tests, and
  `cargo xtask gate` per `EVIDENCE_STANDARD.md`.

### M41 — Make frontier discovery self-scheduling

`foundation/crates/foundation_cli/src/frontier_discovery.rs` currently runs only
when a hand-authored `worker-jobs/audit-*-git-discovery*.toml` packet invokes it.
Propose a recurring invocation (native multithreader cadence, not a new authority
tier) that runs frontier/git discovery on a schedule and emits new bounded
`worker-jobs/*.toml` packets as its output, so the support inbox is populated
continuously instead of by manual authoring.

- **Dependency:** M40 not required; independent
- **Preferred capability:** Rust foundation / scheduling
- **Evidence:** committed scheduling hook, a sample of auto-generated packets
  reviewed and accepted through the normal `worker-jobs/` flow.

### M42 — Define the cloud dispatch adapter contract (`cloud_providers.rs`)

`foundation/crates/foundation_cli/src/cloud_providers.rs` is currently a stub;
`architecture/JOB_PIPELINE.md` states this release "exposes no paid dispatch."
Scope a contract (not an implementation) for how a cloud routine session reads
`published`/`eligible` queue state and writes results back through the same
`Ready → Claimed → Review → Done` state machine already governing local work,
instead of coordinating with the cloud routine purely through
`governance/SCHEDULE.md` convention. This is the highest-authority milestone in
this plan (it touches claim/dispatch machinery) and should stay a primary-lane,
human-reviewed design, not a local-worker deliverable — the local worker may only
be asked to analyze the existing stub and produce a read-only comparison report
against `contracts/local-proposal-v1.schema.json` for design input.

- **Dependency:** M40, M41 informative but not blocking
- **Preferred capability:** Release governance / distributed coordination design
- **Evidence:** an accepted RFC-style contract doc before any implementation PR.

### M43 — Auto-partition non-crossing work packets

Use `foundation/crates/foundation_cli/src/dependency_review.rs` and
`component_inventory.rs` output to derive non-overlapping file/module lanes before
a packet is published, so concurrently dispatched local and cloud work does not
collide on `governance/COORDINATION_QUEUE.md` claims. Scope as a read-only report
generator first (propose lane boundaries), not an automatic claim writer.

- **Dependency:** M41 (needs a steady packet supply to partition)
- **Preferred capability:** Dependency graph analysis
- **Evidence:** committed lane-derivation tool, a reviewed sample run against the
  current queue with no incorrect overlap.

### M44 — Expand job templates beyond read-only analysis, with local pre-proofing

Only one job template (read-only wire research) is enabled today. Cautiously add
`test-fixture` and bounded `code-draft` templates already supported by the packet
schema (`mode` field), gated by `foundation_cli/src/cross_verifier.rs` and
`consensus_engine.rs` before any draft reaches `Review`. This is the milestone most
likely to need its own review-gate design pass; do not batch it with M40–M43.

**Local pre-proofing (decided 2026-09-20, see Decisions log):** every `code-draft`
output is applied to a throwaway local worktree and run through the existing gate
stack — `cargo xtask gate`, `ast_auditor`, `typos`, then `cross_verifier`/
`consensus_engine` — *before* it is ever presented as a candidate. A draft that
fails any local gate is retained as failed evidence per `LOCAL_WORKER_SERVICE.md`
and never reaches `Review`. This is proofing, not authority: the local worker still
never pushes a branch or opens a PR. Proofing happens locally rather than via a
pushed branch and hosted CI specifically because `Aarogaming/aaroneous` is a public
repository — a branch pushed by an unattended local model would be publicly visible
and would run hosted CI before any human reviewed it. Local proofing gets the same
"is this even correct" signal without that exposure.

Review of proofed candidates is batched, not one-off, against a fixed rubric:
**intent** (did the output do what the job's `instruction` asked), **location**
(right file/module, consistent with `component_inventory.rs` boundaries),
**method** (idiomatic, doesn't violate execution-host dependency policy or other
`WORKLIST.md` constraints), and **tweaks** (what to adjust before it's usable). This
replaces one-draft-at-a-time review with a checklist pass over several proofed
candidates at once.

- **Dependency:** M42 (needs the adapter contract settled so draft volume doesn't
  outstrip review capacity from both local and cloud sources at once)
- **Preferred capability:** Release governance / verification design
- **Evidence:** cross-verifier acceptance criteria documented and exercised on at
  least one real draft before the template is marked enabled; the local pre-proof
  harness itself needs committed source and a passing `cargo xtask gate` run before
  its own status can read `Complete`.

### M45 — Unify draft output through Aaroneous's own cratification pipeline

Aaroneous already has a working "code in, compliant Aaroneous component out"
pipeline: `crates/cratify` re-exports `adaptation_engine::{harvest, harvest_path}`
(ingest external/candidate source), `adaptation_engine::AutoWrapperEngine` /
`NativeComponentRunner` (adapt into a runnable component), `transpiler::scaffold_crate`
(produce a compliant crate scaffold), and `ast_auditor::run_workspace_audit`
(compliance audit). This is the same machinery `contracts/FRONTIER_FILTRATION_STANDARD_V1.md`
already targets for external-crate ingestion outcomes (`DIRECT_ADOPT_WITH_WRAPPER`,
`CRATIFY_KERNEL_EXTRACTION`, `BORROW_SPEC_AND_SYNTHESIZE`).

Route M44's local-worker `code-draft` output through this same pipeline instead of
treating it as a one-off JSON artifact: a proofed draft becomes a `harvest` input,
gets wrapped/scaffolded by the existing cratify tooling, and is audited by
`ast_auditor` using the same acceptance bar as an externally-adopted crate. The
payoff stated as the goal for this milestone: if every accepted draft — whether
local-worker-generated, frontier-discovered, or cloud-authored — is forced through
one compliance-producing pipeline, then every acceptance *by construction* extends
Aaroneous's own component surface, and the framework's own tooling is what scales
its expansion capacity, not ad hoc integration per source.

- **Dependency:** M44 (needs proofed drafts to feed in) and M41 (frontier-discovered
  candidates are the other input source this unifies with)
- **Preferred capability:** Component architecture / compiler-adjacent tooling
- **Evidence:** at least one local-worker draft and one frontier-discovered crate
  both accepted through the same `cratify` → `ast_auditor` path with no
  source-specific special-casing in the acceptance criteria.

## What this plan delegates to the local worker today

Per `governance/LOCAL_WORKER_SERVICE.md`, the local worker accepts `analysis`,
`test-fixture`, or bounded `code-draft` jobs submitted as `worker-jobs/*.toml`
packets, polled every two minutes inside dispatch windows (minutes 06–20 and
41–50), each bounded to 1–8 declared files (32 KiB each, 64 KiB total). Given that:

- **Good delegation candidates now:** M40's read of `concurrence_engine.rs` +
  `scheduler.rs` and a first-draft integration proposal; M41's inventory of current
  `frontier_discovery.rs` call sites; M43's lane-derivation report generation. These
  are exactly the "analysis / bounded code-draft in scratch space" shape the
  worker is built for, and routing them through `worker-jobs/` instead of a cloud
  session is real token savings with no authority change.
- **Keep on a primary (human-reviewed) lane:** M42's contract design — it decides
  claim/dispatch authority, which is out of scope for a proposal-only worker by
  `LOCAL_AGENT_CONTROL_PLANE.md`'s own terms. M44's verification-gate design for
  the same reason.
- **Not worth delegating:** one-off planning documents like this one. The
  worker-jobs dispatch cadence (2-minute poll, narrow dispatch windows, 8-record
  pending cap) adds latency that exceeds the cost of a cloud session writing a
  single document directly; delegation pays off for repeated, bounded, structurally
  similar subtasks (drafts, fixtures, inventories), not single narrative writing.

## Decisions log

Dated decisions from planning discussion, kept so later sessions don't re-litigate
settled scope:

- **2026-09-20 — Proofing stays local, not CI-on-a-pushed-branch.** Considered
  letting the local worker push branches and rely on hosted GitHub Actions CI to
  prove drafts before review. Rejected for now: `Aarogaming/aaroneous` is public,
  so a pushed branch and its CI run would be visible before any human reviewed the
  content. Local proofing (apply to a throwaway worktree, run `cargo xtask gate` +
  `ast_auditor` + `typos` locally) gets the same "does it even work" signal without
  that exposure. Revisit only if a private mirror/fork is set up specifically for
  this purpose — see M44.
- **2026-09-20 — Review is batched against a fixed rubric, not ad hoc.** Proofed
  candidates are reviewed together against intent / location / method / tweaks,
  rather than one draft prompting one isolated review pass. See M44.
- **2026-09-20 — Long-term goal: one compliance pipeline for every draft source.**
  Rather than building source-specific acceptance logic for local-worker drafts,
  frontier-discovered crates, and cloud-authored changes separately, route
  everything through Aaroneous's existing `cratify`/`adaptation_engine`/`ast_auditor`
  pipeline so every acceptance produces a compliant component by construction. See
  M45. This does not change Aaroneous's dependency direction — Aaroneous still has
  no dependency on devtools; devtools calls into `cratify` as a consumer, the same
  way it already does for `ast_auditor` per `WORKLIST.md` B6.
- **2026-09-20 — Concurrence score stays advisory.** Reaffirmed from this plan's
  first draft: `ConcurrenceSnapshot` informs the integration owner, it does not
  grant the local worker auto-approval authority. See "Governing constraint" above.

## Sequencing

M40 and M41 are independent and can both enter `Ready` immediately. M43 depends on
M41's packet supply. M42 is the gating design work for M44; neither should be
claimed until M42's contract is accepted. M45 depends on M44 (and benefits from
M41's packet supply as a second input source) and should not be claimed before a
proofed local draft actually exists to route through it. No milestone in this plan
authorizes paid cloud dispatch or local write/claim authority — those remain
explicit future decisions, not defaults unlocked by completing M40–M45.
