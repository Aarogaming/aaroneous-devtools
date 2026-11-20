# Constellation Ontology Matrix & Embodiment Architecture

> **STATUS**: Working Draft, Unreviewed (Companion to `Aarogaming/aaroneous#153`)
> **APPLIES TO**: ALL REPOSITORIES IN THE AAROGAMING CONSTELLATION
> **DATE**: 2026-11-18
> **CORRESPONDING PRODUCT PR**: [Aarogaming/aaroneous#153](https://github.com/Aarogaming/aaroneous/pull/153)

---

## 1. Executive Summary & The Core Invariant

The Aarogaming constellation comprises 12 distinct repositories evolved across multiple generations of embodiment, automation, and systems research. Rather than attempting brute-force repository consolidation or lossy rewrites, this document establishes a unifying **Layer 0 Working Ontology** and the foundational separation invariant:

$$\mathbf{Facts} \neq \mathbf{Authority} \neq \mathbf{Actions}$$

### The Three Boundaries

1. **Facts (Perception & Observation)**
   - *Responsibility:* Observe environment, establish trusted facts, qualify timestamps and memory provenance.
   - *Rule:* **Facts describe reality. Facts NEVER authorize actions.**
   - *Constellation Subsystems:*
     - `[Implemented]` `platform_bridge` DXGI/Win32 hook ingestors and input receivers.
     - `[Implemented]` `AutoWizard101` UI tree parsers (`DevTools/UiAuditRunner/Program.cs`) and Win32 automation hooks.
     - `[Implemented]` `Wizard101_DanceBot` image template matching and minigame detection (`image.py`, `dance_game.py`).
     - `[Hypothesized]` Unified `FactSource` trait for qualified cross-environment perception.

2. **Authority (Governance & Trust)**
   - *Responsibility:* Determine permission, verify cryptographic signatures, enforce safety interlocks, manage recovery holds.
   - *Rule:* **Authority decides permitted transitions. Authority NEVER performs transitions.**
   - *Constellation Subsystems:*
     - `[Implemented]` `aaroneous` SMT interlocks and resource governors (`crates/governance`).
     - `[Implemented]` `aaroneous-devtools` multi-agent quorum verification (`scripts/merge_gate.py`).
     - `[Implemented]` `MyFortress` credential fences and gRPC gateway (`gateway/api/grpc_server.py`).
     - `[Hypothesized]` Independent `AuthorityIssuer` with sealed token constructors accessible only to governance.

3. **Actions (Execution & Actuation)**
   - *Responsibility:* Execute transitions within authorized bounds, consume execution tickets, emit operational telemetry.
   - *Rule:* **Actions execute transitions. Actions NEVER self-authorize; actions consume authority.**
   - *Constellation Subsystems:*
     - `[Implemented]` `core/hypervisor` scan-loop execution reducers.
     - `[Implemented]` `AaroneousAutomationSuite` (`AAS`) process launcher (`AAS_Launcher.bat`, `fix_kernel.py`).
     - `[Hypothesized]` `Guild` multi-process IPC orchestration (`tools/maelstrom_ipc_tail.py`, `uaf/orchestrator.py`).
     - `[Hypothesized]` `ActionSink` trait consuming validated `AuthorityToken` instances without minting capability.

---

## 2. Empirical Repository Census Baseline

All metrics below are derived directly from authenticated recursive Git tree inspections via the GitHub API (`GET /repos/Aarogaming/{repo}/git/trees/{sha}?recursive=1`) executed across the `Aarogaming` organization on 2026-11-18. In Git object terminology, "Blobs" correspond to individual files, and "Trees" correspond to directories.

| Repository | Head Ref & Commit | Tree Entries | Blobs (Files) / Dirs | Top File Extensions (Count & %) | Operational Focus & Key Paths |
|---|---|---|---|---|---|
| **`aaroneous`** | `main` @ `cc794cb3` | 1,346 | 1,094 files / 252 dirs | `.rs` (766, 70.0%), `.md` (125, 11.4%), `.json` (80, 7.3%), `.toml` (64, 5.9%) | Microkernel engine, lock-free ring buffers (`crates/ipc_bus`), formal governance (`crates/governance`) |
| **`aaroneous-devtools`** | `main` @ `a87ec8f2` | 598 | 487 files / 111 dirs | `.md` (119, 24.4%), `.json` (103, 21.1%), `.rs` (88, 18.1%), `.py` (72, 14.8%) | Multi-agent quorum, verification harnesses, steward daemons (`foundation/crates/steward`, `scripts/merge_gate.py`) |
| **`AaroneousAutomationSuite`** | `master` @ `4b52088a` | 6,669 | 6,254 files / 414 dirs | `.md` (2,486, 39.7%), `.json` (2,060, 32.9%), `.py` (1,105, 17.7%), `.meta` (185, 3.0%), `.cs` (79, 1.3%) | Windows process automation, `fix_kernel.py`, `repair_kernel.py`, `AAS_Launcher.bat` |
| **`Maelstrom`** | `main` @ `bb6e3608` | 2,073 | 1,728 files / 345 dirs | `.png` (325, 18.8%), `.py` (240, 13.9%), `.jpg` (168, 9.7%), `.cs` (152, 8.8%), `.json` (118, 6.8%) | Game perception overlay (`Client/merlin_overlay.py`), combat perception (`minigames/dueling_diego.py`) |
| **`AutoWizard101`** | `main` @ `5b5c2400` | 3,888 | 3,361 files / 527 dirs | `.png` (588, 17.5%), `.gif` (551, 16.4%), `.js` (388, 11.5%), `.py` (210, 6.2%), `.jpg` (168, 5.0%) | UI test runners (`DevTools/UiAuditRunner/Program.cs`), Win32 automation hooks, template matching assets |
| **`Guild`** | `main` @ `b3749df8` | 43,099* | 38,317 files / 4,775 dirs | `.txt` (20,232), `(no_ext)` (5,023), `.md` (3,110), `.py` (2,771), `.json` (2,278) | Multi-process tooling (`tools/maelstrom_ipc_tail.py`, `uaf/orchestrator.py`); *listing truncated at GitHub API limit; includes 1,966 blobs under `.venv_precommit/` |
| **`Merlin`** | `main` @ `191b9371` | 5,295 | 4,606 files / 689 dirs | `.json` (1,411, 30.6%), `(no_ext)` (1,134, 24.6%), `.timestamp` (530, 11.5%), `.d` (465, 10.1%), `.rmeta` (437, 9.5%) | Multi-stack reasoning engine (Electron JS, Kotlin Android/Desktop, Unity C#) |
| **`MyFortress`** | `main` @ `3a9b922f` | 383 | 350 files / 33 dirs | `.json` (238, 68.0%), `.py` (60, 17.1%), `.md` (34, 9.7%), `(no_ext)` (7, 2.0%) | Credential fence, gRPC gateway (`gateway/api/grpc_server.py`), security tokens |
| **`Library`** | `main` @ `9ee5442d` | 340 | 282 files / 58 dirs | `.json` (112, 39.7%), `.md` (75, 26.6%), `.py` (63, 22.3%), `(no_ext)` (13, 4.6%) | Plugin catalog registry (`plugins/catalog/plugin.py`), discovery schemas |
| **`Workbench`** | `main` @ `7df1b0ca` | 1,198 | 1,034 files / 164 dirs | `.py` (586, 56.7%), `.md` (88, 8.5%), `.json` (79, 7.6%), `.cs` (77, 7.4%), `.ps1` (52, 5.0%) | Asset ingestion pipeline (`Assets/asset_ingestion.py`), inspection utilities |
| **`AndroidApp`** | `main` @ `b68dbf35` | 328 | 249 files / 79 dirs | `.kt` (134, 53.8%), `.xml` (34, 13.7%), `.md` (30, 12.0%), `.json` (14, 5.6%) | Mobile client interface, accessibility rules (`app/src/androidTest/java/com/aaroneous/AccessibilityChecksRule.kt`) |
| **`Wizard101_DanceBot`** | `main` @ `ca0ec613` | 63 | 55 files / 8 dirs | `.py` (21, 38.2%), `.png` (18, 32.7%), `(no_ext)` (2, 3.6%), `.md` (2, 3.6%), `.ico` (2, 3.6%) | Computer-vision arrow detection (`dance_game.py`, `image.py`, `gui.py`) |

---

## 3. The Six Layer 0 Primitives

Every subsystem across the constellation maps onto six foundational primitives:

1. **State:** Immutable point-in-time observations and system register snapshots.
2. **Transition:** Pure functions mapping state and input to candidate next states ($S_{t+1} = f(S_t, I)$).
3. **Relationship:** Structural, dependency, and topological linkages between components.
4. **Constraint:** Invariant mathematical bounds, preconditions, and safety rules enforced by governance.
5. **Trust:** Provenance, cryptographic signatures, attestation records, and qualification flags.
6. **Perspective:** Contextual viewports, agent reasoning horizons, and heuristic planners.

---

## 4. Architectural Separation: FactSource, AuthorityIssuer, and ActionSink

To maintain the separation invariant without creating self-authorizing action sinks, cross-environment interaction is decomposed into three distinct interfaces:

```
┌─────────────────┐         Observations          ┌────────────────────┐
│   FactSource    │ ────────────────────────────> │ Continuous Planner │
│ (Observer-only) │                               │  / State Reducer   │
└─────────────────┘                               └────────────────────┘
                                                            │
                                                     Intent │
                                                            v
┌─────────────────┐       AuthorityToken          ┌────────────────────┐
│   ActionSink    │ <──────────────────────────── │  AuthorityIssuer   │
│ (Executor-only) │  (sealed constructor private  │    (Governance     │
└─────────────────┘        to governance)         │     Interlocks)    │
                                                  └────────────────────┘
```

1. **`FactSource` (Perception)**: Exposes read-only observation streams. Cannot execute actions, mutate external environments, or mint credentials.
2. **`AuthorityIssuer` (Governance)**: Evaluates requested intents against formal constraints, safety interlocks, rate limits, and operational budgets. Holds private token minting capability; no other component can construct an `AuthorityToken`.
3. **`ActionSink` (Actuation)**: Consumes a validated, non-expired `AuthorityToken` matching the requested transition. Verifies governance provenance before dispatching any command. Cannot generate or elevate its own authority.

---

## 5. The 4-Layer Superproject Unification Roadmap

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
│    - [Aspirational] Lock-free SWMR ring buffers (crates/ipc_bus)      │
│    - [Aspirational] Native Model Context Protocol server               │
├────────────────────────────────────────────────────────────────────────┤
│  Layer 3: Wrap-First Teacher Pipeline (Constellation Ingestion)        │
│    Wrap -> Observe -> Measure -> Extract -> Normalize -> Replace      │
│    - [Aspirational] Legacy runtimes execute in sandboxed sidecars      │
│    - [Aspirational] Telemetry qualified into Rust core-contracts       │
├────────────────────────────────────────────────────────────────────────┤
│  Layer 4: Unified Operational Bootstrap                                │
│    - [Aspirational] Planned turnkey command: cargo xtask run-system    │
│    - Boots hypervisor, connects bridges, initializes agent supervisor  │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Non-Self-Certification & Seat Separation

All cross-agent reviews and automated merges across the constellation adhere strictly to the peer review protocol documented in `governance/CROSS_AGENT_COORDINATION_PROTOCOL.md`:
* **Implementer Exclusion:** Any model seat that authors or pushes a commit on a branch is an implementer and cannot post a counted `accept` verdict for that head.
* **Independent Quorum:** Merging requires $\ge 2$ independent accept attestations from $\ge 2$ distinct model families (e.g., Claude, Codex, Gemini, OpenCode).
* **Evidence Obligation:** Every counted attestation must cite executable command evidence and exit codes.
