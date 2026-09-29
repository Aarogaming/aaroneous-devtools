# Foundry control transition

## Decision

Aaroneous is the product engine. It owns component contracts, product policy,
native `xtask` verification, crate topology, and the final Git history.
`aaroneous-devtools` is the Foundry control transformer: an explicit-input
operator that selects a pinned Aaroneous revision, coordinates bounded work,
maintains pre-Git virtual patch history, runs declared gates, and presents
reviewable evidence.

Devtools does not become an Aaroneous dependency, duplicate product policy, or
replace Git. A virtual patch set is pre-staging history; a materialized Git
branch and its PR remain the integration record.

```text
explicit Aaroneous revision + capability manifest
                    |
                    v
devtools Foundry: matrix -> reservation -> proposal -> virtual patch -> gates
                    |                                           |
                    |                                      reviewer selection
                    v                                           v
             evidence ledger                         isolated Git materialization
                                                              |
                                                              v
                                                    Aaroneous native gates + PR
```

## Authority map

| Concern | Aaroneous owns | Devtools Foundry owns | Rule |
| --- | --- | --- | --- |
| Product conformance | Constitution, crate contracts, AST/xtask gates | Invocation, captured evidence, readable failure report | Devtools never declares a clean scan to be product proof. |
| Change representation | Product diff and final Git history | C73 virtual patch-set schema, digest, parent/proposal lineage | A patch set cannot silently mutate a checkout. |
| Verification | Native commands and their meaning | Explicit revision/worktree selection and retained command results | Gates are declared by the selected revision, not guessed from the host. |
| Coordination | None required for product runtime | Queue, reservations, reviewer capacity, handoffs and review debt | Open claim PRs are reservations even before they reach `main`. |
| Local-model assistance | No runtime dependency | Bounded evidence bundles, proposal artifacts, validator and disposition | Models draft or classify; they never claim, execute, accept, or merge. |
| Versioning | Git branches, commits, PRs and tags | Immutable virtual patch revisions and evidence digest links | Do not reimplement Git. |

## Current-component disposition

| Existing component | Disposition | Foundry role / transition condition |
| --- | --- | --- |
| `virtual_patch` and its versioned contract (C73) | Keep | Canonical pre-Git proposal format; add consumers before adding a competing patch loop. |
| `local_proposal` and its schema | Keep and wrap | The only direct local-model boundary; feed it an explicit evidence bundle and retain its proposal. |
| `local_worker` and approved `worker-jobs/` inbox | Merge operationally | One deterministic support-worker service, shadow-only until restart, cancellation and disposition evidence is proven. |
| `multithreader` / `scheduler` | Merge, do not multiply | Supply reservation, quiet-window and capacity policy to the Foundry matrix; retire duplicate loops only after a replacement passes replay tests. |
| `agentic_registrar::promote` | Constrain then replace at the seam | No direct checkout mutation as the preferred path; future promotion consumes reviewer-approved virtual patch materialization. |
| C76 `conformance` module | Wrap as a rejector | A pure Drafted-to-Reviewable filter beside compile checks; never a conformance authority and not automatically wired until its claim and review land. |
| `apply_snippet.rs`, `dump.rs`, `overseer_snippet.rs`, `db_reader/` | Retire or adopt by separate handoff | They remain untracked prototypes, not Foundry inputs; no silent adoption. |
| Aaroneous `xtask`, `ast_auditor`, Cratify and crate tests | Wrap | Invoke through an explicit capability manifest; do not copy their logic into devtools. |

## Machine state model

The machine ledger is append-only and records explicit IDs, patch digests,
selected revision, commands, exit results, reviewer identity and timestamps.
The Markdown queue is the human coordination view; it must cite the ledger
record rather than becoming a second mutable implementation state store.

```text
Discovered -> Reserved -> EvidencePrepared -> Proposed -> PatchValidated
    -> Reviewable -> Materialized -> Verified -> PRSubmitted
    -> Accepted | Rejected | Blocked | Expired
```

- `Reserved` requires an unowned Ready lane and an exact scope lease.
- `Proposed` is only a structured local/cloud model artifact.
- `PatchValidated` verifies schema, base revision, allowed paths and digest.
- `Materialized` requires a selected reviewer and an isolated worktree.
- `Verified` records the selected Aaroneous commands and their actual exits.
- `Accepted` requires a different agent from the implementer. Until then the
  visible state is `Review`, never `Done`.

## Local LLM assistance pattern

The stable pattern is evidence-in, proposal-out:

1. The integration owner selects a declared evidence bundle and acceptance
   question from a claimed lane.
2. Devtools compiles the bounded prompt with `qwen3.5:9b-q6`, temperature
   `0.0`, at least 4096 tokens, explicit endpoint/model/output paths, and the
   no-command/no-write policy preamble.
3. The worker emits a versioned proposal: scope, cited observations, risks,
   test plan and optional scratch patch.
4. Deterministic validators reject malformed output, missing evidence links,
   disallowed paths and unsupported state transitions.
5. An owner may adopt a proposal into a virtual patch set; a different agent
   reviews it before materialization or acceptance.

Offline Ollama, malformed output, exhausted capacity, or an occupied
reservation are normal `Blocked` outcomes. They do not trigger a retry storm
or a synthetic completion claim.

## Transition sequence

1. **Map and pin.** Add an Aaroneous capability manifest that names the
   selected revision, product gates, required inputs and output contracts.
2. **Shadow matrix.** Have devtools compute reservations, eligible gates,
   review debt and worker liveness with no repository writes.
3. **Use C73 end to end.** Route one owner-reviewed local proposal through a
   virtual patch set, validation and isolated materialization; retain replay
   evidence.
4. **Unify service lifecycle.** Fold worker/scheduler behavior into one
   controllable Foundry service with `status`, `serve`, `pause`, `resume`,
   `dispatch`, `review`, `materialize` and `tail`. No Windows Task Scheduler
   is the control plane; an OS adapter may only launch the service.
5. **Graduate writes selectively.** Enable claim, PR publication, or other
   mutations one transition at a time after shadow replay and independent
   review. Auto-merge and self-acceptance remain out of scope.

## Acceptance for this transition

This document is a boundary decision and migration map, not proof that a
Foundry service exists. A future implementation lane must name the surviving
components, its state-store contract, replay fixture, pause/cancellation
behavior, and independent acceptance gate before it changes runtime code.
