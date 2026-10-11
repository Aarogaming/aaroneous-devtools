# Proposal: Rolling-Window Pipeline & Autonomous Staged Lifecycle

- **Status**: Proposal (Discussion Draft)
- **Origin**: Round Table Session Entries 151, 152, 159, 160
- **Scope**: Devtools Architecture, Ingestion, Buffer Lifecycle, and Core Aaroneous Graduation
- **Intent**: Clarify how incoming signals (chat, issues, exploratory diffs, feature intent) move through a developer context buffer toward Aaroneous-compliant core code without requiring manual human bottlenecking.

---

## 1. Motivation and Problem Statement

Discussions across HUD v2 (PRs #159–#177) and the Codex rebase plan highlighted a recurring structural ambiguity:
how code evolves from rough ideas and perimeter signals into strict, zero-allocation Aaroneous components.

If every stage requires hands-on human review, lane claim approvals, and manual host handoff certificates, the human operator becomes a throughput bottleneck.
Conversely, if checks are relaxed globally, unverified or non-compliant logic risks leaking into the core runtime.

This document records the proposed **rolling-window pipeline** model discussed on the Round Table board. It frames the relationship between:
1. Outer signals and perimeter intake.
2. An intermediate buffer (devtools and active worktrees) where experimental code can iterate.
3. The formal boundary separating intermediate code from Aaroneous core.

---

## 2. The Three-Zone Topology

```
┌─────────────────────────────────────────────────────────────────┐
│ Zone 2: Outer Surface (Signals & Discovery)                     │
│ Raw chat logs, issue requests, upstream churn, exploratory runs │
└───────────────────────────────┬─────────────────────────────────┘
                                │ (Intake classification & typing)
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│ Zone 1: Rolling Median Buffer (Incubation & Refinement)         │
│ devtools worktrees, active PRs, prototype adapters, DEFER tags  │
└───────────────────────────────┬─────────────────────────────────┘
                                │ (Automated verification & graduation)
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│ Zone 0: Pinned Aaroneous Core (Immutable Bedrock)               │
│ core/hypervisor, ipc_bus, compute, strict Cratify profiles      │
└─────────────────────────────────────────────────────────────────┘
```

### Zone 0: Pinned Aaroneous Core
- **Characteristics**: Immutable base, strict zero-heap hot paths (`kernel`), pure reducers (`control`), zero ambient authority, and full adherence to `AGENTS.md` and `CRATIFY_SPEC.md`.
- **Constraint**: Code inside Zone 0 cannot depend on devtools or intermediate tooling. It represents certified, reproducible production components.

### Zone 1: The Rolling Median Buffer (Developer Context Window)
- **Characteristics**: The active working surface (devtools repository, task branches, draft pull requests, and experiment worktrees).
- **Blast Radius**: Failures, panics, or memory allocations here are isolated from Zone 0. They cannot compromise production hypervisor execution.
- **Handling Incomplete Work (`DEFER`)**: Annotations such as `// DEFER: <reason>` are permitted as tracking mechanisms within Zone 1. They serve as machine-readable task pointers rather than gate blockers for internal exploration. However, unmitigated `DEFER` annotations cannot cross into Zone 0.

### Zone 2: Outer Surface (Signals and Discovery)
- **Characteristics**: External inputs including user chat transcripts, feature proposals, issue reports, upstream dependency updates, and automated fuzzing/probe telemetry.
- **Role**: Feeds observations and requests into Zone 1 as structured records.

---

## 3. Four-Stage Lifecycle (From Signal to Core)

The discussion models the progression of code through four operational stages:

### Stage 1: Perimeter Intake
- Raw inputs from Zone 2 are gathered.
- Rather than feeding unstructured text directly to high-privilege operations, software-based extraction (regex, AST symbol analysis, file digests) parses the request into a typed intake card (e.g., following `contracts/job-intake-v1.md`).
- Output: A structured task record with bounded scope.

### Stage 2: Median Incubation
- Work on the task proceeds inside Zone 1 branches and worktrees.
- Operators and automated agents test alternatives, write fixtures, and run shadow comparisons.
- Changes remain isolated within the devtools workspace.

### Stage 3: Refinement and Boundary Alignment
- As prototype logic stabilizes, it is refactored against Aaroneous architectural contracts:
  - Eliminating ambient authority (injecting configuration and clock handles).
  - Assigning appropriate Cratify profiles (`kernel`, `control`, `presentation`, `tooling`).
  - Replacing ad-hoc model reasoning with compiled AST rules or deterministic state reducers where possible.

### Stage 4: Autonomous Graduation Gate
- When a component or crate in Zone 1 is considered ready to enter Zone 0 (Aaroneous core), it must satisfy objective, automated criteria:
  1. **Zero unresolved `DEFER` annotations** within the candidate code.
  2. **Full pass of `cargo xtask gate`** (encoding, strict clippy, zero unwrap ratchet growth, AST compliance, cross-target compilation for native, ARM, and WebAssembly).
  3. **Multi-agent cryptographic verification**: Multi-seat cryptographic attestation (`accept`) from registered keys in `governance/identities.json`.
- Upon passing, the component is moved to the product repository under its declared trait boundary, and any external/mock wrapper in devtools is retired.

---

## 4. Operational Principles Under Consideration

1. **Autonomous Operation**: Day-to-day lane claims, test execution, and peer reviews are handled via automated tooling and multi-agent cryptographic consensus, keeping the human operator free from routine pipeline gating.
2. **Deterministic Pre-filtering**: Mechanical tasks (checking lint rules, dependency direction, diff bounds, test execution) are performed by compiled Rust tools rather than large-model prompts.
3. **Traceability**: Decisions, attestations, and receipts remain head-bound to immutable git commits and content digests.

---

## 5. Current Status

This lifecycle is an **architectural proposal under review** in devtools Round Table discussions. It provides the reference framing for how PRs in lane C119 (HUD v2) and the Codex rebasing slices relate to one another. No automated promotion tooling has been implemented yet.
