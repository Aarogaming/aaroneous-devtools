# Capability Taxonomy and Authority Model

## Status and purpose

This M73 decision brief records vocabulary and constraints for future Aaroneous work. It does not add a product API, change the existing capability registry, authorize a component, or certify an implementation.

It is grounded in the component-framework primer, the existing typed `CapabilityDescriptor` registry described by M47, and the five architecture notes supplied on 2026-09-23. Those notes are design input, not source-of-truth product requirements. Aaroneous's own `AGENTS.md` and `docs/ARCHITECTURE.md` remain authoritative for product work.

## Terms

| Term | Meaning | Not the same as |
| --- | --- | --- |
| Component | An independently testable execution block with explicit inputs and bounded output. | A user goal or package label. |
| Capability | A declared, bounded effect or transformation a component can provide under stated authority. | Permission to invoke arbitrary code. |
| Intent | A requested outcome that may select and compose capabilities. | A capability or authority grant. |
| Emergent system | A composed system whose behavior arises from multiple capabilities. | A new primitive merely because it has a name. |
| Authority grant | A scoped permission to observe, transform, store, communicate, actuate, or govern a named resource through a declared boundary. | A broad role, ambient access, or category label. |

An intent planner can propose a composition. It cannot manufacture authority, declare a proposal verified, claim a queue lane, or bypass human review and native verification gates.

## Proposed taxonomy

Use two independent dimensions. A zone describes architectural placement; authority describes permitted effects. Neither dimension alone is a substitute for the other.

| Zone | Role | Dependency rule |
| --- | --- | --- |
| Foundation | Contracts, execution-host policy, transport boundaries, and governance primitives. | Must not depend on higher zones. |
| Primitive | Narrow machine abilities such as bounded observation, storage, computation, or transport. | Depends only on Foundation or peer contracts with no cycle. |
| Operational | Reusable machine skills such as scheduling, telemetry, simulation, and coordination. | May compose lower zones through explicit contracts. |
| Domain | Problem-specific behavior such as PLC, robotics, diagnostics, or digital-twin work. | May compose lower zones; it does not redefine their authority. |
| Expression | Human-facing API, CLI, UI, or adapter layers. | Exposes capability results; it cannot silently broaden their authority. |
| Emergent | A named composition of capabilities. | Is not a dependency shortcut or a new primitive by default. |

The rule is acyclic dependency geometry, not a claim that every legitimate interaction is strictly downward. Governance and evidence review are cross-cutting constraints; they do not create hidden calls or mutable global state in a reducer.

## Authority and state

Each future capability specification should declare identity and zone; required and provided authority grants naming the resource and boundary; explicit inputs and outputs; state mode (owns, borrows, or observes) and the state-transition owner; bounded dependencies and resource limits; and verification obligations and lifecycle evidence.

Authority is a set of scoped grants. A diagnostic capability may observe a CAN frame stream and write a bounded report without gaining authority to actuate a vehicle. A capability with several effects declares each separately. The component framework's explicit-authority rule still applies: paths, handles, endpoints, buffers, configuration, and time inputs arrive through constructors or tick inputs rather than ambient process state.

## Time and lifecycle

Wall-clock reads, waiting, retries, timeouts, and blocking are acquisition or output concerns. A deterministic reducer receives any required time value or deadline explicitly and must not obtain it itself. A future time model should specify clock source, precision, timeout ownership, cancellation semantics, and replay behavior before it is used as a cross-component contract.

The lifecycle labels below are evidence states, not marketing labels.

| Stage | Minimum evidence |
| --- | --- |
| Candidate | Bounded proposal with scope, risk, and acceptance criterion. |
| Verified | Relevant tests and required technical gate pass for the declared scope. |
| Crystallized | Independently reviewed, with a stable contract and documented owner. |
| Foundational | Other approved capabilities depend on it; compatibility and retirement policy are explicit. |
| Legacy | Replacement or retirement window and consumer evidence are recorded. |

No local-model result may advance a lifecycle label by itself. It remains a `pending-owner-review` proposal under the local-agent control plane.

## Adoption sequence

1. Map the existing M47 registry fields to this vocabulary without changing its public shape.
2. Inventory two or three representative existing capabilities and identify missing authority, state, or time information.
3. Propose a versioned capability-specification contract only if the inventory demonstrates a stable common shape.
4. Add product code or enforcement only through a separately claimed Aaroneous lane with native verification and independent review.

## Explicit non-decisions

- No new global intent layer, privilege level, scheduler, or autonomy policy is approved here.
- No static authority enum is sufficient without resource scope and a boundary.
- No classification is a claim of current implementation completeness.
- Devtools remains a producer of evidence and proposals, never an Aaroneous runtime dependency.
