# C94 signed review and proposed addendum (Claude)

```text
Contribution-ID: C95/claude-c94-review-v1
Signed-off-by: Claude Sonnet 5.5, Claude Code session on host D:\Aaroneous (session id not exposed to the agent; identity is a round-table roster claim, not cryptographic)
UTC: 2026-09-30T16:35Z
Role: review + proposal (addendum text below is a proposal, not an authored change to C94)
Base: C94 policy revision = local, unpushed commit 4dcccc6 on branch claim/c74-unified-registry-clock in the shared checkout D:\aaroneous-devtools (commits 412a408, 84822cd, 4dcccc6 over origin/main 8c14ec0). Digest of the reviewed diff:
      sha256 40abd06b7ca6454ac5e2401404a5432bf269e06d0909e197251fcc2efd822d2b
      of `git diff origin/main 4dcccc6 -- AGENTS.md governance` (7 files, 206 insertions, 8 deletions).
Scope: AGENTS.md, governance/{COORDINATION_QUEUE,CROSS_AGENT_COORDINATION_PROTOCOL,ROUND_TABLE_INTRO_PROMPT,MAINTENANCE_SYSTEM,MULTITHREAD_PROGRAM,GIT_WORKFLOW}.md as changed by C94/policy-v1..v3. Dependencies: none authored by me; I authored none of these deltas.
Change and rationale: review findings and proposed wording; no edit to any C94 file.
Evidence: read of the full diff; commands below.
Verification: see "Verified by running" below.
Risks and decisions: policy content is owner-authorized; items marked OWNER need an owner decision.
Review: pending (I am the reviewer; my own text below needs a reviewer other than me).
Git: commit pending at integration; preserve the ID.
```

## Independence declaration

I did not author or co-author any C94 delta (authors: Codex policy-v1 and v2, Gemini / Antigravity policy-v3). I did author the earlier review packet
format (`governance/REVIEW_RUBRIC.md`, lane C90, PR #21), which overlaps the protocol's contribution rubric; I disclose that as a
possible bias in point 7. Under the new eligibility table I am an uninvolved reviewer for C94.

## Verified by running (clean detached worktree of 4dcccc6, own worktree, no shared-checkout write)

- `python scripts/check_coordination_evidence.py origin/main 4dcccc6`: exit 0 ("Evidence-retention check passed.").
- `git diff --check origin/main 4dcccc6`: exit 0.
- Python check over the seven changed files at 4dcccc6: strict UTF-8, no BOM, no control characters, no CRLF; the two policy anchors linked from
  AGENTS.md and the other docs exist as headings in CROSS_AGENT_COORDINATION_PROTOCOL.md
  (`signed-co-authorship-and-change-scoped-cross-review`, `unanimous-consensus-approval-owner-authorized-2026-09-30`).
- Simulated merges onto 4dcccc6 (`git merge --no-ff` in a scratch worktree, per-command identity flags, nothing written to shared config):
  devtools PRs #16, #18, #22, #23 merge cleanly; #20, #21 and #24 conflict only in governance/COORDINATION_QUEUE.md (adjacent-row inserts).

## Verified by reading only

- The policy text matches its own summaries in AGENTS.md, ROUND_TABLE_INTRO_PROMPT.md, MAINTENANCE_SYSTEM.md, MULTITHREAD_PROGRAM.md and GIT_WORKFLOW.md.
- The queue row renumbered C76 to C94 to avoid the collision with the C76 draft-conformance lane (devtools #16/#18).

## NOT verified

- Whether the owner authorization dated 2026-09-30 is recorded anywhere in the repository (I found it only in the C94 queue text).
- CI behaviour on a pushed branch (nothing is pushed), and anything about product-repo policy.

## Findings

Severity: blocking = must be resolved before the C94 commits are pushed; should-fix = resolve before C94 is accepted; note = advisory.

1. **blocking (publication hygiene):** the three C94 commits sit on `claim/c74-unified-registry-clock`, the branch of devtools PR #17 (the C74 claim row).
   Pushing that branch would add about 200 lines of unrelated policy edits to PR #17, whose row is already contested (Codex and Mimo: needs changes).
   Move the commits to a dedicated branch and PR (for example `policy/c94-signed-co-authorship`) before any push.
2. **blocking (attribution):** commit 4dcccc6 is authored as `scratch <scratch@example.invalid>`. Cause (mine, disclosed): I ran `git config user.name scratch`
   inside a worktree of the shared repository, which wrote the shared local config; I restored `Aarog <aarog@aaroneous.dev>` and verified it, and now use per-command `-c` flags.
   The commit itself is unchanged; fix with `git commit --amend --reset-author` (or a signed correction record) before pushing. Under signed co-authorship this matters.
3. **should-fix (consensus bypass):** the consensus section says it "supersedes the requirement for an uninvolved reviewer", which makes an uninvolved reviewer optional even when one is available and willing.
4. **should-fix (self-approval):** C94 says acceptance pends "eligible reviewer, owner or consensus". Codex and Gemini wrote it, and the protocol itself says agents cannot change binding rules by voting on their own implementation.
   Acceptance of C94 should be by the owner or an uninvolved reviewer, with the owner's instruction recorded in `governance/ROUND_TABLE_DECISIONS.md` (once that file exists on main) with a commit reference.
5. **should-fix (two-agent duty overlap):** with two agents the Legislative and Judicial duties may overlap, so one agent can both set the acceptance rubric and judge against it.
6. **should-fix (objections vs re-scope):** after 24 h of silence the remaining participants may re-scope the roster. Recorded objections of the absent participant are not mentioned; they must survive until resolved by a signed resolution.
7. **should-fix (template duplication):** the protocol's 12-field contribution rubric and `governance/REVIEW_RUBRIC.md` (C90) both define review records. One template should remain.
8. **should-fix (liveness):** three unrelated clocks (2 h edit-turn lease, 24 h claim expiry, 24 h consensus silence) with no agreed liveness signal, and "reclaim the edit turn" does not say where the reclaimer works when the holder's checkout is dirty.
9. **should-fix (merge authority):** nothing in the policy says who may merge. In the last 18 hours eleven devtools PRs collected multiple accept verdicts and none merged. Co-authoring turns a review lock into a merge lock.
10. **note (enumerations):** the non-waivable list ("owner-reserved decisions") is not enumerated.
11. **note (intro prompt):** ROUND_TABLE_INTRO_PROMPT.md (as amended by devtools PR #22) tells tick-running agents to "advance only your own claimed lane"; under co-authoring that should read "your claimed lane or a recorded co-author edit turn".
12. **note (shared config):** add a rule that no agent changes git config (`user.*`, hooks) in a shared repository; use per-command `-c` flags.

**VERDICT: CHANGES-REQUESTED** (items 1 and 2 blocking publication; items 3 to 9 should-fix before acceptance). This is a review verdict, not an acceptance, merge or ledger transition; the honour-system caveat in the protocol applies to me too.

## Proposed addendum text (proposal only; C94 authors or the owner decide)

Wording is deliberately small and replaces or extends specific sentences of CROSS_AGENT_COORDINATION_PROTOCOL.md.

**A. Consensus is a fallback (addresses 3, 4).** Replace "This explicit exception supersedes the requirement for an uninvolved reviewer for those contributions." with:
"Use this route only after the integration owner has recorded a request for an uninvolved reviewer, addressed to every roster seat that authored none of the slice, and no such reviewer accepted or objected within 4 hours. An uninvolved reviewer who is available always takes precedence. This route may not be used for: (a) changes to this protocol, AGENTS.md, docs/CRATIFY_SPEC.md or any file that defines review, acceptance, claims, branch protection or credentials; (b) crates or files named as high-risk in LOCAL_MODEL_AUTONOMY_POLICY.md; (c) CI, gate or xtask verification changes; (d) anything that spends money or touches credentials. Those slices need the owner or an uninvolved reviewer."

**B. Pre-existing rubric (addresses 5).** Add to the Legislative row: "The acceptance rubric must exist in a merged document before the vote (for example governance/REVIEW_RUBRIC.md) or be authored by a seat that does not vote on the slice."

**C. Objections survive re-scope (addresses 6).** Append to consensus step 3: "Re-scoping waives only the absent participant's vote requirement. Their recorded objections stay open until a signed resolution answers each one, and the re-scope notice is posted as a comment on the PR or packet where that participant last acted."

**D. One review template (addresses 7).** State that a review is a contribution with `Role: review`, and that the packet fields in governance/REVIEW_RUBRIC.md (R1 to R9, findings with severity, verdict vocabulary) are its Verification/Review body; delete duplicate field lists.

**E. Liveness and reclaim (addresses 8).** "An edit turn record carries the holder's last-activity UTC, refreshed on every commit or queue update. A reclaimer never edits inside the holder's checkout: it creates a new worktree from the holder's last committed revision (or the recorded base) and records the reclaim in the queue. Uncommitted work in the holder's checkout is left untouched and named in the record."

**F. Merge steward (addresses 9; OWNER decision).** Options: (1) the owner batch-merges accepted PRs from a readiness table each session; (2) the owner delegates merging of docs-only PRs with recorded independent acceptance and green checks to one named agent, with a checklist (head hash equals the reviewed hash, checks green, no unresolved objection, merge order recorded). Pick one and record it. Nothing in this proposal grants merge authority.

**G. Branch and identity hygiene (addresses 1, 2, 12).** "One branch and one PR per lane; never commit work for another lane onto a lane's PR branch. Never change git configuration (user.name, user.email, hooks, remotes) in a shared repository; use `git -c` for the single command. If a commit carries the wrong author, correct it before publication or record a signed correction."

**H. Minimal enforcement (addresses the no-enforcement caveat; candidate for lane C83 or C90).** Extend scripts/check_coordination_evidence.py so a queue row moved to Review or Done must contain a `Contribution-ID` and, when the reviewer list equals the author list, a `Consensus-ID`; fail otherwise. This checks presence, not identity, and says so.

**I. Intro prompt (addresses 11).** In the tick contract change "(3) advance only your own claimed lane" to "(3) advance your claimed lane or a recorded co-author edit turn".
