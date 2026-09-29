# Evidence-First Living Operations

## Status

This is the Aaroneous operations standard for keeping governance, product
contracts, documentation and automation aligned with demonstrated behavior.
It applies the useful parts of Knowledge-Centered Service, docs-as-code,
living/executable documentation and test-driven bug fixing to both product and
companion work.

Its rule is simple: **a discovered discrepancy is not complete work until its
evidence and disposition are durable.** “Documentation first” means record the
expected behavior and the evidence state before changing implementation. It
does not mean presenting an unproved hypothesis as a product fact.

## Required loop

```text
observation
  -> recorded hypothesis
  -> reproducible evidence
  -> classified gap or rejected premise
  -> contract/documentation + corrective disposition
  -> isolated implementation and native verification
  -> independent acceptance and current-state update
```

1. **Observe.** Record the exact source, report, command output, user-visible
   behavior or review claim. Preserve the revision, paths and timestamp.
2. **State the hypothesis.** Say what is expected, what was observed, and what
   would disprove the concern. Until reproduced, label it `Hypothesis`, not
   defect, regression or completion.
3. **Make it reproducible.** Prefer an automated test, fixture, schema check,
   gate invocation or deterministic command. A retained command, exit result
   and revision are the minimum when a test cannot yet exist.
4. **Classify.** Resolve it as a proved gap, duplicate, false premise,
   accepted exception, or blocked dependency. A proved behavioral gap receives
   an executable regression test whenever that is practical; an API or policy
   gap receives a contract/fixture check.
5. **Update the durable facts.** Update the appropriate product contract,
   architecture/API/service document, and the Foundry gap register. Create or
   link a bounded queue lane, blocked owner decision, or explicit rejection.
   The queue remains the only source of claims.
6. **Rectify under normal authority.** The owner of an exact-path lane changes
   only its declared scope, runs the selected native gate, and records actual
   evidence. Models may draft evidence or tests but cannot classify, claim,
   accept, or merge.
7. **Independently accept and close the loop.** A different reviewer verifies
   scope, proof and gates. The corresponding register entry and user-facing
   documentation state what changed, what remains blocked, and what evidence
   makes the present statement true.

## What counts as living documentation

The strongest documentation is executable and revision-bound:

| Subject | Living record |
| --- | --- |
| Product behavior | Focused regression test plus product contract or architecture note |
| Native conformance | `xtask`/AST/Cratify gate invocation, fixture and retained exit result |
| Broad API/MCP behavior | Versioned capability schema plus authorization and request/response fixtures |
| Foundry coordination | Queue lane, evidence record, virtual-patch digest and independent review |
| Local-model assistance | Versioned evidence bundle, proposal schema, deterministic validation and reviewer disposition |
| Operational procedure | Explicit input/output/pause/restart test or a documented blocked limitation |

Narrative documentation explains the decision and boundaries; it never
substitutes for a test, schema or retained command when one is available.

## Accuracy guardrails

- Do not silently “fix the docs” by changing a claim to match unverified code.
  Preserve the earlier statement, evidence and correction rationale.
- Do not call a one-off local result a general property. State sample size,
  environment and the reopen condition.
- Do not create busywork. A lane must name an affected boundary, exact scope,
  acceptance evidence and dependency state.
- Do not let a passing check erase a design gap. If the check cannot express
  the requirement, create the contract/test/checker lane first.
- Do not use a model verdict as evidence. It may identify a hypothesis; only
  source, fixture, gate or independently reviewed result advances the record.

## Mechanical enforcement and review

C83 extends the existing coordination-evidence workflow rather than creating
a parallel checker. Its fixtures must reject an open Foundry register entry
without source evidence or a queue/blocked/rejected disposition, and reject a
claimed closure whose linked lane lacks independent acceptance. It must accept
an explicitly rejected or blocked finding with its owner decision.

Reviewers ask four questions:

1. Is the observation distinguishable from a hypothesis?
2. Is the evidence reproducible and bound to a revision/environment?
3. Does the documented contract describe the observed system rather than the
   desired system?
4. Does the gap have a bounded corrective lane, explicit block, or rejection?

If any answer is no, the handoff is incomplete—not because more prose is
needed, but because the system lacks a durable next action.
