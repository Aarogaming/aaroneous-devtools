# Local worker operations: delegation, inbox and outbox

One page that answers "what do I give the local model, how do I load and clear its
queues, and what is not yet automated". It indexes the detailed docs instead of
replacing them: [LOCAL_WORKER_SERVICE.md](LOCAL_WORKER_SERVICE.md),
[LOCAL_MODEL_AUTONOMY_POLICY.md](LOCAL_MODEL_AUTONOMY_POLICY.md),
[MULTITHREADER_SERVICE.md](MULTITHREADER_SERVICE.md),
[LOCAL_AGENT_CONTROL_PLANE.md](LOCAL_AGENT_CONTROL_PLANE.md).

State recorded 2026-09-29 (Claude), from `origin/main` at `8c14ec0` plus the host
inspection below. Re-verify before relying on it; the runtime facts are host-specific.

## 1. Which delegation path to use

| Path | Entry point | Input bound | Output | Use for | Not for |
| --- | --- | --- | --- | --- | --- |
| Support worker (default) | reviewed TOML in `worker-jobs/` -> `foundation local-worker-tick` / `local-worker-serve` | 1-8 declared evidence files, 32 KiB each, 64 KiB total | versioned JSON record in `results/`, status `pending-owner-review` | fixture drafts, boundary reviews, report classification | anything needing a compile, a claim, or a product write |
| Multithreader review | `foundation multithreader-review` / `multithreader-serve` | evidence files named in its config | candidate-inbox record | proposing new work from declared evidence | implementing work |
| Registrar dispatch | `foundation` `dispatch` / `promote` (agentic registrar) | registered task + target file | virtual patch set; promotion is gated | a bounded draft for one target file; promotion has a high-risk allowlist and circuit breaker | safety-critical crates (`ipc_bus`, `wire`, `hypervisor`, `governance`, ...) |
| Ad hoc script | product `scripts/local_agent_delegate.ps1` | none enforced | free-form text to a file | throwaway drafting on the owner's machine | anything recorded as evidence; it has no bounds, schema, or inbox |

Rule of thumb: start with the support worker. Use registrar dispatch only when a
reviewer will materialize the result through a virtual patch set. Treat the ad hoc
script as scratch, not as part of the pipeline.

Policy that applies to every path: models draft or classify; software validates;
a different seat accepts (LOCAL_MODEL_AUTONOMY_POLICY tiers 0-3). Suggested job shapes,
from the C56 trial and later probes: `test-fixture` jobs on narrow files scan clean
with or without the rule pack, `analysis` review jobs are where claims must be
checked against the evidence (see `conformance::unanchored_observations`, C76).

## 2. The inbox: priming

The inbox is the directory of reviewed job manifests, `worker-jobs/*.toml` in devtools.
Priming today is manual: write a manifest, get it reviewed, commit it.

Fields: `schema_version = 1`, unique `id`, `mode` (`analysis`, `test-fixture`,
`code-draft`), 1-8 exact existing relative `evidence` paths, bounded `instruction`.
A job whose evidence is referenced by a Claimed/Review queue row is skipped (a textual
scope check, not a glob parser). Selection is oldest-last-attempted-first.

Limits checked against `local_worker.rs` on `main` (`8c14ec0`): 1-8 evidence files, each read up to
32768 bytes, total input at most 65536 bytes, instruction at most 8192 bytes, job id at most 64 characters,
symlink evidence refused. **Dispatch happens only during minutes 6-20 and 41-50 of the hour**
(`dispatch_window`); a tick at any other minute does nothing, so "the worker did not run my job" is
usually the window, the `max_pending` cap, or a scope reservation, not a fault.

Not implemented: no command to list, validate, or lint manifests before a tick;
validation happens when a tick runs. (Gap: see the C92 row in the queue.)

## 3. The outbox: reviewing and clearing

`results/` holds one JSON record per attempt; `archive/` holds reviewed records.

- Dispatch stops when `results/` holds `max_pending` records (1-16 configured); failures
  count. Each directory is limited to 256 files.
- Clearing is manual: pause the service, move reviewed files from `results/` to
  `archive/`, resume. Archive history is what prevents an unchanged job from being
  regenerated, so do not delete it casually.
- To retry a failed job, change its `instruction` (record the reason) after reviewing
  or archiving the failure. Failures are never retried automatically.
- Partial files (`*.partial`) are ignored and retained for inspection.

Not implemented: no `status`, `list`, `archive`, or `reset` command for the worker.
`reset-circuit-breaker` exists for the registrar only. The only runtime status signal is
`status.txt`. (Gap: C92.)

## 4. Recorded host state (2026-09-29)

- No Windows scheduled task matching `Aaroneous|Devtools|LocalWorker|Multithreader` was
  found; consistent with `PERSISTENT_OPERATIONS.md` (the multithreader task was removed
  on 2026-09-17). Older docs that say a task is "installed" describe 2026-09-16.
- Runtime root `C:\Users\aarog\.codex\worktrees\aaroneous-devtools\artifacts\local-worker-service`
  contains `foundation.exe` dated 2026-09-16, `worker.toml`, `results/` (4 records) and
  `archive/` (4 records), last written 2026-09-17. That binary predates every change on
  `main` since; rebuild before reuse.
- Local model: `qwen3.5:9b-q6` lives in the LM Studio model store
  (`...\.lmstudio\models\lmstudio-community`). The Ollama desktop app is already configured for it: its
  settings database records that directory as the model location and the model as selected, and its server
  logs show that directory as `OLLAMA_MODELS`. A separate `ollama serve` started from a shell does NOT inherit
  the app's setting and, with the default model directory, lists only models pulled there; set `OLLAMA_MODELS`
  in that process's environment (process-level only) or let the app start the server. Confirm with
  `GET /api/tags` before a tick. An unavailable model is a `Blocked` condition, not a retry loop.
- Context: the worker's requests set no `num_ctx`; on 2026-09-29 a 6,355-token prompt sent without it was
  processed in full and the first evidence line was quoted back correctly, so silent truncation was not
  observed with this model. Re-check if the model or server configuration changes.
- Known model failure mode: on the paths-review job the model can loop inside one JSON string until the token
  budget is spent (one phrase repeated 117 times), which the worker rejects as truncated. The worker's
  `draft_schema` has no `maxLength` on observation strings; see the C92 row.

## 5. Change control for this page

Update it in the same change as any of: a new job field, a new worker command, a new
directory bound, a change to the dispatch window, or a change to the model tag or
location. Keep facts dated and host-specific facts under section 4.
