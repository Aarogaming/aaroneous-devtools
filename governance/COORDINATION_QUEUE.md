# Parallel Coordination Queue

Start with `MAINTENANCE_SYSTEM.md`, then claim an unowned row in this queue before editing Aaroneous. Claims expire after 24 hours without an
update. A takeover requires a handoff note and must not overwrite uncommitted work.

| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | Native text encoding: `crates/ast_auditor/**`, encoding script, CI/docs | Done | Antigravity / Gemini 3.8 | codex/repository-hardening (Aaroneous) | 2026-09-16T08:02:00Z | Ported native text encoding, BOM detection, and LF/CRLF line ending validation into `crates/ast_auditor` (`src/rules/text_encoding.rs`, `CheckEncoding` CLI subcommand). Exposed public API (`audit_file_encoding`, `audit_tracked_encodings`, `EncodingViolation`). Added unit and integration tests in `crates/ast_auditor/tests/agent_evasion_suite.rs`. Updated `xtask/src/encoding.rs` to delegate cleanly to `ast_auditor -- check-encoding`. Verified with `cargo test -p ast_auditor` (32/32 pass), `cargo clippy -p ast_auditor -- -D warnings`, `cargo fmt -p ast_auditor -- --check`, and `cargo check -p xtask`. Commit: `c52e4fb` pushed to `origin/codex/repository-hardening`. |
| C2 | Security revalidation: `crates/studio_hud/src/plugin_api.rs` and focused tests | Done | Claude Sonnet 5 | codex/repository-hardening (Aaroneous) | 2026-09-16T15:33:27Z | Reproduced SEC-01 (fake signature check: rejected only hashes whose first two bytes were zero, not a real signature) and SEC-03 (fat-pointer `*mut dyn UiCartridge` FFI across a DLL boundary). Original plan (Ed25519 + the existing `UiCartridgeVtbl` C-ABI adapter) was abandoned after discovering `UiCartridgeVtbl`'s assumed methods no longer matched the current `UiCartridge` trait (`name`/`render`), and that even a correct vtable adapter would not fix the real problem: `render(&mut egui::Ui)` hands a non-`repr(C)`, no-stable-layout third-party type across the boundary, which is unsound regardless of authentication. Implemented instead: removed dynamic (`.dll`/`.so`) loading entirely — `UiCartridgeVtbl`, `DynamicPlugin`, `load_dynamic_plugin`, `unload_dynamic_plugin`, `validate_signature`, `compute_file_hash` all deleted. `PluginManager` now exposes only in-process `load_cartridge(Box<dyn UiCartridge>)`, built by the same binary that runs it. Wired the previously-undeclared module into `studio_hud`'s `lib.rs`, added `api` to `studio_hud/Cargo.toml`, added 4 tests with a `MockCartridge`. Also added `#![deny(unsafe_code)]` to `crates/api` (zero unsafe code, a domain crate AGENTS.md requires this for). A sound dynamic-plugin ABI (e.g. a `repr(C)` command buffer the plugin draws into, replayed by the host) is real design work, not a bounded fix — left to the plugin-lifecycle RFC. Verified: `cargo check -p api -p studio_hud --all-targets`; `cargo clippy -p api -p studio_hud --lib -- -D warnings`; `cargo fmt --check`; `cargo test -p studio_hud --lib plugin_api` (4/4 pass); `cargo run -p ast_auditor -- audit` (0 violations, 3 files). Commit: `ffb02ea`, pushed to `origin/codex/repository-hardening`. Follow-up for whoever picks up M4: `plugin_api.rs` is now wired in and compiles, but still has zero call sites outside its own tests — the wire-up-or-remove decision is unchanged, only now against sound code instead of unsound dead code. |
| C3 | Dependency review configuration and report format | Done | Claude Sonnet 5 | main (devtools) | 2026-09-16T07:40:00Z | `foundation dependency-review --checkout <path> --output <path>` (foundation/crates/foundation_cli/src/dependency_review.rs): runs `cargo audit --json` + `cargo metadata` as external processes, writes a versioned report per `contracts/dependency-review/v1/SCHEMA.md`. Read-only; no dependency updates. Classifies findings `external` vs `local_workspace_name_collision` (real collision found: `crates/scratchpad` vs. an unrelated vulnerable crates.io `scratchpad`) using `cargo metadata`'s workspace-member list crossed against Cargo.lock's `source` field. Ran for real against Aaroneous `41b6408`: 10 external findings (hickory-net/proto, rkyv, rustls, atomic-polyfill, bincode, cgmath x2, paste, lru), 2 correctly-classified collisions — see `contracts/dependency-review/v1/reports/aaroneous-41b6408.json`. `cargo test`/clippy -D warnings/fmt --check all pass in foundation/. Commit: 64b634f. External findings are recorded, not fixed — S2 is revalidation, remediation is a separate reviewed Aaroneous change. |
| C4 | WIT, registry, config, and terminology inventory: `specs/wit/**`, `registry/**`, `config/**`, and residual terminology across Aaroneous | Done | Antigravity / Gemini 3.8 | main (devtools) | 2026-09-16T07:30:00Z | Full read-only inventory completed against Wave 11 baseline (`6a80fb2`). Cataloged 103 WIT files (zero `wit-bindgen` consumers), 49 registry schemas (fallback chains documented), 25 config files (5 safe for dead-code pruning), and residual biological/mythological shims across 5 core crates. Classifications and phased remediation plan published at `inventories/c4_wit_registry_config_terminology.md`. Aaroneous working tree preserved untouched. Commit: 9df34c3. |
| C5 | Component profile foundation: explicit-input crate inventory and invariant report contracts in devtools | Done | Codex | main (devtools) | 2026-09-16T15:32:55Z | `foundation component-inventory --checkout ABS --crate-path REL --output ABS`; contract `contracts/component-inventory-v1.schema.json`; reviewed with format, 11 foundation tests, strict Clippy, focused forbidden-pattern inspection, and clean diff check. Read-only report exercised against `crates/ast_auditor`. Implementation: `ce9a976`. |
| C6 | Local proposal foundation: explicit evidence bundle, structured proposal contract, and native local-Ollama request boundary | Done | Codex | main (devtools) | 2026-09-16T16:00:00Z | Native `foundation local-proposal` enforces explicit endpoint/evidence/output, temperature 0.0, 4096+ budget, 64 KiB evidence limit, and structured JSON. Reviewed with format, 12 tests, strict Clippy, a focused forbidden-pattern scan, clean diff check, and one bounded Qwen 9B Q6 proposal. Implementation: `cc5d920`. |
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
