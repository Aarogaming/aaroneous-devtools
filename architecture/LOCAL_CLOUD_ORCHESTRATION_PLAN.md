# Local/cloud orchestration plan

**Status:** Dormant/Roadmapped (per `governance/EVIDENCE_STANDARD.md`) — no code has
been written against this plan yet. This document records scope, sequencing, and
governance boundaries for milestones M40–M47 in `governance/MULTITHREAD_PROGRAM.md`.

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

**Contract decided 2026-09-20** (per `FOUNDATION_BOUNDARY.md`: devtools takes no
product dependency on Aaroneous, so this cannot be a Cargo dependency on
`crates/compute` — it is a file contract, explicit input/output only). Verified by
reading the actual source (not assumed): `core/hypervisor/bin/hypervisor.rs:1365`'s
`concurrence.shm` read path has **no writer anywhere in the codebase** — it is dead
code today, always printing "No concurrence data found." The real gap is a missing
writer, not a missing export flag on existing plumbing:

- **Aaroneous side:** add `#[derive(serde::Serialize)]` to `ConcurrenceSnapshot`
  (`crates/compute/src/concurrence_engine.rs:36-55`; currently only `Debug, Clone,
  Copy`). In `core/hypervisor/src/supervisory_loop.rs`, near the existing
  `concurrence_snapshot.write()` update at line 1579 (inside the heartbeat thread
  that already holds the live snapshot), add a periodic JSON serialize-and-write to
  an explicit output path (e.g. `paths.data().join("concurrence_report.json")`),
  using the struct's real fields (`rolling_concurrence`, `total_ticks`,
  `total_agreements`, `avg_confidence`, `avg_reward`, `divergence_mse`, `graduated`,
  `ticks_since_graduation`). This uses only safe code the heartbeat thread already
  has in memory — it does not touch, fix, or rely on the existing unsafe
  `mem::transmute` shm-file reader at `hypervisor.rs:1383`, which stays untouched
  and out of this milestone's scope (that dead/unsafe path is a separate, unstarted
  cleanup item, not part of M40).
- **Devtools side:** `foundation/crates/foundation_cli/src/scheduler.rs` accepts an
  optional explicit `--concurrence-report <path>` input (matching the existing
  explicit-input-path convention in `foundation_cli`). When present and parseable,
  attach `rolling_concurrence`/`graduated` as an advisory field on eligible-work
  recommendations. Missing or stale file: recommendations proceed without the
  field — never block on it.
- This is Tier 1 on both sides per `governance/REVIEW_CONDITIONS.md` (no `unsafe`,
  no FFI, no named `SECURITY.md` surface); full `cargo xtask gate` required on the
  Aaroneous side, `cargo fmt`/`cargo test`/strict Clippy for `foundation/` on the
  devtools side.

- **Dependency:** none
- **Preferred capability:** Rust systems review, read-only cross-repo analysis
- **Evidence:** committed source in both repos, passing `cargo xtask gate`
  (Aaroneous) and the devtools Rust checks, per `EVIDENCE_STANDARD.md`.

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

### M42 — Consensus-to-queue readiness report (corrected scope, 2026-09-20)

**This milestone's original framing was wrong, corrected after reading the actual
source instead of trusting `architecture/JOB_PIPELINE.md`'s "exposes no paid
dispatch" claim.** `foundation/crates/foundation_cli/src/cloud_providers.rs` is
**not** a stub — it has real, working `AnthropicClaudeProvider`,
`GoogleGeminiProvider`, `OpenAiCodexProvider`, and `LocalOllamaProvider`
implementations of a shared `AgentProvider` trait, wired into the `cross-verify`
CLI command (`main.rs` ~line 719). `cross_verifier.rs`'s `cross_verify_proposal`
already enforces the exact invariant `governance/CHANGE_VERIFICATION_TIERS.md`
Tier 2 calls for — it hard-`bail!`s if the auditor provider/model matches the
author's, i.e. an agent cannot cross-audit its own proposal. `consensus_engine.rs`
already combines cross-verdicts plus mechanical check results into a
`ConsensusStatus` persisted in a `ConsensusLedger`. **The adversarial
second-independent-pass requirement this plan called for is already implemented,
not aspirational.**

The real remaining gap, verified against `governance/COORDINATION_QUEUE.md`'s
actual history: nothing links a `ConsensusStatus::Approved` record back to a
queue row's status. That link is manual today — a reviewer reads the consensus
result and edits the markdown table by hand (see queue row C9's entry: an
independent reviewer explicitly re-ran the claimed tests themselves before
marking it `Done`, rather than trusting the row). **This is deliberate, not a
missing feature.** `CONTRIBUTING.md`'s Definition of Done states outright: "An
independent reviewer marks it Done after checking scope and verification. Do not
self-certify generated artifacts." Automating "consensus approved → row marked
Done" would let one pipeline both produce and certify its own work — exactly the
self-certification this repo already explicitly forbids.

**Revised scope:** a read-only consensus-readiness report, structurally identical
to `scheduler.rs`'s existing `ScheduleReview` pattern (same explicit-input-path
convention, same read-only guarantee). It reads a `ConsensusLedger` and
`COORDINATION_QUEUE.md`, and surfaces which `Ready`/`Claimed` rows have an
`Approved` consensus record — but it never writes queue status itself. A human or
independent reviewer still performs the actual `Done` edit, informed by this
report instead of having to manually cross-reference the ledger. This keeps the
milestone Tier 1 (read-only, additive, no claim/dispatch authority change) instead
of the Tier-2-adjacent authority question the original framing implied.

- **Dependency:** M40 (file-contract pattern precedent), existing
  `consensus_engine.rs`/`cross_verifier.rs` (already built, this milestone only
  adds a read-only report over their output)
- **Preferred capability:** Rust systems review, read-only report generation
- **Evidence:** committed report-generation code with passing devtools checks; a
  real run against an actual `ConsensusLedger` showing correct row matching.

### M43 — Auto-partition non-crossing work packets (corrected scope, 2026-09-20)

**Corrected after reading the actual source**: `foundation/crates/foundation_cli/
src/local_worker.rs:162` already has `fn scope_reserved(queue: &str, evidence:
&[String]) -> bool` — a textual scope-overlap check against
`governance/COORDINATION_QUEUE.md`'s `Claimed`/`Review` rows, including bracketed
directory scopes (`crates/paths/**`), used today to skip dispatching a local-worker
job whose evidence overlaps active work. This is most of what M43 asked for,
already built — just private to `local_worker.rs` and checked one packet at a
time against the queue, not across a freshly-generated *batch* of candidates
before they're all written to `worker-jobs/`.

**Real remaining gap:** M41's `frontier_discover-serve` and M47's
`capability-compose` can each emit several candidate packets per cycle. Nothing
today checks whether two candidates from the *same* batch would overlap each
other's scope before both land in `worker-jobs/` (only checked against the queue,
not against each other). Revised scope: extract `scope_reserved`'s matching logic
into a small shared, reusable function (or expose it `pub(crate)` if extraction is
riskier than reuse — judgment call for whoever implements this), and add a
batch-level check: given a set of freshly-generated candidate packets, report which
pairs overlap in declared evidence, so a batch can be trimmed to non-crossing
candidates before any of them are written. Still read-only/report-only, per the
original intent — it does not touch `COORDINATION_QUEUE.md` or auto-resolve
overlaps, only reports them.

- **Dependency:** M41 (needs a batch of candidates to check for cross-overlap)
- **Preferred capability:** Rust systems review, reuse of existing logic over new
  design
- **Evidence:** committed extraction/reuse of `scope_reserved` plus the new
  batch-overlap report, with a test proving two overlapping candidates in one
  batch are correctly flagged and non-overlapping ones are not.

### M44 — Expand job templates beyond read-only analysis, with local pre-proofing

**Precise hook point, verified 2026-09-20:** `foundation/crates/foundation_cli/src/
local_worker.rs:414-415` already computes `let is_code_draft = matches!(job.mode,
Mode::CodeDraft);` and immediately discards it: `let _ = is_code_draft;`. The
`Mode` enum (`Analysis`/`TestFixture`/`CodeDraft`, line ~37) and `Draft.rust_draft:
Option<String>` (line ~50) already parse and carry a code draft end to end — the
mode is recognized, just not yet branched on. This is exactly M44's hook: replace
`let _ = is_code_draft;` with real branching that triggers local pre-proofing when
true, nothing more needs to be built to recognize the mode.

**Pre-proofing must follow the existing `dogfood.rs` pattern, not invent a new
one.** `dogfood::metadata(checkout: &Path, output: &Path)` (~line 15) already runs
a command against an explicit, caller-supplied Git checkout, writes a versioned
JSON report, and never creates or otherwise mutates the checkout beyond what's
explicitly asked. Per `FOUNDATION_BOUNDARY.md` ("no... automatic Aaroneous source
mutation") and `CONTRIBUTING.md`'s "explicit input and output paths" rule,
devtools code may **not** autonomously create an Aaroneous git worktree on its own
initiative — the throwaway worktree pre-proofing runs against must be an explicit,
already-prepared checkout path passed in (by whoever configured the local-worker
service, per `LOCAL_WORKER_SERVICE.md`'s existing config-driven model), not
something `local_worker.rs` spins up itself. Extend `dogfood.rs` with a function
in the same shape — e.g. `verify_draft(checkout: &Path, target_file: &Path, draft:
&str, output: &Path)` that writes `draft` to `checkout.join(target_file)`, runs
`cargo xtask gate` (or a narrower single check, TBD by whoever implements this —
full gate may be too slow for a per-job pre-proof loop; consider starting with
`cargo check`/`cargo test -p <crate>` scoped to the touched crate, escalating to
full gate only before final promotion) via `Command::new("cargo")` exactly as
`metadata` already does, and reports pass/fail — rather than adding pre-proofing
logic directly inside `local_worker.rs`'s dispatch loop.

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

- **Dependency:** none — corrected 2026-09-20. This previously depended on M42
  under the old (incorrect) framing that M42 was a cloud dispatch/claim-authority
  build-out; M42 is now a read-only report over already-existing
  `cross_verifier.rs`/`consensus_engine.rs`, which M44 can use directly without
  waiting on that report's existence.
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
- **Keep on a primary (human-reviewed) lane:** M44's verification-gate design —
  it decides what evidence is sufficient, which is a judgment call, not analysis.
  (M42 no longer needs this caveat since its corrected scope is a read-only report,
  not a claim/dispatch authority decision — see M42's own corrected-scope note.)
- **Not worth delegating:** one-off planning documents like this one. The
  worker-jobs dispatch cadence (2-minute poll, narrow dispatch windows, 8-record
  pending cap) adds latency that exceeds the cost of a cloud session writing a
  single document directly; delegation pays off for repeated, bounded, structurally
  similar subtasks (drafts, fixtures, inventories), not single narrative writing.

### M46 — Work-packet meta-tagging and multi-variant exploration

`worker-jobs/*.toml`'s schema today is flat (`schema_version, id, mode, evidence,
instruction`) with no way to link several attempts at one goal or distinguish the
angle each attempt takes. Add three optional fields to the packet schema:

- `perspective` — the angle a given attempt takes (e.g. `correctness-first`,
  `performance-first`, `minimal-diff`). Free text, not an enum yet — constrain it
  once real usage shows a stable vocabulary, per this repo's own preference for
  evidence-driven rather than speculative structure.
- `goal` — a short id naming the problem being solved, independent of how any
  given attempt solves it.
- `exploration_group` — an id shared by every packet attempting the same `goal`
  from a different `perspective`, so they are structurally linked rather than
  coincidentally similar.

Add a comparison step that runs **before** full local pre-proofing (M44): when an
`exploration_group` has multiple completed drafts, rank or shortlist candidates
first, so the expensive full-gate proofing pass (`cargo xtask gate`, `ast_auditor`,
etc.) is spent on the strongest candidates, not every attempt. Rejected siblings in
a group are archived with the reason they lost — per the same evidence discipline
`EVIDENCE_STANDARD.md` already requires elsewhere — so a later session doesn't
blind-retry an angle already tried and rejected.

- **Dependency:** M41 (needs a steady packet supply worth branching into variants)
  and M45 (needs one acceptance funnel for the winning variant to enter)
- **Preferred capability:** Data/schema design, workflow tooling
- **Evidence:** updated `contracts/` schema and fixtures for the new fields; at
  least one real `exploration_group` with multiple drafts, a documented selection,
  and archived rejected siblings with recorded reasons.

### M47 — Internal capability composition explorer

The internal half of the discovery/research/filter/translate/recreate/review/repeat
loop discussed in planning: not "what does the ecosystem have that we don't"
(M41), but "what do our own existing primitives unlock when combined that neither
does alone." Grounded in real code, not invented structure: Aaroneous's
`core/hypervisor/src/capability_broker.rs` already has a typed capability registry
— `CapabilityDescriptor { id, name, description, category: CapabilityCategory,
parameters: Vec<CapabilityParameter> }`, where `CapabilityParameter` carries
`name`, `param_type`, `required`, `default_value`. Real categories already
registered include `Specialist`, `DevTools`, `MemoryFabric`, `ModelFoundry`,
`SystemBus`, `ScreenAutomation`, `SafetyInterlock`.

**Contract, same file-based pattern as M40** (devtools takes no Cargo dependency on
Aaroneous): Aaroneous side adds a read-only JSON export of the registered
`CapabilityDescriptor` list (reusing the same atomic-write pattern M40 established
— write to a `.partial` file, then rename). Devtools side reads that JSON as an
explicit input and proposes compositions: pairs (or bounded n-tuples) where one
capability's parameter shape plausibly accepts another's described output, scored
and pruned using `governance/BUILD_WORTHINESS_STANDARD.md`'s Quadrant
I/II scoring (capability value, implementation feasibility) rather than
brute-force enumeration — the combinatorial-explosion risk named when this idea
was first discussed. A proposal that scores above `BUILD_WORTHINESS_STANDARD.md`'s
`ADMIT_TO_READY`/`ADMIT_WITH_SCOPING` threshold becomes a `worker-jobs/*.toml`
packet (feeding M41's/M44's packet supply); everything below is archived with its
score per that standard's `ARCHIVE_AS_ROADMAPPED`/`REJECT_LOG_REASON` outcomes, not
silently discarded.

**Scoping decision (2026-09-20):** the devtools side does not attempt to
mechanically *compute* a Quadrant I/II value judgment from capability metadata
alone — that's a real judgment call, not arithmetic, and claiming a text-matching
heuristic can do it would overclaim what's actually happening. Instead it
generates bounded candidate pairs (real `CapabilityDescriptor` data, not
speculative ones) and emits each as a `worker-jobs/*.toml` packet
(`mode = "analysis"`) asking a reviewer to score it against
`BUILD_WORTHINESS_STANDARD.md` — the same pattern `frontier_discovery::
create_frontier_job` already uses for external candidates (it doesn't auto-score
either; it creates a job for a reviewer to assess). This keeps M47 honest about
what's mechanical (pairing, bounding, packet emission) versus what needs judgment
(the actual score).

- **Dependency:** M40 (proves the export/import file-contract pattern works before
  reusing it here); `governance/BUILD_WORTHINESS_STANDARD.md` (the scoring method
  the emitted packets ask a reviewer to apply)
- **Preferred capability:** Combinatorial/graph analysis, Rust systems review
- **Evidence:** committed export code (Aaroneous) and pair-generation/packet-
  emission code (devtools) with passing gates on both sides; at least one real
  composition packet actually scored and either admitted or archived through
  `BUILD_WORTHINESS_STANDARD.md`, not just plumbing with nothing run through it.

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
- **2026-09-20 — Verification scales with blast radius, not a standing authority
  grant.** Discussed writing a document that would let agents self-approve merges
  with no human review, on the reasoning that the owner cannot personally review
  Rust. That draft was rejected (flagged by the session's own safety tooling as
  instruction-poisoning-shaped: a file, readable by future agent sessions, whose
  content is "you may bypass review" is a risk independent of whether the request
  originated from the owner in good faith). Replaced with
  `governance/CHANGE_VERIFICATION_TIERS.md`, which scales *evidence required*
  by blast radius instead of granting standing authority — what to do with a
  verified change stays a live, per-session decision rather than something a file
  pre-authorizes.
- **2026-09-20 — Closed a naming gap: M47 was missing.** The internal-discovery
  idea (composing existing capabilities to find hidden value, distinct from M41's
  external ecosystem discovery) was discussed and agreed on in planning, then not
  actually written up as its own milestone — M46 became packet meta-tagging
  instead and the internal-discovery idea was quietly dropped rather than
  recorded. Added as M47 once the gap was noticed, rather than left missing.
- **2026-09-20 — Delivered the "judgement, document-driven" rubric.** Requested
  earlier in planning and not delivered at the time. `governance/
  BUILD_WORTHINESS_STANDARD.md` generalizes `FRONTIER_FILTRATION_STANDARD_V1.md`'s
  scoring method to internally-generated ideas (M47's primary consumer) instead of
  external crate candidates, using the same quadratic-scoring math so the two
  standards stay comparable.

## Sequencing

M40 and M41 are independent and can both enter `Ready` immediately. M43 depends on
M41's packet supply. M42 no longer gates M44 (corrected 2026-09-20 — M42 is a
read-only report over existing `cross_verifier.rs`/`consensus_engine.rs`, not a
prerequisite contract); M42 and M44 can proceed independently and in parallel.
M45 depends on M44 (and benefits from
M41's packet supply as a second input source) and should not be claimed before a
proofed local draft actually exists to route through it. M46 depends on both M41
and M45 and is deliberately last — meta-tagging and multi-variant exploration add
real complexity and are only worth it once there is enough packet volume and one
stable acceptance funnel to organize. M47 depends on M40 (proves the file-contract
pattern) and on `governance/BUILD_WORTHINESS_STANDARD.md` existing; it can proceed
in parallel with M41/M44/M45/M46 rather than waiting on them, since it feeds the
same packet supply M41 does rather than depending on it. No milestone in this plan
authorizes paid cloud dispatch or local write/claim authority — those remain
explicit future decisions, not defaults unlocked by completing M40–M47.
