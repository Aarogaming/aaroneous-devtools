---
notation: v1
purpose: Let the most capable qualified machine host the hall, and hand leadership over between running sessions without shutting any of them down
authority: propose
status: proposal, advisory, grants nothing
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

## The request

Owner direction in chat, 2026-10-10: the most capable machine hosts operations, after gaining permission through
qualifications from the current host, if one precedes it. Devtools sessions work together and promote the most qualified
to leader naturally, through a handoff protocol, rather than forcing another session to shut down.

## Principles

1. **Leadership governs scheduling, not authority.** The leader hosts the hall for one domain: the card store, availability,
   leases, intake and the agent API ([02](02-agent-door-and-mcp.md), [03](03-lan-roster-and-domains.md)). It does **not**
   own repository authority. Claims, reviews and merges stay with git, GitHub and the steward gate. A mistaken or duplicate
   leader can therefore cause duplicate scheduling at worst, which the git-level claim rules then expose as conflicts. This
   bound is what makes a two-machine design acceptable.
2. **Earned, not assumed.** A host leads only with a valid qualification certificate. Qualification is measured, expires,
   and is renewed.
3. **No forced shutdowns.** A handoff turns the old leader into a standby that keeps running, replicating and serving
   read-only. It is never told to stop.
4. **One leader per domain at a time.** Each leader term has an epoch number; every write and every lease carries it, and
   anything with an older epoch is refused.
5. **Handoff is all or nothing.** Until the transfer record is written by both sides, the incumbent keeps leading; any
   failure before that point aborts the handoff and changes nothing.
6. **Deterministic by construction.** Election and handoff are a pure reducer, `next_state = step(state, input)`, with
   time passed in as an input. That matches the Aaroneous style of deterministic state reducers and makes the protocol
   testable by simulation.

## Vocabulary

| Term | Meaning |
|---|---|
| Host | a machine running a hall session for a domain |
| Domain | one hall, one repository, one machine's worth of state (see 03) |
| Leader / standby | the host currently serving the domain / the others, which replicate and wait |
| Epoch | a counter that increases by one at every change of leader |
| Capability | measured facts about a host: GPU and memory, free disk, network reach to the workers, models available, observed uptime |
| Qualification certificate | a signed statement by the current leader (or the owner) that a named host passed the qualification for a domain, with its score, the epoch and an expiry |
| Snapshot | the leader's state at a point, identified by the tip of its hash-chained event log |

## Capability and qualification

- **Score:** a weighted sum of capability facts, with the weights and a minimum margin as settings. "Most capable" is
  therefore a measured, tunable ranking, not a hard-coded rule.
- **Eligibility** needs more than a score. A candidate must pass a **qualification** administered by the current leader:
  1. replay a recent snapshot and verify its hash chain;
  2. serve the agent API for a test interval at a set request rate within a latency budget;
  3. process worker heartbeats and leases for a synthetic fleet;
  4. restore from a backup and arrive at the same state digest;
  5. show that the registered workers can reach it (a reachability probe from the workers' side).
- **Trust prerequisite:** the host's own seat must hold the trust level that hosting requires (see 08).
- **Certificate:** signed with the leader's key using the same detached-signature scheme the attestations use, in its own
  namespace; fields: host, domain, score, epoch at issue, issued-at (leader's clock), expiry, and the leader's own
  certificate hash (a chain back to genesis). Verifiers check it against the host and seat key registry.
- **Bootstrap:** the first leader holds a **genesis certificate signed by the owner**. The owner can at any time pin a
  leader, force a handoff, or revoke a certificate; these actions are owner-only in the permission engine ([01](01-settings-and-permissions.md))
  and are not available to any seat.

## States and the handoff

Leader states: `Leading`, `Draining`, `Standby`. Candidate states: `Standby`, `Qualifying`, `Certified`, `Syncing`, `Ready`,
`Leading`.

1. **Request.** A standby whose score exceeds the leader's by the margin, sustained for the window, asks for qualification.
2. **Qualify.** The leader runs the qualification above. Failure returns the candidate to `Standby` with the reason; the leader
   is unaffected.
3. **Certify.** The leader signs the certificate.
4. **Drain.** The leader stops issuing *new* leases, lets short jobs finish or expire, and records `drain-complete` in its log.
5. **Snapshot and sync.** The leader sends its state and event log up to a tip hash; the candidate verifies the chain and
   replays it, then confirms the digest.
6. **Ready.** The candidate announces `Ready` with the tip it holds.
7. **Transfer.** The leader writes `transfer(epoch+1, candidate, tip)` to its log and signs it; the candidate writes
   `accept(epoch+1)` after checking the signature and that its tip matches. **This pair of records is the commit point.**
8. **Announce.** The new leader publishes the new epoch. Workers read the candidate endpoints from settings and follow the
   highest valid epoch. The old leader becomes `Standby`: read-only, replicating, still running.

Before step 7 any timeout, signature failure, tip mismatch or loss of the candidate aborts the handoff; the incumbent resumes
issuing leases. After step 7 the candidate leads. If the incumbent vanishes between its `transfer` record and the candidate's
`accept`, the candidate may accept on the strength of the signed `transfer` and the certificate.

## Unplanned failure and the limits of two machines

- A standby treats the leader as down only after a **trip** threshold of missed heartbeats and returns to normal only after a
  separate, stricter **recovery** threshold (a deadband), the same fault-tolerance idea the Aaroneous documents prescribe.
- Only a **certified** standby with the highest score may take over, by publishing `epoch+1` with its certificate.
- **Two machines cannot form a majority,** so a network partition can leave both believing they should lead (split brain).
  The design does not hide this. It limits the harm (principle 1), adds an optional **external tie-breaker** (a lease ref in the
  git remote, updated atomically when online), and in the absence of any proof of the lease puts the hall in **SafeHold**: no
  new exclusive leases, existing jobs continue, and operators are told. Modes follow the product's `Nominal`, `Degraded`,
  `SafeHold` pattern with separate trip and recovery thresholds.
- **Detection and repair:** two leaders show up as conflicting epochs and a forked event log. The higher epoch wins unless the
  owner overrules; the losing side's divergent entries are quarantined and listed for inspector review (see 07), not silently
  merged or dropped.

## Stability

Hysteresis prevents flapping: a score margin, a sustained window, a minimum tenure for each leader, and a cool-down after a
handoff. All are settings. Certificates expire, so a host that degrades loses eligibility at the next renewal.

## Shape of the implementation

- A small crate for the election and handoff reducer, `control` profile in Aaroneous terms: no clock, no network, no
  threads inside; time and messages arrive as inputs and outputs. The adapters (the MCP and HTTP endpoints, the log
  store, the signer) sit outside it. See [12](12-compliance-and-graduation.md).
- **Tests:** a deterministic simulator that drives several hosts through message loss, duplication, reordering, delay,
  partitions and crashes at every step; golden message sequences; an invariant check that no two hosts ever lead in the same
  epoch. Model checking the reducer is an option worth evaluating (not assessed here).
- Reuse the existing hash-chained event log and its tip sidecar for snapshots rather than inventing a state format.

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| H1 | the pure reducer with the simulator and the invariant tests; no network | 01 |
| H2 | capability probe and the qualification trials | H1, 03 |
| H3 | signed certificates and snapshot, sync and verify using the event log | H2 |
| H4 | the handoff messages over the agent API and MCP adapter | H3, 02 |
| H5 | unplanned failure handling, SafeHold and the optional git lease ref | H4 |
| H6 | owner controls (pin, force, revoke) and a view in the application | H4, 10 |

## Decisions for this feature

1. OPEN: the score weights, the margin, the window and the minimum tenure (proposed as settings with conservative defaults).
2. OPEN: who signs certificates: the current leader (proposed), with the owner able to countersign or revoke.
3. OPEN: leadership per domain (proposed) or one leader for all domains.
4. OPEN: whether the optional git lease ref is allowed as a tie-breaker when online.
5. OPEN: whether the old leader stays as a hot standby (proposed yes).
6. DECIDED in principle by the owner: the most capable qualified machine leads, by handoff, without shutting others down.

## Review focus

- Is "leadership governs scheduling only, repository authority stays in git" a sound way to bound the harm of a split brain?
- Is the commit point (the signed `transfer` and the candidate's `accept`) correct, and is every abort path safe?
- Is the qualification meaningful (does it prove the host can really serve), and are the certificate contents enough to verify?
- Does SafeHold plus the epoch and fork rules handle a partition acceptably for two machines?
- Is the pure-reducer-with-simulator approach the right way to gain confidence?

## Not verified

- Nothing here was implemented or run. The protocol has not been model-checked or simulated.
- That the existing hash-chained event log and its tip sidecar support snapshot, replay and verification as described; I have not read it closely enough to confirm.
- That the owner's two machines have clocks and network quality good enough for the heartbeat thresholds; only the leader's own clock is used for expiry, but this is unmeasured.
- That the detached-signature scheme used for attestations can be reused for host certificates in a separate namespace without change.
- Whether a git lease ref is fast and reliable enough to serve as a tie-breaker, and whether the owner wants a dependency on the remote for it.
- That measured capability (GPU and memory) predicts the ability to host well; it may not.
