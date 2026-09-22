# Aaroneous Framework Integration Plan

**Status**: Research & Discovery  
**Author**: mimo (opencode)  
**Date**: 2026-09-21  
**Branch**: integration/local-cloud-orchestration

## Context

devtools currently operates as an independent toolchain with **zero Aaroneous dependencies**. It reimplements several capabilities that Aaroneous already provides as proper crate APIs. This document plans the integration of Aaroneous crates into devtools to reduce duplication and leverage the framework we built.

### Dependency Direction

- **devtools → Aaroneous**: Allowed. devtools is the workspace management system that sits on top of the framework.
- **Aaroneous → devtools**: Forbidden. Aaroneous is the pristine component framework and must never depend on devtools.

### Operating Model

Per `governance/OPERATING_MODEL.md`: devtools follows the companion operating model, not Aaroneous's zero-allocation execution-plane constraints. Ordinary Rust services, heap allocation, async I/O are acceptable. Dependencies should be added for concrete needs with license, platform support, and maintenance cost considered.

## Current Duplication

| Capability | devtools Implementation | Aaroneous Equivalent | Gap |
|---|---|---|---|
| Ollama HTTP calls | Hardcoded `reqwest` to `localhost:11434` | `llm_gateway::LocalLLMProvider` | No model discovery, no rate limiting |
| Priority sorting | Manual `(rank, timestamp, id)` sort | `orchestrator::PriorityScheduler` | No backpressure, no admission control |
| AST safety checks | Inline `verify_proposal_quality()` | `ast_auditor::audit_source_file()` | Subset of checks, not a proper audit |
| Reservation locking | 3 identical `Reservation(PathBuf)` structs | `ipc_bus` primitives | Deduplication needed |
| Circuit breaker | Boolean `tripped` flag | `governance::GenerationalJournal` | No history, no rollback |

## Integration Candidates

### Tier 1: Low Friction (No Async Runtime)

#### 1. ast_auditor - Proposal Quality Audit Library

**What**: Import `ast_auditor` as a library dependency to replace inline proposal quality checks.

**Why**: devtools reimplements a subset of ast_auditor's checks in `verify_proposal_quality()`. The proper audit library does more thorough analysis with syn-based visitors.

**Friction**: LOW. Fully synchronous, no tokio, no reqwest. Dependencies: syn, walkdir, sha2, serde.

**Integration Path**:
1. Add `ast_auditor = { path = "../../Aaroneous/crates/ast_auditor" }` to foundation_cli Cargo.toml
2. Replace inline checks in `verify_proposal_quality()` with `audit_source_file()` calls
3. Keep the existing checks as a fast first-pass; ast_auditor as the deep audit

**Risk**: syn with full features adds compile time (~30s first build). No runtime risk.

#### 2. Shared Reservation Module

**What**: Deduplicate the 3 identical `Reservation(PathBuf)` structs into one devtools-internal module.

**Why**: `local_worker.rs`, `multithreader.rs`, and `agentic_registrar.rs` each define the same struct with the same `Drop` implementation. This is pure duplication.

**Friction**: NONE. Internal refactoring, no new dependencies.

**Integration Path**:
1. Create `foundation/crates/foundation_cli/src/reservation.rs`
2. Move `Reservation` struct and `Drop` impl there
3. Update all three files to import from the shared module

### Tier 2: Medium Friction (Dependency Audit Needed)

#### 3. governance - Resource Governors & Rollback Journal

**What**: Import `governance` crate for `GenerationalJournal`, `ResourceGovernor`, and `InterferenceChecker`.

**Why**: devtools' circuit breaker is a boolean flag. `GenerationalJournal` maintains promotion attempt history with rollback capability. `ResourceGovernor` could manage GPU memory budgets.

**Friction**: MEDIUM. governance depends on `compute` (unknown weight). Need to audit whether `compute` brings in tokio.

**Investigation Needed**:
- Audit `compute` crate's dependency tree
- Check if governance types can be used without `compute` features
- Consider feature flags to exclude heavy transitive deps

#### 4. llm_gateway - Types-Only Import

**What**: Import `llm_gateway` types for model configuration and discovery, but not the async runtime.

**Why**: devtools hardcodes `qwen3.5:9b-q6` and `http://localhost:11434`. The gateway has `ModelRegistry`, `AutoDiscoveryResult`, and `LLMConfig` types that could inform model selection.

**Friction**: HIGH for full integration (async-only). LOW for types-only import.

**Investigation Needed**:
- Check if types are in a separate module without async deps
- Consider extracting types to a shared `llm_types` crate
- Alternative: copy the type definitions we need

### Tier 3: High Friction (Extraction Required)

#### 5. PriorityScheduler - Extract from Orchestrator Monolith

**What**: Extract `PriorityScheduler`, `PriorityTier`, `TaskMetadata`, and `SchedulerBackpressureError` from the orchestrator crate into a standalone micro-crate.

**Why**: The orchestrator is a monolith with tokio-full, reqwest, llama-gguf (default-on), and the entire agent runtime. The scheduler itself is 354 lines of pure sync code with only `uuid` and `thiserror` dependencies.

**Friction**: VERY HIGH for direct import. LOW for extraction.

**Investigation Needed**:
- Verify the scheduler's exact dependency boundary
- Check if it can be feature-gated in orchestrator
- Alternative: reimplement the scheduler logic in devtools (354 lines)

## Implementation Order

1. **Shared Reservation Module** (0 deps, pure deduplication)
2. **ast_auditor Integration** (low friction, high value)
3. **PriorityScheduler Extraction** (medium effort, high value)
4. **governance Audit** (investigate compute weight)
5. **llm_gateway Types** (investigate async boundary)

## Research Findings

### ast_auditor: CONFIRMED FEASIBLE

- **Zero async dependencies** - fully synchronous, no tokio
- **Zero workspace-internal dependencies** - all deps are crates.io crates
- **Can audit any .rs file** - no hardcoded path assumptions about Aaroneous checkout
- **5 audit visitors**: AmbientAuthority, HotPathAlloc, PrefixStutter, Soundness (todo!/unimplemented!), MemoryGeometry
- **Blockers**: Two `workspace = true` references (serde_json, tempfile) need resolution
- **Recommendation**: Import directly. Replace inline `verify_proposal_quality()` checks with `audit_source_file()`.

### PriorityScheduler: CONFIRMED TRIVIAL EXTRACTION

- **222 lines of production code** (354 total with tests)
- **Only 2 external deps**: `uuid` and `thiserror`
- **Zero coupling** to orchestrator internals (zero `use crate::` imports)
- **Fully synchronous** - stdlib timing, no tokio
- **4 existing unit tests** covering ordering, capacity, backoff, and failure escalation
- **Recommendation**: Extract to micro-crate. Copy file, create 3-line Cargo.toml, done.

### governance: BLOCKED BY COMPUTE, BUT 9/11 MODULES LIGHTWEIGHT

- **`compute` is a hard dependency** - pulls in tokio (full), candle-core, cranelift, hnsw_rs, rustfft
- **9 of 11 modules have zero compute dependency** - GenerationalJournal, ResourceGovernor, InterferenceChecker, LatticeVerifier, SmtActionInterlock, ConstraintInspector, SystemHealthGovernor, UniversalMetricsExporter, JitAudit
- **2 modules use compute**: throughput_governor (kalman, mpc, thermodynamics) and load_governor (monte carlo)
- **Solution**: Make compute optional with feature flag `adaptive-governance`
- **Recommendation**: Add feature gate in Aaroneous, then import governance without the flag for devtools.

### llm_gateway: TYPES EXTRACTABLE, RUNTIME NOT USABLE

- **Types are fully sync** - all 20+ POD types in types.rs, LLMConfig, ModelInfo, ModelType, ModelRegistry, ModelEnvironment, AutoDiscoveryResult struct
- **Runtime is async-only** - LLMClient, LLMProvider trait, providers, cache, rate_limiter all require tokio
- **Clean separation already exists** - types/config/registry use only std+serde, runtime layer is behind async
- **Extraction path**: Create `llm_types` crate with types.rs, config structs, registry, environment detection. No tokio, no moka, no reqwest.
- **One trivial refactor needed**: ModelLoader::initialize() is declared async but body is sync
- **Recommendation**: Extract types to separate crate. Use for model selection and configuration. Don't import the async runtime.

## Open Questions

1. ~~Should we feature-gate the scheduler in orchestrator or extract to a new crate?~~ **EXTRACT** - trivial, 222 lines, 2 deps
2. ~~Can governance be used without compute?~~ **YES** with feature gate on 2 modules
3. ~~Is there a clean async/sync boundary in llm_gateway for types-only import?~~ **YES** - types are fully sync, runtime is async
4. Should devtools pin a specific Aaroneous version or use path dependencies?

## Next Steps

- [x] Audit `compute` crate dependency tree
- [x] Verify ast_auditor API compatibility with devtools' verification needs
- [x] Check PriorityScheduler's exact dependency boundary in orchestrator
- [x] Investigate llm_gateway module structure for types extraction
- [ ] Implement shared Reservation module
- [ ] Integrate ast_auditor as library dependency
- [ ] Extract PriorityScheduler to micro-crate
- [ ] Add feature gate for compute in governance
- [ ] Extract llm_gateway types to separate crate
