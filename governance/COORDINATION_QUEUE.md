# Parallel Coordination Queue

Start with `MAINTENANCE_SYSTEM.md`, then claim an unowned row in this queue before editing Aaroneous. Claims expire after 24 hours without an
update. A takeover requires a handoff note and must not overwrite uncommitted work.

| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | Native text encoding: `crates/ast_auditor/**`, encoding script, CI/docs | Claimed | Antigravity / Gemini 3.8 | codex/repository-hardening (Aaroneous) | 2026-09-16T08:02:00Z | Port native text encoding and line ending validation into `crates/ast_auditor`, add tests, integrate with CI/ast_auditor CLI and xtask gate. |
| C2 | Security revalidation: `crates/studio_hud/src/plugin_api.rs` and focused tests | Ready | — | — | — | **Handoff from Claude Sonnet 5, 2026-09-16T07:40Z, reassigned to C3 before repair.** Reproduction findings (source: current HEAD on codex/repository-hardening, and docs/archive/stale_sprawl/audits/AUDIT_FINDINGS_CONSOLIDATED.md SEC-01/SEC-03 on branch codex/text-encoding-integration): (1) `crates/studio_hud/src/plugin_api.rs` is **not declared as a module anywhere** — no `mod plugin_api;` in `crates/studio_hud/src/lib.rs`, no external references, no build.rs magic. It is dead code today: zero live attack surface. (2) SEC-01 (arbitrary DLL/SO execution) is still true in substance if the file were ever wired up: `validate_signature()` (L59-73) does not check a real signature — it hashes the file and only rejects a hash whose first two bytes are both zero, i.e. it passes almost any file including malicious ones, despite doc comments claiming Ed25519 verification. (3) SEC-03 (cross-DLL heap corruption via fat-pointer FFI) is also still true: `UiCartridgeVtbl` (L10-15), a `#[repr(C)]` raw-fn-pointer struct defined specifically to avoid fat-pointer UB per its own doc comment, is defined but **never constructed or used** — `load_dynamic_plugin`/`unload_dynamic_plugin` still pass `*mut dyn UiCartridge` (a Rust trait-object fat pointer, unspecified ABI across a compilation-unit/DLL boundary) through the `extern "C"` constructor/destructor symbols, and `Box::from_raw` on the host side of a plugin-allocated pointer risks cross-allocator heap corruption if a plugin never implements the optional `free_plugin` export (the fallback path at L127 hits this directly). Next owner: a real fix needs (a) actual Ed25519 verification against an injected trusted-key list, failing closed with no keys configured, and (b) replacing the fat-pointer FFI with the already-defined `UiCartridgeVtbl` C-ABI (host-side adapter forwarding `UiCartridge` calls through the raw fn pointers) — both are scoped, bounded changes since there are no callers to break. Not started; releasing for the next owner or a later Claude pass. |
| C3 | Dependency review configuration and report format | Done | Claude Sonnet 5 | main (devtools) | 2026-09-16T07:40:00Z | `foundation dependency-review --checkout <path> --output <path>` (foundation/crates/foundation_cli/src/dependency_review.rs): runs `cargo audit --json` + `cargo metadata` as external processes, writes a versioned report per `contracts/dependency-review/v1/SCHEMA.md`. Read-only; no dependency updates. Classifies findings `external` vs `local_workspace_name_collision` (real collision found: `crates/scratchpad` vs. an unrelated vulnerable crates.io `scratchpad`) using `cargo metadata`'s workspace-member list crossed against Cargo.lock's `source` field. Ran for real against Aaroneous `41b6408`: 10 external findings (hickory-net/proto, rkyv, rustls, atomic-polyfill, bincode, cgmath x2, paste, lru), 2 correctly-classified collisions — see `contracts/dependency-review/v1/reports/aaroneous-41b6408.json`. `cargo test`/clippy -D warnings/fmt --check all pass in foundation/. Commit: 64b634f. External findings are recorded, not fixed — S2 is revalidation, remediation is a separate reviewed Aaroneous change. |
| C4 | WIT, registry, config, and terminology inventory: `specs/wit/**`, `registry/**`, `config/**`, and residual terminology across Aaroneous | Done | Antigravity / Gemini 3.8 | main (devtools) | 2026-09-16T07:30:00Z | Full read-only inventory completed against Wave 11 baseline (`6a80fb2`). Cataloged 103 WIT files (zero `wit-bindgen` consumers), 49 registry schemas (fallback chains documented), 25 config files (5 safe for dead-code pruning), and residual biological/mythological shims across 5 core crates. Classifications and phased remediation plan published at `inventories/c4_wit_registry_config_terminology.md`. Aaroneous working tree preserved untouched. Commit: 9df34c3. |
| C5 | Component profile foundation: explicit-input crate inventory and invariant report contracts in devtools | Ready | — | — | — | Start with a read-only report for one named Aaroneous crate; use `governance/COMPONENT_DEVELOPMENT_PLAYBOOK.md` and keep all analysis outside product build/runtime dependencies. |
| M3 | Turn dependency-review evidence (C3) into one bounded remediation plan: `governance/` (devtools) | Done | Claude Sonnet 5 | main (devtools) | 2026-09-16T08:01:18Z | `governance/DEPENDENCY_REMEDIATION_PLAN.md`: reconciled against a fresh dependency-review run at Aaroneous `5af683c` (rustls now fixed; 6 real "no patch available" findings remain) plus PR #18's actual green `cargo-audit`/`cargo-deny` CI. Actually attempted (not just proposed) the cheapest remaining next step — `cargo update -p lru --precise 0.18.2` — confirmed it fails on the same root cause as the two hickory advisories (iroh-relay's `lru = "^0.16.3"` pin), so all three now share one ranked next step: an iroh version bump. Cargo.lock confirmed untouched (git status clean) by the failed attempt. Commit: c39f597. |

| F1 | Standalone Rust foundation workspace and explicit-input CLI | Done | Codex | main | 2026-09-16 | `foundation_cli` tests and strict clippy pass. |
| F2 | Versioned evidence contracts and fixtures | Done | Codex | main | 2026-09-16 | Input Report v1 schema, fixture, and conformance test pass. |
| F3 | Rust stack-manifest validator | Done | Codex | main | 2026-09-16 | Local Rust manifest validates; tests and strict clippy pass. |
| F6 | Foundation repository baseline: CI, dependency updates, Git workflow, release and decision records | Done | Codex | main | 2026-09-16 | Cross-platform Foundation CI, Dependabot, and governance docs added; 8 tests and strict clippy pass. |

| F7 | Multithread program: priorities, capacity, dependencies, and maintenance cadence | Done | Codex | main | 2026-09-16 | Prioritized portfolio, capacity model, review gates, and weekly lifecycle published. |
| F8 | Local-agent control plane: Qwen proposal boundary, Rust stack roles, and delivery gates | Done | Codex | main | 2026-09-16 | `governance/LOCAL_AGENT_CONTROL_PLANE.md`; no autonomous repository mutation or product dependency. |

## Available portfolio lanes

Use the priority portfolio in `MULTITHREAD_PROGRAM.md` to select the next claim. The current
recommended sequence is C1/M2, M3 after its dependency report review, and M5 after a packaging
check. Do not open M5, M6, and M7 together because they share the C4 inventory and require the
same compatibility review.
## Recent completed context

- **Claude / Sonnet continuity:** completed the native compliance auditor and CI sweep, then
  repaired `ipc_bus::swmr_shm` crash recovery and snapshot failure reporting.
- **Antigravity / Gemini continuity:** completed test-hygiene, zero-warning, hot-path unwrap,
  devtools-migration, terminology, and full-verification waves. Wave 11 is the baseline for any
  further terminology work.
## Claim format

Add the owner, branch, exact paths, UTC start, and intended evidence to the row. On completion,
replace the status with `Done` and add commit plus verification. On blockage, record the blocker
and release the paths.

## Thread roles

- **Codex:** Rust implementation, verification, and integration.
- **Gemini 3.8 / Antigravity:** independent implementation or review lane.
- **Claude Sonnet 5 / Claude:** independent implementation or review lane.
- **Qwen:** local, bounded, owner-reviewed scratch subtask only.
