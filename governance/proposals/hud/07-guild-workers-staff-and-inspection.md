---
notation: v1
purpose: Local models as advisory guild workers on a low-permission queue, staff roles as job cards, and inspection access
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:01-settings-and-permissions
  - depends-on: doc:03-lan-roster-and-domains
  - relates-to: doc:LOCAL_MODEL_AUTONOMY_POLICY.md
checked-by: none yet (see "Not verified")
---

# Guild workers, staff roles and inspection (proposal 07)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 07 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## Guild workers: local models are clerks, not authors

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
| W2 forms | fill forms and brief slots (see [06](06-brief-compiler-and-packets.md)) from evidence | field-by-field validation; values must trace to sources or be marked `proposed` |
| W3 research | gather and cite from retained evidence, repositories or approved sources | every citation resolves; claims without a source are labelled `unverified` |

Not available to workers by default, as a setting that only the owner changes: code changes, review verdicts,
anything touching git or GitHub, and any class not listed above. A worker that finds something needing a change opens a
**proposal card** for a seat; it does not make the change.

**Deterministic checks come first.** Because the classes are chosen to be checkable, software validators grade most worker
output (schema, citations resolve, numbers reconcile, quotes match) before any person or model looks at it. A failed
validation bounces the job; repeated failures in a class trip a per-worker circuit breaker, the same idea as
the existing local-promotion breaker. The sponsor spot-checks a configured fraction.

**Trust for workers** uses the same machinery as [08](08-roles-skills-and-trust.md) on this separate ladder: qualification trials with known
answers (a log with planted anomalies, a form with known correct fields, a ledger whose totals must match), then the live
record of validator and spot-check results. The sponsor is a guide, so it is **not** the independent party for the
worker's promotion; another seat or the deterministic validators are.

**Relation to the existing autonomy policy.** `LOCAL_MODEL_AUTONOMY_POLICY.md` lets the local model draft bounded changes
(test files, fixtures) and lets `agentic_registrar::promote` push an isolated `local-agent/*` branch. This proposal's
defaults are narrower, following the owner's direction above: the HUD gives workers advisory classes only and never
calls `promote`. The existing tools are unchanged; the difference should be reconciled in review, and the owner can widen
the classes later with one setting.
## Staff roles are job-card types on the worker queue

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
## Inspection access ("admin") and in-line diagnosis

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
## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| W1 | worker job-card type, the low-permission queue lane, and `worker:` actors in the permission engine | 01, C106 |
| W2 | software validators for W0 and W1 (schema, citations resolve, arithmetic reconciles, quotes match) and the per-worker circuit breaker | W1 |
| W3 | trace record for every worker job; inspector role; rehearsal, sampled audit and live attach | W2, 05 |
| W4 | qualification trials with known answers; worker trust ladder | W2, 08 |
(Proposed labels for this document; not claimed.)

## Decisions for this feature

1. Local models are guild workers for advisory work, not authors for the pull-request stack. DECIDED by the owner, 2026-10-10.
2. OPEN: which seats hold the inspector role by default (proposed: the owner and the frontier seats) and the default sampling rates for audits.
3. OPEN: retention for traces that contain user text.

## Review focus

- Are the four worker classes (W0 to W3) the right set, and is each cheaply checkable by software?
- Is "no repository writes, no branches, no PRs" for workers the intended boundary for now?
- Is the inspector role (specific actions, never the floor) the right meaning of "admin"?
- The conflict with `LOCAL_MODEL_AUTONOMY_POLICY.md` (which lets the local model draft bounded changes and use `promote`): reconcile it, or keep the HUD's narrower default and record the difference.

## Not verified

- Whether inspection traces can be kept at acceptable size and cost.
- Whether a different-family inspector reliably catches a worker asking the wrong thing; to be measured with planted faults in rehearsal.
- That the four worker classes cover the tasks the owner has in mind, and that software validators can check each cheaply; unmeasured.
- Whether qualification trials can place a small model reliably; to be measured.
