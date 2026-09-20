# Multithread Program

This program expands the available roadmap without expanding active work beyond review capacity.
It coordinates any number of contributors through one queue, while normally allowing three primary
implementation lanes at once.

## Capacity model

| Capacity | Use |
| --- | --- |
| 3 primary lanes | Default: one focused product change per primary thread. |
| 1 support lane | Qwen performs a bounded, owner-reviewed analysis or fixture task in scratch space. |
| 1 review lane | A primary owner reviews a completed change before another broad lane opens. |
| Expansion | Add a primary lane only when file scopes, dependencies, and reviewer capacity are all independent. |

A large roadmap is useful; a large number of simultaneous edits is not. A claim moves through
`Ready → Claimed → Review → Done` or `Blocked`. Claims contain exact paths, dependencies, and
evidence, not just an issue title.

## Priority portfolio

| Priority | ID | Work | Dependency | Preferred capability |
| --- | --- | --- | --- | --- |
| P0 | M1 | Converge and verify the active integration branch | CI result | Codex integration |
| Done | M2 | Port the text-encoding checker into `ast_auditor` | C1 | Rust CLI / tests |
| Done | M3 | Turn dependency-review evidence into one bounded remediation plan | C3 done | Supply-chain review |
| Done | M4 | Decide removal versus repair for dead `studio_hud` plugin API code | C2 handoff | Rust API review |
| Done | M5 | Validate and prune the five stale configuration manifests | C4 inventory | Read-only inventory then focused deletion |
| Done | M6 | Verify WIT and fabrication packaging/runtime usage before archival | C4 inventory | Build/package analysis |
| Done | M7 | Deprecate terminology aliases with migration tests | C4 inventory | Compatibility-focused Rust refactor |
| P1 | M8 | Establish the bounded reducer reference path | A2 | Deterministic systems testing |
| Done | M9 | Build a read-only dogfood harness in devtools | F1–F3 done | Foundation Rust |
| P2 | M10 | Enforce execution-host dependency policy | M8 | Architecture and Cargo policy |
| P2 | M11 | Narrow hypervisor public surface | M10 | API refactor |
| Done | M12 | Define plugin lifecycle RFC | M4 | Interface design |
| P2 | M13 | Publish measured performance assurance | M8 | Benchmarking |
| P3 | M14 | Add thin platform adapters around stable foundation commands | M9 | Windows/Linux integration |
| P3 | M15 | Evaluate Candle, Rig, Burn, and ONNX Runtime in isolated foundation crates | M9 | Experimental Rust stacks |
| Done | M16 | Add a bounded local-agent proposal path and prompt/evaluation contracts | F7 | Rust foundation / Ollama |
| Done | M17 | Build explicit-input component inventory and invariant profile reports | F7, C4 | Rust foundation / component architecture |
| Done | M18 | Make local verification and CI command coverage mechanically identical | M7 evidence | CI / xtask / regression tests |
| Done | M19 | Audit temporal tests for fixed sleeps and establish deterministic synchronization guidance | Recent platform/hypervisor flake fixes | Test reliability |
| P1 | M20 | Execute the dependency plan's blocked iroh upgrade in a dedicated compatibility lane | M3 evidence | Cargo dependency remediation |
| Done | M21 | Reconcile root README assurance language with reproducible evidence and current contracts | README verification-scope notices | Documentation / measurement review |
| Done | M22 | Deprecate legacy `paths` preset keys with persisted-data migration tests | C4 inventory | Compatibility-focused Rust refactor |
| Done | M23 | Define the removal window and downstream-consumer policy for deprecated terminology aliases | M7 done | Release / API governance |
| Retired | M24 | Consolidate root TODO and frontier roadmap into an evidence-labeled product roadmap | Conflicting roadmap/history sources | Documentation architecture |
| Done | M28 | Establish an evidence standard for "Complete"/"In Progress" labels and reconcile the roadmap's `v1.x`/Phase-N version scheme against the real `0.3.2` workspace version | none — see `governance/ROADMAP_RECONCILIATION_PLAN.md` | Release governance / documentation architecture |
| P2 | M29 | Fix `MASTER_ROADMAP.md`'s text-corruption bug (dropped characters, stray newlines, broken code fences) | none | Mechanical text fix |
| Done | M30 | Verify or relabel roadmap Phases 1–8 (`v0.4.0`–`v1.2.0`) against actual code/tests — Phase 5's "Complete" dynamic-plugin-loading claim is already known false (C2/M4/M12) | M28 | Rust systems review / evidence verification |
| P2 | M31 | Verify or relabel roadmap Phases 9–37 (`v1.2.0`–`v1.6.0`) against actual code/tests | M28 | Rust systems review / evidence verification |
| Done | M32 | Verify or relabel roadmap Phase 38 (`v1.7.0`, "In Progress") — the currently-active work claims | M28 | Rust systems review / evidence verification |
| P2 | M33 | Resolve `MASTER_ROADMAP.md`/`docs/roadmap.md`/`TODO.md` redundancy and fix dead cross-references (`COMPLETED_PHASES_ARCHIVE.md`, `ARCHITECTURE_AUDIT.md`) | M30, M31, M32 | Documentation architecture |
| Done | M25 | Trace reachability of no-patch advisories before accepting, isolating, or replacing them | M3 dependency report | Supply-chain analysis |
| P2 | M26 | Add property-oriented boundary tests for IPC and wire parsers selected by component profiles | M17 component inventory | Protocol correctness |
| P2 | M27 | Add a bounded benchmark-result contract and baseline capture for one hot-path component | M8, M13 | Performance assurance |

## Devtools closeout increments

The [lane closeout plan](LANE_CLOSEOUT_PLAN.md) breaks down owner-directed review
debt without declaring old implementations independently accepted. Consult the
queue before claiming; portfolio entries are not Ready source reservations.

| Priority | ID | Bounded work / source | Dependency | Acceptance |
| --- | --- | --- | --- | --- |
| P1 | M34 | Independent per-lane acceptance: old C10-C13/C15/C16/C19 evidence | Independent reviewer / reserved owner verdict | Revision/gate-based decisions; no self-certification; queue closure only for accepted slices |
| P1 | M35 | Current docs/cadence/adoption correction: C23 | Old source scopes released by handoff | Accurate current claims, dated history, valid links and UTF-8/LF |
| P1 | M39 | Packet server lifecycle hardening: C24, main.rs body/respond | Old C13 scope released; isolated reproduction | Bounded stalled/disconnected socket behavior, responsive status/shutdown, native gates |
| P2 | M36 | Authenticated dashboard integration: REGISTRAR.md gap | C15/C19 acceptance | Explicit same-origin credential setup and negative lifecycle tests; no product writes |
| P2 | M37 | Worker liveness and persistence failure policy: local_worker.rs/multithreader.rs | Ownership and deployment inputs | Observable quiet/reserved/error states, bounded permanent failure policy, crash-marker tests |
| P2 | M38 | Utility client CI: standalone client.test.mjs gate | Reviewer capacity / exact workflow claim | Existing client tests automated on Windows/Linux, read-only bounded CI |
| P2 | M40 | Surface `ConcurrenceSnapshot` as an advisory confidence field on scheduler recommendations | none — see `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md` | Rust systems review / read-only cross-repo analysis |
| P2 | M41 | Make `frontier_discovery.rs` self-scheduling and packet-generating instead of manually invoked | none — see `architecture/LOCAL_CLOUD_ORCHESTRATION_PLAN.md` | Rust foundation / scheduling |
| P3 | M42 | Scope (design only) a `cloud_providers.rs` dispatch adapter contract for cloud-routine queue read/write | M40, M41 informative | Release governance / distributed coordination design |
| P2 | M43 | Auto-derive non-crossing work-packet lanes from dependency/component inventory | M41 | Dependency graph analysis |
| P3 | M44 | Expand worker-jobs templates past read-only analysis, gated by cross_verifier/consensus_engine | M42 | Release governance / verification design |

## Intake rules

A new lane needs a source, priority, exact scope, dependency, expected evidence, and rollback or
retirement decision. The worklist may contain future work; the coordination queue contains only
work ready to own. No task enters the queue because it merely sounds useful.

## Review gates

- Product code: native Aaroneous verification plus focused tests.
- Foundation code: Foundation CI, contract fixture update, and explicit-input validation.
- Config/interface retirement: runtime, packaging, and external-consumer evidence.
- Public terminology change: compatibility migration and deprecation window.

## Weekly portfolio review

Reprioritize only from evidence: CI failures, completed dependency reports, reproduced security
findings, or finished inventory work. Keep at least one lane available for a small corrective task.
