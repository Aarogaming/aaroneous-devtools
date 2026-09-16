# C4 Inventory: WIT, Registry, Config, and Terminology

- **Date**: 2026-09-16
- **Baseline**: Aaroneous Wave 11 (`6a80fb2`)
- **Author**: Antigravity / Gemini 3.8
- **Status**: Completed (Read-Only Inventory & Compatibility Analysis)
- **Target Repository**: `Aaroneous` (`codex/repository-hardening`)

---

## 1. Executive Summary

This inventory audits four distinct areas in the Aaroneous codebase:
1. **WebAssembly Interface Type (WIT) interfaces** (`core/hypervisor/wit/`, `data/fabrication/*/wit/`, `specs/wit/`)
2. **Schema and preset registries** (`registry/**`)
3. **Configuration trees** (`config/**`)
4. **Residual biological/mythological/analogy-based terminology** across active Rust crates (`core/hypervisor`, `crates/adaptation_plane`, `crates/orchestrator`, `crates/paths`, `crates/governance`, `crates/compute`)

All findings are categorized by architectural and operational risk according to the following impact taxonomy:
- **Cat 1: Wire ABI / Inter-Process Serialization Risk** (High)
- **Cat 2: File Format / On-Disk Persistence Incompatibility** (Medium-High)
- **Cat 3: Configuration Breaking Incompatibility** (Medium)
- **Cat 4: Public Trait / Rust Symbol Deprecation** (Low-Medium)
- **Cat 5: Zero-Risk Dead Code / Documentation Artifacts** (Zero)

---

## 2. WebAssembly Interface Type (WIT) Inventory

### 2.1 Discovered Interfaces

| Path | Package Name | Worlds / Interfaces Defined | Host / Plugin Functions | Active Callers in Rust Crates |
| :--- | :--- | :--- | :--- | :--- |
| `core/hypervisor/wit/worker.wit` | `aaroneous:enzyme` | `world enzyme`, `interface host` | Host: `log`, `get-synapse-data`, `set-synapse-data`<br>Plugin: `process-task`, `get-metabolism` | **0** (No `wit-bindgen` or macro invocations) |
| `data/fabrication/*/wit/sab.wit` (102 instances) | `aaroneous:sab` | `world sab-plugin`, `interface core` | Plugin: `describe`, `execute` | **0** (Generated or historical fabrication manifests) |

### 2.2 Verification Findings
1. **Build Tooling & Bindgen**: A workspace-wide search for `wit_bindgen`, `wit-bindgen`, and `.wit` confirmed that **zero** Cargo manifests in Aaroneous reference `wit-bindgen` or `wasmtime::component::bindgen`.
2. **Wasm Execution Reality**: In `core/hypervisor/src/action_executor.rs` and `splicing_engine.rs`, Wasm modules are loaded via Wasmtime using standard WASI or raw exported functions (`process`, `alloc`, `dealloc`), not Component Model WIT interfaces.
3. **Terminology Violations**:
   - `core/hypervisor/wit/worker.wit` contains package `aaroneous:enzyme` (prefix stutter + biological term) and exports `get-metabolism` and `get-synapse-data`.
   - `data/fabrication/*/wit/sab.wit` contains docstrings referencing `"Omni Relic"`.

### 2.3 Compatibility Impact
- **Classification**: **Cat 5 (Zero-Risk Dead Code)**.
- **Remediation Recommendation**: The `.wit` files are non-blocking artifacts. If Aaroneous transitions to standard WebAssembly Component Model plugins in the future, these should be rewritten under standard systems naming (e.g., `aaroneous_component:worker` or `system:module`) with pure POSIX/host capabilities instead of biological concepts. For now, they can be safely archived or maintained as historical interface sketches.

---

## 3. Schema & Preset Registries (`registry/**`)

### 3.1 Overview
The `registry/` directory contains 49 JSON/TOML files defining node capabilities, presets, model configurations, and tool descriptors.

### 3.2 Key Registry Files & Consumers

| File | Primary Consumer | Active Keys vs. Legacy Keys | Compatibility Impact |
| :--- | :--- | :--- | :--- |
| `registry/node_map.json` | `core/hypervisor/src/profile_registry.rs` | Migrated top-level schema to `nodes`. Individual entry keys still retain `node_id: "specialist-argus-v1"`, `SUPERVISOR_ID: "specialist-sentinel"`. | **Cat 2: File Format** |
| `registry/agent_*.json` (e.g., `agent_argus.json`, `agent_merlin.json`, `agent_ariel.json`) | `crates/paths/src/lib.rs` (`agent_preset`), `core/hypervisor/src/profile_persistence.rs` | Top-level metadata uses `agent_id`, `capabilities`, `traits`. Contains historical model weights and prompt anchors. | **Cat 2: File Format** |
| `registry/module_store_*.json` | `crates/paths/src/lib.rs` (`relic_hox_preset`) | Historical store profiles. Fallback in `paths` targets `hox_{name}.json` if missing. | **Cat 2: File Format** |
| `registry/sab_matrix.json` | `core/hypervisor/src/sabs.rs` (`SabMatrix`) | Maps 102 fabrication SABs to capability IDs. Contains stale source paths (`src/federation/hive/mod.rs`, `src/event_loop.rs`). | **Cat 2 / Cat 5** |
| `registry/genome/schemas/*.json` | None (Orphaned JSON schemas) | Schemas for `genome_manifest`, `epigenetic_profile`, `organ_descriptor`. | **Cat 5: Dead Code** |
| `registry/target_crates.txt` | Developer audit scripts | Plaintext list of target crates for refactoring. | **Cat 5: Dead Code** |

### 3.3 Compatibility & Invariant Nuances
1. **Fallback in `crates/paths`**:
   - `WorkspacePaths::agent_preset(name)` checks for `registry/agent_{name}.json`, and if not present, falls back to `registry/hox_{name}.json`.
   - `WorkspacePaths::relic_hox_preset(name)` checks for `registry/module_store_{name}.json`, and if not present, falls back to `sovereign_hox_preset(name)`.
2. **Database Invariants**:
   - `core/hypervisor/src/profile_registry.rs`: SQLite table was successfully migrated to `node_capabilities`, but deserialization structs and test harnesses still accept `enzyme_hash` and `hox_id` as fallback column aliases.

---

## 4. Configuration Tree (`config/**`)

### 4.1 Overview
The `config/` directory contains 25 files across TOML, JSON, and schema formats.

### 4.2 Configuration File Audit

| File | Status | Primary Consumer | Content / Legacy Elements | Compatibility Risk |
| :--- | :--- | :--- | :--- | :--- |
| `config/config.toml` | **Active** | `core/hypervisor/bin/hypervisor.rs`, `bootstrap` | System execution loop, core pinning, telemetry, IPC shared memory buffer sizes (`swmr_shm_size_mb`). Clean systems schema. | **Cat 3: Config Invariant** (Must not break without schema bump) |
| `config/predictive_models.toml` | **Active** | `core/hypervisor/src/predictive_load_balancer.rs` | GGUF model paths, context lengths, batch dimensions. | **Cat 3: Config Invariant** |
| `config/agent_registry.json` | Stale | None (Replaced by `registry/node_map.json`) | Lists historical agents (`argus`, `merlin`, `hephaestus`). | **Cat 5: Dead Code** |
| `config/capability_map.json` | Stale | None | Maps capability tokens to organ IDs. | **Cat 5: Dead Code** |
| `config/capability_evolution_system.json`| Stale | None | Biological evolution parameters (`mutation_rate`, `genetic_crossover`). | **Cat 5: Dead Code** |
| `config/district_nats_topology.json` | Stale | None | NATS mesh configuration for distributed multi-node clusters. | **Cat 5: Dead Code** |
| `config/runtime.manifest.json` | Stale | Historical packager | References deleted paths (`data/chromosomes/`, `bin/a-run.exe`, `hox_map.json`). | **Cat 5: Dead Code** |

### 4.3 Safe Deletion & Retention Matrix
- **MUST KEEP**:
  - `config/config.toml`
  - `config/predictive_models.toml`
  - `config/default.toml` / environment overrides
- **SAFE TO ARCHIVE OR PURGE (Cat 5)**:
  - `config/agent_registry.json`
  - `config/capability_map.json`
  - `config/capability_evolution_system.json`
  - `config/district_nats_topology.json`
  - `config/runtime.manifest.json`

---

## 5. Residual Biological, Mythological & Analogy Terminology

Despite Waves 1–11 cleaning major types in `crates/ipc_bus`, `crates/compute`, and `crates/orchestrator`, several layers of backward-compatibility shims and residual internal identifiers remain.

### 5.1 `core/hypervisor/src/lib.rs`
The crate root maintains multiple compatibility re-exports that expose biological modules to external or test consumers:

```rust
// Residual module declarations and re-exports in hypervisor::lib.rs:
pub extern crate ipc_bus as nervous_system;
pub extern crate adaptation_plane as evolution;
pub extern crate governance as biology;
pub use governance as system_health;

pub mod genetics {
    pub use evolution::genetics::*;
}
pub mod digestion {
    pub use evolution::self_digestion::*;
}
pub mod skills {
    pub use evolution::skills::*;
}

// Biological type re-exports:
pub use crate::genetics::{
    BreedingOperation, EpigeneticState, GeneticAnalyzer, GeneticCategory, GeneticLocus, LociSource,
    SpecialistGenome,
};
pub use crate::digestion::{
    DigestionConfig, DigestionEngine, DigestionEvent, DigestionTask, ExperienceProfile,
    NarrativeProfile, PersonalityProfile, RelationalProfile, SpecialistPersona,
};
pub use biology::{
    MetabolicForecast, MetabolicGovernorConfig, PredictiveMetabolicGovernor,
    SpecialistHealth, SpecialistMetabolism, SystemBiology, SystemHealthReport,
};
```

- **Impact**: **Cat 4 (Public Trait / Re-export Shim)**.
- **Analysis**: These shims are not used in production binaries, but removing them requires updating downstream integration tests and legacy test modules that still invoke `hypervisor::genetics::*`.

### 5.2 `crates/adaptation_plane/src/capability_spec.rs`
The adaptation plane renamed canonical types to `ProfileLocus`, `ProfileCategory`, `AdaptationState`, and `AgentProfile`. However, public type aliases were retained:

```rust
pub type EpigeneticState = AdaptationState;
pub type SpecialistGenome = AgentProfile;
pub type ParameterGenome = ParameterProfile;
pub type GeneticLocus = ProfileLocus;
pub type GeneticCategory = ProfileCategory;
pub type GeneticRelationship = ProfileRelationship;
pub type GeneticAnalyzer = ProfileAnalyzer;
```

- **Impact**: **Cat 4 (Public Trait / Re-export Shim)**.
- **Analysis**: Zero runtime overhead. The aliases allow code to compile without atomic multi-repo lockstep updates.

### 5.3 `core/hypervisor/src/profile_compiler.rs` & `profile_schema.rs`
- In `profile_compiler.rs`:
  - `pub type GenomeCompiler = ProfileCompiler;`
  - Doc comments and error messages still reference `"Genome compilation error"`.
- In `profile_schema.rs`:
  - `pub struct HoxMap` is still the canonical struct name deserializing node configurations.
  - `pub type HoxPermissions = NodePermissions;`

### 5.4 Mythological / Specialist Identifiers in `crates/orchestrator` & `crates/capabilities`
- `crates/orchestrator/src/agents.rs`:
  - Agent IDs: `"argus"`, `"merlin"`, `"ariel"`, `"dionysus"`, `"hephaestus"`.
  - These represent functional operational roles (e.g. Argus = Vision/Security Sentinel, Merlin = Knowledge/Inference, Ariel = UI/Audio).
  - While named mythologically, they function as string configuration constants / actor IDs rather than type constructs.

---

## 6. Compatibility Impact Classification Table

| Component / Path | Current Item | Target Standard Terminology | Compatibility Impact | Safe to Mutate Now? |
| :--- | :--- | :--- | :--- | :--- |
| `core/hypervisor/wit/worker.wit` | `package aaroneous:enzyme` | `package aaroneous:worker` or `system:module` | **Cat 5 (Dead Code)** | Yes (No runtime callers) |
| `data/fabrication/*/wit/sab.wit` | `package aaroneous:sab` | Archive / Standardize | **Cat 5 (Dead Code)** | Yes (Build does not read them) |
| `registry/node_map.json` | `specialist-*`, `SUPERVISOR_ID` | `node-*`, `supervisor_id` | **Cat 2 (File Format)** | No — Requires coordinated schema update in `profile_registry.rs` |
| `registry/agent_*.json` | Filename and ID fields | `node_*.json` | **Cat 2 (File Format)** | No — `WorkspacePaths::agent_preset` expects `agent_*.json` |
| `crates/paths/src/lib.rs` | `agent_preset` / `relic_hox_preset` | `node_preset` / `module_store_preset` | **Cat 4 (Public Trait / API)** | Deprecate with forwarding shims |
| `config/config.toml` | Systems config | Keep canonical schema | **Cat 3 (Config Invariant)** | Production contract; do not break keys |
| `config/*.manifest.json` | Historical manifests | Remove / Archive | **Cat 5 (Dead Code)** | Yes (Unreferenced by Cargo workspace) |
| `core/hypervisor/src/lib.rs` | `pub mod genetics`, `pub mod digestion` | Remove or relocate to `adaptation_plane` | **Cat 4 (API Re-export)** | Phased cleanup after updating internal tests |
| `crates/adaptation_plane/src/capability_spec.rs` | `pub type SpecialistGenome = ...` | Deprecate aliases with `#[deprecated]` | **Cat 4 (API Aliases)** | Safe to add `#[deprecated]` attributes |

---

## 7. Recommended Phased Remediation Order

To maintain workspace stability, zero-downtime test passing, and CI compliance, residual cleanups should proceed across three distinct waves:

### Phase 1: Pure Dead Code Pruning (Zero Risk)
- Archive or remove unreferenced configuration files: `config/district_nats_topology.json`, `config/capability_evolution_system.json`, `config/runtime.manifest.json`, `config/agent_registry.json`.
- Archive or isolate orphaned schema definitions under `registry/genome/`.

### Phase 2: Deprecation Markings & Internal Symbol Migration (Low Risk)
- Add `#[deprecated(note = "use Profile...")]` to legacy biological type aliases in `crates/adaptation_plane/src/capability_spec.rs` and `core/hypervisor/src/lib.rs`.
- Replace all remaining internal usages of `SpecialistGenome` and `EpigeneticState` in integration tests with `AgentProfile` and `AdaptationState`.
- Update `core/hypervisor/src/profile_schema.rs` to introduce `NodeMap` as the canonical struct name and make `HoxMap` a deprecated type alias.

### Phase 3: File Format & Registry Stabilization (Medium Risk)
- Introduce a versioned migration for `registry/node_map.json` and `registry/agent_*.json` that normalizes actor identifiers (`specialist-argus` &rarr; `node-sentinel-argus`).
- Update `crates/paths/src/lib.rs` to deprecate `relic_hox_preset` in favor of `module_preset`, preserving backward-compatible filesystem lookups during the transition.
