---
notation: v1
purpose: Roles, a taste-versus-correctness boundary, skill-gated delegation and trust earned from reviewed outcomes
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:01-settings-and-permissions
  - depends-on: doc:07-guild-workers-staff-and-inspection
  - overlaps: lane:C112
  - overlaps: lane:C113
checked-by: none yet (see "Not verified")
---

# Roles, skill gating and earned trust (proposal 08)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 08 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## The request, and how I read it

Owner direction in chat, 2026-10-10: treat the system as if it were built to walk a **stranger** through making a
feature for a program they want to build. It is built on Aaroneous, runs devtools orchestration, and is improved by the
models and by the users. **The user is always right in matters of taste**, and feedback taken directly in the application
is valuable. The application acts as scheduler, moderator and delegator, "like a guild hall": a receptionist takes the
request; a complicated one gets a meeting with a supervisor who gathers everything, compares it with local knowledge
and operator availability, gives it a skill level and difficulty, and gates who may take it so that a very small model
does not get in over its head. The identity and registration system stays, and treats operators as improving and
being allowed to do more over time. Three kinds of participant: operators, contributors, users.

My reading: this is **delegation with earned trust**. The application never does privileged work itself; it records,
routes, asks and refuses, and trust in each operator is derived from evidence.

## What already exists and is reused

| Need | Existing piece |
|---|---|
| Registered operators with keys and GitHub identities | `governance/identities.json` (seats: claude, mimo, codex, gemini, ubuntu-worker-01), `seat_app_setup.py`, signed attestations |
| Availability and host compute | `governance/seat-status/<seat>.md`, `check_seat_status.py` (lane C112) |
| Typed job intake | `contracts/job-intake-v1.md` (lane C106), packets in `maintenance_ui` |
| Scheduling, claims, receipts, cards | steward cards and receipts, the coordination queue, `foundation guild tick|status` (C113) |
| Tiered trust and a first skill gate | `LOCAL_MODEL_AUTONOMY_POLICY.md`: Tier 0 deterministic gate, Tier 1 local model, Tier 2 automatic escalation, Tier 3 human; `is_high_risk` crate allowlist; a circuit breaker after repeated verification failures |
| Independent review before anything lands | `review_coverage.py`, `merge_gate.py`, the auto-merge gate |
| Interview and approval screens | HUD v2 brief compiler ([06](06-brief-compiler-and-packets.md)), Socratic pinning and the 3-option intent mirror from the Aaroneous architecture |

So the guild hall is mostly **wiring and one new idea**: a per-operator, per-skill trust level that is *earned from
reviewed outcomes* and checked when work is handed out. The autonomy policy's `is_high_risk` list and circuit breaker
are the seed of it, applied to one tier today.

## Roles

| Role | Who | How identified | May do |
|---|---|---|---|
| **User** | a person with a request ("I want this feature") | a local profile (see 6) | state needs, answer interview questions, judge previews, decide matters of taste, see status |
| **Contributor** | a person who also changes code or content | a profile plus, for shared repositories, a signing key | everything a user may, plus propose changes and review others' work within their level |
| **Seat (operator)** | a model or agent with a registered identity | `identities.json` (existing) | take pull-request-stack jobs it is cleared for, produce changes in isolated workspaces, report results, review others |
| **Guild worker** | a local model operated by a sponsor seat | registered with the hall under its sponsor | take advisory work classes (see [07](07-guild-workers-staff-and-inspection.md)); no repository writes |
| **Owner / steward** | the repository owner | `identities.json` `owner` | set policy and levels, veto promotions, final say on repository-level decisions |

In `hud_core` terms (lane C120), `human` and `seat:<name>` actors exist today. Roles extend this with `role:<user|contributor|operator|worker>`, `worker:<name>` and `user:<id>` actor patterns in rules, a backward-compatible addition
planned as a follow-up to C120.

### Authority: taste versus correctness

"The user is always right in matters of taste" needs a boundary, or it becomes either meaningless or unsafe.

- **Taste (user decides, final):** how it looks and reads; names within the project's naming rules; tone; which of
  several *valid* options to take; priorities and order; what is out of scope. The system records each taste decision
  as a preference with its source (feeds the preference log in [the visualizer and twin proposal](09-visualizer-exploration-and-twin.md)) and does not argue it.
- **Not taste (gates decide):** whether it is correct, safe, secure, licensed, and whether it passes the project's
  verification. The user cannot waive these.
- **When a taste choice collides with a gate,** the system says so plainly, names the gate, and offers the nearest
  option that passes. It neither silently overrides the user nor silently complies.
- A request that is *wrong* (it will not do what the user wants) is a correctness matter: the system tells the user
  with evidence, and the user decides what to do next.
## The flow

1. **Reception.** Software plus the light model take the request as a typed intake record (C106): deduplicate against
   open work, answer it directly if it is a question the retained evidence answers (no operator needed), otherwise
   classify it.
2. **Triage.** Software features first: crates and profiles touched, whether `is_high_risk` applies, size of the
   change, whether a deterministic verifier exists. Simple and low-risk work gets a quote (who, how long, cost) and goes
   straight to matching.
3. **Meeting.** Complicated or ambiguous work goes to a supervisor session: the brief compiler's interview
   (Socratic pinning) with the user, the local-area pack (the project's conventions, constraints, past decisions,
   open lanes), and the three-option mirror. The output is a brief with confirmed slots.
4. **Rating.** The brief gets `difficulty` (L0 to L3 by default), `risk` class, required `skills`, and whether it needs
   independent review.
5. **Matching and gating.** Work is first split by class: pull-request-stack work goes to seats, advisory work (see [07](07-guild-workers-staff-and-inspection.md)) to guild workers. A pure function then decides who may take it (section 5). Candidates are ordered by
   cleared level, availability (`seat-status`), host capacity and token budget (HUD v2 settings).
6. **Work.** Operators work in isolated workspaces or branches and never touch the user's tree directly. The Tier 0
   deterministic gate runs on whatever they produce.
7. **Taste checkpoints.** At defined points (a preview, a naming choice, a visible change) the user is asked. The answer
   is final for taste and is recorded.
8. **Review and delivery.** Independent review where required; the result is delivered with its evidence. Merges and
   other repository authority stay with the steward gate (the HUD permission floor).

The application is the scheduler, moderator and delegator in steps 1 to 5 and 7, and a record-keeper in the others.
## Difficulty, skill and the gate

**Job fields:** `difficulty` (L0 trivial and mechanically verifiable, L1 bounded, L2 cross-module or judgement-heavy,
L3 design-level or safety-relevant), `risk` (reusing `is_high_risk` and the crate profiles), `skills` (tags such as
docs, tests, ui, rust-presentation, kernel), `needs_review`.

**Operator profile:** a level **per skill**, not one number, each backed by a list of reviewed outcomes; plus model
facts from configuration (context size, tier, host). A small model's ceiling also comes from configuration, so it
cannot be given work that exceeds its context.

**The gate is a pure function** `can_take(operator_profile, job, availability, budget) -> Allow | Refuse(reason)`:
the operator's level in every required skill is at least the job's difficulty; the operator is not suspended (the
circuit breaker generalised); a high-risk job needs the configured minimum level; the job's review requirement can be
met by someone other than the operator. Unknown or unrated means **higher** difficulty, not lower. Being a pure
function, it gets golden-vector tests like the steward does.

**Earning trust (all numbers are settings):**
- A new operator starts at the lowest level on work that has a deterministic verifier, or on short **qualification
  trials** with known answers. Placement is measured, not self-declared.
- Promotion needs a configured number of outcomes at the current level that were accepted by an independent reviewer,
  with no reversals in a configured window. The reviewer is never the operator, and the owner may veto.
- Demotion or suspension follows a reversal, a failed verification or a policy breach, and levels decay if unused.
- The evidence is the existing receipts and attestations, so the ledger is derived data that can be recomputed and
  audited, not a new source of truth.

**Calibration:** the rating itself is an estimate. The system records rated difficulty against actual outcome so
mis-rated work shows up, and the rating rules are adjusted from that evidence.

**What "improving the model" can honestly mean here:** the application learns *which operator is good at what* and how
to brief each one (routing, trust, context, examples). It does not change any model's weights. Using accepted
outcomes and taste decisions as training data would be a separate decision with consent and is not proposed.

### Outside evidence: what an independent evaluator can and cannot do for a promotion

Owner question, 2026-10-10: is there an independent third party that can quantify the models' capability, so a model
"applies for a raise" by reaching the next milestones, without us running every kind of test in house?

**Independent evaluators exist.** Found by web search on 2026-10-10 from secondary sources (blogs, forum posts,
aggregators); the figures were not checked at the primary sites and are not repeated here. Treat the list as leads.
- **METR** measures the length of software task that an agent completes at a given success rate (a "time horizon").
  Its suite is limited, and estimates near its measurement ceiling carry wide confidence intervals.
- **Epoch AI** publishes a capability index that combines many benchmarks with an item-response-theory model, and runs
  SWE-bench Verified with its own scaffold.
- **Artificial Analysis** publishes a composite intelligence index (it added agentic benchmarks such as Terminal-Bench)
  with speed and price data.
- **Refreshing, contamination-resistant benchmarks** such as LiveCodeBench and LiveBench, the SWE-bench family, and
  (from my own knowledge, not checked now) human-preference arenas and EleutherAI's open `lm-evaluation-harness` for
  running standard benchmarks yourself.

**Why they cannot decide a promotion here.**
1. They mostly cover frontier cloud models. Our local quantised model with our context size and sampling settings is
   not on any of them.
2. A score belongs to model plus scaffold. The same model scores differently under a different agent harness, and ours
   is different.
3. Public benchmarks leak into training data and saturate; a survey of 283 benchmarks reported widespread contamination.
   One secondary report says about half of SWE-bench-passing patches were rejected by maintainers for quality and
   convention reasons: a pass is not the same as mergeable.
4. None of them measure our conventions, our gates or our repositories, which is what a promotion is about.
5. A third party cannot grade our private tasks without seeing them. Independence and secrecy trade off.

**What they are good for:** priors and cross-checks. They suggest which models to admit, the starting level to
try, and routing hints, and a large disagreement between a model's outside ranking and our own ledger is a signal to
investigate. They are **not** promotion criteria.

**Promotion rests on our own milestone suites, built to avoid the "mixed review" problem.**
- Each level has a suite of tasks taken from our repositories' history (a commit before and after, with its tests) or
  small variants of it, **each with a deterministic verifier**: tests, golden vectors, the project gate. Pass or fail
  needs no reviewer's opinion.
- A hidden holdout is rotated so a model cannot memorise it. Results are reported as a pass rate with a confidence
  bound (for example the lower bound of a Wilson interval), because a suite of twenty tasks is a small sample.
- A promotion needs the suite **and** accepted work in the live ledger over time, not one exam.
- Where a verifier cannot exist (taste, readability), a model judge may be used only with these rules: a different
  model family from the candidate, answer order swapped, length controlled, never self-grading, and the judge's
  agreement with test outcomes measured. A reported failure mode is judges that favour answers that look like a
  clean reference over answers that work; agreement with tests is how it would show. The owner's taste samples are the
  human spot-check.
- Review independence: the author, any co-author and any **guide** of the work are never its independent reviewer.
  An Ubuntu local worker guided by an OpenCode session is a party; the guide cannot be the one who clears it.
- The thresholds for each level are settings, so a "raise" is a mechanical result of evidence, not a negotiation.
## Identity and registration

- **Operators:** unchanged. `identities.json`, keys, the seat Apps. Levels are stored beside the profile, not inside
  `identities.json`, so the trust file stays small and reviewable.
- **Users and contributors:** a lightweight **profile**: pseudonymous id, role, creation date, consent flags (what may
  be retained, whether preferences may be stored), nothing else required. Contributors who submit to a shared
  repository additionally register a signing key through the existing route.
- **Stranger-facing use is a different threat model** from the current loopback tool. Their text is untrusted data
  (including prompt injection); they must not reach the owner's checkouts, credentials or other users' data; each
  needs an isolated workspace, quotas and a retention and deletion path; contributions need clear authorship and
  licence terms. **First version: the owner's own machines on the local network (see [03](03-lan-roster-and-domains.md)) with guest profiles.**
  Hosting for remote strangers is a later phase with its own security review before any code.
## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| G1 | roles and profiles in the permission engine; profile store with consent flags | 01 |
| G2 | job rating fields and the pure `can_take` gate with golden vectors; qualification-trial format | G1, C106 |
| G6 | progression ledger derived from receipts and attestations; matching with `seat-status`; Guild CLI hooks | G2, C112, C113 |

## Decisions for this feature

1. The taste/correctness boundary: confirm or amend. OPEN.
2. Levels: four (L0 to L3) as the default; more are a setting. OPEN.
3. Who may promote an operator: an independent reviewer proposes, the owner may veto (recommended). OPEN.
4. The stranger's program lives in a separate repository or workspace (recommended). OPEN.
5. Fine-tuning from outcomes stays off (confirm). OPEN.
6. Outside benchmarks are priors only (confirm). OPEN.

## Review focus

- Is the taste/correctness boundary right, including "the system names the gate and offers the nearest option that passes"?
- Is a per-skill trust level, with promotion and demotion from independent reviewed outcomes, the right mechanism? Is `can_take` as a pure function with golden vectors the right form?
- Is "outside benchmarks are priors, promotion rests on our own deterministic milestone suites" the right policy?
- Are the identity and consent rules for users and contributors adequate for a first version on the owner's own machines?

## Not verified

- Nothing here was built or run. The levels, thresholds and phases are proposals.
- Whether a difficulty rating from software features predicts outcomes; the calibration record is how it would be tested, and it needs data that does not exist yet.
- That the existing receipts and attestations carry enough information to derive a per-skill ledger; I have read the formats but have not built the derivation.
- The strength of isolation needed for strangers; undetermined until a separate security review.
- The names, scope and figures of outside evaluators come from search results and my own knowledge, not from their primary sites.
- Whether our milestone suites can be built from repository history with enough tasks per level; unmeasured.
