---
notation: v1
purpose: An operating review of the tasking, scheduling and delegation system: measured bottlenecks, missing components, time and tick design, departments, internal affairs, and how much model-driven work software can take over
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:03-lan-roster-and-domains
  - depends-on: doc:07-guild-workers-staff-and-inspection
  - depends-on: doc:13-card-schema-retrieval-and-plan-graphs
  - relates-to: doc:scripts/steward_run.py
  - relates-to: doc:foundation/crates/foundation_cli/src/guild.rs
checked-by: partial: the measurements below were run once; see "Not verified"
---

# Operating review: gaps and bottlenecks (proposal 14)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 14 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## The request

Owner questions, 2026-10-11: are we missing components a tasking, scheduling, management and delegation system should have? Do we
scan in parallel? Do we use dates and times in operations? Do we have departments? How much of the model-driven work can be
replaced by programmatic action? What are the gaps and bottlenecks? Do we have per-frequency ticks? How do we manage internal affairs?

## Method

What I read and ran on 2026-10-11, so a reviewer can repeat it: the steward tick (`python scripts/steward_run.py ...`, timed once
with `time`); the event log at `~/.steward/aaroneous-devtools/events.jsonl` (counted by event type); `git grep` and `grep` over
`foundation/crates` for threads, model endpoints and timers; the open pull requests via `gh`; and the HUD proposal set's own
history. Each finding says what it rests on. None of it is a controlled measurement.

## Findings

### 1. Scans are serial and one tick is slow
- One steward tick took **93 seconds wall time with 41 open pull requests** (a single run, 2026-10-11). State gathering shells out to
  `gh` through a synchronous helper (`review_coverage.gh_json`) inside per-PR loops, so the calls run one after another. I did not
  profile it; serial calls are the likely cause, not a proven one.
- The Rust code is also single-threaded in its loops. The only `thread::spawn` calls in `foundation_cli` are in the registrar's HTTP
  server and the intake generator's timeout. The Guild daemon sleeps for a fixed `--interval-seconds` (default 600) between ticks.
  (The pull request description for the Guild CLI spoke of adaptive pacing; the code I read shows a fixed interval, so the guide text
  that repeats "adaptive pacing" should be corrected.)
- **Proposal:** make scans incremental and parallel under a bound. Fetch only what changed since the last tick (use `updatedAt`
  and conditional requests), run a bounded worker pool of requests (the limit is a setting, chosen against the API rate limit),
  cache each PR's state by head digest, and keep the reducer pure so the result does not depend on request order. Event-driven
  triggers can come later; polling with conditional requests works on a LAN-only setup.

### 2. Most cards go unfinished
Counted in the event log (tick 536): **401 leases granted, 40 completed (10%), 52 released (13%), 304 expired (76%)**, with 1,489 stall
events. A card that nobody picks up is not a scheduling success. I have not established why leases expire; likely causes are that
seats are not running between ticks, cards the seat cannot act on, or cards too large to finish in the lease. The remedy depends on
which, so this needs a breakdown by seat and card type first.

### 3. Review capacity is the current bottleneck
Fifteen pull requests from this proposal set have been open for hours with CI green and no review. Independent review is required
by design and is scarce: one seat has no registered signing key, and the steward only began issuing review cards for the new PRs on
tick 536. **Proposal:** treat review as a first-class queue with its own capacity, aging and escalation (finding 6), and let
deterministic validators and the cheap worker lane absorb the mechanical parts of a review.

**A measured example of the review bottleneck (PR #156, Codex key registration, 2026-10-11).** Three independent seats posted signed accepts for the same
head, and a report-only run of `merge_gate.py` (exit 0) still said *blocked on checklist box 5*. The gate counts a verdict as evidenced only if the
signed attestation record itself has a `commands` field (`review_coverage.py` line 488), nothing reads commands from the comment prose, and
`make_attestation.py` has no option to add them, so a reviewer who follows the documented tool cannot satisfy the box. A second, undiagnosed
discrepancy: the checklist counts seats `claude, mimo` while a third seat's signed accept is present. Suggested follow-ups: a `--command` option on
`make_attestation.py` (and a test), and a clearer message when a verdict is missing evidence. This is evidence for the proposal below, not a fix.

### 4. The queue file is one hot spot
Every claim and proposal inserts a row into the same file, usually after the same line. In this work I hit lane-ID collisions twice and the
C119 and C120 rows both insert after C118, so the second to merge conflicts. **Proposal:** one small file per lane (generated index for
the table), so claims never touch the same lines. This is a design change to an existing process and needs the queue's owners.

### 5. Many clocks, none declared in one place
| Clock | Where | Period |
|---|---|---|
| Steward tick and lease length | `steward_run.py`, leases counted in ticks | per heartbeat; leases a few ticks |
| Guild daemon | `foundation guild daemon` | 600 s default |
| HUD worker poll | `maintenance_ui` | 120 s |
| Update check | `maintenance_ui` | periodic (lane C118) |
| Auto-merge workflow | `.github/workflows/auto-merge.yml` | 20 min |
| Seat-status freshness | `check_seat_status.py` | max age 120 min |
| Claim expiry | the queue | 24 h |
| Round Table polling | my routine | about hourly |

**Per-frequency ticks:** these are separate clocks, not a designed set. **Proposal:** declare **tick classes** in settings, each with a period,
a budget and jitter: *fast* (sub-second to seconds: the interactive event bus and lease renewal; the Gemini seat suggests 10 to 100 Hz for the interactive bus and 1 to 5 Hz for worker heartbeats and queue rings, to be measured, entry 75), *medium* (minutes: scans, availability, validators), *slow*
(hours: board polling, qualification renewal, retention, calibration) and *daily* (audits, internal-affairs sweeps, reports). Each domain
(one per repository and machine, see [03](03-lan-roster-and-domains.md)) can run its own clock. Components subscribe to a class instead of
owning a timer.

### 6. Time and dates: used for records, not yet for management
In use: ISO UTC timestamps in queue rows and seat-status, tick numbers and a hash-chained event log, 24-hour claim expiry, lease
expiry in ticks, and the dispatch time windows in `local_worker.rs`. Missing: **deadlines and service targets on cards, aging, escalation
and calendar awareness** (a stale review never escalates). The product's rule that library code takes time as an input rather than reading a clock is
the right shape: time enters as a tick input.
**Proposal:** `due`, `aging` and an escalation ladder as settings (for example: remind, raise priority, offer to another seat, tell the owner),
driven by the tick classes above. Cross-machine decisions use the hall's clock for leases and never compare timestamps from two hosts.

### 7. Departments do not exist; roles do
Today the roles are stances (builder, reviewer for others, skeptic, synthesis, scout, unblocker) and review lenses. **Proposal:** departments
as *routing groups* defined in settings, each a queue lane with a permission set, an accountable role and a work-class list. Suggested set,
mapped to existing parts:

| Department | Work | Existing parts |
|---|---|---|
| Reception | intake, deduplication, answer from evidence | C106 intake, packets |
| Planning | briefs, rating, plan graphs | proposals [06](06-brief-compiler-and-packets.md), [13](13-card-schema-retrieval-and-plan-graphs.md) |
| Engineering | the pull-request stack | seats, steward |
| Review and audit | independent review, attestations | `review_coverage`, `merge_gate` |
| Records and bookkeeping | reconciliation, accounting, ledger | worker class W1, token accounting |
| Maintenance | housekeeping, cache and branch proposals | the cleaner role |
| Research | gathering with citations | worker class W3 |
| Hosting and operations | the hall, leadership, resources | [11](11-hall-host-election-and-handoff.md) |

**A second axis, by domain (Gemini seat, entry 75):** besides these functional departments, review and claims can be routed by *domain expertise*: kernel and memory geometry; cross-platform (Linux, ARM); tooling and syntax-tree invariants; governance and merge gates. Routing a pull request to the domain that knows it, instead of broadcasting it to every seat, should cut reviewer fatigue; the domain is a skill tag on the card ([13](13-card-schema-retrieval-and-plan-graphs.md), [08](08-roles-skills-and-trust.md)). The seat proposes simulating four domain partitions against a flat broadcast over a 30-lane synthetic backlog, measuring time-to-merge and reviewer fatigue (NOT RUN).
| Internal affairs | audits the operators and the process | finding 9 |

### 8. How much model-driven work can software replace? (adopting entries 74 and 78 of the Gemini seat)
Model calls in the code I searched are confined to four places: the registrar's dispatch (code drafts), the intake generator (a proposal
from free text), local proposals, and the HUD's worker jobs and packets. The scheduler, steward, merge gate, collectors, topic-case, conformance
and the validators proposed here are already programmatic. So by call sites, little work depends on a model; by **volume** I cannot say,
because there is no per-call accounting yet ([04](04-token-optimization.md)).
**Candidates the Gemini seat named (entry 78), all programmatic in Rust:** (1) dependency and blast-radius calculation from `cargo metadata` and a syntax-tree parser; (2) a diff conformance check using the existing auditor and a parser for the Cratify profiles, the unwrap ratchet and forbidden functions; (3) the queue and lease state machine as a pure reducer; (4) mechanical attestation and envelope building and verification; (5) deriving most brief slots from the git diff and repository metadata. Several of these already exist in part (the auditor, the steward reducer, attestation scripts). The same seat estimates 70 to 80 percent of model tasks are replaceable and proposes an end-to-end comparison of a syntax-tree-driven intake against the model-driven survey; both are NOT RUN here.

**Method rather than a number:** (1) count calls by task type once accounting exists; (2) for each type, ask what a rule, a template or a
lookup does before the model; (3) keep the model only for language work that software cannot do (summarise, draft, classify ambiguous text,
interview), always behind a validator; (4) re-measure. Candidates to move to software first: classification of known record types, field
extraction from structured text, status reconciliation, citation checking, duplicate detection. Whether a GGUF model is the right backend for what remains is a per-task question for the
model profiles ([05](05-model-profiles-and-prompt-adaptation.md)), not an architecture question.

### 9. Internal affairs
Controls exist but are scattered: the evidence-retention check, claim-expiry rules, no self-review, trailer checks, attestation signing, the
circuit breaker, and the conformance auditor. What is missing is an **independent function that audits the operators and the process
itself**, and an incident practice (the batch merge on 2026-10-08 was handled ad hoc).
**Proposal:** an internal-affairs function with: a periodic sweep (tick class *daily*) over claim hygiene, evidence retention, review
independence, lease behaviour, token use against budget and trace samples ([07](07-guild-workers-staff-and-inspection.md)); findings that go
to the inspector role and the owner, never to the operator being checked; a short incident record (what happened, detection, cause,
fix, a test or rule added); and consequences expressed only through the trust ledger ([08](08-roles-skills-and-trust.md)) and permissions.
No operator audits itself.

### 10. Other components I would expect and do not find
- **Flow metrics:** lead time per card, review latency, work in progress per department, the lease outcomes in finding 2, shown in the HUD.
- **Capacity and priority across lanes:** `schedule-review` counts lanes and recommends claims; there is no cross-lane priority or capacity
  model (resource slots are proposed in [03](03-lan-roster-and-domains.md)).
- **Backpressure:** a limit on work in progress so that review can keep up (finding 3).
- **Notification and escalation to the owner** with a quiet budget (the owner wants fewer interactions, not more).
- **Retention and archive policy** for records, traces and boards.
- **Disaster recovery:** restoring the hall and the event log from backup is part of qualification ([11](11-hall-host-election-and-handoff.md)).
- **Single points of failure:** the owner, the board host, one GPU, the hall leader; the last is addressed by proposal 11.
- **A test for the tests:** golden vectors exist for the steward; most new components have none yet.

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| O1 | breakdown of lease outcomes by seat and card type; profile one tick | none (read-only) |
| O2 | incremental, bounded-parallel scan with per-PR cache | O1 |
| O3 | tick classes in settings; components subscribe | 01 |
| O4 | `due`, aging and an escalation ladder on cards | 13, O3 |
| O5 | flow metrics view in the HUD | 10, O1 |
| O6 | departments as routing groups in settings | 01, 08 |
| O7 | internal-affairs sweep and incident record | 07, 08, O3 |
| O8 | per-lane queue files (needs the queue owners) | owners |

## Decisions for this feature

1. OPEN: whether to profile and parallelise the steward tick before building more.
2. OPEN: the tick classes and their default periods.
3. OPEN: the department set and who is accountable for each.
4. OPEN: whether the queue moves to per-lane files.
5. OPEN: who runs internal affairs (the inspector role; the owner's veto) and its report path.

## Review focus

- Do the findings follow from the evidence cited, and are there bottlenecks I missed?
- Is incremental, bounded-parallel scanning the right first fix, and is the rate-limit risk handled?
- Are tick classes and departments useful structure or needless ceremony at the current scale?
- Is the internal-affairs design independent enough, and are consequences kept inside the trust ledger?

## Not verified

- The 93-second figure is one run; the cause (serial calls) is inferred from code, not profiled.
- The lease counts come from one reading of the log; why leases expire is not established, and the log may include leases that no seat was running to receive.
- "Four places call a model" comes from searching `foundation/crates` for model endpoints; model calls in scripts or in other repositories were not searched.
- The department set and tick classes are proposals; nothing was built.
- I did not measure the GitHub API rate limit headroom for parallel scans.
