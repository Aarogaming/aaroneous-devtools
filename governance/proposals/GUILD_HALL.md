---
notation: v1
purpose: Proposal for the operating model of the HUD application: roles, intake, triage, skill-gated delegation and earned trust
authority: propose
status: proposal, advisory, grants nothing
relations:
  - extends: doc:proposals/HUD_V2.md
  - depends-on: doc:LOCAL_MODEL_AUTONOMY_POLICY.md
  - depends-on: doc:identities.json
  - overlaps: lane:C112
  - overlaps: lane:C113
checked-by: none yet (see "Not verified")
---

# The guild-hall model: roles, intake, skill-gated delegation, earned trust (proposal)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (companion to `HUD_V2.md`, same lane and PR).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. No code, no policy change. Nothing here is accepted until a reviewer other than the
author records it.

## 1. The request, and how I read it

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

## 2. What already exists and is reused (read from `origin/main` and the product repo, 2026-10-10)

| Need | Existing piece |
|---|---|
| Registered operators with keys and GitHub identities | `governance/identities.json` (seats: claude, mimo, codex, gemini, ubuntu-worker-01), `seat_app_setup.py`, signed attestations |
| Availability and host compute | `governance/seat-status/<seat>.md`, `check_seat_status.py` (lane C112) |
| Typed job intake | `contracts/job-intake-v1.md` (lane C106), packets in `maintenance_ui` |
| Scheduling, claims, receipts, cards | steward cards and receipts, the coordination queue, `foundation guild tick|status` (C113) |
| Tiered trust and a first skill gate | `LOCAL_MODEL_AUTONOMY_POLICY.md`: Tier 0 deterministic gate, Tier 1 local model, Tier 2 automatic escalation, Tier 3 human; `is_high_risk` crate allowlist; a circuit breaker after repeated verification failures |
| Independent review before anything lands | `review_coverage.py`, `merge_gate.py`, the auto-merge gate |
| Interview and approval screens | HUD v2 brief compiler (4.3a), Socratic pinning and the 3-option intent mirror from the Aaroneous architecture |

So the guild hall is mostly **wiring and one new idea**: a per-operator, per-skill trust level that is *earned from
reviewed outcomes* and checked when work is handed out. The autonomy policy's `is_high_risk` list and circuit breaker
are the seed of it, applied to one tier today.

### 2.1 Three layers

Owner framing, 2026-10-10: the HUD-HUB manages a **stable Aaroneous core**, with devtools as an **operational layer**
on top, and the **machine-dependent models** we host tied to their machine as local workers.

| Layer | What it is | Rule |
|---|---|---|
| Aaroneous core | the product repository and its crates; the pinned base | changes only through its own gate; devtools depends on it, never the reverse |
| Devtools (operational layer) | the hall: intake, queue, policy, steward and Guild, packets, settings | operates on repositories; holds no product authority |
| Hosted models | the local model on each machine, served by Ollama, vLLM or LM Studio | bound to its machine and its sponsor seat; low permission |

The user is the **guild manager**. The local model is the **staff** (secretary, organiser, bookkeeper, auditor, cleaner),
taking work as job cards at its own level (3.3).

## 3. Roles

| Role | Who | How identified | May do |
|---|---|---|---|
| **User** | a person with a request ("I want this feature") | a local profile (see 6) | state needs, answer interview questions, judge previews, decide matters of taste, see status |
| **Contributor** | a person who also changes code or content | a profile plus, for shared repositories, a signing key | everything a user may, plus propose changes and review others' work within their level |
| **Seat (operator)** | a model or agent with a registered identity | `identities.json` (existing) | take pull-request-stack jobs it is cleared for, produce changes in isolated workspaces, report results, review others |
| **Guild worker** | a local model operated by a sponsor seat | registered with the hall under its sponsor | take advisory work classes (3.2); no repository writes |
| **Owner / steward** | the repository owner | `identities.json` `owner` | set policy and levels, veto promotions, final say on repository-level decisions |

In `hud_core` terms (lane C120), `human` and `seat:<name>` actors exist today. Roles extend this with `role:<user|contributor|operator|worker>`, `worker:<name>` and `user:<id>` actor patterns in rules, a backward-compatible addition
planned as a follow-up to C120.

### 3.1 Authority: taste versus correctness

"The user is always right in matters of taste" needs a boundary, or it becomes either meaningless or unsafe.

- **Taste (user decides, final):** how it looks and reads; names within the project's naming rules; tone; which of
  several *valid* options to take; priorities and order; what is out of scope. The system records each taste decision
  as a preference with its source (feeds the preference log in `HUD_V2.md` 10b) and does not argue it.
- **Not taste (gates decide):** whether it is correct, safe, secure, licensed, and whether it passes the project's
  verification. The user cannot waive these.
- **When a taste choice collides with a gate,** the system says so plainly, names the gate, and offers the nearest
  option that passes. It neither silently overrides the user nor silently complies.
- A request that is *wrong* (it will not do what the user wants) is a correctness matter: the system tells the user
  with evidence, and the user decides what to do next.

### 3.2 Guild workers: local models are clerks, not authors

Owner direction in chat, 2026-10-10: the local models (Qwen and similar) do not feed the pull-request stack directly.
They act indirectly, through cards, maintenance, user requests and operations, as **guild workers**: research,
checking logs, making sure things add up, helping fill in forms. We are not relying on them for large changes at this time.

**Two kinds of member, two kinds of work.**

| | Seat (operator) | Guild worker |
|---|---|---|
| Examples | claude, codex, gemini, mimo (OpenCode on Windows), ubuntu-worker-01 (OpenCode on Ubuntu) | the local model on each machine, served by Ollama, vLLM or LM Studio |
| Work | the pull-request stack: code and document changes, reviews | advisory work: reports, checks, drafts of forms and brief slots, research notes |
| Output goes to | branches and PRs, under the steward gate | the hall: attached to a card, maintenance run, operation or user request. **No repository writes, no branches, no PRs** |
| Accountable | itself (identity, keys, trust ledger) | its **sponsor seat**: the OpenCode seat on the same machine |

**Work classes for workers** (a ladder; the classes, and who may take them, are settings):

| Class | What it does | How the result is checked first |
|---|---|---|
| W0 triage | classify, deduplicate, summarise | schema valid; every item traceable to its source |
| W1 reconcile | read logs and state; check that counts, totals and chains add up | arithmetic re-done by software; each quoted line exists at the cited place |
| W2 forms | fill forms and brief slots (HUD v2 4.3a) from evidence | field-by-field validation; values must trace to sources or be marked `proposed` |
| W3 research | gather and cite from retained evidence, repositories or approved sources | every citation resolves; claims without a source are labelled `unverified` |

Not available to workers by default, as a setting that only the owner changes: code changes, review verdicts,
anything touching git or GitHub, and any class not listed above. A worker that finds something needing a change opens a
**proposal card** for a seat; it does not make the change.

**Deterministic checks come first.** Because the classes are chosen to be checkable, software validators grade most worker
output (schema, citations resolve, numbers reconcile, quotes match) before any person or model looks at it. A failed
validation bounces the job; repeated failures in a class trip a per-worker circuit breaker, the same idea as
the existing local-promotion breaker. The sponsor spot-checks a configured fraction.

**Trust for workers** uses the same machinery as section 5 on this separate ladder: qualification trials with known
answers (a log with planted anomalies, a form with known correct fields, a ledger whose totals must match), then the live
record of validator and spot-check results. The sponsor is a guide, so it is **not** the independent party for the
worker's promotion; another seat or the deterministic validators are.

**Relation to the existing autonomy policy.** `LOCAL_MODEL_AUTONOMY_POLICY.md` lets the local model draft bounded changes
(test files, fixtures) and lets `agentic_registrar::promote` push an isolated `local-agent/*` branch. This proposal's
defaults are narrower, following the owner's direction above: the HUD gives workers advisory classes only and never
calls `promote`. The existing tools are unchanged; the difference should be reconciled in review, and the owner can widen
the classes later with one setting.

### 3.3 Staff roles are job-card types on the worker queue

The local model takes staff duties as ordinary job cards. Each role is a bundle of the worker classes in 3.2, so a role is a
card type with a fixed, checkable output, not a free-form assistant:

| Staff role | Typical cards | Classes used |
|---|---|---|
| Secretary | summarise a thread or a day's events, draft a status note, take a request through intake | W0, W2 |
| Organiser | tag and cross-link documents and lanes, keep the index and the idea graph current | W0, W3 |
| Bookkeeper | reconcile queue rows against PR state, token and budget accounting, lease and receipt tallies | W1 |
| Auditor | classify audit results, internal compliance triage, check that logs and chains are consistent | W1, W3 |
| Cleaner | find stale branches, caches and temporary files and **propose** what to clear | W1, then a seat or the owner acts |

This extends direction already in the product `AGENTS.md` (section 0.1, "Local maintenance operator"): recurring, bounded
work near the workspace; existing tools gather the facts; the model interprets declared evidence and proposes; outcomes of
tools and findings of the model are recorded separately; and a model recommendation grants no deletion authority. The
cleaner is therefore advisory by construction: it lists candidates with the retention rule that qualifies each one, and an
authorised tool or a person performs any deletion.

**The worker queue is a low-permission lane in the same card system.** It is part of the steward and Guild priority and
delegation scheme, with its own priority classes and budgets, and it is the **lowest-risk door into the hall**: a user,
another seat, a contributor or a script may post a card to it, because what a card can cause is bounded by the worker classes
and permissions, not by who posted it. Posting still goes through intake and the permission engine, and every card records
its poster.

### 3.4 Inspection access ("admin") and in-line diagnosis

Owner idea, 2026-10-10: higher models, and people, can step inside a worker operation to check that it is saying and asking
the right things, with administrative privileges.

- **Every worker job leaves a trace:** the brief and slots, the compiled prompt (with template, adapter and model
  versions), the model's outputs and questions, validator results and token counts. Traces are retained evidence, subject
  to the retention and consent settings.
- **Inspector** is a role granted by rule to named seats and to the owner (for example the frontier seats). It carries
  *specific* actions, not general power: `read_trace`, `annotate_trace`, `pause_job`, `resume_job`, `requeue_job`,
  `propose_template_change`. It cannot widen worker classes, skip a validator, change the permission floor or act on the
  repository; those stay with the owner's settings and the steward gate. Admin here means insight and control of the
  *operation*, not of the system.
- **Three inspection modes:**
  1. **Rehearsal (pre-release):** before a new or changed template goes live, an inspector reads the compiled prompt and runs
     it against the qualification set, checking it asks for the right things and nothing more. This is the shadow/inert
     landing the graduation rule already requires.
  2. **Sampled audit (running):** a configured fraction of live traces, higher for new templates and for any trace that failed
     a validator, is reviewed by a higher model from a different family than the worker's model.
  3. **Live attach (on demand):** an inspector or the owner opens a running job's trace, annotates it and may pause or requeue
     it. The worker keeps going on its own unless paused.
- **Cost control:** inspection spends cloud tokens, so rates are settings, scaled by risk and maturity, and tracked in the same
  accounting as everything else.
- **Findings feed back:** an annotation becomes a proposed template change, a validator rule or a trial case, and goes through the
  normal review, so the system learns from inspections instead of repeating them.
- **Privacy:** traces can hold user text. Read access is by rule, and a user's consent flags decide whether their text is retained.

## 4. The flow

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
5. **Matching and gating.** Work is first split by class: pull-request-stack work goes to seats, advisory work (3.2) to guild workers. A pure function then decides who may take it (section 5). Candidates are ordered by
   cleared level, availability (`seat-status`), host capacity and token budget (HUD v2 settings).
6. **Work.** Operators work in isolated workspaces or branches and never touch the user's tree directly. The Tier 0
   deterministic gate runs on whatever they produce.
7. **Taste checkpoints.** At defined points (a preview, a naming choice, a visible change) the user is asked. The answer
   is final for taste and is recorded.
8. **Review and delivery.** Independent review where required; the result is delivered with its evidence. Merges and
   other repository authority stay with the steward gate (the HUD permission floor).

The application is the scheduler, moderator and delegator in steps 1 to 5 and 7, and a record-keeper in the others.

## 5. Difficulty, skill and the gate

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

### 5.1 Outside evidence: what an independent evaluator can and cannot do for a promotion

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

## 6. Identity and registration

- **Operators:** unchanged. `identities.json`, keys, the seat Apps. Levels are stored beside the profile, not inside
  `identities.json`, so the trust file stays small and reviewable.
- **Users and contributors:** a lightweight **profile**: pseudonymous id, role, creation date, consent flags (what may
  be retained, whether preferences may be stored), nothing else required. Contributors who submit to a shared
  repository additionally register a signing key through the existing route.
- **Stranger-facing use is a different threat model** from the current loopback tool. Their text is untrusted data
  (including prompt injection); they must not reach the owner's checkouts, credentials or other users' data; each
  needs an isolated workspace, quotas and a retention and deletion path; contributions need clear authorship and
  licence terms. **First version: the owner's own machines on the local network (section 7) with guest profiles.**
  Hosting for remote strangers is a later phase with its own security review before any code.

## 7. Local network first: the roster, the hall and the wire

Owner direction in chat, 2026-10-10: local area network first; the Ubuntu worker works alongside the Windows-hosted
worker; each machine also runs an OpenCode session, giving four workers on the LAN; then Codex, Claude and Gemini;
and the owner supplies inspiration and guidance. Treat it as a very advanced adventurers' or merchants' guild.
This **replaces** the earlier "loopback only" decision (`HUD_V2.md` section 9, decision 4) for the guild's own
machines; remote strangers stay deferred (G7).

### 7.1 Roster: seats, workers and instances

Corrected 2026-10-10 from the owner's statement: the OpenCode session on Windows is the `mimo` seat; the OpenCode
session on Ubuntu is the `ubuntu-worker-01` seat ("Ubuntu-1"). Both run OpenCode, and each operates a local model on its
own machine. Confirmed by the owner, 2026-10-10: **MiMo is the model behind OpenCode on both machines, and the seat name `mimo` names
the model identity; OpenCode is the application used to reach it.** The Ubuntu seat is the same model on the second machine
under a host-specific seat name. The application and the model are recorded separately on each instance.

| Member | Kind | Machine | Notes |
|---|---|---|---|
| `mimo` | seat (operator), OpenCode with a cloud model | Windows | sponsor of the Windows guild worker |
| Windows guild worker | local model (Ollama, vLLM or LM Studio) | Windows | advisory classes only (3.2); GPU shared with the UI and the 3D view |
| `ubuntu-worker-01` | seat (operator), OpenCode with a cloud model | `aaroneoushome` (RTX 4060 per its seat-status file) | sponsor of the Ubuntu guild worker |
| Ubuntu guild worker | local model | Ubuntu | advisory classes only (3.2) |
| `claude`, `codex`, `gemini` | seats (operators), frontier cloud-backed agents | wherever each session runs | which are local sessions and which are cloud sessions decides whether they can reach the LAN directly |
| Guildmaster | the owner | Windows | gives inspiration and guidance; see 7.4 |

That is five seats, two guild workers and the owner. **Operator (a registered identity with a trust ledger) and instance
(one running process: host, runtime, model, capacity, heartbeat) remain different things:** trust and levels attach to
the seat and its skills; availability, capacity and leases attach to the instance. No new seat is needed for the
Windows machine; its local model is a worker under `mimo`. The worker instances are registered with the hall under their
sponsor, not in `identities.json`, because they do not sign attestations or reviews.

### 7.2 One hall, workers that pull

- **One hall** (the HUD core with its MCP/HTTP adapter) runs on one machine; I recommend the Windows machine, where the
  owner works. Every worker is a client that **registers, heartbeats and pulls** work. The hall never needs to reach
  into a worker, so only the hall opens a port and workers behind other networks still work. This is the same
  "models ping the hall" shape already in `HUD_V2.md`.
- **Register and heartbeat:** an instance announces itself (operator, host, runtime, model, skills it claims, resource
  slots) and refreshes on a short interval. Missed heartbeats mark it `offline`. This supersedes the hand-edited
  `seat-status` files for live use; the files remain as the retained, reviewed record (lane C112).
- **Leases, not trust in clocks:** a job is handed out under a lease measured by the hall's own clock; a dead worker's
  lease expires and the job returns to the board. Two machines will not have perfectly synchronised clocks, so
  nothing depends on comparing their timestamps.
- **Artefacts move through git, not through the hall.** Workers push branches; the hall passes references (branch,
  commit, digests). This is how the steward already works and keeps the hall small.
- **Resource slots per host,** as settings: for example one GPU slot, one heavy-build slot, a VRAM budget. The
  seat-status file already speaks of "the heavy Cargo slot" and an idle GPU; the hall schedules against those so two
  workers do not fight over one graphics card, and the 3D view yields to the model (HUD v2 4.3b).

### 7.3 The wire: what changes when it leaves loopback

- The bind address is a setting (`network.bind`). Default stays `127.0.0.1`. A LAN address must be named explicitly;
  `0.0.0.0` is never the default. The accepted `Host` names are a configured list, replacing the single exact
  `127.0.0.1:8787` match.
- Bearer tokens on a LAN cross the network, so they need an encrypted path. Options, in my order of preference:
  (a) an encrypted overlay the machines already share (the Ubuntu worker's address `100.109.8.12` is in the range
  commonly used by a Tailscale-style overlay; **I have not verified that one is in use**); (b) TLS with a pinned
  certificate between the hall and each worker; (c) an SSH tunnel per worker. A setting (`network.require_encrypted`,
  default true) makes the hall refuse tokens over plain HTTP unless the interface is declared trusted.
- Per-instance tokens, hashed at the hall, revocable, scoped by the permission engine (default read-only, `HUD_V2.md`
  4.2a). A worker that is only registered and pulling can read its card and work list; reporting results and running
  operations remain `ask` or `deny` until the rules say otherwise.
- All the earlier limits stay: body caps, per-instance rate limits, untrusted-data handling of everything a worker sends.
- Cloud sessions that run off the LAN cannot reach it. For them the existing GitHub route (cards, receipts,
  queue) remains the channel, and the hall reads it. That is the same arrangement as today, not a regression.

### 7.4 The guild vocabulary mapped onto real parts

| Guild idea | Real part |
|---|---|
| Quest board | the job queue: the C106 intake records plus the coordination queue |
| Rank and skill badges | per-skill levels from section 5, derived from receipts and attestations |
| Charter | the settings layers and the permission floor (merge, push, review and attest are never delegated) |
| Party | a job staffed by several operators with roles (draft, implement, review); the author is never the reviewer |
| Reputation ledger | the derived trust ledger (section 5); recomputable from retained evidence |
| Guildmaster's guidance | owner directives and taste decisions: entered through the same intake with attribution, they raise priority or set direction, are logged as preferences, and are never a bypass of a gate |
| Merchant side | a resource economy in compute terms, not money: GPU minutes on a host, local model time (cheap), cloud tokens (costly). Jobs carry a quote; workers advertise capacity; budgets (settings) cap spending; the hall prefers the cheapest cleared operator that meets the deadline |

Everything in the table is configuration over parts that exist or are proposed; none of it is new authority.

### 7.5 Domains: one hall per repository and machine, with crossplay

Owner idea, 2026-10-10: run several devtools sessions with different clocks, one pointed at each repository. The Ubuntu
machine works on the Aaroneous repository, with devtools as the source of operations and Aaroneous as the pristine
base; devtools keeps advancing; because devtools is built on Aaroneous, both improve by dogfooding.

**Domain** = one hall + one repository + one machine, with its own settings layers, queue, permission floor and
**clock** (tick interval, lease length, heartbeat; all settings). A fast clock suits the interactive Windows hall; a slow
batch clock suits the Ubuntu build machine.

**Proposed first arrangement:** hall A on Windows over the devtools repository; hall B on Ubuntu over the Aaroneous
repository. The two halls do not share mutable state. Each repository keeps its own coordination queue, and lanes are
claimed only by a claim PR in the repository that owns the lane (the existing rule), so there is no cross-hall
lock to get wrong.

**Crossplay** means lending an instance: a worker registered with its home hall serves a job from the other hall under a
*visiting contract*. The home hall keeps the identity and the trust ledger; the visited hall issues leases scoped to its
repository; credentials are never shared; the result returns as git refs and receipts. Trust earned while visiting counts
in the home ledger only for the skills it actually exercised there, and only after the home hall verifies the receipts.
Skills are domain-specific: kernel-style Rust in Aaroneous is not the same skill as documents in devtools.

**The dogfood loop and the dependency rule.**
- Devtools pins an Aaroneous revision as its base and moves the pin by reviewed pull request.
- Aaroneous stays pristine because it changes only through its own gate and its graduation rule (code arriving from
  outside is proven in its origin, placed behind a workspace trait, landed inert, and the origin copy deleted).
- Hall B proposes changes to Aaroneous as pull requests under the product's rules; devtools depends on Aaroneous and
  never the reverse, and Aaroneous never depends on devtools.
- Whether hall B's operators may merge in the Aaroneous repository is the owner's decision; the default here is that
  they open pull requests and do not merge.

## 8. Terminology

The guild-hall language is good for the interface (reception, supervisor, quest-board style status). Code, crates and
settings use plain names: intake, triage, planner, dispatcher, operator, profile, level. This follows the product's
naming rule and the existing "Guild" name for the steward service.

## 9. Everything is configuration

Levels and thresholds, which skills exist, how many outcomes promote, decay, the minimum level per risk class,
which events trigger a taste checkpoint, role permissions, quota sizes and the profile consent defaults all live in
the settings layers of `HUD_V2.md` 4.2a, so the owner can retune them later without a release.

## 10. Phases (each its own lane and review; nothing is claimed here)

| Phase | Deliverable | Depends on |
|---|---|---|
| G0 | This proposal reviewed; decisions in section 10 | owner |
| G0b | Network and roster: operator versus instance, register/heartbeat/lease, resource slots, `network.*` settings, encrypted path chosen (section 7) | G0, C112 |
| G1 | Roles and profiles in the permission engine (`role:` and `user:` actors); profile store with consent flags | C120 |
| G2 | Job rating fields and the pure `can_take` gate with golden vectors; qualification-trial format | G1, C106 |
| G3 | Reception and triage on the HUD intake (software classifier, answer-from-evidence) | HUD v2 P3 |
| G4 | Supervisor meeting: brief compiler with the local-area pack and three-option mirror | HUD v2 P3b |
| G5 | Taste checkpoints and preference capture | G4 |
| G6 | Progression ledger derived from receipts and attestations; matching with `seat-status`; Guild CLI hooks | G2, C112, C113 |
| G7 | Hosting for remote users (separate security review first) | owner decision |

## 11. Decisions needed from the owner

1. **First version is the owner's machines on the LAN** (section 7), with guest profiles; remote strangers are deferred to G7.
2. **The taste/correctness boundary** in 3.1: confirm or amend.
3. **Levels:** four (L0 to L3) as the default; more are a setting.
4. **Who may promote an operator:** an independent reviewer proposes, the owner may veto (recommended). Alternatives:
   owner approves every promotion; or fully automatic by threshold.
5. **Where the stranger's program lives:** a separate repository or workspace from devtools and Aaroneous
   (recommended), so the guild hall operates on it as a project and cannot touch ours.
6. **Fine-tuning from outcomes:** not proposed; confirm it stays off.
7. **Roster.** RESOLVED by the owner in chat, 2026-10-10: `mimo` is the Windows OpenCode seat and `ubuntu-worker-01` is the Ubuntu OpenCode seat; the local models are guild workers under them (3.2). Confirmed: "Mimo" is the model identity behind OpenCode on both machines; OpenCode is the application.
8. **Which machine hosts the hall,** and how the machines are joined (existing overlay network, certificate-pinned TLS, or SSH tunnels).
9. **Which of Codex, Claude and Gemini run as local sessions on these machines,** and which as cloud sessions.
10. **Domains (7.5):** first arrangement hall A on Windows over devtools, hall B on Ubuntu over Aaroneous; and whether hall B's operators merge in the Aaroneous repository (default: pull requests only).
11. **Outside benchmarks are priors only;** promotion rests on milestone suites built from our own repositories with deterministic verifiers (5.1). Confirm.
12. **Inspectors (3.4):** which seats hold the inspector role by default (proposed: the owner and the frontier seats), and the default sampling rates for audits.
13. **Prompts are reviewed files** in the repository (proposed), so a prompt change is a reviewed change; the alternative is settings-only prompts.

## 12. Not verified

- Nothing here was built or run. The levels, thresholds and phases are proposals.
- Whether a difficulty rating from software features predicts outcomes; the calibration record in section 5 is how it
  would be tested, and it needs data that does not exist yet.
- Whether qualification trials can place a small model reliably; to be measured.
- That the existing receipts and attestations carry enough information to derive a per-skill ledger; I have read the
  formats but have not built the derivation.
- That a rule engine extended with roles stays simple; to be checked when G1 is designed.
- The strength of isolation needed for strangers; undetermined until the security review in G7.
- Whether inspection traces can be kept at acceptable size and cost, and what retention the owner wants for traces that contain user text.
- Whether a different-family inspector reliably catches a worker asking the wrong thing; to be measured with planted faults in rehearsal.
- The roster in 7.1 is the owner's statement of 2026-10-10; whether the seat names in `identities.json` match the running OpenCode sessions was not checked against the machines.
- That the four worker classes (W0 to W3) cover the tasks the owner has in mind, and that software validators can check each cheaply; unmeasured.
- The names, scope and figures of outside evaluators in 5.1 come from search results and my own knowledge, not from their primary sites.
- Whether our milestone suites can be built from repository history with enough tasks per level; unmeasured.
- That the machines share an encrypted overlay network; the `100.x` address is only suggestive.
- How the OpenCode sessions authenticate to GitHub and report today.
- That a pull-and-heartbeat design matches how each tool (OpenCode, Codex, Claude, Gemini) can actually call the hall; MCP support in each client must be tested (HUD v2 P1b).
