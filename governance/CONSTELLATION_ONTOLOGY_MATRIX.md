# Constellation Ontology Matrix & Embodiment Architecture

> **STATUS**: BINDING GOVERNANCE STANDARD (Companion to `Aarogaming/aaroneous#153`)  
> **APPLIES TO**: ALL REPOSITORIES IN THE AAROGAMING CONSTELLATION  
> **DATE**: 2026-11-18  
> **CORRESPONDING PRODUCT PR**: [Aarogaming/aaroneous#153](https://github.com/Aarogaming/aaroneous/pull/153)

---

## 1. Executive Summary & The Core Invariant

The Aarogaming constellation comprises 12 distinct repositories evolved over multiple generations of embodiment, automation, and systems research. Rather than attempting brute-force repository consolidation or lossy rewrites, this governance standard formally establishes the **Layer 0 Working Ontology** and the foundational separation invariant:

$$\mathbf{Facts} \neq \mathbf{Authority} \neq \mathbf{Actions}$$

### The Three Boundaries

1. **Facts (Perception & Observation)**
   - *Responsibility:* Observe environment, establish trusted facts, qualify timestamps and memory provenance.
   - *Rule:* **Facts describe reality. Facts NEVER authorize actions.**
   - *Constellation Subsystems:* `W101FactsStore`, `AutoWizard101` OCR/UI tree parsers, `DanceBot` arrow detectors, `platform_bridge` DXGI/Win32 hook ingestors.

2. **Authority (Governance & Trust)**
   - *Responsibility:* Determine permission, verify cryptographic signatures, enforce safety interlocks, manage recovery holds.
   - *Rule:* **Authority decides permitted transitions. Authority NEVER performs transitions.**
   - *Constellation Subsystems:* `MyFortress` credential fences & gRPC gateway, `aaroneous` SMT interlocks (`crates/governance`), `aaroneous-devtools` multi-agent quorum and `merge_gate.py`.

3. **Actions (Execution & Actuation)**
   - *Responsibility:* Execute transitions within authorized bounds, consume execution tickets, emit operational telemetry.
   - *Rule:* **Actions execute transitions. Actions NEVER self-authorize; actions consume authority.**
   - *Constellation Subsystems:* `core/hypervisor` scan-loop reducers, `AAS` process orchestration launcher, `platform_bridge` input dispatchers.

---

## 2. Empirical Repository Census Baseline

All metrics and topologies below are derived directly from authenticated recursive Git tree inspections across the `Aarogaming` organization on GitHub (2026-11-18):

| Repository | Head Ref & Commit | Object Count | Primary File Types | Operational Focus & Key Paths |
|---|---|---|---|---|
| **`aaroneous`** | `main` @ `cc794cb3` | 1,400+ | `.rs` (92%), `.toml` | Microkernel engine, lock-free ring buffers (`ipc_bus`), formal governance |
| **`aaroneous-devtools`** | `main` @ `a87ec8f2` | 2,800+ | `.py` (60%), `.json`, `.tsx` | Multi-agent quorum, verification harnesses, steward daemons, GUI |
| **`AaroneousAutomationSuite`** | `master` @ `4b52088a` | 6,669 | `.md` (2,486), `.json` (2,060), `.py` (1,105) | Windows process automation, `src/kernel/fix_kernel.py`, `AAS_Launcher.bat` |
| **`Maelstrom`** | `main` @ `bb6e3608` | 2,073 | `.png` (325), `.py` (240), `.cs` (152) | Game perception overlay (`Client/merlin_overlay.py`), combat planning |
| **`AutoWizard101`** | `main` @ `5b5c2400` | 3,888 | `.png` (588), `.js` (388), `.py` (210), `.cs` (154) | UI test runners (`UiAuditRunner`), Win32 automation, OCR scripts |
| **`Guild`** | `main` @ `b3749df8` | 43,099 | `.txt` (20,232), `.py` (2,771), `.json` (2,278) | Multi-process IPC hub (`core/ipc_hub.py`), checked-in `.venv_precommit` |
| **`Merlin`** | `main` @ `191b9371` | 5,295 | `.json` (1,411), `.py` (212), `.kt`, `.cs` | Multi-stack reasoning engine (Electron JS, Kotlin Android/Desktop, Unity C#) |
| **`MyFortress`** | `main` @ `3a9b922f` | 383 | `.json` (238), `.py` (60) | Credential fence, gRPC gateway (`gateway/api/grpc_server.py`), rate limiters |
| **`Library`** | `main` @ `9ee5442d` | 340 | `.json` (112), `.py` (63) | Plugin catalog registry (`plugins/catalog/plugin.py`), discovery schemas |
| **`Workbench`** | `main` @ `7df1b0ca` | 1,198 | `.py` (586), `.cs` (77) | Asset ingestion pipeline (`asset_ingestion.py`), inspection tools |
| **`AndroidApp`** | `main` @ `b68dbf35` | 328 | `.kt` (134), `.xml` (34) | Mobile client interface, accessibility rules (`AccessibilityChecksRule.kt`) |
| **`Wizard101_DanceBot`** | `main` @ `ca0ec613` | 63 | `.py` (21), `.png` (18) | Computer-vision arrow calibration (`calibrate_arrows.py`), minigame timing |

---

## 3. The Six Layer 0 Primitives

Every subsystem across the constellation maps onto the six Layer 0 primitives:

1. **State:** Immutable point-in-time observations and current system registers.
2. **Transition:** Pure functions mapping state and input to candidate next states ($S_{t+1} = f(S_t, I)$).
3. **Relationship:** Structural, dependency, and topological linkages between entities.
4. **Constraint:** Invariant mathematical rules, bounds, and preconditions enforced by governance.
5. **Trust:** Provenance, cryptographic signatures, attestation records, and qualification flags.
6. **Perspective:** Contextual viewports, agent reasoning horizons, and heuristic planners.

---

## 4. The 4-Layer Superproject Unification Roadmap

To transition `aaroneous` and `aaroneous-devtools` from disjoint repositories into an integrated, self-governing cybernetic platform:

```
┌────────────────────────────────────────────────────────────────────────┐
│                          SUPERPROJECT PLATFORM                         │
├────────────────────────────────────────────────────────────────────────┤
│  Layer 1: Structural Manifest & Repository Topology                    │
│    - One-way dependency isolation (product never depends on devtools)  │
│    - Workspace manifests coordinate version pins and contracts         │
├────────────────────────────────────────────────────────────────────────┤
│  Layer 2: Real-Time Nervous System (IPC & MCP Bridge)                  │
│    - Lock-free SWMR ring buffers (crates/ipc_bus) in shared memory     │
│    - Native Model Context Protocol server (crates/mcp_server) for live │
│      agent inspection and parameter control                            │
├────────────────────────────────────────────────────────────────────────┤
│  Layer 3: Wrap-First Teacher Pipeline (Constellation Ingestion)        │
│    Wrap -> Observe -> Measure -> Extract -> Normalize -> Replace      │
│    - Legacy Python/C# runtimes execute in sandboxed sidecars           │
│    - Telemetry qualified into Rust core-contracts                      │
├────────────────────────────────────────────────────────────────────────┤
│  Layer 4: Unified Operational Bootstrap                                │
│    - Single turnkey command: cargo xtask run-system                    │
│    - Boots hypervisor, connects bridges, initializes agent supervisor  │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Non-Self-Certification & Seat Separation

All cross-agent reviews and automated merges across the constellation must adhere strictly to the peer review protocol documented in `governance/CROSS_AGENT_COORDINATION_PROTOCOL.md`:
* **Implementer Exclusion:** Any model seat that authors or pushes a commit on a branch is an implementer and cannot post a counted `accept` verdict for that head.
* **Independent Quorum:** Merging requires $\ge 2$ independent accept attestations from $\ge 2$ distinct model families (e.g., Claude, Codex, Gemini, OpenCode).
* **Evidence Obligation:** Every counted attestation must cite executable command evidence and exit codes.
