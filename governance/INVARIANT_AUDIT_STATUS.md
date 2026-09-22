# Continuous Invariant Audit Status

> **STATUS**: AUTOMATED INVARIANT AUDIT ROLLUP
> **PURPOSE**: Aggregates continuous invariant and architectural audits performed by the local GPU worker (`qwen3.5:9b-q6`) across Aaroneous and devtools.
> **CONSTITUTION**: All findings are advisory proposals (`pending-owner-review`). Continuous audits monitor codebase health and verify compliance against AGENTS.md.

## Executive Invariant Matrix

| Invariant Family | Latest Pass | Status | Submitted | Executive Summary |
| :--- | :--- | :--- | :--- | :--- |
| **AUDIT-01-AST-SAFETY** | `AUDIT-01-AST-SAFETY-P0193` | `pending-owner-review` | unix `1790050741` | Update text_encoding.rs to include --others --exclude-standard in git ls-files and implement magi... |
| **AUDIT-02-HOT-PATH-ALLOC** | `AUDIT-02-HOT-PATH-ALLOC-P0016` | `pending-owner-review` | unix `1790050748` | Refactor `PluginManager` to use fixed-size arrays or a pre-allocated slab allocator to eliminate ... |
| **AUDIT-03-ZERO-AMBIENT-AUTH** | `AUDIT-03-ZERO-AMBIENT-AUTH-P0016` | `pending-owner-review` | unix `1790050754` | Verify that ambient authority usage is strictly limited to workspace discovery and that no hardco... |
| **AUDIT-04-DEPRECATION-POLICY** | `AUDIT-04-DEPRECATION-POLICY-P0015` | `pending-owner-review` | unix `1790050733` | Verify that no other crates in the workspace reference these deprecated aliases in production cod... |
| **AUDIT-09-HOT-PATH-REDUCERS** | `AUDIT-09-HOT-PATH-REDUCERS-P0016` | `pending-owner-review` | unix `1790050761` | Review the implementation in `dev/emulator_harness/src/reducer.rs` to verify explicit address fil... |
| **AUDIT-10-ORCHESTRATOR-BOUNDS** | `AUDIT-10-ORCHESTRATOR-BOUNDS-P0016` | `pending-owner-review` | unix `1790050772` | Implement size validation in SharedMemorySynapse and enforce supervisor budget limits in TaskRout... |
| **AUDIT-11-GIT-DISCOVERY** | `AUDIT-11-GIT-DISCOVERY` | `pending-owner-review` | unix `1790051387` | Review `docs/ARCHITECTURE.md` Section 3 to verify compliance with the ban on `OnceLock` and `lazy... |
| **PREP-02-COMPUTE-ENGINE-CONTAINER** | `PREP-02-COMPUTE-ENGINE-CONTAINER-P0016` | `pending-owner-review` | unix `1790050776` | Implement configurable memory allocation limits and enforce JIT execution bounds to mitigate OOM ... |
| **PREP-03-GOVERNANCE-SMT-INTERLOCKS** | `PREP-03-GOVERNANCE-SMT-INTERLOCKS-P0016` | `pending-owner-review` | unix `1790050788` | Verify LatticeVerifier default constants (max_spatial_width/height, max_linear_memory_bytes) alig... |
| **PREP-04-ORCHESTRATOR-TASK-SCHEDULER** | `PREP-04-ORCHESTRATOR-TASK-SCHEDULER-P0016` | `pending-owner-review` | unix `1790050796` | Refactor IntelligenceEngine to use a custom error enum for structured propagation and implement f... |
| **PREP-06-STUDIO-HUD-PANIC-CONTAINMENT** | `PREP-06-STUDIO-HUD-PANIC-CONTAINMENT-P0016` | `pending-owner-review` | unix `1790050824` | Implement `catch_unwind` wrappers around `replay_commands` and add fault tracking logic to `Plugi... |

---

## Detailed Invariant Findings

### AUDIT-01-AST-SAFETY

- **Task Description**: Audit AST auditor rules, UTF-8/LF line ending enforcement, and gate command alignment between xtask/src/gate.rs and AGENTS.md Section 6.
- **Latest Job Pass**: `AUDIT-01-AST-SAFETY-P0193`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790050741`

#### Recommended Next Step
Update text_encoding.rs to include --others --exclude-standard in git ls-files and implement magic byte detection. Modify xtask/src/gate.rs to pass the repository root to ast_auditor instead of hardcoded paths.

#### Key Observations
- text_encoding.rs: audit_tracked_encodings invokes git ls-files without --others --exclude-standard, failing to audit untracked files.
- text_encoding.rs: is_binary_extension relies solely on a static BINARY_EXTENSIONS list, lacking magic byte detection for unknown binary formats.
- xtask/src/gate.rs: Gate 6 restricts ast_auditor to hardcoded directories (core/, crates/, dev/emulator_harness/), violating the requirement to audit all Git-tracked text files.

#### Identified Risks
- Untracked files containing binary content or invalid encodings may bypass the audit entirely.
- Text files with non-standard extensions may be incorrectly classified as binary and skipped.
- Gate 6 may miss files located outside the hardcoded directories, allowing invariant violations to persist.
- Static extension lists may become outdated as new file types are introduced to the repository.
- Magic byte detection is absent, reducing the robustness of binary file classification.
- Hardcoded paths in gate.rs create a maintenance burden and potential for drift from actual repository structure.

---

### AUDIT-02-HOT-PATH-ALLOC

- **Task Description**: AUDIT-02-HOT-PATH-ALLOC-P0016
- **Latest Job Pass**: `AUDIT-02-HOT-PATH-ALLOC-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790050748`

#### Recommended Next Step
Refactor `PluginManager` to use fixed-size arrays or a pre-allocated slab allocator to eliminate dynamic heap allocations and Mutex contention in the hypervisor core.

#### Key Observations
- Forbidden dynamic heap allocations detected: `std::collections::HashMap` in `PluginManager` struct.
- Forbidden dynamic heap allocations detected: `Vec<Library>` in `PluginManager` struct.
- Forbidden dynamic heap allocations detected: `std::sync::Mutex` in `PLUGIN_MANAGER` static initialization.

#### Identified Risks
- Use of `HashMap` and `Vec` in `PluginManager` violates the zero-copy and bounded memory allocation invariants required for a hypervisor core.
- Use of `Mutex` in `PLUGIN_MANAGER` static initialization may introduce contention and potential deadlocks in a high-concurrency hypervisor environment.
- Dynamic loading of plugins via `hotload` combined with heap-allocated descriptors creates an unpredictable memory footprint during runtime.

---

### AUDIT-03-ZERO-AMBIENT-AUTH

- **Task Description**: Audit paths crate for ambient authority violations
- **Latest Job Pass**: `AUDIT-03-ZERO-AMBIENT-AUTH-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790050754`

#### Recommended Next Step
Verify that ambient authority usage is strictly limited to workspace discovery and that no hardcoded absolute paths exist outside of OS-specific fallbacks.

#### Key Observations
- WorkspacePaths::discover() relies on std::env::var for CARGO_MANIFEST_DIR, current_dir, and current_exe to traverse upward for workspace root discovery.
- discover_root_from_system() uses std::env::current_exe to locate the executable parent directory for path resolution.
- WorkspacePaths::from_root() defaults temp_dir to dirs::cache_dir() if not explicitly configured via constructor injection.

#### Identified Risks
- Ambient authority via std::env::var could be exploited if CARGO_MANIFEST_DIR is manipulated to point to an arbitrary directory.
- Traversing upward from current_dir or current_exe allows path traversal attacks if the initial directory is compromised.
- dirs::cache_dir() fallback in from_root() may resolve to a user-writable location outside the intended workspace boundary.

---

### AUDIT-04-DEPRECATION-POLICY

- **Task Description**: Audit deprecated biological/mythological type aliases against docs/DEPRECATION_POLICY.md. Verify zero production call sites remain and migration equivalence tests pass.
- **Latest Job Pass**: `AUDIT-04-DEPRECATION-POLICY-P0015`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790050733`

#### Recommended Next Step
Verify that no other crates in the workspace reference these deprecated aliases in production code, then proceed with the v0.4.0 removal PR as a standalone change.

#### Key Observations
- Deprecated type aliases in capability_spec.rs are defined with #[deprecated(since = "0.3.3", note = "...")] attributes matching policy requirements.
- Migration test test_legacy_terminology_aliases_equivalence exists at line 590-621 using TypeId assertions to verify deprecated aliases resolve to canonical types.
- No production call sites found in capability_spec.rs using deprecated aliases; all usage is limited to definition sites and the migration test block itself.

#### Identified Risks
- If the migration test fails, it indicates a mismatch between deprecated aliases and their canonical replacements, which would break the removal process.
- If any production code in other crates references these deprecated aliases, the removal will fail with compilation errors.
- The removal PR must be standalone and not mixed with feature work per policy section 4.1.
- Missing documentation updates in DEPRECATION_POLICY.md section 3 regarding current status of aliases.
- Potential confusion if developers attempt to use deprecated aliases in new code without noticing the deprecation warnings.
- Incomplete migration if any internal consumers were missed during the M7 and M22 migrations.

---

### AUDIT-09-HOT-PATH-REDUCERS

- **Task Description**: Audit compute engine and emulator harness for zero-allocation trace reduction, bounded execution budgets, and deterministic trace replay guarantees.
- **Latest Job Pass**: `AUDIT-09-HOT-PATH-REDUCERS-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790050761`

#### Recommended Next Step
Review the implementation in `dev/emulator_harness/src/reducer.rs` to verify explicit address filtering logic and zero-allocation guarantees match the test expectations.

#### Key Observations
- Implementation of `extract_state_delta` and `reduce_trace_bounded` is deferred to external modules (`reducer`, `types`) rather than being inline in the provided evidence file.
- Test `test_bounded_reducer_budget_and_replay` validates deterministic replay by asserting equality between unlimited and limited budget runs, but does not verify the internal filtering logic against the `0xDEAD` address event.
- The `TraceEvent` struct uses `bytemuck` for zero-cost POD layout verification, confirming memory safety constraints for trace replay.

#### Identified Risks
- Deferred implementation prevents immediate verification of zero-allocation claims for `extract_state_delta` and `reduce_trace_bounded`.
- Test assertions rely on expected outputs without validating that the `0xDEAD` address event was correctly filtered out by the implementation.
- Lack of inline implementation obscures potential non-deterministic behavior in state delta extraction.

---

### AUDIT-10-ORCHESTRATOR-BOUNDS

- **Task Description**: AUDIT-10-ORCHESTRATOR-BOUNDS-P0016
- **Latest Job Pass**: `AUDIT-10-ORCHESTRATOR-BOUNDS-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790050772`

#### Recommended Next Step
Implement size validation in SharedMemorySynapse and enforce supervisor budget limits in TaskRoutingEngine to address identified invariant violations.

#### Key Observations
- Static AtomicUsize counter in IntelligenceEngine::new and new_async creates race conditions in multi-instance deployments.
- SharedMemorySynapse::new_sync and new lack explicit size validation or bounds checking before allocation.
- TaskRoutingEngine::new and update_specialist_performance lack documented or enforced supervisor budget limits per task type or agent class.

#### Identified Risks
- Race condition in fallback shared memory key generation may cause collisions in multi-instance environments.
- Ambient authority in SharedMemorySynapse allocation could lead to resource exhaustion if size validation is absent.
- Lack of supervisor budget enforcement in TaskRoutingEngine may allow unbounded task routing and resource consumption.

---

### AUDIT-11-GIT-DISCOVERY

- **Task Description**: AUDIT-11-GIT-DISCOVERY
- **Latest Job Pass**: `AUDIT-11-GIT-DISCOVERY`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790051387`

#### Recommended Next Step
Review `docs/ARCHITECTURE.md` Section 3 to verify compliance with the ban on `OnceLock` and `lazy_static` in runtime state initialization.

#### Key Observations
- Workspace utilizes Rust 2024 edition with resolver 2, indicating a modern, forward-looking dependency resolution strategy.
- Architecture enforces strict determinism via Pure State Transition Machines, prohibiting side effects in reducers to ensure reproducibility.
- Memory safety is prioritized through SWMR ring buffers and explicit bans on mutexes in hot paths to eliminate thread parking latency.

#### Identified Risks
- Edition 2024 and Rust 1.85 are cutting-edge; potential compatibility issues with older CI runners or specific toolchains.
- Strict prohibition of mutexes in hot paths increases complexity in debugging race conditions and requires rigorous testing.
- Complexity of the 5-ring layered protection model may introduce integration friction between subsystems like `orchestrator` and `platform_bridge`.

---

### PREP-02-COMPUTE-ENGINE-CONTAINER

- **Task Description**: Analyze ComputeEngine for invariant violations regarding memory allocation and JIT execution bounds.
- **Latest Job Pass**: `PREP-02-COMPUTE-ENGINE-CONTAINER-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790050776`

#### Recommended Next Step
Implement configurable memory allocation limits and enforce JIT execution bounds to mitigate OOM risks and unbounded resource consumption.

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

- **Task Description**: Governance SMT verification audit
- **Latest Job Pass**: `PREP-03-GOVERNANCE-SMT-INTERLOCKS-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790050788`

#### Recommended Next Step
Verify LatticeVerifier default constants (max_spatial_width/height, max_linear_memory_bytes) align with hardware specifications and audit requirements.

#### Key Observations
- SmtActionInterlock::new() correctly initializes LatticeVerifier with epsilon bound, maintaining single-source-of-truth configuration.
- evaluate_action_graph() returns denial certificates for SMT prover rejections instead of propagating errors, preserving batch verification integrity.
- evaluate_concurrent_actions() enforces individual graph safety via evaluate_action_graph before invoking verify_non_interference.

#### Identified Risks
- LatticeVerifier default spatial bounds (7680x4320) may exceed intended hardware limits if not explicitly configured.
- Default max_linear_memory_bytes (64MB) is hardcoded in LatticeVerifier::default() and may not reflect dynamic resource constraints.
- Default max_free_energy_epsilon (100.0) in LatticeVerifier differs from strict mode (0.05) in SmtActionInterlock, creating potential configuration drift.
- verify_spatial_containment() does not validate coordinate offsets against NativeComputationalGraph schema, relying on prior audit assumptions.
- Thermodynamic bound checks in evaluate_action_graph() and LatticeVerifier::verify_thermodynamic_bound() use separate logic paths, risking inconsistent enforcement.
- Emergency killswitch state is not persisted across process restarts, potentially allowing unauthorized execution after system reboot.

---

### PREP-04-ORCHESTRATOR-TASK-SCHEDULER

- **Task Description**: Verify invariant violations in IntelligenceEngine error handling and routing logic.
- **Latest Job Pass**: `PREP-04-ORCHESTRATOR-TASK-SCHEDULER-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790050796`

#### Recommended Next Step
Refactor IntelligenceEngine to use a custom error enum for structured propagation and implement fallback logic in route_task.

#### Key Observations
- IntelligenceEngine::new and new_async return anyhow::Result, violating reviewer feedback requiring concrete error enum.
- IntelligenceEngine::analyze_task returns anyhow::Result, violating reviewer feedback requiring structured error propagation.
- IntelligenceEngine::route_task returns RoutingDecision directly without error handling, masking potential routing failures.

#### Identified Risks
- Silent failures in task routing if the router encounters an error state.
- Lack of deterministic core affinity pinning may lead to cache thrashing.
- Undefined execution budgets may cause supervisor tick overflow.

---

### PREP-06-STUDIO-HUD-PANIC-CONTAINMENT

- **Task Description**: Analyze studio_hud FFI boundary panic containment and TickResult fault handling compliance.
- **Latest Job Pass**: `PREP-06-STUDIO-HUD-PANIC-CONTAINMENT-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790050824`

#### Recommended Next Step
Implement `catch_unwind` wrappers around `replay_commands` and add fault tracking logic to `PluginManager` to handle `TickResult::PluginFaulted` and enforce rate-limiting as specified in RFC-0006 Section 7.

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

