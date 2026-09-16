# Parallel Coordination Queue

Claim items from `WORKLIST.md` before editing Aaroneous. Claims expire after 24 hours without an
update. A takeover requires a handoff note and must not overwrite uncommitted work.

| ID | Scope | Status | Owner | Branch | Started UTC | Evidence / handoff |
| --- | --- | --- | --- | --- | --- | --- |
| C1 | Native text encoding: `crates/ast_auditor/**`, encoding script, CI/docs | Ready | — | — | — | — |
| C2 | Security revalidation: `crates/studio_hud/src/plugin_api.rs` and focused tests | Ready | — | — | — | — |
| C3 | Dependency review configuration and report format | Ready | — | — | — | — |
| C4 | WIT, registry, config, and terminology inventory | Ready | — | — | — | — |

## Claim format

Add the owner, branch, exact paths, UTC start, and intended evidence to the row. On completion,
replace the status with `Done` and add commit plus verification. On blockage, record the blocker
and release the paths.

## Thread roles

- **Codex:** Rust implementation, verification, and integration.
- **Gemini 3.8 / Antigravity:** independent implementation or review lane.
- **Claude Sonnet 5 / Claude:** independent implementation or review lane.
- **Qwen:** local, bounded, owner-reviewed scratch subtask only.
