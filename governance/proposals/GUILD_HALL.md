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

## 3. Roles

| Role | Who | How identified | May do |
|---|---|---|---|
| **User** | a person with a request ("I want this feature") | a local profile (see 6) | state needs, answer interview questions, judge previews, decide matters of taste, see status |
| **Contributor** | a person who also changes code or content | a profile plus, for shared repositories, a signing key | everything a user may, plus propose changes and review others' work within their level |
| **Operator** | a model or agent: a registered seat, or a local model | `identities.json` (existing) | take jobs it is cleared for, produce drafts into isolated workspaces, report results |
| **Owner / steward** | the repository owner | `identities.json` `owner` | set policy and levels, veto promotions, final say on repository-level decisions |

In `hud_core` terms (lane C120), `human` and `seat:<name>` actors exist today. Roles extend this with `role:<user|contributor|operator>` and `user:<id>` actor patterns in rules, a backward-compatible addition
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
5. **Matching and gating.** A pure function decides who may take it (section 5). Candidates are ordered by
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

## 6. Identity and registration

- **Operators:** unchanged. `identities.json`, keys, the seat Apps. Levels are stored beside the profile, not inside
  `identities.json`, so the trust file stays small and reviewable.
- **Users and contributors:** a lightweight **profile**: pseudonymous id, role, creation date, consent flags (what may
  be retained, whether preferences may be stored), nothing else required. Contributors who submit to a shared
  repository additionally register a signing key through the existing route.
- **Stranger-facing use is a different threat model** from the current loopback tool. Their text is untrusted data
  (including prompt injection); they must not reach the owner's checkouts, credentials or other users' data; each
  needs an isolated workspace, quotas and a retention and deletion path; contributions need clear authorship and
  licence terms. **This proposal therefore keeps the earlier decision (loopback only, one machine)** and treats the
  first version as the owner's machine with guest profiles. Hosting for remote strangers is a later phase with its own
  security review before any code.

## 7. Terminology

The guild-hall language is good for the interface (reception, supervisor, quest-board style status). Code, crates and
settings use plain names: intake, triage, planner, dispatcher, operator, profile, level. This follows the product's
naming rule and the existing "Guild" name for the steward service.

## 8. Everything is configuration

Levels and thresholds, which skills exist, how many outcomes promote, decay, the minimum level per risk class,
which events trigger a taste checkpoint, role permissions, quota sizes and the profile consent defaults all live in
the settings layers of `HUD_V2.md` 4.2a, so the owner can retune them later without a release.

## 9. Phases (each its own lane and review; nothing is claimed here)

| Phase | Deliverable | Depends on |
|---|---|---|
| G0 | This proposal reviewed; decisions in section 10 | owner |
| G1 | Roles and profiles in the permission engine (`role:` and `user:` actors); profile store with consent flags | C120 |
| G2 | Job rating fields and the pure `can_take` gate with golden vectors; qualification-trial format | G1, C106 |
| G3 | Reception and triage on the HUD intake (software classifier, answer-from-evidence) | HUD v2 P3 |
| G4 | Supervisor meeting: brief compiler with the local-area pack and three-option mirror | HUD v2 P3b |
| G5 | Taste checkpoints and preference capture | G4 |
| G6 | Progression ledger derived from receipts and attestations; matching with `seat-status`; Guild CLI hooks | G2, C112, C113 |
| G7 | Hosting for remote users (separate security review first) | owner decision |

## 10. Decisions needed from the owner

1. **First version is single-machine with guest profiles** (recommended), with remote strangers deferred to G7.
2. **The taste/correctness boundary** in 3.1: confirm or amend.
3. **Levels:** four (L0 to L3) as the default; more are a setting.
4. **Who may promote an operator:** an independent reviewer proposes, the owner may veto (recommended). Alternatives:
   owner approves every promotion; or fully automatic by threshold.
5. **Where the stranger's program lives:** a separate repository or workspace from devtools and Aaroneous
   (recommended), so the guild hall operates on it as a project and cannot touch ours.
6. **Fine-tuning from outcomes:** not proposed; confirm it stays off.

## 11. Not verified

- Nothing here was built or run. The levels, thresholds and phases are proposals.
- Whether a difficulty rating from software features predicts outcomes; the calibration record in section 5 is how it
  would be tested, and it needs data that does not exist yet.
- Whether qualification trials can place a small model reliably; to be measured.
- That the existing receipts and attestations carry enough information to derive a per-skill ledger; I have read the
  formats but have not built the derivation.
- That a rule engine extended with roles stays simple; to be checked when G1 is designed.
- The strength of isolation needed for strangers; undetermined until the security review in G7.
