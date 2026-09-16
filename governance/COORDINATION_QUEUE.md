# Parallel Coordination Queue

Start with `MAINTENANCE_SYSTEM.md`, then claim an unowned row in this queue before editing Aaroneous. Claims expire after 24 hours without an
update. A takeover requires a handoff note and must not overwrite uncommitted work.

| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | Native text encoding: `crates/ast_auditor/**`, encoding script, CI/docs | Ready | — | — | — | — |
| C2 | Security revalidation: `crates/studio_hud/src/plugin_api.rs` and focused tests | Ready | — | — | — | **Handoff from Claude Sonnet 5, 2026-09-16T07:40Z, reassigned to C3 before repair.** Reproduction findings (source: current HEAD on codex/repository-hardening, and docs/archive/stale_sprawl/audits/AUDIT_FINDINGS_CONSOLIDATED.md SEC-01/SEC-03 on branch codex/text-encoding-integration): (1) `crates/studio_hud/src/plugin_api.rs` is **not declared as a module anywhere** — no `mod plugin_api;` in `crates/studio_hud/src/lib.rs`, no external references, no build.rs magic. It is dead code today: zero live attack surface. (2) SEC-01 (arbitrary DLL/SO execution) is still true in substance if the file were ever wired up: `validate_signature()` (L59-73) does not check a real signature — it hashes the file and only rejects a hash whose first two bytes are both zero, i.e. it passes almost any file including malicious ones, despite doc comments claiming Ed25519 verification. (3) SEC-03 (cross-DLL heap corruption via fat-pointer FFI) is also still true: `UiCartridgeVtbl` (L10-15), a `#[repr(C)]` raw-fn-pointer struct defined specifically to avoid fat-pointer UB per its own doc comment, is defined but **never constructed or used** — `load_dynamic_plugin`/`unload_dynamic_plugin` still pass `*mut dyn UiCartridge` (a Rust trait-object fat pointer, unspecified ABI across a compilation-unit/DLL boundary) through the `extern "C"` constructor/destructor symbols, and `Box::from_raw` on the host side of a plugin-allocated pointer risks cross-allocator heap corruption if a plugin never implements the optional `free_plugin` export (the fallback path at L127 hits this directly). Next owner: a real fix needs (a) actual Ed25519 verification against an injected trusted-key list, failing closed with no keys configured, and (b) replacing the fat-pointer FFI with the already-defined `UiCartridgeVtbl` C-ABI (host-side adapter forwarding `UiCartridge` calls through the raw fn pointers) — both are scoped, bounded changes since there are no callers to break. Not started; releasing for the next owner or a later Claude pass. |
| C3 | Dependency review configuration and report format | Claimed | Claude Sonnet 5 | codex/repository-hardening (Aaroneous) | 2026-09-16T07:40:00Z | Building a reproducible dependency-vulnerability review (S2): lockfile-aware scanner output with advisory IDs, dependency paths, and fixed-version/mitigation recommendations recorded — no dependency updates in this pass. Builds on the existing security-audit.yml (cargo-audit/cargo-deny) CI job. |
| C4 | WIT, registry, config, and terminology inventory: `specs/wit/**`, `registry/**`, `config/**`, and residual terminology across Aaroneous | Claimed | Antigravity / Gemini 3.8 | codex/repository-hardening (Aaroneous) | 2026-09-16T07:30:00Z | Performing full read-only inventory of WIT definitions, schema registries, config trees, and residual biological/mythological/analogy terms against Wave 11 (`6a80fb2`). Classifying compatibility impact (wire ABI, file format, config breaking, public trait breaking vs internal dead-code) before recommending or executing any deletions/renames. Target artifact: `inventories/c4_wit_registry_config_terminology.md`. |

| F1 | Standalone Rust foundation workspace and explicit-input CLI | Done | Codex | main | 2026-09-16 | `foundation_cli` tests and strict clippy pass. |
| F2 | Versioned evidence contracts and fixtures | Claimed | Codex | main | 2026-09-16T07:30:54Z | Contract-only lane. |
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
