# Aaroneous Graduation Map

**Status**: Active plan (supersedes the 2026-09-21 "integrate Aaroneous into devtools" research draft)  
**Policy**: `Aaroneous/docs/CRATIFY_SPEC.md` v2, sections 5-6; [FOUNDATION_BOUNDARY.md](../FOUNDATION_BOUNDARY.md)  
**Updated**: 2026-09-23

## Direction

Growth is Aaroneous-centric. Devtools is the foundry: capabilities are grown and proven here, then
distilled into Aaroneous under the Cratify graduation gate, after which devtools consumes the
Aaroneous crate and deletes its own copy. The sorting rule:

> Code describing **how an agent, job, model, or verdict works** belongs in Aaroneous.
> Code describing **how this owner runs their development operation** stays in devtools.

## Already landed

| Item | Where | Notes |
|---|---|---|
| `ast_auditor` used as a library | `local_worker.rs` (`audit_source_file`) | Complete. |
| `governance` without `compute` | Aaroneous `f965955` (`adaptive-governance` feature) | Complete; devtools imports without the feature. |
| `llm_gateway_types` extracted | Aaroneous `e2224a0` | Complete; `LLMConfig` still duplicated in Aaroneous (see Phase 0). |
| `llm_gateway` Gemini provider | Aaroneous `crates/llm_gateway/src/providers/gemini.rs` (uncommitted) | In progress; devtools `local_worker_db.rs` and `overseer_snippet.rs` already call it. |
| Shared `Reservation` guard | `foundation_cli/src/reservation.rs` | Partial: `multithreader::RunLock` and `maintenance_ui::Lock` remain separate copies. |
| `PriorityScheduler` | `foundation/crates/priority_scheduler` | **Defect:** byte-for-byte copy of `Aaroneous/crates/orchestrator/src/priority_scheduler.rs`. Two maintained copies; resolved in Phase 2. |

## Graduation map

| Devtools today | Aaroneous destination | Classification | Devtools keeps |
|---|---|---|---|
| `priority_scheduler` crate | New `crates/scheduler` extracted from `orchestrator`, consumed by the new `llm_manager` | Upgrade (dedupe) | Nothing; delete crate |
| Ollama `/api/chat` clients in `local_worker`, `agentic_registrar`, `local_proposal`, `intake_generator`, `frontier_discovery`, plus `cloud_providers.rs` (Anthropic/Gemini/OpenAI) | `llm_gateway` providers: add `ollama`, `anthropic`; reuse `gemini`, `openai`. Credentials injected, never `std::env::var` | Upgrade | Provider selection and endpoint config |
| `local_worker` dispatch loop core: prompt assembly, retry/backoff, audit logging | New `crates/llm_manager` (stateful control plane over the stateless `llm_gateway`), per the LLM Manager & Priority-Constrained Scheduler spec | New component | Service wrapper, TOML configs, quiet windows |
| `Draft`, `JobRecord`, `JobState`, `IterationAttempt`, draft schema | New `crates/job_contracts` or `core-contracts` (types only, no async) | New component | Job manifests (`worker-jobs/`) |
| `ledger.rs`, `state_cache.rs`, `local_worker_db.rs` | New `crates/ledger`: storage trait + compliant backend over the `ipc_bus` WAL | New component | `sled` backend behind the trait (sled is rejected as an Aaroneous dependency; consider redb for devtools) |
| `cross_verifier`, `consensus_engine`, `fact_checker` | `compliance_auditor` as the evaluator role (strict findings schema, model diversity vs. the generator) | Upgrade | Choice of reviewer providers |
| `verify_proposal_quality`, `verify_proposal_citations` | `ast_auditor` proposal rules module | Upgrade | - |
| Job pipeline stages (`frontier_discovery` -> intake -> draft -> verify -> promote) | Pure assimilation state machine (IDLE -> QUARANTINED -> AUDITING -> SYNTHESIZING -> CERTIFYING -> COMMITTED/REJECTED) in `orchestrator/src/assimilation.rs` | Upgrade | Driving the machine: source discovery, quarantine mounts, model calls |
| `cratification_intake`, `target_decomposition` | `cratify` subcommands (`intake`, `decompose`) | Upgrade | - |
| `candidate_metadata`, `candidate_retrieval` | `capabilities` or new `crates/retrieval` | New component | Accumulated records |
| `decision_trace` (SFT samples) | `adaptation_plane` | Upgrade | Trace corpus |
| `reservation.rs`, `RunLock`, `maintenance_ui::Lock` | `paths` file-lock guard | Upgrade | - |
| `batch_overlap`, `scope_reserved` | `governance::interference_checker` | Upgrade | Queue-row inputs |
| `hypervisor_mcp.rs` | Delete; use `mcp_server` | - | - |

**Stays in devtools** (operational, not general): `portfolio_jobs`, `portfolio_digest`,
`scheduler.rs`, `consensus_readiness`, `audit_rollup`, `discovery_digest`, `frontier_discovery`'s
web crawling, `dogfood`, `dependency_review`, `local_staging`, `maintenance_ui`, `gui/`, PowerShell
scripts, `worker-jobs/`, and all governance markdown.

## Sequence

Portfolio IDs are in [governance/MULTITHREAD_PROGRAM.md](../governance/MULTITHREAD_PROGRAM.md).

| Phase | Portfolio | Work |
|---|---|---|
| 0 | M75 | Aaroneous plane consolidation: fold `orchestration_plane` into `orchestrator`, merge `adaptation_plane`/`adaptation_engine`, dedupe `action_executor`/`decision_engine`/`mcp_bridge`/`mcp_service`, collapse four `LLMConfig` definitions into `llm_gateway_types`, fold `orchestrator/src/lmstudio_client.rs` and `orchestrator/src/llm/` into `llm_gateway` |
| 1 | M76 | Provider consolidation into `llm_gateway` with injected credentials; devtools deletes its six HTTP clients |
| 2 | M77 | Extract `crates/scheduler`; delete devtools `priority_scheduler`; `paths` file-lock guard |
| 3 | M78 | `job_contracts` shared types; `ledger` storage trait with WAL backend and devtools sled backend |
| 4 | M79 | Verification graduation: proposal rules into `ast_auditor`, evaluator role into `compliance_auditor` |
| 5 | M80 | `llm_manager` + assimilation state machine; split `local_worker.rs` / `agentic_registrar.rs` into Aaroneous reducers and thin devtools service wrappers |

Every phase follows the graduation gate: behind an Aaroneous trait, landed inert or in shadow mode,
origin copy deleted in the same cycle, violations ratcheted into `ast_auditor`.

## Open decisions

- **Pinning:** foundation crates use `../../Aaroneous` path dependencies. As devtools leans harder on
  Aaroneous crates, pin to a git revision so an in-progress Aaroneous branch cannot break the foundry.
- **Devtools storage:** stay on `sled` 0.34 (last stable; 1.0 still alpha) or move to `redb` when the
  `ledger` trait lands.

## History

The 2026-09-21 draft (author: mimo/opencode) planned the reverse direction (importing Aaroneous into
devtools) and produced findings that remain accurate and were acted on: `ast_auditor` is synchronous
and importable; `PriorityScheduler` is ~222 lines with only `uuid`/`thiserror` dependencies and zero
coupling to orchestrator internals; 9 of 11 `governance` modules do not need `compute`; `llm_gateway`
types are synchronous while its runtime is async. See Git history for the full text.
