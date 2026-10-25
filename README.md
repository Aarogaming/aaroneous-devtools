# Aaroneous Devtools

Aaroneous's private development companion: reusable Rust maintenance tools, a guided user interface, and local automation for tasks Git does not perform. Devtools uses practical, lighter implementation standards while preserving Aaroneous's independent build and runtime.

## Start here

- [Operator guide](governance/OPERATOR_GUIDE.md): one index that routes a task to the right tool or document, including getting a seat key and running the steward.
- [Companion operating model](governance/OPERATING_MODEL.md): organization, standards and everyday use.
- [Foundation boundary](FOUNDATION_BOUNDARY.md): what belongs here and what stays in Aaroneous.
- [Agent guide](AGENTS.md) and [contributing](CONTRIBUTING.md): ownership, verification and handoffs.
- [Framework primer](governance/AARONEOUS_COMPONENT_FRAMEWORK_PRIMER.md) and [component playbook](governance/COMPONENT_DEVELOPMENT_PLAYBOOK.md): context for helping Aaroneous.

## What works today

`foundation/` is a standalone Rust workspace. Its CLI validates inputs and stacks, inventories components, records dependency/dogfood evidence, reviews scheduling capacity and processes bounded local-model jobs. [Contracts](contracts/README.md) describe its versioned artifacts.

The native [candidate reviewer](governance/MULTITHREADER_SERVICE.md) and [local worker](governance/LOCAL_WORKER_SERVICE.md) provide local support automation. Approved jobs are in `worker-jobs/`; generated drafts remain unverified until reviewed. No active Codex maintenance schedule remains. See [the schedule](governance/SCHEDULE.md) for installed cadence and limitations.

The [packet utility](governance/UTILITY.md) offers readiness, read-only research and gated job authoring. Plan and Build create or revise definitions; the [job pipeline](architecture/JOB_PIPELINE.md) independently executes published jobs. Devtools can use Aaroneous contracts without becoming a framework dependency.

The React/TypeScript GUI and registrar prototype currently exist as unfinished, uncommitted work in a separate local checkout. Their simulated success states are not verification. [The application delivery map](architecture/MAINTENANCE_PROGRAM.md) records integration blockers and the path to a user-facing program.

## Work and architecture

- [Priority portfolio](governance/MULTITHREAD_PROGRAM.md): what to work on next.
- [Coordination queue](governance/COORDINATION_QUEUE.md): who owns exact files now.
- [Maintenance system](governance/MAINTENANCE_SYSTEM.md): evidence and planning lifecycle.
- [Foundation architecture](architecture/FOUNDATION_ARCHITECTURE.md), [technology policy](governance/TECH_STACKS.md), [ecosystem catalogue](stacks/RUST_ECOSYSTEMS.md) and [dogfooding](governance/DOGFOOD_PROTOCOL.md).
- [Git workflow](governance/GIT_WORKFLOW.md), [decisions](governance/DECISIONS.md), [releases](governance/RELEASES.md) and [security](SECURITY.md).

## Historical material

`migrations/` preserves transferred tooling for classification and selective adoption. It is not active automation or a product dependency. Generated `artifacts/` and `reports/` are ignored runtime storage. The operating model defines the maintained directory map and safe adoption rules.
