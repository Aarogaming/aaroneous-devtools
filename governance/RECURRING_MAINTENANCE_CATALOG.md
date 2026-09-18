# Recurring Maintenance & Audit Catalog (`RECURRING_MAINTENANCE_CATALOG.md`)

This document defines the canonical specification for high-value, constructive filler audit jobs executed by the local background GPU worker (`qwen3.5:9b-q6` via Ollama).

## Governance Principles

1. **Priority Hierarchy**: Primary coordination queue tasks (`COORDINATION_QUEUE.md` rows in `Ready` or `In Progress`) take absolute precedence over recurring maintenance filler jobs.
2. **Zero Slop Invariant**: Every filler job targets explicit, verified relative workspace files and evaluates concrete architectural contracts. No vague, open-ended, or unstructured jobs are generated.
3. **Automatic Queue Sync & Refill**: The foundation worker pipeline (`local_worker.rs` + `job_converter.rs`) automatically syncs active queue rows and refills missing catalog jobs during every worker tick.

---

## Catalog Job Definitions

### `AUDIT-01-AST-SAFETY`
- **Mode**: `analysis`
- **Evidence Files**: `crates/ast_auditor/src/rules/text_encoding.rs`, `xtask/src/gate.rs`, `AGENTS.md`
- **Instruction**: Audit AST auditor rules, UTF-8/LF line ending enforcement, and gate command alignment between `xtask/src/gate.rs` and `AGENTS.md` Section 6.

### `AUDIT-02-HOT-PATH-ALLOC`
- **Mode**: `analysis`
- **Evidence Files**: `core/hypervisor/src/lib.rs`, `crates/ipc_bus/src/lib.rs`, `crates/compute/src/lib.rs`, `dev/emulator_harness/src/lib.rs`
- **Instruction**: Audit hot-path modules for forbidden dynamic heap allocations (`String`, `Vec`, `Box`, `HashMap`), unwraps/panics, and missing zero-copy/`#[repr(C)]` memory alignment invariants.

### `AUDIT-03-ZERO-AMBIENT-AUTH`
- **Mode**: `analysis`
- **Evidence Files**: `crates/paths/src/lib.rs`, `crates/orchestrator/src/lib.rs`, `crates/platform_bridge/src/lib.rs`
- **Instruction**: Audit crates for ambient authority violations (banned functions `std::env::var`, `temp_dir`, `canonicalize`). Verify path normalization via `paths::normalize_path` and constructor injection.

### `AUDIT-04-DEPRECATION-POLICY`
- **Mode**: `analysis`
- **Evidence Files**: `crates/adaptation_plane/src/capability_spec.rs`, `docs/DEPRECATION_POLICY.md`
- **Instruction**: Audit deprecated biological/mythological type aliases against `docs/DEPRECATION_POLICY.md`. Verify zero production call sites remain and migration equivalence tests pass.

### `AUDIT-05-TEMPORAL-SYNC`
- **Mode**: `analysis`
- **Evidence Files**: `crates/omni/src/matrix/sab_matrix.rs`, `core/hypervisor/src/bus_test.rs`, `governance/TEMPORAL_TEST_SYNCHRONIZATION_GUIDANCE.md`
- **Instruction**: Audit test suites for fixed sleep hazard calls (`thread::sleep`, `tokio::time::sleep`). Verify deterministic synchronization (FileTimes backdating, join barriers, bounded polling).

### `AUDIT-06-FOUNDATION-CLI`
- **Mode**: `analysis`
- **Evidence Files**: `foundation/crates/foundation_cli/src/local_worker.rs`, `foundation/crates/foundation_cli/src/job_converter.rs`
- **Instruction**: Audit local worker and foundation CLI for lock safety, response validation, transport bounds, and safe path traversal without parent directory escaping.

### `AUDIT-07-WIRE-ABI-GEOMETRY`
- **Mode**: `analysis`
- **Evidence Files**: `crates/ipc_bus/src/wire.rs`, `crates/wire/src/lib.rs`
- **Instruction**: Audit IPC bus and wire parser structs for `#[repr(C)]`, bytemuck `Pod`/`Zeroable` derives, explicit padding fields (`_pad0`), and zero-copy SWMR ring buffer guarantees.

### `AUDIT-08-ROADMAP-INVARIANTS`
- **Mode**: `analysis`
- **Evidence Files**: `docs/roadmap.md`, `MASTER_ROADMAP.md`, `governance/EVIDENCE_STANDARD.md`
- **Instruction**: Audit roadmap work claims and milestone status labels against codebase evidence per `governance/EVIDENCE_STANDARD.md`.

---

## Pre-Task Research & Spec Preparation Jobs

### `PREP-01-IPC-BUS-RING-BUFFER`
- **Mode**: `analysis`
- **Evidence Files**: `crates/ipc_bus/src/lib.rs`, `crates/ipc_bus/src/wire.rs`
- **Instruction**: Research and investigate IPC bus ring buffer zero-copy contract, lock-free memory alignment, and SWMR read/write invariants.

### `PREP-02-COMPUTE-ENGINE-CONTAINER`
- **Mode**: `analysis`
- **Evidence Files**: `crates/compute/src/lib.rs`
- **Instruction**: Research and investigate compute engine JIT and .si container engine interfaces, execution bounds, and allocation limits.

### `PREP-03-GOVERNANCE-SMT-INTERLOCKS`
- **Mode**: `analysis`
- **Evidence Files**: `crates/governance/src/lib.rs`
- **Instruction**: Research and investigate governance Z3 SMT verification rules, safety interlocks, and state reduction constraints.

### `PREP-04-ORCHESTRATOR-TASK-SCHEDULER`
- **Mode**: `analysis`
- **Evidence Files**: `crates/orchestrator/src/lib.rs`
- **Instruction**: Research and investigate task scheduler core affinity, execution budgets, and supervisor tick boundaries.

### `PREP-05-MAINTENANCE-UI-CONTRACTS`
- **Mode**: `analysis`
- **Evidence Files**: `foundation/crates/maintenance_ui/src/lib.rs`, `governance/UTILITY.md`
- **Instruction**: Research and investigate maintenance UI packet server handlers, authentication constraints, and HTTP status contracts.

