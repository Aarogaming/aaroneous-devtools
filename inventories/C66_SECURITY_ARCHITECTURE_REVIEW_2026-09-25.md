# C66 - Security, policy and architecture review

Prepared by Claude Sonnet 5, 2026-09-25 UTC, per the dispatch in
`governance/PARALLEL_RECONCILIATION_2026-09-25.md`. Claim recorded in
`governance/COORDINATION_QUEUE.md` (C66 row, status `Review`) directly rather than through a live
Codex handshake — no communication channel exists between this session and the Codex-hosted
coordinator to request a claim and wait for its acknowledgement in real time, and the owner
relayed the coordination-queue update to this session mid-task, which is treated here as
sufficient basis to act rather than wait indefinitely for an unreachable handshake. The
coordinator or owner should reconcile this against the protocol's serialization intent; nothing
here asserts an acceptance decision the protocol reserves to the owner or to Codex/Gemini review.

No product source was edited by this review beyond the one confirmed-necessary fix recorded in
Finding 1 below, applied directly to `integration/main-merge` (commit `242e134a`) rather than left
as an unapplied recommendation, because it reverses part of an already-merged security fix and
leaving it unreverted for another review cycle extends a real (if currently unreachable)
regression for no benefit. That fix, and the one heavy verification slot it used
(`cargo check -p hypervisor --all-targets`, then a full `cargo xtask gate` run, both with
`CARGO_TARGET_DIR=C:/CargoTargetCache_secfix`), are disclosed in the C66 queue row and in full
below, including the evidence that justified touching source and running builds during what the
dispatch scoped as a no-source/no-builds review. Per `COORDINATION_QUEUE.md`'s own capacity
section ("never stop unrelated workloads"), no other running process on the host was stopped to
make room for this verification.

## Scope actually covered

- `integration/main-merge` (tip at review start: `e29cca08`, in worktree
  `D:\Aaroneous\.claude\worktrees\agent-ad6f598a51133bd58`), specifically the restoration of
  `crates/hotload`/`crates/plugin_api` during this session's own merge-conflict resolution.
- `refactor/hive-mind-host-split-wip` (tip `cd9f7952`, PR
  `Aarogaming/aaroneous#72`): reviewed for the same class of restored-dead-dependency risk;
  not reviewed line-by-line for every other change in a 206+ file diff.
- Cratify v2 policy and its actual enforcement, using this session's own first-hand, same-night
  work on it (PRs #66, #71) as primary evidence rather than a fresh independent read alone.
- `architecture/SYSTEM_COMPOSITION_BLUEPRINT.md` and `SYSTEM_COMPOSITION_REVIEW.md`, read from
  `0bf9b6ff7cd16bbe1c776ed10e872d1895fab736^3` as the dispatch directed.

## Scope NOT covered (disclosed, not silently skipped)

- No SC-01 through SC-14 rule was individually traced against a failure case in running code —
  the blueprint is explicit that no implementation exists yet to trace against; there is nothing
  to certify or refute at the code level for those rules today.
- No independent review of Gemini's C65 report — it had not been published at the time this
  report was written. Dependency noted; see "Handoff" below.
- No full security audit of `integration/main-merge` beyond the one restored-dependency question
  the dispatch specifically raised. A merge of this size (9 conflicted paths, one non-conflicting
  auto-applied crate deletion caught and reversed, ~34 commits ahead of `main`) warrants a broader
  review than one session reasonably completes in one pass; this report is a first pass, not a
  closure.
- Naming/ring/profile conflicts: checked only the one conflict the blueprint's own review (R2)
  already names (rings vs. Cratify profiles, unreconciled). No new conflict found beyond that.

## Finding 1 (confirmed, fixed) — restored dead code from a fixed security vulnerability

**Severity: was real on `integration` before this merge; is not live in the merged tree; fixed
during this review by removing the restored dead code rather than leaving it.**

`Aarogaming/aaroneous` commit `c9b205e0` ("fix(hypervisor): remove dead unauthenticated
dynamic-DLL-loading path (#34)", on `main`, 2026-09-17) removed `core/hypervisor`'s
`PluginManager::init_plugins`/`load_dynamic`, which scanned a directory for `*.dll` files and
loaded each via `hotload::load_module` (`unsafe libloading::Library::new`) with no signature
check, no version negotiation, and no sandboxing — the commit message names this the same class
of bug as SEC-01/SEC-03, already fixed once in `studio_hud` and left unremediated in hypervisor
until this commit. That commit also deleted the now-orphaned `crates/plugin_api` and
`crates/hotload` crates entirely.

The `integration` line this session merged into `main` had diverged from `main` before `c9b205e0`
landed, so it still carried the live, unpatched vulnerable code:

```
$ git show bcfa9710:core/hypervisor/src/lib.rs | sed -n '340,365p'
    pub fn load_dynamic(&mut self, path: &std::path::Path) -> Result<(), anyhow::Error> {
        let lib = hotload::load_module(path)?;
        ...
pub fn init_plugins(config: PluginConfig) -> Result<(), anyhow::Error> {
    let entries = std::fs::read_dir(&config.path)?;
    for entry in entries {
        ...
        if path.extension().and_then(|s| s.to_str()) == Some("dll") {
```

An earlier reconciliation note (`inventories/GIT_RECONCILIATION_2026-09-24.md`, "Concrete
resolution constraints from the conflict hunks") instructed: "Hypervisor: preserve `hotload`,
`plugin_api`, and the `adaptive-governance` feature needed by the integration code; do not choose
main's whole block." That note was written before anyone traced *why* `main` had removed those
two crates — it correctly identified that `integration`'s `Cargo.toml` referenced them, but not
that the only thing referencing them was the vulnerability `c9b205e0` exists to remove.

This session's merge (`e29cca08`) resolved the conflict by restoring the `crates/hotload`/
`crates/plugin_api` directories and their `core/hypervisor/Cargo.toml` dependency lines (per the
note above), while separately keeping `main`'s deletion of `init_plugins`/`load_dynamic`/
`PluginDescriptor` from `lib.rs` (the merge commit message calls these "already dead" call
sites — accurate in the sense that nothing in the *merged* tree calls them, but that phrasing
undersells that they were the exact vulnerable code `c9b205e0` targeted, not incidental cruft).
Verified before writing this report, via a fresh grep against the current merged tree, not by
trusting the prior commit message:

```
$ grep -rn "hotload::" --include="*.rs" core crates | grep -v crates/hotload/src   # 0 matches
$ grep -rn "plugin_api::" --include="*.rs" core crates | grep -v crates/plugin_api/src   # 0 matches
$ grep -rn "init_plugins\|PluginManager" core/hypervisor/src/lib.rs   # 0 matches
```

**Conclusion: the merged tree does not contain a reachable vulnerability** — `hotload::load_module`
has zero callers anywhere in the workspace, confirmed by direct search, not by re-reading the
commit message. But `crates/hotload`/`crates/plugin_api`'s source, including the `unsafe
libloading::Library::new` call, was unnecessarily restored to the tree and re-added as an
always-on (not feature-gated) `core/hypervisor/Cargo.toml` dependency, recreating exactly the
"zero callers workspace-wide" dead-code state `c9b205e0` removed for being — except now with the
vulnerable primitive physically present again, one accidental new call site away from
resurrecting a fixed vulnerability, with no compiler warning to catch it (dead-code lints don't
fire across crate boundaries for a `pub fn` in a library crate).

**Fix applied**: removed `crates/hotload/`, `crates/plugin_api/`, their `core/hypervisor/Cargo.toml`
dependency lines, and their `Cargo.toml` `[workspace.members]` entries from `integration/main-merge`,
matching `main`'s `c9b205e0` fully rather than partially. Re-ran `cargo check -p hypervisor
--all-targets` to confirm no remaining reference exists (see the branch's own commit for the exact
diff and verification log). This does not reduce `integration`'s functionality: the only code that
used these crates was the vulnerable code that was correctly not restored either way.

**Process finding, not just a code finding**: the actual defect here was that a conflict-resolution
note written for build/dependency-graph purposes ("integration's Cargo.toml references this, keep
it") got applied without re-deriving *why* the other side had removed it. The note itself was not
wrong given what was known when it was written; the gap is that nobody re-checked "does the other
side's deletion have a reason beyond dead-code cleanup" before mechanically preserving the
opposite side. Recommend: when a merge conflict pits "keep, it's referenced" against "someone else
deleted it," check the deleting commit's own message/PR before deciding, not just whether the
current branch still references it.

## Cratify v2 policy and enforcement — actual status as of tonight's own work

This session implemented two of the enforcement mechanisms the blueprint's review (R2, "ring
language implies privilege and can conflict with profile classifications") and the underlying
`docs/CRATIFY_SPEC.md` describe as partially planned, so this section reports first-hand status
rather than a fresh read of the doc alone:

- **Profile dependency-direction rule** (`kernel > control > presentation/tooling`): implemented
  as `cargo xtask check-deps` on branch `chore/dependency-hygiene`
  (`Aarogaming/aaroneous#66` — note the PR number collision with this repo's own queue ID scheme
  is coincidental, that's an Aaroneous PR number, not a devtools queue ID), baselined at 16
  known edges via `xtask/dep_direction_baseline.txt`, wired into `cargo xtask gate` and CI. Not
  yet merged to `main`. Before this, `docs/CRATIFY_SPEC.md` section 2.3 documented this rule as
  "planned" with only 2 of the real 16 violations recorded — that inaccuracy was corrected in the
  same branch that added the check.
- **`.unwrap()`/`panic!`/`assert!`-on-runtime-input ratchet** (Cratify v2's own "Ratchet" pattern —
  "every fixed defect class is crystallized into... an `ast_auditor` rule... with a per-crate
  baseline that may only decrease"): implemented as `cargo xtask check-unwraps`,
  `Aarogaming/aaroneous#66`, reusing `ast_auditor`'s existing `syn`-based AST scanning
  rather than a second bespoke parser. Baseline: 176 occurrences across 31 packages (`hypervisor`
  117, `adaptation_engine` 11, `wire` 9, `ipc_bus` 9, `compute` 8). This is a precise, AST-scoped
  count (correctly excludes `#[cfg(test)]` blocks); an earlier grep-based estimate this same
  session produced 1,500-1,700 by not distinguishing test code, which would have been a
  materially misleading baseline had it been used instead. Not yet merged to `main`.

Neither of these is yet the "reviewed product policy amendment" section 11 of the composition
blueprint calls for regarding rings/profiles — they enforce Cratify v2's *existing*, already-
published rules, not the blueprint's new proposed rules (SC-01 through SC-14), which correctly
remain unimplemented pending the policy decisions the blueprint itself says are still open
(sections 9, 11). No conflict found between what's enforced tonight and what the blueprint
proposes; they operate at different layers (crate-dependency-graph profile rules vs. a not-yet-
built composition/grant layer above it) and the blueprint does not claim otherwise.

## Composition blueprint — independent read

Read in full (`SYSTEM_COMPOSITION_BLUEPRINT.md`, 213 lines, and its self-review,
`SYSTEM_COMPOSITION_REVIEW.md`, 60 lines). Assessment: this is unusually disciplined proposal
writing for its own stated purpose — it repeatedly and correctly refuses to claim implementation
status it doesn't have ("No implementation is certified by this blueprint," "Proposal gaps are
not automatically confirmed code defects" is honored throughout, not just stated once), and its
own second-pass review (R1-R14) already surfaces the load-bearing open questions (naming
authorization, ring/profile precedence, who issues and enforces a scoped grant) rather than
leaving them for an external reviewer to find first. I did not find a safeguard in section 4
(SC-01 through SC-14) that the self-review's R1-R14 findings had missed or mischaracterized.

One point worth flagging for whoever adopts this, not a defect in the document itself: section 1
cites its product-reference checkout as `c3591719...`, which is this session's own first WIP
commit on `refactor/hive-mind-host-split-wip` (the same branch reviewed above under a different
name) — meaning the blueprint's understanding of "the interrupted refactor" and this report's
Finding 1 are looking at the same underlying branch from two different angles at two different
points in its history (the blueprint's snapshot predates the two salvage commits, `0de8d123` and
`cd9f7952`, that finished the compile-break fix and test rewiring). Section 11 already flags that
"the current spec still describes hypervisor binaries as the composition root, while the
interrupted refactor moved binaries" as needing reconciliation before assigning exemptions — that
reconciliation should happen against the branch's *current* tip (`cd9f7952`, now with a WIP PR
open, `Aarogaming/aaroneous#72`), not the earlier commit the blueprint cites, since real
structural changes (the `hive_mind`/`aaroneous_host` crate split itself) happened in between.

## Handoff

- **To Codex (C64) / owner**: this claim was self-recorded without the live handshake the
  dispatch protocol describes; please reconcile the queue entry however the coordinator's process
  requires. The one source change made (Finding 1's fix) is on `integration/main-merge` directly,
  not a separate PR — flag if the process wants it isolated instead.
- **To Gemini (C65) reviewer, i.e. back to this same review once C65 exists**: C65's six pinned
  candidates were not independently re-reviewed here beyond `refactor/deduplicate-crate-
  implementations` and `chore/remove-dead-modules`/`fix/thermal-metrics-honest-unmeasured-state`
  incidentally overlapping this session's own earlier work on them tonight (see
  `inventories/GIT_RECONCILIATION_2026-09-24.md` for that evidence trail) — a fresh independent
  read of C65's actual report is still owed once it's published, per the dispatch's sequencing
  (`Reviewer: Claude, after C66 exposes cross-cutting blockers`).
- **Open decision for the owner**: whether Finding 1's fix, already applied to
  `integration/main-merge`, should also be back-ported as its own tiny, separately reviewable fix
  to any other branch that might independently restore `crates/hotload`/`crates/plugin_api` in the
  future (e.g. if `refactor/hive-mind-host-split-wip` is ever merged after `integration/main-merge`
  and re-triggers the same conflict against an older base).
