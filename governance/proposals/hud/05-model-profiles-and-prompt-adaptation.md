---
notation: v1
purpose: Per-model prompt profiles and adapters, chosen by measurement and reviewed as files
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:06-brief-compiler-and-packets
  - depends-on: doc:04-token-optimization
  - informs: doc:07-guild-workers-staff-and-inspection
checked-by: none yet (see "Not verified")
---

# Model profiles and prompt adaptation (proposal 05)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 05 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## Model profiles and prompt adaptation (software-level, model-biased)

Owner question, 2026-10-10: can prompts be enhanced and adapted per model at the software-system level? Yes, and a hand-made
example already exists: the product `AGENTS.md` (section 0.1) tells operators, for the local Qwen model, to send a calibrated
system prompt that suppresses long reasoning traces, use temperature 0 and leave generous token headroom. The proposal is to
move that kind of knowledge out of prose and scripts into data and code that every route shares.

- **Model profile (configuration, keyed by backend and model digest):** preferred prompt shape (delimiters, section order,
  how examples are shown), context size and output ceiling, how structured output is obtained (a schema or grammar when the
  backend supports it, otherwise a repair loop), sampling defaults, the dense-handle encoding that was measured for that
  tokenizer (item 9 of [04](04-token-optimization.md)), and known quirks with their counter-measures.
- **Adapter:** a pure function from a compiled brief (confirmed slots, context pack) and a profile to the prompt actually sent.
  The brief is the same for every model; only its rendering changes. Parsing the answer back is adapted the same way
  (for example, tolerant extraction of a JSON block).
- **Prompts are versioned files** (proposed: in the project settings area of the repository), so changing one is a reviewed
  change with a diff. Cache keys include template version, adapter version and model digest, so a change cannot return a stale
  answer.
- **Chosen by measurement, not folklore:** variants are compared on the qualification suite of [the roles and trust proposal](08-roles-skills-and-trust.md), tuned on
  one split and confirmed on a held-out split to avoid overfitting to the test. The comparison, the winner and the model digest
  are retained, and are re-run when the model or its quantisation changes.
- **Guardrails:** an adapter changes wording, format and parameters only. It cannot add capabilities, widen permissions, drop a
  confirmed slot or alter the permission floor. A round-trip test checks that every confirmed slot reaches the prompt, and the
  compiled prompt is logged in the trace ([the guild workers proposal](07-guild-workers-staff-and-inspection.md)).
- **Cost:** the profile is also where the cheapest correct representation is chosen per model (plain text where the model must
  read, dense handles where it must only carry an identifier).
## Budget check before dispatch (Gemini seat, board entry 97)

A profile carries the means to check a compiled prompt against the model's context window **before** dispatch, so truncation is found by software and not at
inference: the backend's own token count where it offers one, otherwise a conservative estimator with a stated margin. Token counts are tokenizer-specific, so the
estimate is per profile and is replaced by the provider-reported count once a call has been made (see [04](04-token-optimization.md) for the measurement).
**Falsifying experiment (NOT RUN):** compile a brief larger than the profile's window and confirm that dispatch is refused before any model call.

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| P4 (part) | model profiles, adapters, variant comparison on the qualification set, retained results | 04, 06, 08 |

## Decisions for this feature

1. Prompts are reviewed files in the repository (proposed), so a prompt change is a reviewed change; the alternative is settings-only prompts. OPEN.
2. Glue model: the existing Ollama `qwen3.5:9b-q6` endpoint is the initial value of a `models` setting, not a constant. DECIDED. Local models are guild workers for advisory work (see 07).

## Review focus

- Is moving per-model knowledge (such as the calibrated system prompt for Qwen) into data and a pure adapter the right design?
- Do the guardrails (wording, format and parameters only; round-trip check that every confirmed slot reaches the prompt) hold?
- Is tuning on one split and confirming on a held-out split enough to avoid overfitting the test?

## Not verified

- Whether the qualification suites exist yet in a form that can compare prompt variants; they are proposed in 08.
- That the profile keys (backend plus model digest) can be read from every backend.
