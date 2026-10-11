# Multithread Program

## Current execution priority - 2026-09-25

The owner requested cross-agent reconciliation before further integration. The [current queue](COORDINATION_QUEUE.md) and [parallel dispatch](PARALLEL_RECONCILIATION_2026-09-25.md) govern this round: C64 Codex devtools/history review, C65 Gemini narrow product-candidate review, C66 Claude security/policy review, and C67 blocked proposal-only Qwen support pending backlog checks. These are three primary lanes, not permission for three simultaneous workspace builds. The queue serializes heavy verification and records acknowledgements before claims.

The portfolio below is the older main-branch snapshot, not current completion evidence. The preserved integration portfolio at `d89b0e1226b783b7f594c6cd16b813f701364162:governance/MULTITHREAD_PROGRAM.md` extends through M80; C64 will reconcile it by source-qualified task and commit. Do not regenerate work because a row here is missing or still says P1/P2. In particular, C23/C24 have later completion reports, and C25-C27 were reassigned between historical ledgers. New composition/Core runtime construction and M75-M80 graduation stay on hold pending policy and baseline review. No accepted product rule is changed by this dispatch.

No external agent session or worker was launched by this update. Preferred assignees must acknowledge their handoffs. Keep policy, implementation, verification, independent acceptance, merge and deployment as separate facts.

This program expands the available roadmap without expanding active work beyond review capacity.
It coordinates any number of contributors through one queue, while normally allowing three primary
implementation lanes at once.

## Capacity model

| Capacity | Use |
| --- | --- |
| 3 primary lanes | Default: one focused product change per primary thread. |
| 1 support lane | Qwen performs a bounded, independently reviewed analysis or fixture task in scratch space. |
| 1 review lane | An independent reviewer accepts a completed change before another broad lane opens. |
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
| P1 | M28 | Establish an evidence standard for "Complete"/"In Progress" labels and reconcile the roadmap's `v1.x`/Phase-N version scheme against the real `0.3.2` workspace version | none — see `governance/ROADMAP_RECONCILIATION_PLAN.md` | Release governance / documentation architecture |
| P2 | M29 | Fix `MASTER_ROADMAP.md`'s text-corruption bug (dropped characters, stray newlines, broken code fences) | none | Mechanical text fix |
| P1 | M30 | Verify or relabel roadmap Phases 1–8 (`v0.4.0`–`v1.2.0`) against actual code/tests — Phase 5's "Complete" dynamic-plugin-loading claim is already known false (C2/M4/M12) | M28 | Rust systems review / evidence verification |
| P2 | M31 | Verify or relabel roadmap Phases 9–37 (`v1.2.0`–`v1.6.0`) against actual code/tests | M28 | Rust systems review / evidence verification |
| P1 | M32 | Verify or relabel roadmap Phase 38 (`v1.7.0`, "In Progress") — the currently-active work claims | M28 | Rust systems review / evidence verification |
| P2 | M33 | Resolve `MASTER_ROADMAP.md`/`docs/roadmap.md`/`TODO.md` redundancy and fix dead cross-references (`COMPLETED_PHASES_ARCHIVE.md`, `ARCHITECTURE_AUDIT.md`) | M30, M31, M32 | Documentation architecture |
| Done | M25 | Trace reachability of no-patch advisories before accepting, isolating, or replacing them | M3 dependency report | Supply-chain analysis |
| P2 | M26 | Add property-oriented boundary tests for IPC and wire parsers selected by component profiles | M17 component inventory | Protocol correctness |
| P2 | M27 | Add a bounded benchmark-result contract and baseline capture for one hot-path component | M8, M13 | Performance assurance |
| P2 | C106 | Upgrade existing native typed intake (`maintenance_ui::NewJob` / `approve_job`), one responsibility: lint and explicitly publish bounded worker-job manifests. Exact future scope: `foundation/crates/maintenance_ui/src/lib.rs`, new `foundation/crates/maintenance_ui/src/job_intake.rs`, new `foundation/crates/maintenance_ui/src/bin/job-intake.rs`, new `foundation/crates/maintenance_ui/tests/job_intake_cli.rs`, new `contracts/job-intake-v1.md`. Preserve Git-root default; explicit runtime-evidence opt-in; read-only lint default; enforce 3 manifests / 8 pending results under the existing reservation; refuse missing, stale, incomplete or reserved queue/PR evidence; retain exclusive publication and typed TOML serialization. Source: owner-authorized maintenance failure follow-through, PR #70; new behavior NOT VERIFIED. Rollback/retirement: revert only the reviewed increment or retire this candidate if existing intake already meets the contract; preserve all runtime history. | Future work only: implementation awaits independently established path/dependency/reviewer capacity, an exact Ready queue claim followed by Claimed ownership, and independent review. Fresh 43-open-PR inspection found zero exact future source overlaps; native schedule-review exit 0 still reports 4 active/review rows for default 3. Retained command/exit logs: `C:/Users/aarog/.codex/qwen-live/maintenance/native-intake/evidence/claim-revision-20261006T160321Z/commands.json`. Gemini board32 volunteers non-implementing review; Claude PR70 review at `20b130b2` is needs_revision, not acceptance. No capacity waiver, dispatch, source edit or deployment authority. | Codex implements only after claim; non-implementing Claude/Gemini independently reviews pinned head and actual fmt, workspace/all-target tests, strict Clippy, Foundation CI, and negative cap/contention/freshness/path/serialization/publication cases. |
| P2 | Unassigned | Qualify and package the existing runtime output-lifecycle prototype for devtools origin without adopting it by default. Exact possible future scope only: `experiments/output-lifecycle/Cargo.toml`, `experiments/output-lifecycle/Cargo.lock`, `experiments/output-lifecycle/src/lib.rs`, `experiments/output-lifecycle/src/catalog.rs`, `experiments/output-lifecycle/src/main.rs`, `experiments/output-lifecycle/src/tests.rs`, `experiments/output-lifecycle/README.md`, `experiments/output-lifecycle/CONTRACT.md`, `experiments/output-lifecycle/ADMISSION.md`, `experiments/output-lifecycle/verify-prototype.ps1`. Candidate behavior includes bounded native rows, snapshot-consistent reads, capability-confined recoverable JSON publication, and fault tests; all remain unqualified until fresh repository checks and independent execution review. Canonical frozen v4 recovery bundle is `governance/archive/prototypes/output-lifecycle/v4/source-v4-canonical.tar`, SHA-256 `43FD5C3857517B0C8B5EB93F822EEDBEE1C501D8F5AD110B923134E8B0B57344`, with per-file index `governance/archive/prototypes/output-lifecycle/v4/source-hashes-v4.json`, repository-object SHA-256 `67AF4076FA46D600C3346626121E938C90C84D1CFABA6A9EDE1E460969CECA3D`. The original retained `source-v4.tar`, SHA-256 `D7925DF689ACAA7D6D634C986C7046C019C9B805C30E832287845BDEC82F63CE`, is preserved as historical evidence but is not the recovery bundle because it omitted the indexed `verify-prototype.ps1`. Retrieval procedure: verify the canonical bundle and index hashes, extract only into an isolated scratch directory, compare every extracted file to the index, and execute nothing until a future exact-path claim authorizes qualification. Operator-injected runtime roots and absolute paths remain local-receipt inputs only; v5 and later work require separate frozen snapshots and must not rewrite v4. These archived artifacts are evidence inputs, not acceptance, executable integration, or runtime authority. Retirement: omit or remove the experiment if qualification fails, existing C99/C100 capabilities subsume it, or its contract cannot remain isolated; never rewrite runtime history. | The existing C99/C100 steward, identity, and signature stack is canonical on `origin/main` at `1c6e3a1d` after PRs #99-#101 merged; qualification must reuse those capabilities rather than create parallel stewardship, identity, signing, or publication paths. Bounded prerequisite to reproduce before qualification: C99 non-PR backlog actions need stable ID plus revision keys that participate in existing leases, completion-receipt matching, and deduplication without fabricated PR numbers; the runtime v5 shadow observation is not canonical evidence and must be reproduced under a future claim. Ordinary capacity remains unavailable: the 2026-10-07 canonical queue recheck found 8 `Claimed`/`Review` lanes for capacity 3, 5 existing `Ready` lanes, and 0 available slots. Recheck exact-path ownership, open-PR overlap, ordinary lane capacity, and reviewer independence before assigning an ID or opening a queue claim. No capacity exception, runtime adoption, deployment, or merge authority. | Rust qualification plus independent non-implementing review of fresh formatting, tests, strict Clippy, CLI cases, native-intake validation, byte-limit, snapshot, recovery, confinement, and publication-fault evidence at a pinned head. |

## Devtools closeout increments

The [lane closeout plan](LANE_CLOSEOUT_PLAN.md) breaks down owner-directed review
debt without declaring old implementations independently accepted. Consult the
queue before claiming; portfolio entries are not Ready source reservations.

| Priority | ID | Bounded work / source | Dependency | Acceptance |
| --- | --- | --- | --- | --- |
| P1 | M34 | Independent per-lane acceptance: old C10-C13/C15/C16/C19 evidence | Independent reviewer / reserved-decision verdict | Revision/gate-based decisions; no self-certification; queue closure only for accepted slices |
| P1 | M35 | Current docs/cadence/adoption correction: C23 | Old source scopes released by handoff | Accurate current claims, dated history, valid links and UTF-8/LF |
| P1 | M39 | Packet server lifecycle hardening: C24, main.rs body/respond | Old C13 scope released; isolated reproduction | Bounded stalled/disconnected socket behavior, responsive status/shutdown, native gates |
| P2 | M36 | Authenticated dashboard integration: REGISTRAR.md gap | C15/C19 acceptance | Explicit same-origin credential setup and negative lifecycle tests; no product writes |
| P2 | M37 | Worker liveness and persistence failure policy: local_worker.rs/multithreader.rs | Ownership and deployment inputs | Observable quiet/reserved/error states, bounded permanent failure policy, crash-marker tests |
| P2 | M38 | Utility client CI: standalone client.test.mjs gate | Reviewer capacity / exact workflow claim | Existing client tests automated on Windows/Linux, read-only bounded CI |

## Intake rules

A new lane needs a source, priority, exact scope, dependency, expected evidence, and rollback or
retirement decision. The worklist may contain future work; the coordination queue contains only
work ready to own. No task enters the queue because it merely sounds useful.

### Future intake candidates (unapproved, not ready to own)

| Priority | ID | Bounded work / source | Dependency | Acceptance & Rollback |
| --- | --- | --- | --- | --- |
| P1 (proposed) | C98 | Owner-requested maintenance progression guard: new `foundation/crates/foundation_cli/src/progression_guard.rs`, new standalone `foundation/crates/foundation_cli/src/bin/maintenance_progress.rs`, and `governance/MAINTENANCE_PROGRESSION_GUARD.md`. Source: Owner request 2026-10-04. | C91/#36 and scheduler independence (unresolved); Ready authorization | Acceptance: focused/workspace tests, fmt, all-target Clippy, diff/evidence checks. Rollback: retire if existing native scheduler/component already provides the guard; preserve findings and reproducers; documentation rollback restores prior worklist revision with no runtime change. |
| P2 (proposed) | C102 | Design only: supported Windows/Linux operations server, common stage/stack contract, one canonical writer, topic case projection, client-owned short/long-term memory and bounded local operating loops. Intended output: `governance/proposals/SUPPORTED_OPERATIONS_SERVER.md`. Reconcile existing maintenance_ui, steward stages and PR #59. Source: Owner request for design-first evolution. | Ready authorization and acknowledged claim before design PR; PR #59 context | Acceptance: strict UTF-8/no-BOM/LF, additive diff, git diff --check, evidence checks with retained receipts. Rollback: if design is declined, retire this intake without deleting runtime evidence; documentation rollback restores prior worklist revision and never changes runtime deployment. |

## Review gates

- Product code: native Aaroneous verification plus focused tests.
- Foundation code: Foundation CI, contract fixture update, and explicit-input validation.
- Config/interface retirement: runtime, packaging, and external-consumer evidence.
- Public terminology change: compatibility migration and deprecation window.

## Weekly portfolio review

Reprioritize only from evidence: CI failures, completed dependency reports, reproduced security
findings, or finished inventory work. Keep at least one lane available for a small corrective task.
