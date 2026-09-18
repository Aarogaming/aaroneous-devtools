# Evidence Standard for Status Labels & Version Mapping

This governance document defines the evidence standards required to assign status labels (`Complete`, `In Progress`, `Dormant`, `Retired`) across Aaroneous product roadmaps and devtools coordination ledgers, as well as the reconciliation model mapping target phase versions to workspace Cargo versions.

---

## 1. Evidence Requirements for Status Labels

| Status Label | Required Evidence Standard | Prohibited Anti-Patterns |
| --- | --- | --- |
| **`Complete`** | 1. Concrete source code committed in tracked repository branches.<br>2. Clean automated test suite execution (e.g., `cargo test --workspace` or component tests passing).<br>3. Successful AST auditor & gate verification (`cargo xtask gate` or `ast_auditor`). | - Unverified design docs or checklists without source code.<br>- Deleted/unsound implementations (e.g., unverified FFI dynamic loading).<br>- `todo!()` or `unimplemented!()` macros on hot paths. |
| **`In Progress`** | 1. Active, claimed row in `COORDINATION_QUEUE.md` with explicit owner and started timestamp (< 24 hours).<br>2. Work-in-progress branch or unexpired source file reservations. | - Dormant tasks with no active owner or expired claims (> 24h).<br>- Speculative aspirational features with zero active work. |
| **`Attempted / Redesigned`** | 1. Documented RFC or post-mortem (e.g., RFC-0006 for UI cartridge ABI).<br>2. Explicit record of prior attempt removal and design boundary constraints. | - Labeling prior broken/removed prototypes as "Complete". |
| **`Dormant / Roadmapped`** | 1. Documented technical specification or portfolio brief in `MULTITHREAD_PROGRAM.md`. | - Claiming active work status without a claimed queue row. |

---

## 2. Version Reconciliation Model

- **Authoritative Version**: The root `Cargo.toml` workspace version (`version = "0.3.2"`) is the single authoritative product version for all crates, binaries, and runtime artifacts.
- **Milestone Phase Mapping**: Phase tags (`v0.4.0` through `v1.7.0`) in `MASTER_ROADMAP.md` represent internal architectural milestone horizons (Horizons 1–7), not published semver tags.
- **Mapping Rules**:
  - `Workspace 0.3.x` -> Horizon 0 (Foundation, Zero-Alloc Microkernel, IPC SWMR Ring Buffers, Verification Gates).
  - `Phase 1–8 (v0.4.0–v1.2.0)` -> Horizon 1 (Adaptive Engine & Dynamic Cartridges redesign under RFC-0006).
  - `Phase 9–37 (v1.2.0–v1.6.0)` -> Horizon 2–5 (SSM Compute, Solid-State Storage, Distributed P2P Mesh).
  - `Phase 38+ (v1.7.0+)` -> Horizon 6–7 (Frontier Autonomous Orchestration).

---

## 3. Governance Integration

All roadmap updates (in `docs/roadmap.md`, `MASTER_ROADMAP.md`, or `TODO.md`) must cite explicit evidence per this standard before adjusting status labels or version targets.
