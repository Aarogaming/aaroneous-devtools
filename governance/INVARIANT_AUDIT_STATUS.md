# Continuous Invariant Audit Status

> **STATUS**: AUTOMATED INVARIANT AUDIT ROLLUP
> **PURPOSE**: Aggregates continuous invariant and architectural audits performed by the local GPU worker (`qwen3.5:9b-q6`) across Aaroneous and devtools.
> **CONSTITUTION**: All findings are advisory proposals (`pending-owner-review`). Continuous audits monitor codebase health and verify compliance against AGENTS.md.

## Executive Invariant Matrix

| Invariant Family | Latest Pass | Status | Submitted | Executive Summary |
| :--- | :--- | :--- | :--- | :--- |
| **AUDIT-01-AST-SAFETY** | `AUDIT-01-AST-SAFETY-P0194` | `pending-owner-review` | unix `1790187867` | Update text_encoding.rs to include --others --exclude-standard in git ls-files and implement magi... |
| **AUDIT-02-HOT-PATH-ALLOC** | `AUDIT-02-HOT-PATH-ALLOC-P0017` | `pending-owner-review` | unix `1790187874` | Refactor `PluginManager` to use a bounded ring buffer or slab allocator for loaded libraries, and... |
| **AUDIT-03-ZERO-AMBIENT-AUTH** | `AUDIT-03-ZERO-AMBIENT-AUTH-P0017` | `pending-owner-review` | unix `1790187881` | Refactor discover_root_from_system() to require explicit root injection via WorkspacePathsConfig:... |
| **AUDIT-04-DEPRECATION-POLICY** | `AUDIT-04-DEPRECATION-POLICY-P0016` | `pending-owner-review` | unix `1790187858` | Verify that no other crates (core/hypervisor, crates/paths) contain production call sites for the... |
| **AUDIT-09-HOT-PATH-REDUCERS** | `AUDIT-09-HOT-PATH-REDUCERS-P0016` | `pending-owner-review` | unix `1790186991` | Refactor `extract_state_delta` and `reduce_trace_bounded` to be inline or provide internal visibi... |
| **AUDIT-10-ORCHESTRATOR-BOUNDS** | `AUDIT-10-ORCHESTRATOR-BOUNDS-P0016` | `pending-owner-review` | unix `1790187002` | Implement thread-local counters for fallback key generation and add explicit size validation in S... |
| **AUDIT-11-GIT-DISCOVERY** | `AUDIT-11-GIT-DISCOVERY-P0004` | `pending-owner-review` | unix `1790187008` | Verify if the 'firmware' exclusion is intentional or a gap in hardware abstraction, and assess if... |
| **PREP-02-COMPUTE-ENGINE-CONTAINER** | `PREP-02-COMPUTE-ENGINE-CONTAINER-P0016` | `pending-owner-review` | unix `1790187742` | Implement dynamic configuration for memory allocation limits and input validation bounds to preve... |
| **PREP-03-GOVERNANCE-SMT-INTERLOCKS** | `PREP-03-GOVERNANCE-SMT-INTERLOCKS-P0016` | `pending-owner-review` | unix `1790187823` | Review LatticeVerifier default spatial bounds (7680x4320) and memory limit (64MB) against hardwar... |
| **PREP-04-ORCHESTRATOR-TASK-SCHEDULER** | `PREP-04-ORCHESTRATOR-TASK-SCHEDULER-P0016` | `pending-owner-review` | unix `1790187833` | Refactor IntelligenceEngine to use a custom error enum and implement error propagation in analyze... |
| **PREP-06-STUDIO-HUD-PANIC-CONTAINMENT** | `PREP-06-STUDIO-HUD-PANIC-CONTAINMENT-P0016` | `pending-owner-review` | unix `1790187842` | Implement `catch_unwind` wrappers around `replay_commands` and add fault tracking logic to `Plugi... |

---

## Detailed Invariant Findings

### AUDIT-01-AST-SAFETY

- **Task Description**: Audit AST auditor rules, UTF-8/LF line ending enforcement, and gate command alignment between xtask/src/gate.rs and AGENTS.md Section 6.
- **Latest Job Pass**: `AUDIT-01-AST-SAFETY-P0194`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790187867`

#### Recommended Next Step
Update text_encoding.rs to include --others --exclude-standard in git ls-files and implement magic byte detection. Modify xtask/src/gate.rs to remove hardcoded directory arguments and pass the repository root to ast_auditor.

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

- **Task Description**: AUDIT-02-HOT-PATH-ALLOC-P0017
- **Latest Job Pass**: `AUDIT-02-HOT-PATH-ALLOC-P0017`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790187874`

#### Recommended Next Step
Refactor `PluginManager` to use a bounded ring buffer or slab allocator for loaded libraries, and replace the global static singleton with a thread-safe, zero-copy message-passing interface or `Arc`-wrapped immutable registry protected by fine-grained locking.

#### Key Observations
- Global static `PLUGIN_MANAGER` re-introduces `Lazy<Mutex<PluginManager>>`, violating zero-copy invariant for static initialization.
- Global `PluginManager` struct contains `HashMap` and `Vec<Library>` fields enforcing dynamic heap allocation.
- Dynamic plugin loading via `hotload::load_module` pushes `Library` handles into a `Vec`, creating unbounded memory growth.

#### Identified Risks
- Unbounded memory consumption from `Vec<Library>` accumulation during runtime.
- Potential deadlock or panic if `Mutex` in static initialization is poisoned or contended.
- Violation of hypervisor memory safety invariants due to heap allocations in core modules.
- Unpredictable memory footprint growth from dynamic plugin loading.
- Performance degradation from heap fragmentation and allocation overhead in hot paths.
- Security risk from dynamic loading of untrusted plugins without memory isolation.

---

### AUDIT-03-ZERO-AMBIENT-AUTH

- **Task Description**: Audit paths crate for ambient authority violations (banned functions std::env::var, temp_dir, canonicalize). Verify path normalization via paths::normalize_path and constructor injection.
- **Latest Job Pass**: `AUDIT-03-ZERO-AMBIENT-AUTH-P0017`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790187881`

#### Recommended Next Step
Refactor discover_root_from_system() to require explicit root injection via WorkspacePathsConfig::with_explicit_root() before any environment variable fallbacks. Remove std::env::current_exe usage entirely. Update from_root() to accept a temp_dir parameter or require explicit configuration.

#### Key Observations
- WorkspacePaths::discover() retains ambient authority via std::env::var for CARGO_MANIFEST_DIR and std::env::current_dir, allowing upward traversal from potentially compromised directories.
- WorkspacePaths::discover_root_from_system() uses std::env::current_exe to locate the executable parent directory, enabling path resolution attacks if the process is launched from an untrusted location.
- WorkspacePaths::from_root() defaults temp_dir to dirs::cache_dir() if not explicitly configured, potentially resolving to a user-writable location outside the intended workspace boundary.

#### Identified Risks
- Ambient authority via std::env::var could be exploited if CARGO_MANIFEST_DIR is manipulated to point to an arbitrary directory.
- Traversing upward from current_dir or current_exe allows path traversal attacks if the initial directory is compromised.
- dirs::cache_dir() fallback in from_root() may resolve to a user-writable location outside the intended workspace boundary.
- Environment variable fallbacks in discover() bypass the principle of least privilege by accepting untrusted system state as authoritative.
- Executable-based path resolution (current_exe) introduces supply chain attack surface if the binary is executed from a malicious location.
- Default temp_dir resolution in from_root() creates a writable boundary that may be accessible to unprivileged users or processes.

---

### AUDIT-04-DEPRECATION-POLICY

- **Task Description**: Audit deprecated biological/mythological type aliases against docs/DEPRECATION_POLICY.md. Verify zero production call sites remain and migration equivalence tests pass.
- **Latest Job Pass**: `AUDIT-04-DEPRECATION-POLICY-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790187858`

#### Recommended Next Step
Verify that no other crates (core/hypervisor, crates/paths) contain production call sites for their respective deprecated aliases before proceeding with v0.4.0 removal PR.

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
- **Timestamp**: unix `1790186991`

#### Recommended Next Step
Refactor `extract_state_delta` and `reduce_trace_bounded` to be inline or provide internal visibility to verify zero-allocation claims, and add assertions to `test_bounded_reducer_budget_and_replay` to explicitly validate the filtering of unrelated `0xDEAD` address events.

#### Key Observations
- The `extract_state_delta` and `reduce_trace_bounded` functions remain external imports from the `reducer` module, preventing direct inspection of their allocation behavior within `lib.rs`.
- The test `test_bounded_reducer_budget_and_replay` includes a `0xDEAD` address event in the input but lacks an explicit assertion verifying that this event was filtered out by the reduction logic.
- The `TraceEvent` struct uses `bytemuck` for zero-cost POD layout verification, confirming memory safety constraints for trace replay.

#### Identified Risks
- Deferred implementation prevents immediate verification of zero-allocation claims for `extract_state_delta` and `reduce_trace_bounded`.
- Test assertions rely on expected outputs without validating that the `0xDEAD` address event was correctly filtered out by the implementation.
- Lack of inline implementation obscures potential non-deterministic behavior in state delta extraction.

---

### AUDIT-10-ORCHESTRATOR-BOUNDS

- **Task Description**: Audit orchestrator crate for ambient authority violations, task routing bounds, and supervisor budget enforcement.
- **Latest Job Pass**: `AUDIT-10-ORCHESTRATOR-BOUNDS-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790187002`

#### Recommended Next Step
Implement thread-local counters for fallback key generation and add explicit size validation in SharedMemorySynapse. Define and enforce supervisor budget limits in TaskRoutingEngine.

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

- **Task Description**: Analyze workspace structure, discover open-source Rust patterns and candidate crates for dogfooding, and evaluate architectural influences without circular dependencies or ambient authority violations.
- **Latest Job Pass**: `AUDIT-11-GIT-DISCOVERY-P0004`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790187008`

#### Recommended Next Step
Verify if the 'firmware' exclusion is intentional or a gap in hardware abstraction, and assess if the 3-phase scan model is strictly enforced in the 'core/hypervisor' crate.

#### Key Observations
- Workspace edition is set to '2024' with rust-version '1.85', confirming reliance on unstable nightly features that may break on standard stable toolchains.
- The 'firmware' directory remains excluded from the workspace, maintaining the deliberate separation of hardware abstraction layers noted in prior audits.
- Profile configuration enforces 'fat' LTO and 'codegen-units = 1', which will significantly increase build times and obscure debugging for the 34-member workspace.

#### Identified Risks
- Rust 2024 edition instability poses a high risk of compilation failures on standard toolchains lacking nightly support.
- Fat LTO and codegen-units=1 significantly increase build times and obscure debugging information for a 34-member workspace.
- Prohibition of Mutexes in hot paths complicates debugging and risks subtle race conditions if not rigorously enforced.
- The 5-ring layered protection model increases cognitive load and potential for integration errors across 34 crates.
- Exclusion of 'firmware' may indicate incomplete hardware abstraction or security boundary gaps.
- Strict separation of concerns via Pure State Transition Machines may complicate handling of real-world non-deterministic inputs.

---

### PREP-02-COMPUTE-ENGINE-CONTAINER

- **Task Description**: Analyze ComputeEngine for invariant violations regarding memory allocation, input validation, and JIT execution bounds.
- **Latest Job Pass**: `PREP-02-COMPUTE-ENGINE-CONTAINER-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790187742`

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
- **Latest Job Pass**: `PREP-03-GOVERNANCE-SMT-INTERLOCKS-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790187823`

#### Recommended Next Step
Review LatticeVerifier default spatial bounds (7680x4320) and memory limit (64MB) against hardware specifications to ensure they do not exceed intended physical limits.

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
- **Latest Job Pass**: `PREP-04-ORCHESTRATOR-TASK-SCHEDULER-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790187833`

#### Recommended Next Step
Refactor IntelligenceEngine to use a custom error enum and implement error propagation in analyze_task and route_task.

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

- **Task Description**: Analyze studio_hud FFI boundary panic containment and RFC-0006 compliance
- **Latest Job Pass**: `PREP-06-STUDIO-HUD-PANIC-CONTAINMENT-P0016`
- **Review Status**: `pending-owner-review`
- **Timestamp**: unix `1790187842`

#### Recommended Next Step
Implement `catch_unwind` wrappers around `replay_commands` and add fault tracking logic to `PluginManager` to handle `TickResult::PluginFaulted` and enforce rate-limiting for automatic unloading.

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

