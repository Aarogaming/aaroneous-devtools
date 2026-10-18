# Aaroneous Devtools Component Framework

This directory houses operational components for Aaroneous development across three lifecycle tiers: `pending`, `management`, and `coordination`.

## 1. Architectural Role

Aaroneous is a Rust capability framework for deterministic execution blocks with zero ambient authority, bounded memory, and explicit capability traits. `aaroneous-devtools` acts as the operational superstructure around Aaroneous:
- Devtools may consume Aaroneous core contracts, capabilities, and presentation layers through explicit adapters.
- Aaroneous remains 100% self-contained and never depends on devtools build, CI, runtime, or scripts.
- When devtools compiles with Aaroneous present, it acts as an in-development operational add-on, exposing components across the three tiers.

## 2. Component Lifecycle Tiers

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        Aaroneous-Devtools                              │
├────────────────────┬───────────────────────┬───────────────────────────┤
│     Pending        │      Management       │       Coordination        │
│ (Graduation Track) │ (Operational Runners) │ (Multi-Agent Consensus)   │
├────────────────────┼───────────────────────┼───────────────────────────┤
│ • Canary Tools     │ • Maintenance Daemon  │ • Steward Consensus       │
│ • Shadow Reducers  │ • Local Worker (Ollama│ • Seat Attestation        │
│ • Candidate Traits │ • Telemetry & Thermals│ • Queue Arbitration       │
│ • Model Adapters   │ • Virtual Patching    │ • Continuum Leases        │
└─────────┬──────────┴───────────────────────┴───────────────────────────┘
          │ (Graduation Gate)
          ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        Aaroneous Core                                  │
│ (crates/core-contracts, capabilities, compute, governance, studio_hud) │
└────────────────────────────────────────────────────────────────────────┘
```

### Tier 1: `components/pending/` (Graduation Track)
- **Purpose**: Incubator for candidate components, tools, and traits being developed or evaluated prior to formal graduation into `aaroneous/crates/`.
- **Requirements**:
  - Pure state reduction (`S(next) = reduce(S(current), input)`).
  - Explicit authority: no ambient clock, filesystem, environment, or network calls.
  - Shadow execution harness: verified in devtools with golden vectors before opening a product PR.
  - Compliance: passes `cratify` profile rules and `ast_auditor` checks.

### Tier 2: `components/management/` (Operational Runners)
- **Purpose**: Durable operational tools for running and maintaining development systems.
- **Scope**:
  - Local model worker dispatch (Qwen/Ollama scan-cycle runner).
  - Continuous maintenance pipeline and atomic file replacement.
  - System telemetry (VRAM, thermal state, load monitoring on Ubuntu/Windows hosts).
  - Bounded virtual patch materialization.

### Tier 3: `components/coordination/` (Multi-Agent Consensus)
- **Purpose**: Multi-agent synchronization, identity verification, and ledger arbitrations.
- **Scope**:
  - Steward state engine and deterministic card dispatch.
  - Seat identity attestation and cryptographic verification (`attest_sig`).
  - Coordination queue arbitration and continuum leases.
  - Cross-platform event stream and telemetry synchronization.

## 3. Graduation to Aaroneous

A component in `pending/` graduates to Aaroneous core only when:
1. It passes all Aaroneous native verification gates (`cargo xtask check-encoding`, `check-deps`, `check-unwraps`, `ast_auditor audit`, `cargo fmt`, `clippy -D warnings`).
2. Its declared Cratify profile (`kernel`, `control`, `presentation`, `tooling`) is verified.
3. An independent seat reviews and accepts the implementation evidence in the coordination queue.
