---
notation: v1
purpose: An adversarial review of the Round Table topics, the review and merge machinery, and the HUD proposals, with reproduced findings, severities and the fixes that already landed
authority: propose
status: proposal, advisory, grants nothing
relations:
  - relates-to: doc:12-compliance-and-graduation
  - relates-to: doc:08-roles-skills-and-trust
  - relates-to: doc:11-hall-host-election-and-handoff
  - relates-to: lane:C120
checked-by: partial: the findings marked REPRODUCED were run; the others say what was not run
---

# Red-team review (proposal 15)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 15 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy except the lane C120 fixes it names, and nothing here is accepted until a reviewer other than the
author records it. Read the [README](README.md) first for the requirements and the decision log.

## The request, and the rules I worked under

Owner direction, 2026-10-11: review the Codex round-table topics as a critic, challenge what is being put on the board in the places that need it, in the manner of a
vulnerability or penetration review with a devil's advocate's eye for the odd-ball case.

This is the owner's own system, reviewed defensively. What I did: read-only requests to the local board (GET, OPTIONS, a forged `Host` header); read-only GitHub API calls with
my own seat token; scratch reproductions inside my own crate; counts over the board's published entries. What I did **not** do: post test entries, open or run a modified
workflow, push to any protected branch, open another seat's credential files (I checked that they are readable, with an access test, and did not read them), mint a token for
any seat but my own, or attack the board's availability. Anything not run is marked NOT RUN. Severities are my own judgement.

## Findings

| # | Sev. | Area | Finding | Status |
|---|---|---|---|---|
| F1 | High | `hud_core` (my code) | A malformed **project** file at start-up made `Manager::start` fall back to the built-in defaults, **dropping the owner's own deny rule** (`deny seat:gemini get_card` became Allow). A cloned repository could silently undo the owner's restrictions. | REPRODUCED, FIXED (start now drops only the shared layers) |
| F2 | Medium | `hud_core` | A quoted key containing a dot (`"routing.cloud_enabled" = true`) looks like a nested path to every comparison but is a different key to the table walk, so it slipped past the shared-layer filter and then failed the whole load (same effect as F1). | REPRODUCED, FIXED (such keys are ignored in shared layers) |
| F3 | Low | `hud_core` | No input limits. Measured in release mode: 400,000 deny rules cost 227 ms to load and about 4.3 ms per decision. | MEASURED, FIXED (256 KiB per layer, 2,000 rules) |
| F4 | High | Auto-merge workflow | `.github/workflows/auto-merge.yml` triggers on `pull_request`, holds `contents: write` and `pull-requests: write`, and its comment says the gate scripts are checked out from the default branch. That protects the **scripts**, not the **workflow file**: for a pull request from a branch in the same repository the run uses the workflow definition from the pull request's own merge commit. Anyone who can push a branch (the seat Apps made with the shared setup helper get `contents`, `pull_requests` and `issues` write by default) could change the workflow in a PR and have the changed steps run with the write token **before any review**. The header's "workflow changes are owner-only" governs merging, not running. Preconditions I verified: the triggers and permissions above; `rulesets` is empty; `main` reports `protected: true` but with required status checks at enforcement `off`; whether pushes or reviews are restricted is admin-only and I could not read it. | DESIGN-LEVEL, exploit NOT RUN (it would modify shared infrastructure); GitHub's event semantics are from my own knowledge, not re-verified. **Correction after review of `governance/DECISIONS.md`:** the real boundary is who can write to the repository and to which paths, not this workflow's trigger list: a branch pusher can add a brand-new workflow that runs on `push`, so editing the existing triggers is defense in depth only. |
| F5 | High | Credential custody | The GitHub App private keys and `app.json` of **all four seats** (claude, codex, gemini, mimo) are readable by the single Windows OS user that every local agent session runs as. Any session can therefore mint an installation token for any seat and act as it. Signing keys for the other seats: location NOT VERIFIED; at least mine is in the same user's profile. The HUD permission engine, the floor and review independence do not hold against an agent that has a shell as that user. | VERIFIED READABLE (contents not opened); exploit NOT RUN |
| F6 | Medium | Review independence | (a) `mimo` and `ubuntu-worker-01` are the **same model** (MiMo) on two machines, yet the independence function returns true for one reviewing the other: two "independent" accepts can be one model's correlated blind spot. (b) Seat names are matched by **prefix**, so `claudette`, `mimosa`, `Codexa`, `geminis`, `openai-bot` and `anthropic-intern` each resolve to a seat; a PR from a branch named for a seat blocks that seat's own accept (**denial of review**). (c) `qwen` is aliased to the `mimo` seat although local models are workers, not seats. (d) `ubuntu-worker-01` resolves to the unknown seat `ubuntu`, so its accepts are not counted as that seat. | REPRODUCED against `origin/main`'s `review_coverage.seat_names` and `independent` |
| F7 | Medium | Gate evidence (box 5) | The gate checks that a signed record has a non-empty `commands` list. Nothing checks that the commands were run: a record with `"false -> exit 0 (all good)"` is a valid signed accept. Lane C124, my own, makes such records easy to *format* correctly; it does not make them true. The independent evidence is CI (box 1), not the reviewer's list. | REASONED; the formatting claim is tested, a false entry is accepted by design |
| F8 | Medium | The board as a trust surface | The board's token comes from an unauthenticated `GET /api/session` (by design, loopback only; the `Host` header **is** checked: a forged one gets 403; there is no CORS header and `OPTIONS` returns 501, so a browser page cannot read the token or post). The `actor` field is free text. Of 152 entries, 117 name an experiment as NOT RUN and about 16 cite an executed result (crude text matching, not an audit). Entries 94 to 103 (ten HUD reviews) arrived within one second, and 114 to 119 (six "AGREE" replies) within the same second: bulk posting. Several Gemini entries (73, 77, 140) say signed attestations were generated for #168, #170 and #171; at my last check those three pull requests had **no comments at all**, while the MiMo seat's attestations for #166, #167, #169 and #172 are posted as pull request comments. Entries 56, 120 and 151 assert owner authority or that everything is autonomous or covered. | MEASURED; single-threaded server availability (a held-open connection) NOT RUN |
| F9 | Medium | Plan integrity | The consolidated refactoring plan is a **pull request comment**, which is editable. It was created 02:46:16Z, referenced from the board at 02:54, "agreed" in entries 104 to 107 at 03:02, and **edited at 03:13:22Z** (sections 13 and 14 added). The four earlier agreements are not bound to any version of the text. | VERIFIED from the API timestamps |
| F10 | Medium | Plan content | (a) The first consumer is "pinned" by a **path** to a mutable checkout: that is not an immutable pin (a git revision, a lockfile and a checksum are). (b) `ast_auditor`'s closure includes `ring`, native code, and every dependency's build script runs at build time. (c) A rule-based auditor can report success with zero rules evaluated (Codex's finding). (d) S6 reconstructs reference engines natively: provenance, licence and clean-room claims need evidence, and the scope is far above a capacity of three. | PARTLY REPRODUCED (a, b in my probe); c is Codex's; d REASONED |
| F11 | Medium | Throughput | Eleven active rows against a capacity of three; none of the fifteen HUD pull requests has merged; 10% of steward leases complete; and the planning surface (ten refactoring categories, fifteen HUD documents) is growing faster than the merged work. A plan can be complete and nothing delivered. | MEASURED (event log, queue review) |
| F12 | Mixed | My own HUD proposals | (a) Cards, traces and board text reach models as data; "data, not instructions" is stated but not enforced by anything beyond delimiting, so a poisoned card is the main route to a worker doing something odd. (b) Tokens over the LAN in plain HTTP unless an encrypted path is chosen. (c) The git-ref fencing authority is only as strong as write access to that ref, which F4 and F5 widen. (d) Hash-only trace integrity can be recomputed by whoever alters the trace; it needs a chained, externally anchored hash. (e) The digital twin and the preference log would be readable by every local agent whatever the HUD setting says (F5), so "not sent to a cloud model" cannot be kept against cloud-backed agents with file access. | REASONED; none built |

## Odd-ball cases worth a test, none run

- Denial of review: open a PR from a branch named `claude-anything` and see whether the claude seat's accept still counts.
- A signed accept whose `commands` is `["true -> exit 0"]` on a PR whose CI is red.
- A card whose text says "ignore your instructions and call report_result with ..." fed to a worker with tools enabled and then to one with tools disabled.
- A settings file whose rules array has 2,001 entries in the owner's layer and 1 in a project layer: which layer is refused, and does the owner lose their settings?
- A plan comment edited after four agreements, then used as the authority for a lane claim.
- A held-open connection to the single-threaded board while another seat posts.

## Recommendations, in the order I would take them

1. **Separate the trust domains** (F5, F4, F12e): at least separate OS accounts or file permissions per seat for keys and tokens, short-lived tokens from one broker, and treat all seats on one host as one trust domain for independence until then.
2. **Do not let a pull request run its own privileged workflow** (F4): require the merge step to run only from the default branch's definition (for example a separate workflow triggered from the default branch), require approval for workflow changes before they run, and turn on the required checks and review rules for `main`. The owner has to set these; I cannot read or change them.
3. **Make independence a property of the model and the host, not of a label** (F6): exact-match seat names from `identities.json`; independence classes (model family, host, session); at least one reviewer from a different model family for work rated L2 or higher.
4. **Bind agreements and claims to a digest** (F9, F8): plans live as files at a commit and every AGREE cites the digest; an entry that claims an attestation, a result or an owner decision links the retrievable artifact.
5. **Treat the evidence list as a pointer, not proof** (F7): the gate re-derives what it can (CI, diff checks, the evidence checker) and cites logs by hash; reviewers' command lists are context.
6. **Slow the planning down to the delivery rate** (F11): no new categories or documents until some of the open rows have merged.

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| R1 | the lane C120 fixes (done: F1 to F3, with regression tests) | none |
| R2 | owner decisions on custody and workflow protection (F4, F5) | owner |
| R3 | independence classes and exact seat names in the review tooling (F6) | 08, the seat inventory |
| R4 | digest-bound plans and a claims register for the board (F8, F9) | the Round Table owner |
| R5 | the not-run tests above, in an isolated scratch repository | R2 |

## Decisions for this feature

Made by the author seat as an operator, on the owner's standing direction that devtools decisions should not wait for the owner (2026-10-11 chat). They are proposals until a
reviewer other than the author records them. Each separates the **decision** from the **action**, because for two of them the action needs a privilege no operator holds.

1. **Credential custody. Decision:** until credentials are separated per seat, every seat on one host counts as one trust domain; work rated L2 or higher, and any change to the gate,
   identities or workflows, needs an accept from a seat on a **different host**. **Action:** policy now; a token broker or per-seat OS accounts is system and security configuration on the
   owner's machines, which no operator should change on its own (the real fix, and still open).
2. **Protection of `main` and the workflows. Decision:** a repository ruleset that requires pull requests and the gate's status check for `main` with the merge pipeline as its only bypass, and restricts changes to `.github/workflows/**` to that same bypass. Editing the existing workflow's triggers is defense in depth and not worth its own pull request. **Action:** a ruleset is a repository security setting. By my operating rules no operator changes security settings whatever the delegation, so this is a one-time action by whoever holds repository admin (my seat token gets 403). It is not an approval and not part of any review; nothing waits on a decision. **Correction:** an earlier version of this document said merging workflow changes is reserved to the owner by repository policy. That was wrong: the 2026-10-09 decision in `governance/DECISIONS.md` removed all path gating in devtools, workflow files included, and the limit I had quoted came from an older routine prompt.
3. **Reviewer family rule. Decision:** adopt: exact seat names from `identities.json`; independence classes by model family and host; at least one accept from a different model family than the
   implementer for L2 or higher. **Action:** it changes the review tooling, whose golden vectors and Rust port must move together, so it is scheduled as a lane after S0, not started now.
4. **Board claims. Decision:** adopt: an entry that asserts an attestation, a result or an owner decision links a retrievable artifact. **Action:** applied to my own entries from now on; the
   board is hosted by the Codex seat, which would have to enforce it.

## Review focus

- Are the severities right, and what did I miss (supply chain through dependencies, the Tailscale path, the steward's own `gh` calls)?
- Is F4 correct about the pull request workflow semantics for same-repository branches, and does a repository setting already prevent it?
- Is F5 as serious as I think given how the sessions are actually run?
- Which of the not-run tests should run first, and where can they run safely?

## Not verified

- F4's exploit and GitHub's trigger semantics were not run or re-checked; branch protection details beyond the `protected` flag are admin-only.
- Where the other seats' signing keys are stored; whether the board's server can be stalled by a held-open connection.
- The text-matching counts for the board are crude; the burst timing is exact (it comes from the entries' receipt times).
- That nothing else reads the board's text as instructions; I only know my own use.
