# Patterns mined from other Rust agent runtimes (proposal)

```text
Author: Claude Sonnet 5.5, Claude Code session on host D:\Aaroneous
Date: 2026-10-07
Status: proposal. Nothing here changes behaviour. Every item names a first step small enough to be its own PR, and
        the seat or person who has to decide.
Sources read: the documentation pages listed at the end (docs.rs, a forum post, a write-up). NOT VERIFIED: any
        claim about what the code does beyond what those pages say; no source repository was cloned or built.
```

## Why this document exists

The question was whether other agent-coordination, gated, Rust-based systems exist that Aaroneous could lean on.
They do. None is a drop-in: Foremerge and ZeroClaw use tokio and bundled SQLite, so under `AGENTS.md` section 4
(dependency admission, "Extract pattern") the answer for each is to clean-room the idea behind a workspace trait,
not to add the crate. Scores for `alloc`, `ambient`, `abi` and `safety` have not been computed; if any of them is
ever proposed as a dependency, that scoring comes first.

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
`check_coordination_evidence.py` that reports, as a warning with a baseline that may only shrink, any two
Ready/Claimed rows whose globs overlap where at least one side is `replace`. Test: two rows, one `replace`, same
path, reported; both `extend`, not reported.
**Decides:** Codex (queue serializer) for the cell grammar; Claude can write the check.

## 2. "Unverified" is a recorded state, not a refusal and not a pass (Foremerge)

**Source.** Work is accepted through a verification gate that runs a named check against an exact candidate; an
acceptance without a passing check is stored as unverified with its reason, so the audit trail stays honest.

**Our gap.** The two open #99 deferrals ("evidence-free verdicts need a validated `commands` field") are this
decision, stuck waiting for a yes or no. Today an accept is counted whether or not it cites a command and exit code;
merge-gate box 5 checks that separately and fails the PR.

**First step.** An optional `evidence` list on the attestation (`command`, `exit`, `log`), validated when present.
Coverage rows gain `verified_accepts` and `unverified_accepts` next to `accept_seats`; nothing about `covered`
changes. Advisory first, which is Foremerge's own `checks policy advisory` mode and our own ratchet shape.
**Decides:** owner, once the counts exist: should an unverified accept ever count toward the two-seat rule.

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

**Our gap.** Attestations bind the head and the merge base, so any advance of `main` invalidates every accept
(this week: the whole stack re-attested after #99, and #62/#69 need it again after #86/#88). `merge_gate.run_checks`
runs in a worktree of the head but stores only command and exit code.
**First step, no semantic change:** record `git rev-parse <head>^{tree}` and `git patch-id --stable` of the
`base...head` diff next to every check result and, optionally, in the attestation. That produces the data for a later
decision on whether an unchanged patch-id may carry an accept across a base move.
**Decides:** owner. Carrying coverage across a base move is a policy change with a real risk (same diff, different
surroundings); this step only collects the evidence.

## 6. Already matches, kept for the record

- **Hash-chained append-only history, expected-sequence writes** (Foremerge, A3S Flow): the steward log is
  hash-chained with a tip sidecar and `--expect-tip`; ticks append under a lock. No change.
- **Idempotency key from identity** (A3S Flow derives retry jitter and step identity from run, step and attempt):
  our `[done:KIND:PR:FULL_HEAD]` receipt is the same idea. `steward_loop.plan_next` has no jitter; with one loop per
  log this does not matter, so no change until a second loop exists.
- **Suspend until an external signal** (A3S `CreateHook`): owner gates already work this way, as a PR waiting on a
  chat decision. Nothing to build; the lesson is only that the wait should cost no worker, which it does not.

## Suggested order

1. Pattern 1 (the one that has cost the most human time), as a claim PR plus the advisory check.
2. Pattern 3 and 2 together once the owner answers the two decisions; both are data first, enforcement later.
3. Patterns 4 and 5 as small recording steps whenever convenient.

## Sources

- Foremerge: https://users.rust-lang.org/t/foremerge-a-git-like-coordination-protocol-for-parallel-coding-agents-one-binary-sqlite-deterministic-conflict-rules/142084 and https://pyshine.com/Foremerge-Catch-Intent-Conflicts-Before-Code-Conflicts/
- ZeroClaw runtime modules: https://docs.rs/zeroclaw-runtime/latest/zeroclaw_runtime/
- A3S Flow: https://docs.rs/crate/a3s-flow/latest
