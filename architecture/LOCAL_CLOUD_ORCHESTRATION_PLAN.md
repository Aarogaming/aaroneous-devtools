# Local/cloud orchestration plan

**Status:** Dormant/Roadmapped (per `governance/EVIDENCE_STANDARD.md`) — no code has
been written against this plan yet. This document records scope, sequencing, and
governance boundaries for milestones M40–M53 in `governance/MULTITHREAD_PROGRAM.md`.

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

**Scope boundary found, 2026-09-20 (documented, not a defect):** an audit of job
sources found this repo actually runs **two structurally separate systems**, not
one — (A) `worker-jobs/*.toml` → the local worker → `cross_verifier.rs`/
`consensus_engine.rs`, which this report reads; and (B) `multithreader.rs`/
`intake_generator.rs` → a `review-NNNNNN.json` inbox → `COORDINATION_QUEUE.md`'s
primary-lane rows directly, which never touches `worker-jobs/` or produces a
`ConsensusRecord` at all. This report can only ever surface System A's work as
"ready for signoff" — a System B-sourced queue row can be genuinely ready and this
report will never say so, because there is no consensus ledger entry for it to
find. This is not a bug to fix in M42; it's a real architectural boundary worth
knowing rather than assuming the report is complete. Closing it (if ever wanted)
would mean either routing System B's candidates through `cross_verify` too, or
building a second, System-B-shaped readiness signal — neither is in scope here.

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

### M45 — Unify draft output through Aaroneous's own cratification pipeline (corrected scope, 2026-09-20)

**Corrected after reading the actual source**: `crates/cratify` is not just a set
of library re-exports needing new wiring — it already has a complete, working
standalone CLI (`crates/cratify/src/main.rs`, 142 lines): `cratify audit
[targets...]` (delegates to `ast_auditor::run_workspace_audit`), `cratify scaffold
<name>` / `cratify generate <name>` (delegates to
`transpiler::create_crate_scaffold_at`), `cratify harvest <src>` (delegates to
`adaptation_engine::harvest`). All four are real, invocable today via `cargo run
-p cratify -- <subcommand>`.

**This means M45 may need little to no new devtools code, once M44 lands.** M44's
`dogfood::verify_draft(checkout, target_file, draft, output, verify_command)`
already accepts an arbitrary caller-specified `verify_command` — M45's entire
scope may reduce to: after a draft passes its ordinary pre-proof check (`cargo
check`/`test`), run a *second* `verify_draft` pass against the same checkout with
`verify_command = ["run", "-p", "cratify", "--", "audit", "<the target_file's
containing directory>"]`, reusing M44's exact machinery rather than building new
integration code. If that reuse proves too narrow once M44's real shape is known
(e.g. `harvest`/`scaffold` need different invocation shapes than a single
"run a command, capture pass/fail" check), the fallback is a thin convenience
wrapper in `dogfood.rs` presetting the cratify command — still built on the same
pattern, not new infrastructure.

**This milestone should not be dispatched to a background agent until M44 has
actually landed and its exact `verify_draft` signature is known** — dispatching
it concurrently risks the M45 work guessing at or duplicating a function M44 is
simultaneously defining in the same file. Hold until M44 reports back.

- **Dependency:** M44 (needs `verify_draft` to exist and its exact signature known
  before reuse can be confirmed, not just "proofed drafts to feed in") and M41
  (frontier-discovered candidates are the other input source this unifies with)
- **Preferred capability:** Component architecture / compiler-adjacent tooling
- **Evidence:** at least one local-worker draft and one frontier-discovered crate
  both run through `cratify audit` (or the fuller harvest/scaffold path if reuse
  alone proves insufficient) with no source-specific special-casing in the
  acceptance criteria.

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

**Verified constraint, 2026-09-20:** `foundation/crates/foundation_cli/src/
local_worker.rs`'s `Job` struct (~line 27-34) already carries
`#[serde(deny_unknown_fields)]`. This means the three new fields below cannot be
added silently — every field must be `#[serde(default)]` (so existing packets
without them keep parsing unchanged) and the struct itself must be edited in this
exact file, not inferred from the `.toml` schema alone. This is real, necessary
code, not just a schema-doc update.

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

**Also hold this milestone until M44 lands**, for the same reason as M45: the
"comparison step before full local pre-proofing" only has something concrete to
run before, once M44's pre-proof mechanism exists and its trigger point
(`local_worker.rs`'s dispatch loop, the same file this milestone edits) is
settled. Dispatching M46 concurrently with M44 risks two agents editing
`local_worker.rs`'s `Job` struct and dispatch loop at the same time.

- **Dependency:** M41 (needs a steady packet supply worth branching into variants),
  M44 (the dispatch-loop hook this milestone's comparison step runs before must
  exist first, and both edit the same `Job` struct/file), and M45 (needs one
  acceptance funnel for the winning variant to enter)
- **Preferred capability:** Data/schema design, workflow tooling
- **Evidence:** updated `contracts/` schema and fixtures for the new fields; at
  least one real `exploration_group` with multiple drafts, a documented selection,
  and archived rejected siblings with recorded reasons.

**Gap found and closed, 2026-09-20:** an audit of "how many job sources exist and
are we accounting for all of them" found that once the `Job` struct changes above
landed (as part of M46's own implementation), **no producer actually populated
the three new fields** — the consumer side (grouping, comparison summaries) was
real and tested, but nothing fed it. Checked all three current producers and
resolved each:

- `frontier_discovery::create_frontier_job` — **wired.** Every candidate found for
  one query now gets `goal = <query>`, `exploration_group = slugify(<query>)`,
  `perspective = <candidate crate name>`. This was a natural, low-cost fit: one
  query producing several crate candidates already *is* an exploration group (the
  same goal — "find something useful for X" — approached via several different
  candidates), and it needed three added fields on an existing struct, not a new
  producer. A single-candidate query still gets tagged; M46's own grouping step
  already drops groups under two members, so unconditional tagging is simpler than
  special-casing.
- `capability_composition::create_composition_job` — **deliberately left
  untagged.** Each candidate pair is already a unique, one-shot proposal, not one
  of several attempts at a shared goal; forcing an `exploration_group` onto it
  would misrepresent what the packets are. If a real use case for grouping
  composition candidates emerges later (e.g. several pairings all targeting one
  named capability gap), revisit then — not speculatively now.
- `job_converter::recurring_catalog()` — **deliberately left untagged.** Each
  catalog item (`AUDIT-01`, `PREP-04`, etc.) is its own distinct, named audit
  concern; they aren't different attempts at solving the same thing. No fit.

**Real remaining gap, not yet closed — M48 below:** none of the three producers
above emit `code-draft` mode packets, which is the actual case M46 was designed
for (the "recreate" stage of the discovery/research/filter/translate/recreate/
review/repeat loop — several different implementation attempts at one goal). There
is currently no producer anywhere that generates more than one drafting attempt at
anything. M46's mechanism is proven end-to-end for analysis-mode candidates
(frontier discovery); it has never been exercised for its originally-intended
code-draft use case, because nothing creates the multi-attempt input it needs.

### M48 — Multi-perspective draft-seeding producer

The producer M46 actually needs and doesn't have: given one `code-draft` goal, emit
several packets attempting it from different angles, sharing one
`exploration_group`, so M46's comparison-summary and M44's pre-proofing have
something real to compare once results come back.

**Design:**

- A new function (e.g. `foundation/crates/foundation_cli/src/exploration_seed.rs`,
  or a small addition to an existing producer-shaped module — implementer's call,
  document the reasoning) taking an explicit base `instruction: &str`, `evidence:
  Vec<String>` (same shape `Job.evidence` already takes), an `output_dir: &Path`,
  and a `perspectives: Vec<String>` (default to a small fixed starter vocabulary —
  `correctness-first`, `performance-first`, `minimal-diff` — the same three named
  as examples when M46 was first scoped, since real usage hasn't yet shown a
  reason to deviate from them).
- For each perspective, emit one `worker-jobs/*.toml` packet: `mode = "code-draft"`,
  a shared generated `exploration_group` (e.g. a short hash or slug of the base
  instruction plus a timestamp, to disambiguate repeated seedings of a similar
  goal), `goal` set to a short caller-supplied label (not the full instruction —
  keep it a label, matching `frontier_discovery`'s use of the query as `goal`), and
  an `instruction` built from the base instruction plus one fixed, perspective-
  specific directive appended (e.g. correctness-first: "Optimize primarily for
  correctness and edge-case coverage over performance or diff size."; performance-
  first: "Optimize primarily for runtime performance, even if the diff is larger or
  less minimal."; minimal-diff: "Optimize primarily for the smallest diff that
  solves the problem.").
- **No new dispatch, pre-proofing, or scoring logic needed.** Each emitted packet
  is an ordinary `code-draft` job; M44's existing dispatch-loop hook and
  pre-proofing already handle it once picked up, and M46's existing grouping/
  comparison-summary already handles multiple drafts sharing an
  `exploration_group` once results land. This milestone is purely packet
  generation — everything downstream of "packet exists in `worker-jobs/`" is
  already built.
- A CLI subcommand (e.g. `Command::SeedExploration { instruction_file, evidence,
  goal, perspectives, output_dir }`, reading the instruction from an explicit file
  per this crate's established convention for larger text input) so a human or a
  cloud session can trigger a seeding round explicitly — this stays a deliberate,
  invoked action, not something that fires on its own; unlike M41/M47's discovery
  loops, there's no natural automatic trigger for "we should try N different
  approaches to X" the way there is for "search the ecosystem on a schedule."

- **Dependency:** M46 (consumer side must exist first — this milestone is pure
  input for it), M44 (drafts this produces flow through pre-proofing unchanged)
- **Preferred capability:** Rust systems review, workflow tooling
- **Evidence:** committed producer code with passing tests; at least one real
  seeded `exploration_group` (3 perspectives on one real goal) carried all the way
  through dispatch, pre-proofing, and M46's comparison-summary artifact — closing
  the loop M46's own evidence bar already asked for but nothing could satisfy
  until this milestone exists.

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

### M49 — Cratification intake: harvest a candidate into a real code-draft, not just an audit

**Corrected framing, 2026-09-20**, after reading `crates/cratify/src/main.rs` and
`crates/adaptation_engine/src/source_extraction.rs` directly rather than assuming
`harvest`/`scaffold`/`audit` already chain: **they don't, and can't as written.**
`harvest(src, config) -> Vec<CrateSpec>` is read-only analysis — it returns
`CrateSpec { name, source_path, code_info: CodeInfo { functions, structs, enums }
}` describing what exists, nothing else. `scaffold`/`generate` takes only a crate
`name` and creates an empty boilerplate skeleton — it has no parameter for
`harvest`'s output and does nothing with it. Neither writes ported logic anywhere.
There is currently **no mechanism, anywhere in either repo, that transforms
harvested source into working code** — and there shouldn't be a naive one: porting
external logic into this framework's zero-allocation, zero-ambient-authority
idioms is exactly the kind of judgment call `contracts/
FRONTIER_FILTRATION_STANDARD_V1.md` itself says needs "the local agent extraction
loop," not a mechanical transform. That standard already names the right shape
(Stage `CRATIFY_KERNEL_EXTRACTION`: "Run local agent extraction loop to isolate
kernel needle... rebase into `crates/<target>/`") — M49 is building the plumbing
that shape currently has none of.

**Real design, reusing what M44/M45 already proved rather than inventing new
mechanism:** treat a harvested candidate exactly like any other `code-draft` job —
`harvest`'s output becomes the *evidence* for a drafting job, not an automatic
transform.

- **Aaroneous side:** `CrateSpec`/`CodeInfo` (and the `inspect::{FunctionInfo,
  StructInfo, EnumInfo}` types they wrap) currently derive only `Debug, Clone` —
  no `Serialize`. Add it (same fix M40 needed for `ConcurrenceSnapshot`), and add
  a `cratify harvest --output <path>` flag exporting the `Vec<CrateSpec>` as JSON
  via the same atomic-write pattern M40/M47 established, instead of only printing
  a human-readable summary to stdout as it does today.
- **Devtools side:** a new producer (mirroring `frontier_discovery::
  create_frontier_job`'s and `capability_composition::create_composition_job`'s
  shape exactly) that reads the exported `CrateSpec` JSON and emits **one
  `code-draft` `worker-jobs/*.toml` packet per harvested crate spec**: `evidence`
  points at the original source path plus the CrateSpec export; `instruction`
  summarizes the discovered functions/structs/enums and asks for a compliant port
  following `CONTRIBUTING.md`'s rules (no ambient authority, no heap on hot paths,
  `#[repr(C)]` where relevant); `mode = "code-draft"`. Once M48 exists, this
  producer should also set `goal`/`exploration_group` so multiple harvested
  candidates — or multiple perspectives on porting one candidate — are comparable
  the same way frontier-discovery candidates already are.
- **Everything downstream is already built:** M44 dispatches the code-draft job
  and pre-proofs the result against an explicit checkout; M45's `cratify_audit`
  checks the proofed draft for compliance the same way it already does for any
  other draft. M49 is genuinely just the missing *front door* — turning "here's
  some external code" into "here's a properly-evidenced drafting job" — not a new
  pipeline stage.
- `scaffold`/`generate` stays a separate, manually-invoked convenience for
  starting a brand-new empty crate; it has no natural role in this flow since
  there's no code yet to scaffold *around* until a draft exists.

- **Dependency:** M44 (dispatch/pre-proof), M45 (compliance audit) — both already
  built, this milestone only adds a new evidence source feeding them
- **Preferred capability:** Rust systems review, cross-repo file-contract design
- **Evidence:** committed export code (Aaroneous) and packet-emission code
  (devtools); at least one real harvested candidate carried through drafting,
  pre-proofing, and `cratify_audit` — proving the front door actually leads
  somewhere, not just that it opens.

### M50 — Clean revert for already-promoted changes

**Real, unbuilt gap**, distinct from everything the circuit breaker/high-risk
allowlist already do. Those prevent a bad change from merging, or halt the
pipeline after a later-discovered problem — neither one **undoes a change that
already reached `main`.** Right now, rejecting something already delivered means
someone manually running `git revert` themselves; there's no tool, no recorded
trail, and no consistency with how every other action in this pipeline works
(isolated branch, explicit review, recorded evidence).

**Verified starting point:** `agentic_registrar::promote` (`foundation/crates/
foundation_cli/src/agentic_registrar.rs`, ~line 1076 `pub fn promote`) already
does exactly one `git add` + one `git commit` per promotion (~line 2081-2107)
before pushing the isolated `local-agent/{task_id}` branch — so every
`promote`-sourced change is, by construction, already a single clean commit.
Combined with `COORDINATION_QUEUE.md`'s existing convention of recording the
squash-merge commit hash in each Done row (e.g. C37/C38's "squash commit
`cedfadc`" / "`46ea26d`"), a revert almost always has exactly one commit hash to
target, already on record — this milestone doesn't need to solve "which commit,"
just "act on it consistently and safely."

**Design:** a new command in `agentic_registrar.rs`, e.g. `revert_promotion
(config_path: &Path, commit: &str, reason: &str)`, mirroring `promote`'s own
pattern rather than inventing a new one:

1. Validate the target commit exists and is actually reachable from `main` (fail
   loudly if not — reverting a commit that was never merged is a user error worth
   catching, not silently no-op-ing).
2. `git revert --no-edit <commit>` (or `-m 1 <commit>` if it's a merge commit, not
   a squash — detect which via `git rev-list --parents -n 1 <commit>` and branch
   on parent count) on a **new isolated branch**, e.g. `local-agent/revert-
   {commit-short-sha}`, not directly onto `main` — a revert can itself have
   unintended interactions with commits made since (a file another change also
   touched), so it goes through the same review gate as any other change, just
   starting from a safer default (undoing something) rather than a riskier one
   (adding something).
3. Push the branch, print the PR-creation URL (same as `promote` already does —
   `agentic_registrar` has no GitHub credential of its own by design, per
   `LOCAL_MODEL_AUTONOMY_POLICY.md`'s existing rationale; a cloud agent with real
   access opens the actual PR).
4. Record the action: append a row to `COORDINATION_QUEUE.md` (or a dedicated
   `governance/REVERT_LOG.md`, implementer's call — a dedicated log may read more
   clearly than mixing revert history into the primary-lane queue) with the
   reverted commit, the stated reason, and the new branch — so "why was this
   undone" survives as evidence, not just a diff.

**Explicitly out of scope for M50:** deciding *when* a revert is warranted. That
stays a human or capable-agent judgment call, the same as every other Tier-2-
adjacent decision this plan has deliberately kept out of mechanical hands. M50
makes "no, undo that" fast, safe, and evidenced once the decision is made — it
does not make the decision.

- **Dependency:** none (builds on `agentic_registrar.rs`'s existing `promote`
  pattern, doesn't require any other M40-M49 milestone)
- **Preferred capability:** Rust systems review, git tooling
- **Evidence:** committed `revert_promotion` command with passing tests
  (successful revert-and-push against a real bare-repo fixture, matching
  `promote`'s own existing test pattern; detects and rejects an unreachable
  commit; correctly branches merge-commit vs. squash-commit revert syntax); a
  real revert exercised end-to-end against a disposable test commit.

### M51 — A running closed loop, not just documented triggers (not yet designed)

`architecture/DISCOVERY_REVIEW_LOOP.md` documents what *should* trigger each stage
from the previous one's output, but nothing currently executes those triggers —
`frontier-discover-serve` runs on its own schedule, and nothing automatically
starts drafting, pre-proofing, or cratification when it produces a candidate. This
milestone is real but genuinely not yet designed to the depth M40-M50 are; it
needs its own grounding pass (reading whatever scheduling/orchestration primitives
exist by the time M40-M50 have landed and settled, since M49's new producer and
M48's seeding producer both change what "the next stage" even consumes) rather
than a premature design written before its dependencies exist. Recorded here so
the gap isn't lost, not scoped further today.

- **Dependency:** M48, M49 (the loop can't be wired until its stages exist)
- **Preferred capability:** unassigned — needs grounding work first, not
  implementation

### M52 — A curated offer/accept UX (deliberately deferred)

The "quest reward" framing from planning discussion: presenting a discovered or
drafted candidate to a person as a friendly accept/reject choice, rather than a
queue row or JSON report for a technical reviewer. Deliberately **not** designed
further right now — per this plan's own recommendation, building a friendly
accept/reject layer on top of a pipeline that can't yet cleanly undo a rejection
(M50) puts the UX before the safety mechanism it depends on. Revisit once M50
exists. `gui/`'s existing read-only dashboard (per `FOUNDATION_BOUNDARY.md`:
"full write/promotion integration remains gated on owner sign-off") is the
plausible eventual home for this, not a new surface.

- **Dependency:** M50 (a reject action needs something real to do)
- **Preferred capability:** unassigned — deferred, not scoped

### M53 — Selectable compliance profiles for cross-repo verification

**Verified, 2026-09-20:** `ast_auditor::run_workspace_audit<I, P>(targets: I) where
P: AsRef<Path>` (`crates/ast_auditor/src/lib.rs:174`) takes arbitrary paths with
no workspace-boundary check — it walks and audits whatever path it's given,
regardless of which Cargo workspace that path belongs to. Combined with
`dogfood::verify_draft`'s existing checkout-agnostic subprocess design (M44), this
means Aaroneous's own compliance tooling can already, mechanically, check
devtools' code — `cargo run --manifest-path D:\Aaroneous\Cargo.toml -p cratify --
audit <absolute-path-into-devtools-foundation>` works today, via the same
subprocess pattern `dogfood.rs` already uses, with zero Cargo dependency from
devtools onto Aaroneous (a subprocess call is not a link — `FOUNDATION_BOUNDARY.md`'s
hard rule stays intact).

**Why this needs to be a selectable profile, not a default:** Aaroneous's
compliance rules are *deliberately stricter* than devtools' own.
`governance/OPERATING_MODEL.md` states devtools follows "the lighter devtools
standards... heap allocation and ordinary service I/O are welcome here." Running
Aaroneous's `ast_auditor` against devtools code as-is would flag devtools' normal,
correct-for-devtools patterns (heap allocation, async I/O) as violations — not
because the code is wrong, but because it would be checked against the wrong
ruleset for that target. `dogfood::cratify_audit` (M45) is already exactly one
hardcoded preset of the fully-generic `verify_draft` — this milestone generalizes
that pattern into an explicit, named, extensible set of profiles instead of one-off
wrapper functions, so "which ruleset applies" is always a stated choice, never an
assumption.

**Design:**

- A small `ComplianceProfile` type (in `dogfood.rs` alongside `cratify_audit`, or
  a small sibling module — implementer's call, following this plan's established
  precedent of leaving genuinely either-way choices to whoever builds it) with at
  least: `AaroneousStrict` (resolves to today's `cratify_audit` behavior —
  `cargo run -p cratify -- audit <target>`), `DevtoolsOperatingModel` (resolves to
  devtools' own Foundation CI check — `cargo clippy --manifest-path
  foundation/Cargo.toml -- -D warnings`, matching `.github/workflows/
  foundation-ci.yml`'s actual lint step), and `Custom(Vec<String>)` (falls through
  to `verify_draft`'s existing fully-generic behavior unchanged, for anything not
  worth a named preset).
- A resolver function turning a `ComplianceProfile` + `checkout` into the concrete
  `verify_command` `verify_draft` already accepts — this is genuinely small, since
  `verify_draft` itself needs no changes at all; this milestone only adds a named,
  documented layer in front of a parameter it already takes.
- Extend (or add a sibling to) the existing `VerifyDraft`/`CratifyAuditDraft` CLI
  subcommands with a `--profile <aaroneous-strict|devtools-operating-model>` flag,
  so the choice is explicit on every invocation rather than implied by which
  wrapper function happened to be called.
- A short governance note (in this file or `governance/REVIEW_CONDITIONS.md`)
  stating plainly that `AaroneousStrict` and `DevtoolsOperatingModel` are not
  interchangeable and naming which applies to which target — making the
  distinction a durable, written fact rather than something only stated once in a
  planning conversation.

**Explicitly in scope, explicitly not yet:** this milestone only makes "check
devtools' code with Aaroneous's own tooling" a real, correct, selectable option —
it does not wire it into any automatic trigger. Whether and when devtools code
actually gets checked this way (routinely, on-demand, as part of M51's eventual
closed loop) is a separate decision, deliberately left open here the same way
M51/M52 are.

- **Dependency:** M44 (`verify_draft`), M45 (`cratify_audit` is the pattern being
  generalized) — both already built, this milestone only adds a named selector
  layer in front of them
- **Preferred capability:** Rust systems review, cross-repo tooling design
- **Evidence:** committed `ComplianceProfile` type and resolver with passing
  tests; a real run of `DevtoolsOperatingModel` against actual devtools code and
  `AaroneousStrict` against actual Aaroneous code from the same tool, proving both
  presets produce correct, non-cross-contaminated results.

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
- **2026-09-20 — Audited "how many job sources exist" and closed what could be
  closed immediately.** Found two structurally separate systems (worker-jobs/
  local-worker vs. multithreader/intake/queue), a real M42 scope boundary from
  that split (documented in M42's section), two stale `worker-jobs/*.toml`
  packets for already-`Done` queue rows (`c33.toml`/`c34.toml`, deleted), and
  M46's actual gap — its consumer side worked but no producer populated the new
  fields. Wired `frontier_discovery::create_frontier_job` to populate them (a
  real, low-cost fit already present in its existing shape); added M48 as the
  still-open gap — a producer for M46's originally-intended `code-draft`
  multi-attempt use case, which nothing generates yet.
- **2026-09-20 — Answered "what's left for reverse-engineering and full
  automation" honestly, then wrote up what could actually be designed today.**
  Corrected an initial assumption that `cratify`'s `harvest`/`scaffold`/`audit`
  already chain — they don't; `harvest` only analyzes, `scaffold` only creates an
  empty skeleton unrelated to any harvest output, and no mechanism ports
  harvested logic anywhere. M49 designs the real fix: harvested candidates become
  evidence for an ordinary `code-draft` job, reusing M44/M45 rather than
  inventing a code-transform mechanism (porting is a judgment call, not
  arithmetic — consistent with this plan's standing position on BUILD_WORTHINESS
  scoring). M50 designs the "clean revert of something already delivered"
  mechanism this plan never had, built on `agentic_registrar::promote`'s existing
  single-commit-per-promotion pattern. M51 (running the closed loop for real) and
  M52 (a friendly accept/reject UX) are recorded but deliberately left
  unscoped/deferred — M51 because its dependencies don't exist yet to design
  against, M52 because building a reject-UX before M50's revert mechanism exists
  would put the experience ahead of the safety it depends on.
- **2026-09-20 — Scoped a self-referential capability deliberately kept internal
  for now.** Planning discussion covered contributing fixes to external
  open-source projects and even Aaroneous auditing devtools' own code with its
  own compliance tooling. Decided to stay internal-only for now (get Aaroneous's
  own Rust compliance solid first) rather than build the outbound-contribution
  track immediately. Verified the self-audit idea is mechanically real today
  (`ast_auditor::run_workspace_audit` takes arbitrary paths, no workspace-boundary
  check) and scoped the one piece worth building now: M53, a named, selectable
  compliance profile, since Aaroneous's and devtools' rules are legitimately
  different (`OPERATING_MODEL.md`'s "lighter devtools standards" vs. Aaroneous's
  strict zero-allocation rules) and a self-audit tool needs to pick the right one
  explicitly, not assume Aaroneous's rules apply everywhere.

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
same packet supply M41 does rather than depending on it. M48 depends on M46
(consumer must exist first) and M44 (its output flows through pre-proofing
unchanged); unlike the other milestones it has no natural automatic trigger, so it
stays an explicitly-invoked producer rather than something a serve loop calls on
its own. M49 depends only on M44/M45 (both already built) and can proceed now,
in parallel with anything else. M50 depends on nothing else in this plan and can
proceed immediately — it only builds on `agentic_registrar.rs`'s existing
`promote` pattern. M51 depends on M48 and M49 (needs its stages to exist before
the loop connecting them can be designed, let alone built) and is intentionally
left ungrounded until then. M52 depends on M50 (a reject action needs a real
mechanism behind it) and is deliberately deferred. M53 depends on M44/M45 (both
already built) and can proceed immediately, entirely independent of everything
else in this plan — it's the smallest milestone here, a named layer in front of
a parameter `verify_draft` already accepts. No milestone in this plan
authorizes paid cloud dispatch or local write/claim authority — those remain
explicit future decisions, not defaults unlocked by completing M40–M53.
