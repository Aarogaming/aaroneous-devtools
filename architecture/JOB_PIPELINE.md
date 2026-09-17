# Job authoring and orchestration

## One-way framework relationship

Aaroneous is the pristine Rust Component Framework. Devtools is a downstream development companion. Devtools may invoke Aaroneous tools, consume its versioned evidence and public APIs, or adopt appropriately isolated component adapters. Aaroneous never imports devtools, starts its services, loads its provider configuration or requires it to build, verify, release or run.

The current utility invokes the separate Foundation worker. Aaroneous supplies the selected component source, agent rules and product-owned verification contracts. Direct framework component reuse is an adapter decision, not an implemented dependency today. No framework changes were required for this utility.

## Separate state machines

```text
job authoring: packet/template -> Plan -> validated blueprint -> Build -> published job version
orchestration: published job -> eligible -> reserved -> executing -> result -> review/disposition
```

Plan and Build describe job creation and modification. They are not a global switch that stops orchestration. Planning a new definition, inspecting a blueprint or returning to Plan must leave previously published jobs and existing schedules intact. An execution failure belongs to that run; it does not reset unrelated plans. Changed evidence invalidates the affected authoring blueprint without deleting published work.

Build publishes a validated job definition. It does not itself invoke a product build, apply a model patch, or mark a result verified. The scheduler independently decides when the published job is eligible and its worker executes only the declared bounded recipe.

## Packet-based user experience

The user operates through prebuilt information packets rather than raw commands or editable file targets:

- Are we all clear? Inspect readiness, blockers and retained process evidence. Missing or stale evidence never means everything passed.
- Research now. Request the shipped, maintainer-approved read-only communication-boundary research packet. Suggestions are brainstorming, not executable authority.
- Develop the blueprint. Create the supported research-job definition's bounded scope, inputs, ordered dependencies, acceptance evidence and retirement strategy.
- Refine the blueprint. Deepen the approved research profile with alternatives, dependencies and falsifiable acceptance tests. Modify definitions in Plan and preserve previous published versions.
- Switch job authoring to Build. Available only when every publication criterion passes.
- Publish the prepared job. Produce an immutable validated manifest for the independent local pipeline.

The first supported authoring template is read-only wire research. There is no arbitrary feature-generation or source-mutation packet. New templates require a concrete contract, authority classification, meaningful tests and maintainer review; do not turn generic model prose into executable instructions.

## Publication criteria

The native utility checks blueprint completeness, no unresolved questions, supported ordered actions, current repository revision/evidence digests, clean scoped evidence, source ownership, explicitly enabled publication and valid native job storage. Unrelated dirty files and active jobs do not freeze authoring. Publication still honors the shared reservation to avoid racing a worker or reviewer.

These are mechanical guarantees for the supported read-only template. They do not establish that arbitrary model reasoning is correct. Any later source-changing template also needs a reviewed problem statement, bounded patch authority, input/output contracts, safety and dependency assessment, an isolated worktree, product-native verification and a rollback/integration decision. Such templates are not enabled in this version.

## Orchestration boundaries

The current utility polls its own published packet jobs every two minutes while its process stays running. It invokes the existing Rust worker; it does not reimplement inference, response validation or source mutation. The worker enforces quiet windows, exact evidence limits, ownership checks, a shared reservation, unchanged-input suppression and review backpressure.

Existing native OS services continue their original schedules and inboxes. They are not paused by authoring mode and are not silently retargeted to the utility inbox. The current UI is not yet a unified service supervisor. Logged-out hosting and consolidation into one durable installation remain explicit operational work.

Run records and input snapshots belong to orchestration; authored blueprints belong to job-definition lifecycle. Neither model output nor a paid provider can grant permissions, waive publication criteria, mark product code verified or claim canonical work.

## Evolution

Add more prebuilt profiles through this contract rather than more raw controls. Keep authoring, eligibility, execution, evidence review and integration separately testable. Cloud provider routing and budgets remain an independent orchestration adapter; this packet-only release exposes no paid dispatch. Reuse Aaroneous where its contracts fit and keep companion operating policy here.

See [the utility guide](../governance/UTILITY.md) for build and launch instructions.
