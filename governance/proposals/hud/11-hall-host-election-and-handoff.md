---
notation: v1
purpose: Let the most capable qualified machine host the hall, first by owner-pinned single-host leadership and manual fenced handoff, and only later by unattended failover once a durable fencing authority exists
authority: propose
status: proposal, advisory, grants nothing; revised after an independent rejection of the first draft
relations:
  - depends-on: doc:03-lan-roster-and-domains
  - depends-on: doc:02-agent-door-and-mcp
  - depends-on: doc:08-roles-skills-and-trust
  - overlaps: lane:C112
  - overlaps: lane:C113
checked-by: none yet (see "Not verified")
---

# Hall host election and handoff (proposal 11)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 11 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

**Revision history.** The first draft treated a signed transfer and an acceptance as the commit point, and argued that a wrong leader could
only cause duplicate scheduling that git would expose. The Codex seat rejected it as an implementation-ready design (review on PR #173,
2026-10-11) with specific protocol counterexamples. This revision accepts that review: leadership is now **owner-pinned and single-host first**,
unattended failover is **gated on a durable fencing authority**, and the bounded-harm argument is withdrawn.

## The request

Owner direction in chat, 2026-10-10: the most capable machine hosts operations, after gaining permission through qualifications from the
current host, if one precedes it. Devtools sessions work together and the most qualified becomes leader naturally, through a handoff protocol,
rather than forcing another session to shut down.

## Principles

1. **Exclusive leadership needs a proof, not a score.** Capability and qualification only *select a candidate*. They cannot replace a durable
   fencing authority that makes at most one host a valid writer at any time. Without that proof the hall does not take new effects.
2. **Duplicate scheduling is not harmless.** A second leader can start local inference or builds, run previously authorised operations, or
   apply stale result and lease updates before any git conflict appears, and git does not undo an external effect. So the default when exclusivity
   cannot be proven is **SafeHold**: no new exclusive leases and no new external effects.
3. **No forced shutdowns.** A fenced former leader keeps running as a read-only standby.
4. **Fence at the consumers.** The current term is checked at every exclusive-lease consumer and every effect adapter; losing freshness means no new effects.
5. **Deterministic by construction.** Election and handoff are a pure reducer with time and messages as inputs, evaluated by a deterministic simulator.

## Phased approach

| Stage | Behaviour | What it needs |
|---|---|---|
| **A. Owner-pinned single host** | the owner names the leader in settings; every other host is a read-only standby that replicates the log; no automatic failover | nothing new; this is the recommended first version |
| **B. Manual fenced handoff** | the owner (or an owner-authorised qualified leader) moves leadership deliberately, using the fencing procedure below | the fencing authority and enforcement at consumers |
| **C. Unattended failover** | a certified standby takes over after proven leader loss | stage B proven in simulation and in use; a durable fencing authority with compare-and-swap that works offline or fails safe |

Stages B and C are not to be built until stage A is in use and the fencing authority is chosen and reviewed.

## Fencing authority (the linearization point)

The **only** thing that makes a host the leader is a successful **compare-and-swap** on a single durable record `(term, host)` held by a fencing
authority. A signed transfer, a certificate or an acceptance record is not enough, and neither is a per-host epoch counter.

- **Candidate mechanisms** (to be chosen and reviewed, not assumed): (a) a ref in the git remote updated atomically with an expected old value, when
  online; (b) an owner-confirmed record. Whichever is chosen must define: the exact atomic-update semantics, how a *stale term* is rejected, an expiry
  (the freshness window) that is measured by the authority, what happens offline (SafeHold, no new effects), and what happens to jobs already running
  when the term moves (they are fenced: their next effect or lease update is refused).
- **Terms are unique.** Only one writer can move `term` to `term+1`; two standbys that both try to publish the same next term cannot both succeed,
  because the swap is conditional on the old value. The loser learns the winner and stands down.
- **Consumers check the term.** Workers, the lease store and every effect adapter carry the term they were granted under and refuse anything from a
  lower term or from a leader whose freshness has lapsed.

## Handoff (stage B) and recovery rules

1. **Select and qualify.** The score ranks candidates; a candidate passes the qualification trials (replay a snapshot and verify the hash chain; serve the
   agent API at a set rate within a latency budget; process synthetic worker heartbeats; restore from backup to the same state digest; be reachable from the
   workers' side). Passing qualification makes a host *eligible*, nothing more.
2. **Fence the incumbent first (write-ahead).** The incumbent durably records `fenced-at(term+1)` **before** it sends any transfer. From that moment it
   may not lead again unless re-elected through the fencing authority at a higher term. This is what prevents it resuming independently after a crash,
   even if the candidate's acknowledgement never arrives.
3. **Drain.** The incumbent stops new leases and lets short jobs finish or expire.
4. **Snapshot and sync.** The incumbent sends state and the event log to a tip hash; the candidate verifies and replays it.
5. **Swap.** The candidate performs the compare-and-swap `(term, incumbent) -> (term+1, candidate)`. Success is the commit point and the only one.
6. **Announce.** Workers learn the new term from the authority and from the new leader; the old leader becomes a read-only standby.

Recovery for every partial state: a crash before step 2 leaves the incumbent leader; between 2 and 5 the incumbent is fenced and the hall is in SafeHold
until the swap succeeds or the owner repins; after 5 the candidate leads and the incumbent, on restart, reads the authority and stands down. There is no
state in which two hosts both hold a valid term.

## Unplanned failure (stage C only)

A standby may take over only after the trip threshold of missed heartbeats (separate from a stricter recovery threshold, a deadband), only if it holds a
valid certificate, and only by winning the compare-and-swap. If the authority cannot be reached the standby does **not** lead: the hall stays in SafeHold.
Two machines cannot form a majority, which is why the authority, not a vote, decides. Certificates carry no wall-clock expiry that depends on comparing two
hosts' clocks; their validity is bound to a term and to revocation recorded at the authority.

## Invariants and tests

- **At most one valid writer at any time, across all terms** (not merely one per term). A writer is valid only if its term is the authority's current term and
  its freshness has not lapsed.
- **No duplicate externally observable operation**, even when acknowledgements are lost and hosts restart.
- **SafeHold on uncertainty.**
- **Falsifying experiment (credit: Codex seat, NOT RUN):** a deterministic two-host simulation drops the acceptance acknowledgement after the transfer and the
  swap are both durable, crashes and restarts the incumbent, and partitions workers so each reaches only one host; assert the three invariants above.
- The reducer is a pure function with time as an input; a simulator drives it through loss, duplication, reordering, delay, partition and crash at every step.

## Cost of leadership is still configuration

Score weights, margin, sustained window, minimum tenure, cool-down, freshness window and thresholds are settings. Leadership is per domain ([03](03-lan-roster-and-domains.md)).

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| H0 | owner-pinned single-host leadership with read-only standbys (stage A); host identity and the term record | 01, 03 |
| H1 | the pure reducer and the deterministic simulator with the three invariants; no network | H0 |
| H2 | choose and review the fencing authority; define its semantics; enforcement points at lease consumers and effect adapters | H1 |
| H3 | capability probe and qualification trials | H0 |
| H4 | manual fenced handoff (stage B) | H2, H3 |
| H5 | unattended failover (stage C) only after H4 is proven | H4 |
| H6 | owner controls and a view in the application | H4, 10 |

## Decisions for this feature

1. DECIDED in principle by the owner: the most capable qualified machine leads, by handoff, without shutting others down.
2. PROPOSED: start with stage A, owner-pinned single host and read-only standbys.
3. OPEN: which fencing authority (git ref with compare-and-swap, an owner-confirmed record, or both), and its offline behaviour.
4. OPEN: score weights, margin, window, tenure, freshness window.
5. OPEN: who may authorise a stage B handoff (the owner, or a qualified leader).

## Review focus

- Is "qualification selects, the fencing authority decides" the right split, and is the compare-and-swap on `(term, host)` a sound linearization point?
- Does write-ahead fencing of the incumbent close the recovery hole, and is every partial state covered?
- Are the invariants (at most one valid writer across all terms; no duplicate observable operation; SafeHold on uncertainty) the right ones?
- Is stage A a reasonable first version, with B and C gated on evidence?

## Not verified

- Nothing here was implemented, simulated or model-checked. This revision responds to review but has not itself been reviewed.
- Which fencing authority will actually work: git compare-and-swap semantics over the owner's remote, its latency and its offline behaviour are unverified.
- That the existing hash-chained event log and its tip sidecar support snapshot, replay and verification as described.
- That measured capability predicts the ability to host well.
- Certificate revocation and freshness under clock skew are designed to avoid cross-host timestamps but are untested.
