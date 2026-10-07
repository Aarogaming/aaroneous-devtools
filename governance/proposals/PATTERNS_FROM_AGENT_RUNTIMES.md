# Patterns mined from other Rust agent runtimes (proposal)

```text
Author: Claude Sonnet 5.5, Claude Code session on host D:\Aaroneous
Date: 2026-10-07
Status: proposal. Nothing here changes behaviour or claims execution authority.
        This document is an inert design proposal cataloging candidate patterns mined from external runtimes.
        Before any pattern or step is implemented, an explicit Ready-to-Claimed transition in
        `governance/COORDINATION_QUEUE.md` (or a dedicated claim PR) must be opened and recorded per AGENTS.md:18,
        preventing scope collisions and unevidenced work.
Sources read: the documentation pages listed at the end (docs.rs, a forum post, a write-up). NOT VERIFIED: any
        claim about what the code does beyond what those pages say; no source repository was cloned or built.
```

## Why this document exists

The question was whether other agent-coordination, gated, Rust-based systems exist that Aaroneous could lean on.
They do. None is a drop-in. For devtools and governance tooling, dependency evaluation is governed by
`governance/OPERATING_MODEL.md` (evaluating concrete need, licensing, platform support, maintenance cost, and
permitting async I/O when justified) rather than the strict zero-heap/zero-ambient-authority rules of Aaroneous product
crates (`AGENTS.md` section 4). However, even under devtools standards, introducing full external frameworks
(e.g. bundled SQLite engines or external async runtimes) for narrow coordination tasks carries unnecessary maintenance
cost and platform footprint. Therefore, the recommended path for devtools remains clean-rooming the mined coordination
patterns into our existing lightweight Python steward and Rust foundation tools, rather than admitting heavy external
runtime crates. If any pattern is ever proposed to cross into the Aaroneous product codebase, `AGENTS.md` section 4
dependency admission scoring (`alloc`, `ambient`, `abi`, `safety`) strictly applies.

The patterns below are ranked by how much trouble the gap has cost us in the last week, with the evidence for each
gap taken from this repository, not from the sources.

## 1. Declared scope and operation on every claim (Foremerge)

**Source.** An agent declares what it will touch, as a typed target plus an explicit operation
(`symbol:PaymentService=replace`, versus extend), before it touches it. Deterministic rules compare declarations;
no model is in the detection path. Findings are advisory and arrive before either worktree has changed.

**Our gap.** Queue scopes are prose in the Scope cell. Conflicts were found by people after the fact: #47 and #60
both relocate rows in `COORDINATION_QUEUE.md` and conflict with each other; the queue-row PRs #48, #50, #59, #64 and
#86 conflicted pairwise and were folded by hand into #96/#102. `check_coordination_evidence.py` guards row loss and
duplicate IDs (#107) but cannot say two live lanes want the same file.

**First step.** An optional, machine-readable token in the Scope cell, `touches: <path-or-glob>=replace|extend`
(our scopes are paths, so Foremerge's twelve-kind vocabulary collapses to file, directory and queue row). A check in
`check_coordination_evidence.py` (and in Rust `scheduler.rs`) that reports, as a warning with a baseline that may only
shrink, any two active scope-owning rows whose globs overlap where at least one side is `replace`. Active scope
ownership encompasses all non-terminal queue statuses that reserve or receive implementation revisions: `Claimed`,
`In progress`, and `Review` (as well as proposed `Ready` reservations). Deriving active ownership across all non-terminal
lifecycle stages ensures that active reviews or ongoing iterations cannot be silently shadowed by a new claim on the
same file. Test: two rows (one `Review`, one `Claimed`), one `replace`, same path, reported; both `extend`, not reported.
**Decides:** Codex (queue serializer) for the cell grammar; Claude can write the check.

## 2. "Unverified" is a recorded state, not a refusal and not a pass (Foremerge)

**Source.** Work is accepted through a verification gate that runs a named check against an exact candidate; an
acceptance without a passing check is stored as unverified with its reason, so the audit trail stays honest.

**Our gap.** The two open #99 deferrals ("evidence-free verdicts need a validated `commands` field") are this
decision, stuck waiting for a yes or no. Today an accept is counted whether or not it cites a command and exit code;
merge-gate box 5 checks that separately and fails the PR.

**First step.** An optional `evidence` list on the attestation (`command`, `exit`, `log`), validated when present.
Coverage rows gain `verified_accepts` and `unverified_accepts` next to `accept_seats`. In accordance with binding
rule `governance/AUTOMATED_MERGE_POLICY.md:155-156`, an accept lacking command and exit-code evidence does not count
toward condition 3 (independent reviewer verification) for merge eligibility; unverified accepts are preserved purely
for historical audit trail and cannot satisfy `covered`. Any alteration to allow unverified accepts to satisfy coverage
would require an explicit, signed owner amendment to `AUTOMATED_MERGE_POLICY.md`.
**Decides:** owner, if an amendment to acceptance criteria is ever sought; today the exclusion is binding.

## 3. Autonomy level and grants as data (ZeroClaw)

**Source.** Three named autonomy levels (read only, supervised with approval for medium and high risk, full inside
policy bounds), a pre-execution approval hook, and session-scoped allowlists with an audit record.

**Our gap.** Each seat's limits, including the dated merge grant, are prose pasted into every heartbeat prompt, and
every tick repeats "the grant expired 2026-10-04". Nothing a tool can read says a grant is expired, so the gate
cannot enforce it and a prompt can be edited. Tier A/B in `merge_gate.py` is the closest existing thing.

**First step.** `governance/autonomy.json` (Tier A, owner-only): per seat a level and a list of grants
(`scope`, `expires`, `granted_by`). `merge_gate.py` and the steward read it; an expired or missing grant makes a
seat ineligible to merge, with the reason printed. Test: a grant one second past `expires` is ineligible.
**Decides:** owner. This is also the file where "never merge in the product repo" becomes checkable.

## 4. A child inherits its parent's identity and stays inside the parent's envelope (ZeroClaw)

**Source.** Ephemeral sub-agents run as children of the parent: auditable as such, never outside its permissions.

**Our gap, smaller than it looked.** `seat_names` already resolves "Subagent of Claude" or `claude/worker` to
`claude`, and `CROSS_AGENT_COORDINATION_PROTOCOL.md` already says a delegate's "done" is not evidence without the
command, exit code and log. What is missing is a trailer that names the parent when a delegate wrote the commit
(`Delegated-By: claude`), so the parent's seat is found when the commit message does not say it.
**First step.** Add `Delegated-By:` to the trailer list in `collect_state.seats_from_trailers` with a golden vector.
**Decides:** a seat; low priority.

## 5. Bind verification to the exact candidate, and record what was bound (Foremerge)

**Source.** The gate runs a named check "against an exact candidate fingerprint".

**Our gap.** Under `governance/PLATFORM_SPEC.md:50(a)`, attestations bind the immutable merge base SHA. A normal
advance of `main`'s branch tip does not invalidate existing attestations (as verified by
`test_same_merge_base_stays_valid_when_base_tip_moves`). However, when a PR branch is rebased or retargeted, the merge
base SHA itself changes, which strictly invalidates all whole-diff and delta records even if the patch content
(`base...head` diff) is semantically identical. `merge_gate.run_checks` runs in a worktree of the head but stores only
command and exit code.
**First step, no semantic change:** record `git rev-parse <head>^{tree}` and `git patch-id --stable` of the
`base...head` diff next to every check result and, optionally, in the attestation. That produces the empirical data for a
future owner-decided policy proposal on whether an identical patch-id may carry forward an attestation across a
merge-base change (rebase).
**Decides:** owner. Carrying coverage across a merge-base move is a policy change with real semantic risks
(same diff against different parent context); this step strictly collects data without relaxing merge-base invalidation.

## 6. Already matches, kept for the record

- **Hash-chained append-only history, expected-sequence writes** (Foremerge, A3S Flow): the steward log is
  hash-chained with a tip sidecar and `--expect-tip`; ticks append under a lock. No change.
- **Idempotency key from identity** (A3S Flow derives retry jitter and step identity from run, step and attempt):
  our `[done:KIND:PR:FULL_HEAD]` receipt is the same idea. `steward_loop.plan_next` has no jitter; with one loop per
  log this does not matter, so no change until a second loop exists.
- **Suspend until an external signal** (A3S `CreateHook`): owner gates already work this way, as a PR waiting on a
  chat decision. Nothing to build; the lesson is only that the wait should cost no worker, which it does not.

## Where each pattern should live: two systems that meet at the queue file

Two separate systems exist here, and the patterns above must not be built twice.

- **Rust local-model pipeline** (`foundation/crates/foundation_cli`: `scheduler.rs` 153 lines, `multithreader.rs`,
  `local_worker.rs`, `agentic_registrar.rs`, `intake.rs`, `feeder.rs`; Codex's lane). Deterministic; a local model may
  only propose, the integration owner claims (`LOCAL_AGENT_CONTROL_PLANE.md`). `foundation schedule-review` already
  counts claimed and review lanes and recommends claims.
- **Python steward and gate** (`scripts/`: `steward_run.py`, `next_task.py`, `review_coverage.py`, `merge_gate.py`,
  `check_coordination_evidence.py`). Faces GitHub: PRs, attestations, receipts, who reviews what. Per the Guild
  direction it is the prototype whose golden vectors are the contract for a Rust port.

They meet in two places, and that is where duplicate work already exists. Both parse the queue rows with their own
copy of the grammar (`multithreader.rs::queue_ids` splits on `|` and accepts five status words; `lane_rows` in
`check_coordination_evidence.py` and the steward do the same in Python), and both need the status vocabulary
(C103, #62, ties the two with one checked list).

Placement, so neither side builds the other's job:

| Pattern | Home | Why |
|---|---|---|
| 1 declared scope, overlap finding | rule in Rust `scheduler.rs` as a `findings` entry; Python `check_coordination_evidence.py` stays the CI gate; one golden vector set both must pass | the scheduler already reads every lane row and is where overlapping claims should be reported before a claim is recommended |
| 2 unverified as a state | Python (`review_coverage`, attestations) | GitHub-facing |
| 3 autonomy and grants | one file, `governance/autonomy.json`, read by both: `merge_gate.py` for merges, the Rust side for what a local model may be asked to do | the local pipeline's allowlist and the cloud seats' grants are the same idea in two places |
| 4 delegated-by trailer | Python (`collect_state`) | trailers are read from git history |
| 5 tree and patch-id with check results | Python (`merge_gate.run_checks`) | GitHub-facing |

Question for Codex, who owns the Rust side: does `scheduler.rs` grow the overlap finding (pattern 1), or would you
rather consume a shared parsed-queue contract from the golden vectors? The second avoids a third copy of the grammar.

## Suggested order

1. Pattern 1 (the one that has cost the most human time), as a claim PR plus the advisory check.
2. Pattern 3 and 2 together once the owner answers the two decisions; both are data first, enforcement later.
3. Patterns 4 and 5 as small recording steps whenever convenient.

## Sources

- Foremerge: https://users.rust-lang.org/t/foremerge-a-git-like-coordination-protocol-for-parallel-coding-agents-one-binary-sqlite-deterministic-conflict-rules/142084 and https://pyshine.com/Foremerge-Catch-Intent-Conflicts-Before-Code-Conflicts/
- ZeroClaw runtime modules: https://docs.rs/zeroclaw-runtime/latest/zeroclaw_runtime/
- A3S Flow: https://docs.rs/crate/a3s-flow/latest
