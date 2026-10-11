---
notation: v1
purpose: Turn an inquiry into a typed job and a survey into a pre-built brief that compiles deterministically into one background prompt
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:01-settings-and-permissions
  - overlaps: lane:C106
  - informs: doc:05-model-profiles-and-prompt-adaptation
checked-by: none yet (see "Not verified")
---

# Brief compiler and packets (proposal 06)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 06 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## Inquiry to job to answer

1. A user types a question or request in the HUD (or a seat posts one). The HUD turns it into a **typed intake record**
   following `contracts/job-intake-v1.md` (lane C106), so one record format serves humans, seats and the CLI.
2. **Software classifier** (no model): keyword/structure rules and queue lookups decide the class: *answerable from
   retained evidence*, *operation to run*, *research*, *blueprint*, or *needs a person/cloud model*.
3. **Local model as glue**: for the classes that need language, it receives a context pack (see [04](04-token-optimization.md)) and does one bounded
   job: summarise, draft the research packet, propose blueprint sections, or choose among the already-listed
   operations. Its output is a structured proposal, never a direct action.
4. The HUD shows the proposal; the user approves, edits or rejects. Approved operations run through the existing
   tools; results are retained. Anything beyond the local model's reach is queued as a job for a cloud seat with the
   pack attached.
## Brief compiler: from a conversation to one background prompt (R9)

The pipeline is built so that **the light model never writes the final prompt**. It fills slots in a pre-built brief;
software assembles the prompt.

1. **Brief template** (configuration, R8): a typed schema chosen by the inquiry class, with named slots such as
   `who` (requester, audience, executing seat/tier), `what` (task, scope, out of scope), `why` (goal, decision it
   serves), `when` (deadline, urgency, schedule), `how` (method, constraints, tools allowed), `inputs` (evidence
   references by digest), `acceptance` (what done looks like, how it is checked), `budget` (tokens, time, tier ceiling)
   and `risk` (what must not happen). Each template marks which slots are required. This extends the existing packet
   types and the C106 intake record; it does not create a third format.
2. **Survey and conversation**: the light model asks one targeted question at a time, for the empty required slots
   only, offers choices drawn from the retained evidence and the queue where it can, and writes its answer into a slot
   as structured output validated against the schema.
3. **Slot states**: `empty`, `derived` (filled by software from the queue or state), `proposed` (filled by the model),
   `confirmed` (accepted or edited by the user), `unverified` (a claim with no evidence). Only `confirmed` and
   `derived` slots reach the compiled prompt unmarked; `proposed` ones are shown for approval, never silently used.
4. **Completeness gate**: a brief cannot be dispatched while a required slot is empty or merely proposed. The user is
   asked only for those, which is where the interaction saving comes from.
5. **Compile**: a deterministic renderer turns the confirmed slots plus a context pack into the final prompt. The same
   brief always compiles to the same prompt, so it is testable, cacheable and cheap to diff.
6. **Dispatch**: the compiled brief becomes a job for the background pipeline with its routing, budget and permission
   level taken from the slots and the settings. The whole conversation is retained as evidence, but **only the
   compiled prompt goes to the larger model**, not the transcript: that is the token saving, and it makes the large
   model's input small, structured and reviewable. Results return as a job result and are presented for review.
## Blueprint and research packets with the user in the loop

Today the packets are generated and shown. Proposed: each packet is a **document with revisions** in the HUD: the model
drafts, the user edits fields in place or comments, the model revises only the affected sections (using the
existing `refine-blueprint`), and each accepted revision is retained with its source evidence digests. "Approve"
produces the existing build-job packet. This keeps the packet logic in `packets.rs` and adds an editing surface and
a revision history, nothing more.
## When the light model's answer is malformed, and stable section anchors

Raised on the Round Table by the Gemini seat (entry 70, 2026-10-11) and adopted here.

- **Bounded repair, never a stall.** A slot answer that is malformed JSON, out of schema or the wrong type is not accepted and does
  not advance the survey. The survey step follows a fixed ladder, each rung a setting: validate; retry once with the validator's
  error quoted back; fall back to a rule-based extractor for slots that have one; then ask the person directly with a plain form for
  that slot. Every failure type and rung used is recorded in the trace so the failure rate can be measured. The survey never blocks
  waiting for the model.
- **Stable anchors.** Blueprint and brief sections carry anchor ids that do not depend on their text, so a refinement can target
  one section and a diff shows only that section. Revisions replace a section by anchor and never resend the whole transcript.
- **Falsifying experiment (credit: Gemini seat, NOT RUN):** feed 50 raw task inquiries through a local brief-slot extractor and
  measure whether malformed or mis-typed output exceeds 15% without the retry step, then with it.

## Alignment with the Aaroneous architecture

The product architecture (`docs/architecture/human_interface_intent_mirror.md`) already specifies Socratic Vector Pinning along three axes (invariants, dependencies, trade-offs) and a three-option intent mirror (conservative, redesign, quick validate). They are the same idea as the brief compiler, so the brief template's slots should include the three axes and the approval screen for a brief should be the three-option mirror. I found no code for either in `crates/*.rs`, so I read them as specification (not verified beyond that search).

## Phases for this feature

| Phase | Deliverable | Depends on | Exit evidence |
|---|---|---|---|
| P3 | inquiry intake in the HUD to typed jobs with a software classifier and answer-from-evidence | C106 | an inquiry answered from retained evidence with zero model calls |
| P3b | brief compiler: templates as configuration, slot states, completeness gate, deterministic compile, dispatch as a background job; survey driven by the light model | P3, C106 | the same brief compiles to the same prompt twice; a missing required slot blocks dispatch |
| G3, G4, G5 | reception and triage; supervisor meeting with the project-knowledge pack and three-option mirror; taste checkpoints and preference capture | P3b | see 08 |

## Decisions for this feature

1. First brief templates: research, blueprint and build (the existing packet types); review and incident later. DEFAULT, not separately answered.
2. Mandatory slots per template are set in configuration; the exact lists are OPEN.

## Review focus

- Is "the light model never writes the final prompt; it fills slots, software compiles" the right division?
- Are the slot states (`empty`, `derived`, `proposed`, `confirmed`, `unverified`) and the completeness gate enough to keep a wrong guess from slipping through?
- Does this extend the C106 intake record and the existing packet types, rather than creating a third format?

## Not verified

- That the C106 intake contract, as written, fits the inquiry and seat-ping cases (I read it; I have not exercised it).
- That a light local model fills brief slots reliably enough as structured output; to be measured against the slot schema before the survey is trusted.
- Whether HIAL, Socratic pinning and the intent mirror exist anywhere other than the specification; my search of `crates/*.rs` found none.
