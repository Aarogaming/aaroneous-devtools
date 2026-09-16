# Parallel Coordination Queue

Start with `MAINTENANCE_SYSTEM.md`, then claim an unowned row in this queue before editing Aaroneous. Claims expire after 24 hours without an
update. A takeover requires a handoff note and must not overwrite uncommitted work.

| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | Native text encoding: `crates/ast_auditor/**`, encoding script, CI/docs | Ready | — | — | — | — |
| C2 | Security revalidation: `crates/studio_hud/src/plugin_api.rs` and focused tests | Ready | — | — | — | Fresh review lane; reproduce before repair. |
| C3 | Dependency review configuration and report format | Ready | — | — | — | Claude continuity lane after compliance-auditor work. |
| C4 | WIT, registry, config, and terminology inventory | Ready | — | — | — | Antigravity continuity lane; diff against Wave 11 first. |

| F1 | Standalone Rust foundation workspace and explicit-input CLI | Done | Codex | main | 2026-09-16 | `foundation_cli` tests and strict clippy pass. |
| F2 | Versioned evidence contracts and fixtures | Ready | — | — | — | Contract-only lane. |
| F3 | Rust stack-manifest validator | Ready | — | — | — | Depends on F1 and F2 design. |
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
