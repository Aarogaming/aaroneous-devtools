---
notation: v1
purpose: One job-card schema (card, record, evidence tiers), a retrieval order for context packs, and a validated plan graph for work that decomposes
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:01-settings-and-permissions
  - depends-on: doc:06-brief-compiler-and-packets
  - depends-on: doc:07-guild-workers-staff-and-inspection
  - depends-on: doc:08-roles-skills-and-trust
  - relates-to: doc:contracts/job-intake-v1.md
  - relates-to: doc:contracts/topic-case-v1.md
  - relates-to: doc:contracts/contribution-envelope-v1.schema.json
checked-by: none yet (see "Not verified")
---

# Job-card schema, retrieval and plan graphs (proposal 13)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 13 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## The request

Owner questions, 2026-10-10: would the application be a good use of retrieval-augmented generation (RAG) and task graphs
(DAGs), and what ends up on the job cards? Is each one a large body of evidence, like a case study or an engineering audit?
This document answers both with a schema, so that the cards, the context packs and the plans use one set of definitions.

## What a card is, and what it is not

A card is **small and pointer-based**; evidence is **kept separately and attached in proportion to risk**. Operators read their
card on every tick, so a heavy card would defeat the token plan ([04](04-token-optimization.md)). Only the top evidence tier
looks like a case study or an audit, on purpose.

| Layer | Contents | Who reads it | Size budget (settings) |
|---|---|---|---|
| **Card** | what the operator needs to act | the operator, every tick | L0 and L1: at most 2 KiB; L2 and L3: at most 4 KiB |
| **Record** | what the hall keeps about the job: brief, rating, routing, events, trace, validator results | the hall, inspectors, the owner | bounded per job; large parts stored as separate objects |
| **Evidence bundle** | proof that the work was done and checked | reviewers, auditors | grows with the evidence tier (below) |

## Relation to formats that already exist

The schema extends these; it does not replace them or add a parallel queue.

| Existing format | Where it appears | How the card uses it |
|---|---|---|
| Steward task card (`TASK`, `STATE NOW`, `APPROACH`, `STEP n`, `DONE WHEN`, `THEN`, `IF BLOCKED`; receipt `[done:KIND:PR:FULL_HEAD]`) | `scripts/steward_run.py`, `next_task.py` | the human-readable rendering of a card is this layout, unchanged |
| Typed job intake | `contracts/job-intake-v1.md` (lane C106) | the card's intake fields are the intake record, not a copy |
| HUD packets | `NewJob { id, mode, evidence, instruction }`, `Blueprint` | a job created from a packet carries the packet digest |
| Topic-case events and outcome receipts | `contracts/topic-case-v1.md` | the L3 evidence tier is an experiment record and outcome receipt in this form |
| Contribution envelope | `contracts/contribution-envelope-v1.schema.json` (lane C117) | provenance, revision pins and trailers come from it |
| Queue rows and the evidence-retention rule | `governance/COORDINATION_QUEUE.md` | PR-stack cards point to their queue row; closing needs a command, exit code, and log or commit |

## The card

All fields are data; unknown fields are rejected; the schema is versioned (`card_version`). "R" required, "O" optional,
"L" required from that level up.

| Field | Meaning | Req |
|---|---|---|
| `id` | stable, never reused | R |
| `digest` | content digest of the card, for addressing and duplicate detection | R |
| `class` | work class: `W0` to `W3` for guild workers, `S` for seat (pull-request-stack) work | R |
| `level` | difficulty `L0` to `L3` and `risk` (low, medium, high) | R |
| `task` | one sentence | R |
| `brief_ref` | digest of the confirmed brief, when one exists | L1 |
| `inputs` | list of `{ ref, digest }`; digests, not content | R |
| `steps` | short ordered list | R |
| `done_when` | the acceptance check, naming the validator or reviewer | R |
| `if_blocked` | what to report and to whom | R |
| `gates` | minimum operator level, review required (who may not review), approval required | R |
| `budget` | tokens, time, and tier ceiling | R |
| `lease` | duration under the **hall's clock** and the epoch ([11](11-hall-host-election-and-handoff.md)); no cross-host timestamps | R |
| `then` | next card or class on success | O |
| `priority` | class and rank (see the operating review, [14](14-operating-review-gaps-and-bottlenecks.md)) | R |
| `due` | deadline and aging rule | O |
| `poster` | who created it and by which door | R |
| `tags` | topics, lanes, departments | O |
| `plan` | when part of a plan graph: plan id, node id | O |

A card never contains raw logs, whole documents, or the conversation that produced the brief.

## The record

Held by the hall; each part is a separate content-addressed object referenced from the record.

- the brief with slot states ([06](06-brief-compiler-and-packets.md)) and the rating inputs and result;
- routing: candidates considered, the operator chosen, which gate decided it, and why others were refused;
- the lease history: handed out, renewed, expired, requeued;
- the trace for worker jobs ([07](07-guild-workers-staff-and-inspection.md)): prompt, adapter and model versions, outputs, token counts;
- validator results with their inputs and versions;
- the event log entries (hash-chained, the existing log) that mention the job.

## Evidence tiers

The tier is set by the rating from [08](08-roles-skills-and-trust.md) and is a minimum; a reviewer can ask for more.

| Tier | When | The bundle holds |
|---|---|---|
| E0 | L0 triage, summaries | output valid against its schema; each item has a source pointer (`path@digest#lines`) |
| E1 | L1 checks and reconciliations | E0, plus validator results (arithmetic re-done, quotes found at the cited place, citations resolved), generated by software |
| E2 | L2 changes and reviews | E1, plus the commands run with exit codes and log hashes, the pinned head commit, and an independent signed, head-bound attestation |
| E3 | L3 design and safety-relevant work, and promotions | E2, plus an experiment record and outcome receipt in the topic-case form: hypothesis, baseline (measured or marked `not_measured`), threshold, budget, result, actual cost, retained dissent, unresolved issues, lesson |

Rules: evidence that software can produce is produced by software, not typed by the operator; bulky items are referenced by
digest and opened on demand; an unrun check is recorded as NOT VERIFIED, never omitted.

## Illustrative cards (not a normative encoding)

```text
card W1-0042  class W1  level L1  risk low  priority P3
task      check that the C91 queue row matches PR #161 and the steward log tip
inputs    queue@sha256:9f3a..  pr161@634c825  log-tip@4136..
steps     1 read both  2 compare status and head  3 list mismatches with file:line
done_when report valid against schema; each mismatch cites a source line; totals recomputed by software
gates     min level W1; spot-check by sponsor; reviewer is not the sponsor
budget    4000 tokens, 120 s, tier local        lease 600 s @ hall, epoch 7
if_blocked name the missing input and its owner

card S-0007  class S  level L3  risk high  priority P1
task      accept or reject the handoff protocol (proposal 11)
brief_ref sha256:c2d1..  (who/what/why/when/how confirmed; acceptance: simulator shows no two leaders in one epoch)
gates     min level L3; reviewer is not the author or any co-author or guide; signed attestation required
evidence  E3: experiment record + outcome receipt
```

## Retrieval

Retrieval produces **context packs**: small, ordered, content-addressed bundles handed to a model in place of files.

1. **Corpus (what is searchable):** the coordination queue; document front matter and relations; contracts; receipts and the
   event log; topic cases; seat-status records; accepted attestations; code and test file headers. Nothing outside the
   configured corpus is read.
2. **Order of attempts, cheapest first:**
   1. **Structural:** exact lookups by id, lane, PR, seat, topic, relation (follow `depends-on` and `relates-to` edges). Most
      questions about this system end here.
   2. **Lexical:** a plain text index over the corpus for names and phrases.
   3. **Vector (only if measured to help):** an embedding index, using the product's `omni` vector index rather than a second
      one ([09](09-visualizer-exploration-and-twin.md) uses the same graph).
3. **Pack format:** an ordered list of items, each `{ ref, digest, span, why, text }`, plus the question, the token budget used,
   and a pack digest. The same inputs yield the same pack, so it is cacheable and diffable.
4. **Citations:** every claim a worker makes cites `path@digest#lines` from a pack item. A validator resolves each citation against
   the stored object; a claim without a resolving citation is labelled `unverified`.
5. **Retrieved text is data, never instructions.** Pack text is delimited and marked as untrusted; instructions inside it are ignored
   and flagged; strangers' text enters the corpus only through intake with the poster recorded.
6. **Freshness:** items are keyed by digest, and the pack digest includes them, so a changed source cannot return a stale answer. A
   source that changed since the pack was built is flagged at the moment of use.
7. **Evaluation before adoption of each stage:** a labelled set of real questions with the records that answer them; measure
   recall at k, the share of citations that resolve, and pack size in tokens. Add a stage only if it improves recall at an
   acceptable size. The labelled set is built from the topic-case and queue history.

## Plan graphs

For work that decomposes, a brief compiles into a **plan graph**, a directed acyclic graph of cards.

- **Node:** a card with its class, level, validator and gates. **Edge:** a dependency on another node's output (data) or a
  required order (control).
- **Validity (checked by software before anything runs):** acyclic; every node type known; node count and depth within
  settings; every node's inputs are brief inputs or earlier outputs; every node individually passes its `can_take` gate; the
  plan's total budget fits the brief's budget.
- **Who may propose a plan:** a model may propose; software validates; a person or the configured rule accepts. A plan that fails
  validation is rejected with the reasons, not repaired silently.
- **Execution:** nodes run when their inputs are ready, respecting resource slots ([03](03-lan-roster-and-domains.md)) and leases.
  A node's result is **cached by the digest of its inputs, template and adapter versions**, so an unchanged branch is never re-run.
  This is the main token saving of using a graph.
- **Failure:** bounded retries per node; then fallback to another operator or tier if allowed; then the plan stops and reports
  which node failed and why. Replanning is allowed a bounded number of times and each revision is a new immutable plan.
- **Relation to Aaroneous:** the product architecture already specifies an Intent DAG registered as an immutable plan with the
  orchestrator before execution. Plan graphs here are that idea for devtools; the shape should be kept compatible so the code can
  graduate ([12](12-compliance-and-graduation.md)).
- **Loops on loops (Gemini seat, entry 74):** a supervising plan may run child plans, each its own loop, in parallel. A child loop has its own
  isolated context and budget, carries a breadcrumb (parent plan and node ids) in every card and trace, and has a bounded blast radius:
  a failure in a child can stop that child and report to its parent but cannot change the parent's nominal state or another child. The
  deterministic scan-and-reduce parts stay sequential and pure; only supervision and scheduling nest.
- **When not to use one:** single-step work stays a single card.

## Plan limits and card digests (Gemini seat, board entry 102)

- A maximum plan **depth** and a maximum **node count** are checked by software before a model is asked to decompose anything and again on the proposed graph, so a model
  cannot propose an unbounded or deeply recursive plan. **Falsifying experiment (NOT RUN):** submit a plan of depth 20 with 500 nodes and confirm that the validator rejects it
  before execution.
- Every card has a content `digest`, so cards are content-addressable and duplicates are found by software.

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| K1 | card, record and evidence-tier schema with validation and golden vectors; rendering to the steward text layout | 01, C106 |
| K2 | structural retrieval and the pack format with citation validation; the evaluation set | K1 |
| K3 | lexical stage, then the optional vector stage via `omni`, each only if the evaluation says so | K2 |
| K4 | plan-graph schema, validator and node-level cache | K1, 08 |
| K5 | plan execution with leases and resource slots; failure handling | K4, 03, 11 |

## Decisions for this feature

1. OPEN: the size budgets per level.
2. OPEN: whether the steward text layout stays the rendering of a card or is retired later.
3. OPEN: whether the vector stage is wanted at all before the evaluation shows a gap.
4. OPEN: limits on plan size, depth and replanning.
5. OPEN: whether E3 uses the topic-case experiment and receipt types unchanged.

## Review focus

- Is "thin card, thick evidence, attached by tier" the right balance, and are the required fields the right minimum?
- Is it right to extend the existing formats (steward card, intake, topic-case receipt) rather than define a new one?
- Is the retrieval order (structural, lexical, then vector only if measured) right for this corpus, and is the citation rule enforceable?
- Are the plan-validity rules enough to stop a model-proposed graph from bypassing the gates?

## Not verified

- No schema, retriever or plan executor exists. Size budgets, tiers and limits are proposals.
- That the existing intake contract and topic-case receipt carry what E1 to E3 need without change.
- That structural retrieval answers most real questions; the evaluation set is how this would be measured, and it does not exist yet.
- The behaviour of the product's `omni` vector index beyond its public signatures, which I read but did not run.
- That node-level caching saves what I expect on real work.
