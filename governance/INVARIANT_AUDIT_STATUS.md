# Continuous Invariant Audit Status

> **STATUS**: AUTOMATED INVARIANT AUDIT ROLLUP
> **PURPOSE**: Aggregates continuous invariant and architectural audits performed by the local GPU worker (`qwen3.5:9b-q6`) across Aaroneous and devtools.
> **CONSTITUTION**: All findings are advisory proposals (`pending-owner-review`). Continuous audits monitor codebase health and verify compliance against AGENTS.md.

## Executive Invariant Matrix

| Invariant Family | Latest Pass | Status | Submitted | Executive Summary |
| :--- | :--- | :--- | :--- | :--- |
| **AUDIT-01-AST-SAFETY** | `AUDIT-01-AST-SAFETY-P0190` | `pending-owner-review` | unix `1790023264` | Remediate text_encoding.rs to include --others --exclude-standard in git ls-files and add magic b... |
| **AUDIT-02-HOT-PATH-ALLOC** | `AUDIT-02-HOT-PATH-ALLOC-P0013` | `pending-owner-review` | unix `1790023276` | Refactor PluginManager to use fixed-size arrays or slab-based allocation for PluginDescriptor and... |
| **AUDIT-03-ZERO-AMBIENT-AUTH** | `AUDIT-03-ZERO-AMBIENT-AUTH-P0013` | `pending-owner-review` | unix `1790023288` | No remediation required. All ambient authority patterns are controlled and documented. The codeba... |
| **AUDIT-04-DEPRECATION-POLICY** | `AUDIT-04-DEPRECATION-POLICY-P0012` | `pending-owner-review` | unix `1790023178` | Verify that no production code in other crates references these deprecated aliases by running git... |
| **AUDIT-09-HOT-PATH-REDUCERS** | `AUDIT-09-HOT-PATH-REDUCERS-P0013` | `pending-owner-review` | unix `1790023300` | Inspect dev/emulator_harness/src/reducer.rs to verify implementation of address filtering, zero-a... |
| **AUDIT-10-ORCHESTRATOR-BOUNDS** | `AUDIT-10-ORCHESTRATOR-BOUNDS-P0013` | `pending-owner-review` | unix `1790023315` | Remediate static counter race condition by introducing per-instance unique identifiers or removin... |
| **AUDIT-11-GIT-DISCOVERY** | `AUDIT-11-GIT-DISCOVERY-P0003` | `pending-owner-review` | unix `1790183800` | Correct the invalid edition in Cargo.toml to '2021' or '2024' (if nightly) and verify that new me... |
| **PREP-02-COMPUTE-ENGINE-CONTAINER** | `PREP-02-COMPUTE-ENGINE-CONTAINER-P0019` | `pending-owner-review` | unix `1790184024` | Implement dynamic configuration for memory allocation limits and input validation bounds to preve... |
| **PREP-03-GOVERNANCE-SMT-INTERLOCKS** | `PREP-03-GOVERNANCE-SMT-INTERLOCKS-P0019` | `pending-owner-review` | unix `1790184031` | Review LatticeVerifier default spatial bounds (7680x4320) and memory limit (64MB) against hardwar... |
| **PREP-04-ORCHESTRATOR-TASK-SCHEDULER** | `PREP-04-ORCHESTRATOR-TASK-SCHEDULER-P0019` | `pending-owner-review` | unix `1790184040` | Refactor IntelligenceEngine to use a concrete error enum for initialization and analysis, and imp... |
| **PREP-06-STUDIO-HUD-PANIC-CONTAINMENT** | `PREP-06-STUDIO-HUD-PANIC-CONTAINMENT-P0019` | `pending-owner-review` | unix `1790184050` | Implement `catch_unwind` wrappers around `replay_commands` and add fault tracking logic to `Plugi... |

---

## Detailed Invariant Findings

### AUDIT-01-AST-SAFETY

- **Task Description**: AUDIT-01-AST-SAFETY-P0190
- **Latest Job Pass**: `AUDIT-01-AST-SAFETY-P0190`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790023264`

#### Recommended Next Step
Remediate text_encoding.rs to include --others --exclude-standard in git ls-files and add magic bytes detection for binary classification. Update xtask/src/gate.rs Gate 6 to audit all Git-tracked files without hardcoded path restrictions.

#### Key Observations
- text_encoding.rs: audit_tracked_encodings uses git ls-files without --others --exclude-standard flag. Untracked files are not audited. | Status: ACTIVE | No remediation detected in current code.
- text_encoding.rs: is_binary_extension uses static BINARY_EXTENSIONS list only. No magic bytes fallback for unknown extensions. | Status: ACTIVE | No remediation detected in current code.
- xtask/src/gate.rs: Gate 6 runs ast_auditor with hardcoded paths 'core/', 'crates/', 'dev/emulator_harness/'. This does not match AGENTS.md Section 6 requirement to audit 'all Git-tracked text files'. | Status: ACTIVE | No remediation detected in current code.

#### Identified Risks
- Untracked files with binary content or invalid encoding may bypass audit.
- Text files with non-standard extensions may be misclassified as binary.
- Gate 6 may miss files outside hardcoded directories (core/, crates/, dev/emulator_harness/).
- Inconsistent audit coverage between CI (gate.rs) and manual audits.

---

### AUDIT-02-HOT-PATH-ALLOC

- **Task Description**: AUDIT-02-HOT-PATH-ALLOC-P0013
- **Latest Job Pass**: `AUDIT-02-HOT-PATH-ALLOC-P0013`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790023276`

#### Recommended Next Step
Refactor PluginManager to use fixed-size arrays or slab-based allocation for PluginDescriptor and Library handles, replacing HashMap and Vec with zero-copy alternatives. Consider using `rkyv` for serialization and `slab` crate for fixed-size storage. Remove dynamic heap allocations from PluginManager struct.

#### Key Observations
- Forbidden dynamic heap allocations detected: `std::collections::HashMap` in `PluginManager` struct.
- Forbidden dynamic heap allocations detected: `Vec<Library>` in `PluginManager` struct.
- Forbidden dynamic heap allocations detected: `std::sync::Mutex` in `PLUGIN_MANAGER` static initialization (Lazy<Mutex<PluginManager>>). While Mutex is often necessary for synchronization, the audit flags it as a potential violation of zero-copy/lock-free invariants in a hypervisor core context unless strictly bounded and justified. However, the primary violations are the HashMap and Vec storing dynamic objects (PluginDescriptor, Library). The Mutex wrapping a Lazy static is a common pattern for singletons, 

#### Identified Risks
- Use of `HashMap` and `Vec` in `PluginManager` violates the zero-copy and bounded memory allocation invariants required for a hypervisor core.
- Use of `Mutex` in `PLUGIN_MANAGER` static initialization may introduce contention and potential deadlocks in a high-concurrency hypervisor environment.

---

### AUDIT-03-ZERO-AMBIENT-AUTH

- **Task Description**: AUDIT-03-ZERO-AMBIENT-AUTH-P0013
- **Latest Job Pass**: `AUDIT-03-ZERO-AMBIENT-AUTH-P0013`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790023288`

#### Recommended Next Step
No remediation required. All ambient authority patterns are controlled and documented. The codebase adheres to the zero-hardcoded-path invariant.

#### Key Observations
- std::env::var used in discover_root_from_system for CARGO_MANIFEST_DIR. This is a controlled ambient authority pattern for workspace discovery, not a hardcoded path. No violation.
- std::env::current_dir used in discover_root_from_system for current_dir traversal. This is a controlled ambient authority pattern for workspace discovery. No violation.
- std::env::current_exe used in discover_root_from_system for executable parent traversal. This is a controlled ambient authority pattern for workspace discovery. No violation.

---

### AUDIT-04-DEPRECATION-POLICY

- **Task Description**: AUDIT-04-DEPRECATION-POLICY-P0012
- **Latest Job Pass**: `AUDIT-04-DEPRECATION-POLICY-P0012`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790023178`

#### Recommended Next Step
Verify that no production code in other crates references these deprecated aliases by running git grep across the workspace. Confirm that the migration test passes. Prepare a standalone removal PR for v0.4.0 that deletes the deprecated type aliases, re-export entries, and migration test assertions.

#### Key Observations
- Deprecated type aliases in capability_spec.rs (ParameterLocus, AgentConfigProfile, ParameterProfile, SpecialistGenome, ParameterGenome, GeneticLocus, GeneticCategory, EpigeneticState, GeneticRelationship, GeneticAnalyzer) are defined with #[deprecated(since = "0.3.3", note = "...")].
- Migration test test_legacy_terminology_aliases_equivalence exists at line 590-621 and uses TypeId assertions to verify deprecated aliases resolve to canonical types.
- No production call sites found in capability_spec.rs using deprecated aliases; all usage is limited to definition sites and the migration test block itself (as documented in DEPRECATION_POLICY.md section 3). Re-export blocks in lib.rs are not present in this file, only in lib.rs per policy documentation. The file contains only the deprecated type definitions and the migration test, which is expected for a definition-site-only file per policy section 3.3.1 (definition sites are to be deleted on removal, but 

#### Identified Risks
- If the migration test fails, it indicates a mismatch between deprecated aliases and their canonical replacements, which would break the removal process.
- If any production code in other crates references these deprecated aliases, the removal will fail with compilation errors.
- The removal PR must be standalone and not mixed with feature work per policy section 4.1.

---

### AUDIT-09-HOT-PATH-REDUCERS

- **Task Description**: AUDIT-09-HOT-PATH-REDUCERS-P0013
- **Latest Job Pass**: `AUDIT-09-HOT-PATH-REDUCERS-P0013`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790023300`

#### Recommended Next Step
Inspect dev/emulator_harness/src/reducer.rs to verify implementation of address filtering, zero-allocation guarantees, and deterministic replay logic.

#### Key Observations
- Evidence file dev/emulator_harness/src/lib.rs contains only public API declarations and test cases; no implementation of reduce_trace_bounded or extract_state_delta is present in the provided snippet.
- Test expectations in test_bounded_reducer_budget_and_replay implicitly filter events by memory_addr (0xBEEF) without explicit code enforcement visible in the provided evidence.
- No explicit address filtering logic is observable in the provided code to validate or remediate the implicit filtering risk identified in prior audit passes.

#### Identified Risks
- Implicit address filtering in production code may hide address-mismatch bugs if not explicitly documented and tested.
- Lack of visible implementation in provided evidence prevents verification of zero-allocation and deterministic replay guarantees.

---

### AUDIT-10-ORCHESTRATOR-BOUNDS

- **Task Description**: AUDIT-10-ORCHESTRATOR-BOUNDS-P0013
- **Latest Job Pass**: `AUDIT-10-ORCHESTRATOR-BOUNDS-P0013`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790023315`

#### Recommended Next Step
Remediate static counter race condition by introducing per-instance unique identifiers or removing static state. Add explicit size validation in SharedMemorySynapse constructors. Implement supervisor budget enforcement in TaskRoutingEngine.

#### Key Observations
- Static AtomicUsize counter in IntelligenceEngine::new and new_async remains present, creating race conditions in multi-instance deployments.
- SharedMemorySynapse::new_sync and new lack explicit size validation or bounds checking before allocation.
- TaskRoutingEngine::new and update_specialist_performance lack documented or enforced supervisor budget limits per task type or agent class.

#### Identified Risks
- Race condition in fallback shared memory key generation may cause collisions in multi-instance environments.
- Ambient authority in SharedMemorySynapse allocation could lead to resource exhaustion if size validation is absent.
- Lack of supervisor budget enforcement in TaskRoutingEngine may allow unbounded task routing and resource consumption.

---

### AUDIT-11-GIT-DISCOVERY

- **Task Description**: Audit workspace structure and architectural invariants
- **Latest Job Pass**: `AUDIT-11-GIT-DISCOVERY-P0003`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790183800`

#### Recommended Next Step
Correct the invalid edition in Cargo.toml to '2021' or '2024' (if nightly) and verify that new members do not violate the 'Zero ambient authority' invariant.

#### Key Observations
- Workspace edition set to '2024' which is invalid; Rust editions are 2015, 2018, 2021, or 2024 (unstable), but '2024' is not a stable release yet, risking compilation failures.
- Workspace contains 34 members including 'core/hypervisor' and 'crates/orchestrator', exceeding the previously noted 28, indicating significant scope expansion or new crate additions.
- Architecture mandates 'Zero ambient authority' and explicit constructor injection, yet 'xtask' and 'benches' members exist which often rely on implicit environment access or global state.

#### Identified Risks
- Invalid edition '2024' will cause immediate build failure on stable toolchains.
- Expansion to 34 crates increases the attack surface for circular dependency violations in the 5-ring model.
- Presence of 'xtask' and 'benches' may introduce implicit global state access, violating the strict constructor injection rule.
- High `codegen-units = 1` with 34 crates will drastically increase compile times, hindering iteration.
- Strict 'panic = abort' combined with complex state machines increases risk of undefined behavior if unwinding is needed for safety checks.
- Architecture document references non-existent or future-dated files (e.g., '2026-09-14'), suggesting potential version drift or hallucination in documentation.

---

### PREP-02-COMPUTE-ENGINE-CONTAINER

- **Task Description**: Analyze ComputeEngine for invariant violations regarding memory allocation, input validation, and JIT execution bounds.
- **Latest Job Pass**: `PREP-02-COMPUTE-ENGINE-CONTAINER-P0019`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790184024`

#### Recommended Next Step
Implement dynamic configuration for memory allocation limits and input validation bounds to prevent OOM errors and malformed input handling.

#### Key Observations
- ComputeEngine::default() enforces a hardcoded 1MB (1024 * 1024) allocation for SharedMemorySynapse in both test and production paths.
- The execute method performs only basic length checks for specific tasks (free_energy, mutual_info) but lacks general input validation or bounds checking for the input slice.
- No explicit execution bounds, memory protection state enforcement, or JIT compilation limits are exposed in the ComputeEngine public API despite available modules.

#### Identified Risks
- Fixed 1MB synapse allocation may cause Out-Of-Memory (OOM) errors under high concurrency or large-scale deployments.
- Absence of explicit JIT execution bounds allows unbounded resource consumption by delegated CraneliftJitEngine operations.
- Lack of input validation beyond specific task length checks leaves the engine vulnerable to malformed or oversized inputs.
- MemoryProtectionState defined in si_jit is not utilized in the ComputeEngine, leaving JIT execution unprotected.
- No external configuration mechanism exists to adjust allocation limits or execution bounds dynamically.
- Hardcoded fallback names in production may lead to shared memory conflicts if multiple instances run simultaneously.

---

### PREP-03-GOVERNANCE-SMT-INTERLOCKS

- **Task Description**: Governance SMT verification rules and safety interlocks audit
- **Latest Job Pass**: `PREP-03-GOVERNANCE-SMT-INTERLOCKS-P0019`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790184031`

#### Recommended Next Step
Review LatticeVerifier default spatial bounds (7680x4320) and memory limit (64MB) against hardware specifications to determine if dynamic configuration is required.

#### Key Observations
- LatticeVerifier default spatial bounds (7680x4320) and memory limit (64MB) are hardcoded in Default impl, creating potential resource constraint mismatches.
- SmtActionInterlock::new() correctly chains LatticeVerifier::default().with_epsilon() to maintain single-source-of-truth configuration for energy bounds.
- evaluate_action_graph() converts SMT prover rejections into denial certificates rather than propagating errors, preserving batch verification integrity.

#### Identified Risks
- Default max_linear_memory_bytes (64MB) is hardcoded in LatticeVerifier::default() and may not reflect dynamic resource constraints.
- Default max_free_energy_epsilon (100.0) in LatticeVerifier differs from strict mode (0.05) in SmtActionInterlock, creating potential configuration drift.
- verify_spatial_containment() does not validate coordinate offsets against NativeComputationalGraph schema, relying on prior audit assumptions.
- Thermodynamic bound checks in evaluate_action_graph() and LatticeVerifier::verify_thermodynamic_bound() use separate logic paths, risking inconsistent enforcement.
- Emergency killswitch state is not persisted across process restarts, potentially allowing unauthorized execution after system reboot.
- LatticeVerifier default spatial bounds (7680x4320) may exceed intended hardware limits if not explicitly configured.

---

### PREP-04-ORCHESTRATOR-TASK-SCHEDULER

- **Task Description**: Verify IntelligenceEngine error handling and routing safety invariants.
- **Latest Job Pass**: `PREP-04-ORCHESTRATOR-TASK-SCHEDULER-P0019`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790184040`

#### Recommended Next Step
Refactor IntelligenceEngine to use a concrete error enum for initialization and analysis, and implement error handling in route_task to prevent silent failures.

#### Key Observations
- IntelligenceEngine::new and new_async return anyhow::Result, violating the requirement for a concrete error enum.
- IntelligenceEngine::analyze_task returns anyhow::Result, violating the requirement for structured error propagation.
- IntelligenceEngine::route_task returns RoutingDecision directly without error handling, masking potential routing failures.

#### Identified Risks
- Silent failures in task routing if the router encounters an error state.
- Lack of deterministic core affinity pinning may lead to cache thrashing.
- Undefined execution budgets may cause supervisor tick overflow.

---

### PREP-06-STUDIO-HUD-PANIC-CONTAINMENT

- **Task Description**: PREP-06-STUDIO-HUD-PANIC-CONTAINMENT-P0019
- **Latest Job Pass**: `PREP-06-STUDIO-HUD-PANIC-CONTAINMENT-P0019`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790184050`

#### Recommended Next Step
Implement `catch_unwind` wrappers around `replay_commands` and add fault tracking logic to `PluginManager` to handle `TickResult::PluginFaulted` and enforce rate-limiting.

#### Key Observations
- The `replay_commands` function in `plugin_api.rs` executes `ui.label` and `ui.button` directly without `catch_unwind` wrappers, violating RFC-0006 Section 7's mandate for panic containment at the FFI boundary.
- The `PluginManager` lacks a mechanism to handle `TickResult::PluginFaulted`, leaving no path to signal plugin failure to the host or trigger automatic unloading per RFC-0006 Section 7.
- The current implementation does not enforce rate-limiting on plugin faults, creating a potential DoS vector where repeated plugin panics could exhaust host resources.

#### Identified Risks
- Uncaught panics in `replay_commands` could corrupt host state or crash the application if a plugin panics during command execution.
- Missing `TickResult::PluginFaulted` handling prevents the host from detecting and recovering from plugin failures.
- Absence of rate-limiting allows malicious or buggy plugins to trigger repeated panics, potentially causing a Denial of Service.
- Direct calls to `egui::Ui` methods without isolation violate the RFC's requirement that the host must replay commands, not execute plugin code directly.
- The current code structure does not support the future requirement of automatic plugin unloading after three consecutive faults.
- Lack of panic isolation contradicts the security revalidation findings that removed dynamic loading due to ABI unsoundness.

---

