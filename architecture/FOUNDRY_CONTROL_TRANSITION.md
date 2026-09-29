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

Aaroneous may be exposed as a broad API/MCP capability surface, but never as
one undifferentiated authority. The Foundry treats product capabilities as
explicitly versioned classes: read-only inspection, bounded proposal,
verification, and separately authorized mutation. Each call binds a selected
revision, declared inputs, output schema and evidence record. The transition
gaps and corrective lanes that make this enforceable are maintained in the
[Foundry gap register](../governance/FOUNDRY_GAP_REGISTER.md).
The operating loop for maintaining those facts is
[Evidence-First Living Operations](../governance/EVIDENCE_FIRST_LIVING_OPERATIONS.md):
an observation becomes a recorded hypothesis, reproducible evidence, and a
linked corrective disposition before it becomes design or authority.

This is an integration boundary, not a claim that Aaroneous lacks
orchestration. The selected product revision already supplies the runtime
control components: `crates/orchestrator` schedules stateful work,
`crates/orchestration_plane` coordinates high-level execution, `crates/llm_gateway`
is the stateless model transport, and `OrchestrationDaemon` is the execution
host. The product's own `AGENTS.md` separately assigns *cross-agent development*
coordination to this companion repository. Foundry must describe and invoke
those product contracts through a pinned capability manifest; it must not copy
their runtime logic into devtools or treat runtime scheduling as a second
development ledger.

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
| Coordination | None required for product runtime | Queue, reservations, reviewer capacity, handoffs and review debt | Open claim PRs are advisory reservations; making that rule authoritative requires an accepted protocol amendment. |
| Runtime orchestration | `orchestrator`, `orchestration_plane`, `llm_gateway` and `OrchestrationDaemon` contracts | Pinned-revision capability manifest, explicit adapter inputs and retained invocation evidence | Devtools may orchestrate development work around these contracts; it does not fork, link, or duplicate their runtime control plane. |
| Local-model assistance | No runtime dependency | Bounded evidence bundles, proposal artifacts, validator and disposition | Models draft or classify; they never claim, execute, accept, or merge. |
| Versioning | Git branches, commits, PRs and tags | Immutable virtual patch revisions and evidence digest links | Do not reimplement Git. |

## Current-component disposition

| Existing component | Disposition | Foundry role / transition condition |
| --- | --- | --- |
| `virtual_patch` and its versioned contract (C73) | Keep | Canonical pre-Git proposal format; add consumers before adding a competing patch loop. |
| `local_proposal` and its schema | Keep and wrap | Target canonical explicit proposal adapter; it is not yet the only direct local-model boundary. |
| `local_worker` and approved `worker-jobs/` inbox | Merge operationally | One deterministic support-worker service, shadow-only until restart, cancellation and disposition evidence is proven. |
| `multithreader` / `scheduler` | Merge, do not multiply | Supply reservation, quiet-window and capacity policy to the Foundry matrix; retire duplicate loops only after a replacement passes replay tests. |
| Direct Ollama clients | Consolidate deliberately | The current inventory is `local_proposal.rs:106-118`, `intake_generator.rs:44-89` (also reached through `multithreader.rs:129`), `local_worker.rs:395-412`, and `agentic_registrar.rs:993,2099-2154`. Each remains maintained until a named lane routes it through the explicit proposal adapter or retires it; the consolidation gate must mechanically inventory the whole declared source set and prove no unapproved bypass remains. |
| `agentic_registrar::promote` | Constrain then replace at the seam | Current policy wins during transition: `promote` remains the sole write path until reviewer-approved C73 materialization passes the graduation assertions below. The replacement increment changes code and policy together; no parallel writer is introduced. |
| C76 `conformance` module | Wrap as a rejector | A pure Drafted-to-Reviewable filter beside compile checks; never a conformance authority and not automatically wired until its claim and review land. |
| `apply_snippet.rs`, `dump.rs`, `overseer_snippet.rs`, `db_reader/` | Retire or adopt by separate handoff | They remain untracked prototypes, not Foundry inputs; no silent adoption. |
| Aaroneous `xtask`, `ast_auditor`, Cratify and crate tests | Wrap | Invoke through an explicit capability manifest; do not copy their logic into devtools. |

## Machine state model

The machine ledger is append-only and records explicit IDs, patch digests,
selected revision, commands, exit results, reviewer identity and timestamps.
The Markdown queue is the human coordination view; it must cite the ledger
record rather than becoming a second mutable implementation state store.
When an active conflict needs an owner decision, the bounded
[round-table decision board](../governance/ROUND_TABLE_DECISIONS.md) records
the question, evidence, options, positions, decision and applied commit; it
never grants a claim itself.

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
- Until separate signed identities or credentials exist, different-agent status
  is the explicit round-table honour system: a named reviewer records the
  reviewed commit and evidence, and the ledger records that declared seat.

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

Offline endpoint, unavailable named model, malformed output, exhausted
capacity, or an occupied reservation are distinct normal `Blocked` outcomes.
The operator records which condition occurred; it does not trigger a retry
storm or a synthetic completion claim.

## Transition sequence

1. **Map and pin.** Add an Aaroneous capability manifest that names the
   selected revision, product gates, required inputs and output contracts.
2. **Shadow matrix.** Have devtools compute reservations, eligible gates,
   review debt and worker liveness with no repository writes.
3. **Use C73 end to end.** Route one owner-reviewed local proposal through a
   virtual patch set, validation and isolated materialization; retain replay
   evidence and satisfy every graduation assertion below.
4. **Unify service lifecycle.** Fold worker/scheduler behavior into one
   controllable Foundry service with `status`, `serve`, `pause`, `resume`,
   `dispatch`, `review`, `materialize` and `tail`. No Windows Task Scheduler
   is the control plane; an OS adapter may only launch the service.
5. **Graduate writes selectively.** Enable claim, PR publication, or other
   mutations one transition at a time after shadow replay and independent
   review. Auto-merge and self-acceptance remain out of scope.

## Binding graduation assertions for implementation lanes

Each transition increment supplies a deterministic fixture and records the
following pass/fail assertions; a prose design or a green compile alone cannot
graduate it:

1. A capability-manifest fixture pins an Aaroneous revision and rejects a
   mismatched manifest fingerprint before a gate is selected.
2. A shadow-cycle fixture produces a reservation/review matrix while proving
   that no checkout, queue, Git ref or index changed.
3. C73 fixtures reject tampered digest, stale base revision and out-of-scope
   paths before any worktree materialization.
4. A materialization fixture uses an isolated worktree and proves unrelated
   refs and the caller's index remain unchanged after success and failure.
5. An acceptance fixture rejects `reviewer == implementer` and retains the
   rejected disposition instead of marking a lane done.
6. A pause/cancel/restart fixture preserves the lease and prior disposition,
   then proves a resumed service neither duplicates dispatch nor loses the
   terminal record.

The increment that first satisfies all six may replace the corresponding
`promote` write path and update its policy in the same independently reviewed
change. Until then, the existing policy remains authoritative.

## Acceptance for this transition

This document is a boundary decision and migration map, not proof that a
Foundry service exists. A future implementation lane must name the surviving
components, its state-store contract, replay fixture, pause/cancellation
behavior, and independent acceptance gate before it changes runtime code.
