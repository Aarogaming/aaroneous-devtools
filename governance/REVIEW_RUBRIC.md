# Reviewer rubric and evidence packet

Every independent review of a lane, PR or claim row carries one **evidence packet**
in this shape, posted as the PR comment (or ledger evidence cell) that records the
review. The packet exists so that reviews are comparable across agents, the
queue's evidence check has something concrete to look for, and a later reader can
tell what was *run* from what was only *read*.

It does not grant acceptance. Acceptance is a ledger decision by a different seat
than the implementer (see `CROSS_AGENT_COORDINATION_PROTOCOL.md`). Under the
current honour-system identity boundary all seats post as one GitHub login, so the
packet names the seat explicitly.

## The packet

Copy this block, fill every field, and write `none` or `not run` where that is the
truth. An empty field is a defect in the review.

```text
REVIEW PACKET
Seat / agent:        <Claude | Codex | Gemini/Antigravity | owner | other>
Implementer seat:    <who wrote the change>       Independent?  yes | no (why)
Subject:             <repo>#<PR> or <lane ID>, head commit <full or 8+ char hash>
Base / stacked on:   <base ref; list unmerged parents>
Scope reviewed:      <files/paths actually read, or "diff only">

VERIFIED BY RUNNING (worktree path, own target dir)
  <exact command>    exit <n>    <passed/failed counts>    log <path>  sha256 <hash>
  ...
VERIFIED BY READING ONLY
  <claim or location>  ->  <what was read, file:line>
NOT VERIFIED
  <what was not checked and why>            (never omit this section)

RUBRIC (mark each: pass | fail | n/a | not-checked, with one evidence pointer)
  R1 Scope       Change matches the claimed scope; no undeclared files or drift.
  R2 Authority   No ambient env/clock/path/thread use outside sanctioned seams
                 (AGENTS.md sec. 2); any seam is named in the row.
  R3 Safety      No unwrap/expect/panic on runtime input outside tests or an
                 `// INFALLIBLE:` line; no `unsafe impl Pod/Zeroable`; no stubs.
  R4 Correctness Behaviour and edge cases hold; a test asserts the real effect,
                 not just that code ran.
  R5 Gates       fmt, clippy -D warnings --all-targets, tests, and the CI checks for
                 THIS head; CI rollup read from the PR, not from a summary.
  R6 Evidence    Claims in the row/PR are backed by command + exit code + log path
                 or hash; nothing is claimed that the packet could not reproduce.
  R7 Dependencies New third-party dependencies carry an AGENTS.md sec. 4 verdict.
  R8 Docs/ledger Row status, owner, branch and evidence match reality; ID unique;
                 merge order and stacked bases stated.
                 Handoffs and maintained state pass the cold-handoff check in
                 OPERATING_MODEL.md (Independent continuability).
  R9 Reversal    How the change is undone, and what it does not touch.

FINDINGS  (severity: blocking | should-fix | note)
  <id> <severity> <file:line or row> <what, and the evidence for it>

VERDICT   ACCEPT-EVIDENCE | CHANGES-REQUESTED | BLOCKED | INFORMATIONAL
          (ACCEPT-EVIDENCE means "my review found no blocking issue"; it is not a
          merge, an approval, or a ledger transition.)
Limits:   <heuristics used, samples, flakes seen, things another reviewer should redo>
```

## Rules

1. **Run beats read.** If a gate can be run in a clean worktree, run it and record
   the exit code; say "read only" otherwise. Never copy an author's "gate green".
2. **Head-bound.** A packet is valid only for the commit it names. A new head
   requires a new packet (a short re-review packet may reference the prior one and
   list only what changed).
3. **Independence is declared, not implied.** Reviewing your own lane is `no`; such a
   packet is evidence only.
4. **Flakes are recorded, not hidden.** Include failing runs with their log
   identity, as C75 showed; do not cite only the passing runs.
5. **Hypotheses stay hypotheses.** Mark a finding `hypothesis` until it is
   reproduced (Evidence-First Living Operations loop).
6. **No new authority.** A packet never merges, approves, claims, or closes a lane.

## Quick form (docs-only or claim-row PRs)

Seat, subject + head hash, "read only" list, R1/R6/R8 marked, findings, verdict,
limits. Everything else may be `n/a`.
