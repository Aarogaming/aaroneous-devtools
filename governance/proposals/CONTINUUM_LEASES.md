# Continuum leases: work that any seat can resume

Status: PROPOSAL (lane C104, phase 1, docs only). Nothing in this file is in force. It changes no rule, script or
queue entry. Author: Claude Sonnet 5.5. Review needed from a seat other than the author; the amendment in section 5
needs the owner's sign-off before anyone edits the 24-hour rule.

## 1. Direction and problem

Owner direction in chat, 2026-10-06: "if we use continuum based operations, it's not an issue for other agents to
pick up where another agent leaves off, and I'd like that to be normalized in the long run. momentum, persistence."

Today a lane is held by a seat, and the hold is long because the work is not portable. A seat that goes dark (a credit
reset, a closed session) blocks its lane until the claim lapses, and the claim is long because the only copy of the
work may be in that seat's local checkout, which no other seat may touch. The long hold protects a possibility that is
avoidable: make the work portable, and the hold can be short.

## 2. What is in force today (read, not run)

| Rule | Where | What it does |
| --- | --- | --- |
| Queue claim, 24 hours | `governance/COORDINATION_QUEUE.md` (section intro), `governance/CROSS_AGENT_COORDINATION_PROTOCOL.md` (step 2 of the claim procedure) | A claim expires after 24 hours without an update. "Expiry never discards dirty work." |
| Steward lease, 6 ticks | `LEASE_TTL` in `scripts/next_task.py` | A card handed to a seat holds that item for the seat until it completes, releases, or 6 ticks pass. A tick is one `steward_run` invocation, not a clock interval (`next_tick` in `scripts/steward_run.py`). |
| Rotation | `scripts/next_task.py` | A seat that let a lease on an item expire is not given that item again while another seat can take it. The rule says nothing about where the work stopped. |

Evidence from the persistent steward log (`C:\Users\aarog\.steward\aaroneous-devtools\events.jsonl`, read
2026-10-06, 789 events, 2026-10-05T04:25Z to 2026-10-06T03:02Z):

- 88 lease grants over 36 distinct items: 5 completed (all by the Claude seat), 66 expired, 13 released.
- Expired by seat: codex 22, mimo 22, gemini 13, claude 9.

How to read that honestly: it does not show that a 6-tick lease is too short. The other three seats are not wired to
read their steward cards (owner statement in chat), so their leases lapse because the card never reached them. A
shorter lease would not have changed that. This proposal fixes what happens after a lapse; it does not claim to fix
the delivery gap, which is separate owner work (pointing each seat's loop at its outbox file).

## 3. Principle

The unit of ownership is the work, not the seat. A lane is a branch plus a trail of checkpoints. A seat is whoever
currently advances that branch. Progress exists only when it is pushed.

This is consistent with limits already in force: seats never touch each other's dirty checkouts, never push to another
agent's branch without the owner's authorization, and independent review is head-bound. Continuum operation does not
relax any of them; it gives every seat a legitimate way to continue that respects all three.

## 4. Proposed rules

Each rule states the failure case it exists for.

**R1. A claim covers a branch, not a seat.** The queue row names the branch (it already does). The holder is
recorded as the current holder, and a lapse makes it the previous holder rather than clearing the row.
Failure case: a seat goes dark and the lane is unavailable for a day with no way for another seat to continue.

**R2. A checkpoint is a push with a handoff note.** The holder pushes the claim branch at least every 60 minutes of
active work, and the tip commit message ends with a trailer block:

```
Handoff-State: <one line: what is true now>
Handoff-Next: <one line: the next concrete step>
Handoff-Verify: <the command that shows the current state, with its last exit code>
```

Trailers survive in git, are bound to a commit SHA, and need no extra service. The steward reads them from the branch
tip. Failure case: a resuming seat cannot tell what is finished, so it repeats or contradicts it.

**R3. A checkpoint renews the lease; an idle lease lapses.** A renewal requires a new commit whose tree differs from
the previous checkpoint's tree (an empty or message-only commit does not renew). A lease with no renewal for 2 hours
lapses (the numbers are proposals, see section 7). A seat that knows it is stopping, for example before a credit
reset, releases explicitly with `Handoff-Next:` filled in and the trailer `Handoff-Release: yes`, so the lane moves at
once instead of after the idle period.
Failure cases: a seat keeps a lease alive with empty commits (the changed-tree requirement); a seat that releases
cleanly is treated like one that vanished (explicit release is not a penalty, see R7).

**R4. A lapse hands the work over; it does not free the lane or grant write access.** The next card for that item
carries a resume block: branch, last checkpoint SHA, the three trailers, and the previous holder. The taking seat:

1. posts a takeover comment on the claim PR naming the checkpoint SHA it starts from;
2. continues on its own branch cut from that SHA (`<branch>-<seat>`), and pushes only there, unless the previous
   holder or the owner explicitly authorizes pushing to the original branch;
3. updates the queue row's current holder through the normal reviewed row edit.

Failure case: two seats resume the same checkpoint at once. The earlier takeover comment (by GitHub timestamp) holds the
lane; the other seat takes the next item. Failure case: the previous holder returns mid-takeover. It finds the lane
reassigned and its card says so; it reconciles with the taking seat's branch or takes another item, and it never
force-pushes over a takeover.

**R5. Unpushed work is not progress and is never taken over.** Dirty local work is neither visible nor resumable, and
no seat may touch it. A seat that wants its work to survive its own absence must push a work-in-progress branch.
Failure case: valuable work exists only in a closed session. This rule does not remove that risk; it makes the risk
the holder's, explicit, and short-lived, instead of a day-long block on everyone else.

**R6. Expiry is by recorded UTC time.** `lease_grant` records `granted_at` and `expires_at` in UTC. Tick numbers stay
for card numbering only. Failure case: today "6 ticks" means a different duration depending on how often the steward
runs. Measured over the 140 ticks in the log, the median gap is 10 minutes and the median span of 6 ticks is 60
minutes, but the maximum gap is 24 minutes, so the same lease can be shorter or longer than an hour.

**R7. Releasing is not penalized; vanishing is.** Rotation counts a clean release as completing a hand-off, not as a
lapse. Repeated silent lapses reduce a seat's weight in the responsiveness ordering, as today.

**R8. Independence follows the work.** A seat that takes over an item becomes an implementer of the result, as do the
previous holders whose commits it builds on (co-author trailers, which the steward's implementer detection in `scripts/collect_state.py` already reads). None
of them may supply the independent accept. Failure case: a seat reviews code it resumed and the head-bound accept is
counted as independent.

## 5. The amendment to the 24-hour rule (needs owner sign-off)

Not applied by this PR. Proposed wording to replace the sentence "Claims expire after 24 hours without an update;
expiry never discards dirty work." in the queue section intro and the protocol's claim procedure:

> A claim covers a branch. It lapses after 2 hours with no checkpoint (a push whose tree differs from the previous
> checkpoint's tree), and an explicit release lapses it at once. A lapse hands the work over, with its last checkpoint,
> to the next seat; it does not free the lane and does not give the next seat write access to the previous holder's
> branch. Unpushed local work is not part of the claim and is never taken over or discarded by another seat.

A claim row with no branch pushed at all keeps the 24-hour window, because there is nothing to resume from.

## 6. Implementation sketch (phase 2, not claimed here)

Blocked on the C99 stack landing on `main` (#48 and the PRs stacked on it), because the scripts are not on `main` yet.

- `scripts/steward_run.py`: `lease_grant` events gain `granted_at` and `expires_at`; `lease_table` expires by time.
- `scripts/next_task.py`: a card for a lapsed item carries a `resume` block (branch, checkpoint SHA, trailers,
  previous holder); the rotation rule counts explicit release separately.
- `scripts/collect_state.py`: read the tip-commit trailers of an item's claim branch.
- Tests that fail without the change: a lapsed lease hands over with branch and SHA; an empty-tree commit does not
  renew; two takeovers resolve to the earlier; an unpushed claim does not count as progress.
- Golden vectors (`scripts/golden/steward/`) for each, so a Rust (Guild) re-implementation can reproduce the contract.

## 7. Open questions for the owner

1. Numbers: is 60 minutes between checkpoints and 2 hours to lapse right for credit-limited seats, or too tight? Both
   are guesses, not measurements.
2. May a taking seat push to the previous holder's branch by default for lanes the owner marks "open continuum", or
   always cut its own branch (the default proposed here)?
3. Do Tier A lanes (AGENTS.md, workflows, settings) follow the same takeover, or does a lapse on them wait for the owner?
4. Should the 24-hour window stay for lanes with no pushed branch, as section 5 proposes?

## 8. Experiments to run before adopting (all NOT RUN)

- E1: replay the real log and, for each of the 66 expired leases, ask whether the item's branch had a pushed checkpoint
  with a handoff note. Expected: almost none, since the trailers do not exist yet. That is a baseline, not a finding.
- E2: run the new rules for one week on one low-risk lane with two seats, count takeovers, collisions and lost work.
- E3: try to defeat R3 with empty, whitespace-only and revert-only commits and record what renews a lease.

## 9. Out of scope

Delivering cards to the other seats; identity and signing (lane C100); any Rust change; any change to AGENTS.md,
workflows, settings or permissions.
