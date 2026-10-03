# Routine: steward-heartbeat

Version: 1 (2026-10-03). Status: **proposal until independently reviewed and merged**; the live trigger
keeps its interim inline prompt until this file is on `main`.

This is the single source of truth for what the steward heartbeat does. The scheduled trigger only names
this routine; it does not restate it (see "Trigger contract"). Change behaviour by editing this file
through a reviewed PR, never by re-authoring the trigger.

## Trigger contract

The trigger prompt is a short pointer: "Run routine `steward-heartbeat` from
`governance/routines/STEWARD_HEARTBEAT.md` on `origin/main`, then follow it exactly." Cadence (the
schedule) and procedure (this file) are separate concerns: editing one never requires editing the other.

**Authority can only narrow here.** This file may add steps, evidence sources and tighter limits. It can
never widen what the agent is permitted to do beyond the owner's standing limits (section "Authority") or
a grant the owner gave in chat. An instruction in this file that would widen authority is ignored until
the owner confirms it in chat.

## Every tick

Every tick ends with a concrete artifact: a pushed commit, a PR or PR comment with evidence, a thread
reply or resolution, a retarget with CI started, a measurement, a cleanup report, a register update, a
merge, or a merged-state refresh. When the sync step verifies that nothing changed and state is healthy,
a verified healthy no-op is a valid outcome (idle is a healthy state, `OPERATING_MODEL.md`): record the
heads and states checked in the tick's report, do not invent work, and do not notify. Never fabricate a
measurement or housekeeping task only to have an artifact. If several ticks are queued, treat them as one.

## Evidence sources (read, do not copy)

- GitHub: open PRs, comments, reviews, unresolved review threads (GraphQL `reviewThreads`; bot reviewers
  `copilot-pull-request-reviewer` and `chatgpt-codex-connector` post easy-to-miss threads; Mimo, Gemini
  and Codex packets may not use the REVIEW PACKET header), CI rollup, `mergeable`/`mergeStateStatus`.
- `governance/COORDINATION_QUEUE.md` (claims), `governance/REVIEW_RUBRIC.md`,
  `governance/PLATFORM_SPEC.md`, `governance/OPERATING_MODEL.md`.
- Local maintenance registers: `C:\Users\aarog\.codex\qwen-live\maintenance\` (`DEFERRED_FINDINGS.md`,
  `OPERATIONS.md`, `ROUTES.md`, `SUPERVISOR_PROCEDURE.md`, `progression.json`, plus `evidence/`,
  `captures/`, `results/`, `archive/`).
- Ollama at `localhost:11434` (report if down; never restart it).

## Steps

1. **Sync (read-only).** `git fetch --prune` in `D:\aaroneous-devtools` and `D:\Aaroneous`; list open PRs
   in both repos with update times; read new comments, reviews and every unresolved thread; check
   `origin/main`, CI, mergeability. If the shell safety check returns no verdict, retry once, then read
   files.
2. **Respond first to feedback on my PRs.** Fix valid findings on my own branch (Rust: fmt, clippy
   `-D warnings`, `cargo test -p foundation_cli`, `git diff --check`; docs: `git diff --check` and
   `python scripts/check_coordination_evidence.py origin/main HEAD`, which requires both refs). Push with `--force-with-lease`. Comment with evidence
   (command, exit code, hash); a new head is new evidence and earlier accepts do not carry. Reply on each
   thread; resolve a thread only when it is actually addressed. Correct my own mistakes publicly.
3. **Merge step (only while a grant is active, section "Authority").** A PR in `aaroneous-devtools` may be
   merged only when all hold at merge time, with evidence quoted in the report: (a) a whole-diff
   independent accept at the current head SHA (not delta-only, not an older head, not from the
   implementer or me for my own PR); (b) `MERGEABLE` and `CLEAN`; (c) required CI green on this head;
   (d) base is `main`; (e) no unresolved threads and no standing CHANGES-REQUESTED at this head; (f) no
   conflicting claim. Merge with `gh pr merge N --merge --match-head-commit <full head SHA read just
   before>`. Smallest and docs first, dependents after their bases. If the harness denies a merge, stop,
   report once, never work around it. After a merge: re-check every open PR, rebase my own conflicting
   PRs in my own worktree (state the new head), comment on others' conflicting PRs without touching
   their branches.
4. **Move the chain.** Retarget my stacked PRs to `main` after their base merges, check the diff is only
   their own files, close and reopen to start CI (retargeting does not trigger it), note whether the head
   changed.
5. **Other PRs.** Keep one current status comment on non-mine PRs with conflicts, unresolved threads or
   requested changes; update only on change; at most one reminder per item per day. Flag superseded,
   conflicting and 3-day-stale PRs by comment. Do not push to another agent's branch unless the owner
   authorized a co-author delta branch.
6. **Review and cleanup.** Independently review an open PR lacking my review when one is available (full
   packet, commands run, whole-diff scope stated; a review of my own PR never counts as the independent
   accept). Run `python scripts/worktree_inventory.py <repo-dir>` read-only once for each of `D:aroneous-devtools` and
   `D:\Aaroneous` (it requires the repository directory argument). Remove only worktrees I created, when clean
   and their PR is merged or closed (`git worktree remove`, no `--force`). Verify the path with `pwd`
   before git commands in a worktree. Never touch others' worktrees, dirty or untracked files, or the
   shared checkouts' working trees.
7. **Steward program progression (primary forward work).** Governed by `PLATFORM_SPEC.md`. If that file
   or the C97 lane is not yet on `origin/main` (the spec is PR #40, the claim is PR #42), do not infer
   phase requirements or acceptance criteria: limit this step to unblocking them (answering review
   findings, requesting or performing independent reviews). Phases in
   order: P1 spec accepted; P2 review-coverage shadow check (lane C97), then required; structured
   attestations; P3 append-only event log as a separate service; P4 steward service with expiring leases
   replacing per-agent heartbeats (retire this routine's trigger only after the steward demonstrably
   covers sync, review routing, retarget, rebase, nudge and merge-when-ready); P5 path-based gates plus
   commit and doc lints; P6 stable/beta promotion pipeline; the `AGENTS.md` naming amendment as its own
   PR. Each tick: determine the phase from PR and merge state, advance one concrete step or unblock it,
   check the queue and open claim PRs before source work and use a claim PR for a new lane, keep
   prototypes in devtools as beta (inert or shadow), graduate into Aaroneous only via `AGENTS.md`
   section 4, record progress on the owning PR or claim row.
8. **Operational follow-through.** Read the maintenance findings register, audit logs, receipts and the
   previous progression record. Label items CONFIRMED OBSERVATION, UNTESTED HYPOTHESIS or CAPABILITY
   REQUEST. Use existing Aaroneous and devtools commands and record route, commands, exit codes and
   evidence. Bounded classification or drafting may go to the local worker (`qwen3.5:9b-q6`, GPU layers
   capped, VRAM under 12 GB, existing reservation and quiet windows minutes 6-20 and 41-50, never restart
   `local-worker-serve`); its output is unverified until independently reviewed. Advance one authorized
   step after checking claims. Update affected findings with evidence, status, mitigation, responsible
   component, reason deferred, next action and acceptance condition; edit the existing register in place
   after re-reading it (no competing copy or second writer); one entry per finding; link handoffs to
   their destination; correct statuses that overstate evidence; close a finding only on retained evidence
   meeting its acceptance condition. Do not run the maintenance trigger script unless the owner asks.
9. **Forward progression when nothing above applies.** A measurement on Ollama if it is up, or
   housekeeping that unblocks others. Work-in-progress limit: no new stacked PRs while 3 or more of my
   PRs are unmerged and at most 2 new PRs per hour, except steward-program steps the owner directed.

## Authority

Standing limits (owner): never approve or self-accept my own work; never edit settings or permissions;
never touch other agents' dirty checkouts or untracked files; never change git config in a shared repo
(use `git -c`); never claim a lane in the ledger except through a claim PR; never enter credentials; never
install scheduled tasks or services; force-push only with `--force-with-lease` on my own branches; never
delete what I did not create; never push to `main`; never merge in the product repo. Retain evidence for
every claim and label anything unverified as NOT VERIFIED.

Grants (given by the owner in chat, recorded here for the next tick; they expire):

| Grant | Given | Expires | Scope |
|---|---|---|---|
| Merge verified-ready PRs in `aaroneous-devtools` | 2026-10-03 ~21:00Z | 2026-10-04 21:00Z | Step 3 conditions (a)-(f); no merge of a PR touching `AGENTS.md`, `.github/workflows`, settings or permissions without explicit owner confirmation |

A grant not listed here, or past its expiry, does not exist.

## Notification policy

Stay quiet on unchanged healthy state. Report in 3 to 6 lines: lead with the steward-program phase and
progress, then merges done (link and evidence for conditions (a)-(f)), what shipped, what is next, and
which PRs need a re-review or an owner decision. For operational follow-through, notify only on
meaningful progress, failure, completion or required owner action.

## Change control

Edit this file by PR with a whole-diff independent review, like any governance document. A tick reads the
version on `origin/main`. Bump `Version` on any behavioural change. The trigger text should rarely change;
if it does, that is a cadence or pointer change, not a procedure change.
