# Continuous Invariant Audit Status

> **STATUS**: AUTOMATED INVARIANT AUDIT ROLLUP
> **PURPOSE**: Aggregates continuous invariant and architectural audits performed by the local GPU worker (`qwen3.5:9b-q6`) across Aaroneous and devtools.
> **CONSTITUTION**: All findings are advisory proposals (`pending-owner-review`). Continuous audits monitor codebase health and verify compliance against AGENTS.md.

## Executive Invariant Matrix

| Invariant Family | Latest Pass | Status | Submitted | Executive Summary |
| :--- | :--- | :--- | :--- | :--- |
| **AUDIT-01-AST-SAFETY** | `AUDIT-01-AST-SAFETY-P0183` | `pending-owner-review` | unix `1790002898` | Remediate text_encoding.rs by adding --others --exclude-standard to git ls-files invocation and i... |
| **AUDIT-02-HOT-PATH-ALLOC** | `AUDIT-02-HOT-PATH-ALLOC-P0006` | `pending-owner-review` | unix `1790002911` | Refactor PluginManager to use fixed-size arrays or slab-based allocation for plugins, remove Hash... |
| **AUDIT-03-ZERO-AMBIENT-AUTH** | `AUDIT-03-ZERO-AMBIENT-AUTH-P0006` | `pending-owner-review` | unix `1790002926` | No remediation required. All ambient authority patterns are controlled and documented. The codeba... |
| **AUDIT-04-DEPRECATION-POLICY** | `AUDIT-04-DEPRECATION-POLICY-P0006` | `pending-owner-review` | unix `1790003115` | Verify that no other crates reference these deprecated aliases by running git grep across the wor... |
| **AUDIT-09-HOT-PATH-REDUCERS** | `AUDIT-09-HOT-PATH-REDUCERS-P0006` | `pending-owner-review` | unix `1790002938` | Verify production implementation of reduce_trace_bounded and extract_state_delta in dev/emulator_... |
| **AUDIT-10-ORCHESTRATOR-BOUNDS** | `AUDIT-10-ORCHESTRATOR-BOUNDS-P0006` | `pending-owner-review` | unix `1790002958` | Remediate static counter race condition by introducing per-instance fallback key generation using... |
| **PREP-02-COMPUTE-ENGINE-CONTAINER** | `PREP-02-COMPUTE-ENGINE-CONTAINER-P0006` | `pending-owner-review` | unix `1790002969` | Remediate by introducing bounded allocation limits, JIT execution bounds, and explicit input vali... |
| **PREP-03-GOVERNANCE-SMT-INTERLOCKS** | `PREP-03-GOVERNANCE-SMT-INTERLOCKS-P0006` | `pending-owner-review` | unix `1790002987` | Inspect source files for smt_action_interlock, lattice_verifier, and interference_checker to veri... |
| **PREP-04-ORCHESTRATOR-TASK-SCHEDULER** | `PREP-04-ORCHESTRATOR-TASK-SCHEDULER-P0006` | `pending-owner-review` | unix `1790003002` | Implement core affinity pinning, execution budget enforcement, and supervisor tick boundary check... |
| **PREP-06-STUDIO-HUD-PANIC-CONTAINMENT** | `PREP-06-STUDIO-HUD-PANIC-CONTAINMENT-P0006` | `pending-owner-review` | unix `1790003016` | Implement RFC-0006 Section 7 panic containment: wrap `replay_commands()` in `catch_unwind` and ad... |

---

## Detailed Invariant Findings

### AUDIT-01-AST-SAFETY

- **Task Description**: AUDIT-01-AST-SAFETY-P0183
- **Latest Job Pass**: `AUDIT-01-AST-SAFETY-P0183`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790002898`

#### Recommended Next Step
Remediate text_encoding.rs by adding --others --exclude-standard to git ls-files invocation and implementing magic byte detection fallback for binary classification. Verify gate.rs alignment with AGENTS.md Section 6.

#### Key Observations
- text_encoding.rs: UTF-8 BOM check remains active (bytes.starts_with(b"\xef\xbb\xbf"))
- text_encoding.rs: audit_tracked_encodings uses git ls-files without --others --exclude-standard (untracked files not audited)
- text_encoding.rs: is_binary_extension uses static list only (no fallback heuristic for unknown extensions or magic bytes). Risk: binary files with text extensions or text files with binary extensions may be misclassified. No file command or magic bytes fallback present. Risk: false negatives on binary files with text extensions or false positives on text files with binary extensions. No file command or magic bytes fallback present. Risk: false negatives on binary files with text extensions or false positive

#### Identified Risks
- Untracked files not audited due to missing --others --exclude-standard flag in git ls-files.
- Binary files with text extensions or text files with binary extensions may be misclassified due to static extension list only.

---

### AUDIT-02-HOT-PATH-ALLOC

- **Task Description**: AUDIT-02-HOT-PATH-ALLOC-P0006
- **Latest Job Pass**: `AUDIT-02-HOT-PATH-ALLOC-P0006`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790002911`

#### Recommended Next Step
Refactor PluginManager to use fixed-size arrays or slab-based allocation for plugins, remove HashMap and Vec, and consider lock-free alternatives for the static singleton.

#### Key Observations
- Forbidden dynamic heap allocations detected: `std::collections::HashMap` in `PluginManager` struct.
- Forbidden dynamic heap allocations detected: `Vec<Library>` in `PluginManager` struct.
- Forbidden dynamic heap allocations detected: `std::sync::Mutex` in `PLUGIN_MANAGER` static initialization (Lazy<Mutex<PluginManager>>). While Mutex is often necessary for synchronization, the audit flags it as a potential violation of zero-copy/lock-free invariants in a hypervisor core context unless strictly bounded and justified. However, the primary violations are the HashMap and Vec storing dynamic objects (PluginDescriptor, Library). The Mutex wrapping a Lazy static is a common pattern for singletons, 

#### Identified Risks
- Use of `HashMap` and `Vec` in `PluginManager` violates the zero-copy and bounded memory allocation invariants required for a hypervisor core.
- Use of `Mutex` in `PLUGIN_MANAGER` static initialization may introduce contention and potential deadlocks in a high-concurrency hypervisor environment.
- Use of `anyhow::Error` for error handling may not be suitable for a hypervisor core where precise error handling and recovery are critical.
- Use of `tokio::task::yield_now()` and `tokio::time::sleep()` in the hypervisor loop may introduce unnecessary latency and jitter in a real-time system.
- Use of `tracing` for logging may introduce performance overhead in a high-throughput hypervisor environment.
- Use of `std::fs::read_dir` and `std::path::PathBuf` for plugin discovery may introduce security risks if not properly validated and sanitized.

---

### AUDIT-03-ZERO-AMBIENT-AUTH

- **Task Description**: AUDIT-03-ZERO-AMBIENT-AUTH-P0006
- **Latest Job Pass**: `AUDIT-03-ZERO-AMBIENT-AUTH-P0006`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790002926`

#### Recommended Next Step
No remediation required. All ambient authority patterns are controlled and documented. The codebase adheres to the zero-hardcoded-path invariant.

#### Key Observations
- std::env::var used in discover_root_from_system for CARGO_MANIFEST_DIR. This is a controlled ambient authority pattern for workspace discovery, not a hardcoded path. No violation.
- std::env::current_dir used in discover_root_from_system for current_dir traversal. This is a controlled ambient authority pattern for workspace discovery. No violation.
- std::env::current_exe used in discover_root_from_system for executable parent traversal. This is a controlled ambient authority pattern for workspace discovery. No violation.

---

### AUDIT-04-DEPRECATION-POLICY

- **Task Description**: AUDIT-04-DEPRECATION-POLICY-P0006
- **Latest Job Pass**: `AUDIT-04-DEPRECATION-POLICY-P0006`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790003115`

#### Recommended Next Step
Verify that no other crates reference these deprecated aliases by running git grep across the workspace. Confirm that the migration test passes. Prepare the removal PR for v0.4.0 by deleting the deprecated type aliases, the migration test, and any #[allow(deprecated)] re-export blocks in lib.rs files.

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

- **Task Description**: AUDIT-09-HOT-PATH-REDUCERS-P0006
- **Latest Job Pass**: `AUDIT-09-HOT-PATH-REDUCERS-P0006`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790002938`

#### Recommended Next Step
Verify production implementation of reduce_trace_bounded and extract_state_delta in dev/emulator_harness/src/reducer.rs to ensure implicit address filtering logic is explicit and robust against address-mismatch bugs.

#### Key Observations
- TraceEvent POD layout verified: 40 bytes, 8-byte alignment, zero-copy serialization via bytemuck confirmed.
- extract_state_delta correctly computes initial/final values and write counts for single address (0xCAFE).
- reduce_trace_bounded enforces ReductionBudget limits; returns TraceError::BudgetExceeded on overflow (limit=2, processed=2). Deterministic replay verified (UNLIMITED == limit=10). Event filtering by memory_addr (0xBEEF vs 0xDEAD) is implicit in test expectations (4 events processed, 2 writes, 1 read). No heap allocations observed in test harness. No todo! or unimplemented! found.

#### Identified Risks
- Implicit filtering of unrelated memory addresses (0xDEAD) in test expectations may hide address-mismatch bugs in production

---

### AUDIT-10-ORCHESTRATOR-BOUNDS

- **Task Description**: AUDIT-10-ORCHESTRATOR-BOUNDS-P0006
- **Latest Job Pass**: `AUDIT-10-ORCHESTRATOR-BOUNDS-P0006`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790002958`

#### Recommended Next Step
Remediate static counter race condition by introducing per-instance fallback key generation using a unique identifier derived from thread-local state or a scoped atomic counter. Add explicit size validation to SharedMemorySynapse constructors. Implement supervisor budget enforcement in TaskRoutingEngine with configurable limits per task type.

#### Key Observations
- IntelligenceEngine::new and new_async retain static AtomicUsize counters for fallback shared memory key generation, creating potential race conditions in multi-instance deployments.
- SharedMemorySynapse::new_sync and new lack explicit size validation or bounds checking before allocation.
- TaskRoutingEngine::new(specialists) and update_specialist_performance lack documented or enforced supervisor budget limits per task type or agent class.

#### Identified Risks
- Race condition in fallback shared memory key generation may cause collisions in multi-instance environments.
- Ambient authority in SharedMemorySynapse allocation could lead to resource exhaustion if size validation is absent.
- Lack of supervisor budget enforcement in TaskRoutingEngine may allow unbounded task routing and resource consumption.

---

### PREP-02-COMPUTE-ENGINE-CONTAINER

- **Task Description**: PREP-02-COMPUTE-ENGINE-CONTAINER-P0006
- **Latest Job Pass**: `PREP-02-COMPUTE-ENGINE-CONTAINER-P0006`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790002969`

#### Recommended Next Step
Remediate by introducing bounded allocation limits, JIT execution bounds, and explicit input validation in the execute method.

#### Key Observations
- Fixed 1MB synapse allocation persists in both test and production paths via SharedMemorySynapse::new_sync with hardcoded 1024 * 1024 bytes.
- JIT execution delegated to cranelift_jit::CraneliftJitEngine via NativeExecutionFn; no direct JIT compilation or execution bounds exposed in public API.
- SI container interfaces (si_binary, si_packer, si_solid_state) define magic bytes, headers, and packing logic but lack explicit execution limits or memory protection state enforcement in ComputeEngine public API (MemoryProtectionState defined in si_jit but not used in execute). JIT_INTENT_DIM defined but not utilized in current execute implementation. No explicit allocation limits or JIT execution bounds enforced in current implementation. The execute method performs no input validation beyond length checks

#### Identified Risks
- Fixed 1MB synapse allocation may cause OOM under high concurrency.
- No explicit JIT execution bounds or memory protection enforcement in the public API.
- Input validation is minimal; potential for buffer overflows in downstream modules.

---

### PREP-03-GOVERNANCE-SMT-INTERLOCKS

- **Task Description**: PREP-03-GOVERNANCE-SMT-INTERLOCKS-P0006
- **Latest Job Pass**: `PREP-03-GOVERNANCE-SMT-INTERLOCKS-P0006`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790002987`

#### Recommended Next Step
Inspect source files for smt_action_interlock, lattice_verifier, and interference_checker to verify remediation of unverified SMT rules, state reduction gaps, and hardware register interference risks.

#### Key Observations
- Module smt_action_interlock exists and exports SmtActionInterlock and GovernanceError, confirming prior audit context.
- Module lattice_verifier exists and exports LatticeVerifier and VerificationReport, confirming prior audit context.
- Module interference_checker exists and exports InterferenceChecker, MAX_HARDWARE_REGISTER, and NonInterferenceReport, confirming prior audit context.

#### Identified Risks
- Unverified SMT rule implementation in smt_action_interlock.
- Potential state reduction gaps in lattice_verifier.
- Hardware register interference risks in interference_checker.

---

### PREP-04-ORCHESTRATOR-TASK-SCHEDULER

- **Task Description**: PREP-04-ORCHESTRATOR-TASK-SCHEDULER-P0006
- **Latest Job Pass**: `PREP-04-ORCHESTRATOR-TASK-SCHEDULER-P0006`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790003002`

#### Recommended Next Step
Implement core affinity pinning, execution budget enforcement, and supervisor tick boundary checks in IntelligenceEngine.

#### Key Observations
- IntelligenceEngine contains TaskRoutingEngine but lacks explicit core affinity pinning (no `pin_current_thread_to_core` usage).
- No execution budget enforcement visible in `analyze_task` or `route_task` methods.
- No supervisor tick boundary checks or `SupervisorBudget` integration in `IntelligenceEngine`.

#### Identified Risks
- Task routing may lack deterministic core affinity, causing cache thrashing.
- Execution budgets may be undefined, risking supervisor tick overflow.

---

### PREP-06-STUDIO-HUD-PANIC-CONTAINMENT

- **Task Description**: PREP-06-STUDIO-HUD-PANIC-CONTAINMENT-P0006
- **Latest Job Pass**: `PREP-06-STUDIO-HUD-PANIC-CONTAINMENT-P0006`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790003016`

#### Recommended Next Step
Implement RFC-0006 Section 7 panic containment: wrap `replay_commands()` in `catch_unwind` and add `TickResult::PluginFaulted` handling with rate-limiting logic to `PluginManager`.

#### Key Observations
- RFC-0006 Section 7 mandates `catch_unwind` at FFI boundary and `TickResult::PluginFaulted` handling, but `crates/studio_hud/src/plugin_api.rs` lacks any FFI boundary wrappers or panic isolation in `replay_commands()`. | Observed: `replay_commands()` executes `ui.label`, `ui.button`, etc., directly without `catch_unwind` wrappers. No panic isolation exists.
- Current `PluginManager` only replays pre-loaded in-process cartridges without fault isolation; no tick execution path invokes external plugins via FFI. | Observed: `PluginManager` holds `static_cartridges` (in-process trait objects) and `replay_commands()` replays RFC-0006 command buffers. No FFI calls exist in current code; dynamic loading is removed per RFC-0006 Section 1.
- No `TickResult::PluginFaulted` handling exists to signal plugin failure to host; no rate-limiting on plugin faults to prevent DoS via repeated panics. | Observed: `TickResult` enum exists in `plugin_api.rs` but is unused. No `plugin_tick` invocation path exists in `PluginManager`. No fault rate-limiting logic exists.

#### Identified Risks
- Uncaught panics in plugin_tick could corrupt host state if FFI boundary is not properly wrapped.
- Missing TickResult::PluginFaulted handling leaves no mechanism to signal plugin failure to host.
- No rate-limiting on plugin faults could allow DoS via repeated plugin panics.

---

