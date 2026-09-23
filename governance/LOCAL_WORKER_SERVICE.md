# Local worker service

This service makes the PC an unattended Rust support-work processor. It continuously polls a directory of approved TOML jobs and processes them with local `qwen3.5:9b-q6`. It uses no cloud-model API and does not spend Codex credits. It is a support lane, not an autonomous product committer.

## Scheduling and isolation

| Service | Schedule | Role |
| --- | --- | --- |
| Native multithreader review | Hourly at :00 | Propose evidence-backed new work |
| Codex audit and Rust reuse review | Hourly at :30 | Audit and research community alternatives |
| Local worker | Adaptive: triggers 5s after previous tick completes, backs off to 60s when quiet | Analyze approved evidence and draft Rust fixtures/code in artifacts |

The local worker operates as a continuous background persistence layer. Dispatch is always allowed; there are no quiet windows or time-based cooldowns. Requests have a 120-second timeout. All local worker ticks acquire the same exclusive `review.lock` used by native multithreader review. The Codex audit is instructed to acquire that reservation for its complete deeper review and defer if it cannot. The shared reservation also covers longer reviews. Other Ollama clients do not participate in this lock; route cooperating agents through this worker.

The worker skips evidence referenced by Claimed or Review queue rows, including quoted directory/** scopes. This is a conservative textual scope check, not a full Markdown/glob parser. Product source is read-only. All proposed code stays inside JSON artifacts, is explicitly unverified, and needs owner review and Aaroneous's native gates before adoption.

## Installed runtime (fallback)

Windows task: `Aaroneous-Devtools-LocalWorker`, limited privileges, logged-in session, overlapping launches ignored, three-minute OS execution limit. Windows denied S4U logged-out registration, so genuinely logged-out execution is not installed. Locked-screen execution still uses the logged-in session. The PC must be awake and Ollama available. No power settings or credentials were changed.

**Preferred mode:** `foundation local-worker-serve --config <path>` runs the adaptive loop in-process. This is faster (5s after activity, not 2min) and more responsive than the Windows task. Use this instead of the scheduled task when possible.

Runtime root is an explicit owner-selected `worker.toml`, not a fixed machine path. The historical `C:\Users\aarog\.codex\worktrees\aaroneous-devtools\artifacts\local-worker-service` root must not be inferred as active simply because it contains a copied executable. For the current reconciliation evidence and selected runtime, see [LOCAL_WORKER_RUNTIME_RECONCILIATION.md](LOCAL_WORKER_RUNTIME_RECONCILIATION.md). Runtime files are ignored by Git and never placed in Aaroneous.

Config fields: schema_version=1, absolute checkout, queue, jobs_directory, results_directory, archive_directory, reservation_directory; explicit loopback HTTP endpoint; max_pending from 1 to 64. Results/archive/reservation must be outside the product. The configured pending limit is runtime-specific and must be read from the selected `worker.toml`; it is not inferred from this document. Bootstrap injects time into dispatch policy; the worker does not infer machine paths.

## Submitting useful work

Add a reviewed TOML manifest under `worker-jobs/` in devtools. This directory is an approved support inbox, separate from primary queue ownership. Jobs never automatically claim primary work.

```toml
schema_version = 1
id = 'EXAMPLE-UNIQUE-ID'
mode = 'test-fixture' # analysis, test-fixture, or code-draft
 evidence = ['dev/emulator_harness/src/lib.rs']
instruction = 'Propose one bounded regression fixture using the actual API; output unverified Rust source and identify assumptions.'
```

Declare 1–8 exact existing relative files. Each file is bounded to 32 KiB, total input to 64 KiB, instruction to 8 KiB, and response to 64 KiB. Do not submit entire crates or ask the model to execute commands. Keep jobs narrowly useful: fixture drafting, boundary reviews, protocol test design, report classification, or component-contract analysis. Prefer deterministic tooling over model analysis where tooling already answers the question.

The initial three jobs cover emulator reducer fixtures, path edge cases and gate behavior; a second batch added 2026-09-18 covers `ast_auditor`'s safety-comment checker, `compliance_auditor`'s review pipeline, and `governance`'s rollback journal — added once the job-selection fairness fix (oldest-last-attempted-first, not alphabetical) landed, so a growing backlog rotates fairly instead of risking one job starving the rest. Polling discovers added jobs; edited instructions or evidence trigger a new attempt. Exact recorded inputs suppress repeated unchanged work, including unchanged failures. The worker intentionally becomes idle when no eligible new work exists rather than manufacturing work to keep the GPU busy.

## Reviewing results and recovery

The worker uses [Ollama schema-constrained structured output](https://docs.ollama.com/capabilities/structured-outputs) and strict Rust deserialization to reject duplicate or unknown fields. No ambient HTTP proxy is used.

Results are versioned JSON containing job ID, submission time, complete input snapshot, proposed observations/risks/test plan, optional rust_draft, and outcome. No draft is executed, formatted, compiled, applied, merged or pushed automatically. A successful generation means pending-owner-review, not a successful product fix.

Review pending records, then move reviewed records from results/ to archive/ while the task is paused; retain them so unchanged jobs are not regenerated. All complete JSON records in results/ count against the configured pending limit, including failures. When that limit is reached, dispatch pauses. This bounds review debt.

Inputs and result directories are bounded to 256 files each. Archive older reviewed batches outside the active archive directory with an explicit retention decision; removing the active input history may allow an unchanged job to regenerate. Results publish through synced .partial staging files and rename; interrupted partial files are ignored and retained for inspection. Stale reservation locks (>3600s) are automatically recovered while retaining crash markers (`crash_marker_retained.json`) in the results directory. Active locks write owner/time/PID metadata and return reserved status (`status.txt`).

Model failures are retained and exit nonzero. They are not retried indefinitely: restore Ollama or correct the job, review/archive the failure and explicitly change its instruction (for example, record the retry reason) to request a fresh attempt. This keeps an outage from flooding the inbox.

## Operations

```powershell
Start-ScheduledTask -TaskName 'Aaroneous-Devtools-LocalWorker'
Get-ScheduledTaskInfo -TaskName 'Aaroneous-Devtools-LocalWorker'
Disable-ScheduledTask -TaskName 'Aaroneous-Devtools-LocalWorker'
Enable-ScheduledTask -TaskName 'Aaroneous-Devtools-LocalWorker'
```

For a manual tick, run the installed foundation.exe with `local-worker-tick --config ABSOLUTE_WORKER_TOML`. Quiet windows and reservations still apply. To update the binary, pause the task, verify it is not running, build foundation, replace the copied executable, then resume. Adding multiple polling services for the same GPU is unnecessary; extend approved job manifests instead.

## Adaptive persistent mode: `local-worker-serve`

`foundation local-worker-serve --config ABSOLUTE_WORKER_TOML` runs ticks in-process on an adaptive interval instead of relying on the installed Windows task's fixed two-minute cadence: it ticks, reads `status.txt` to see what that tick actually did, and only then decides the next delay — doubling on a quiet tick (nothing eligible, backpressure, or `review.lock` already held) up to a 60-second ceiling, and resetting to the 5-second base the moment a job is actually attempted. This is the "trigger, then look at what happened, then decide when to trigger again" pattern, not a blind fixed-interval loop. Same opt-in shape as `multithreader-serve` (C21 in the coordination queue): starting the command is "on", not running it or stopping it (Ctrl+C) is "off" — nothing is installed, so there is nothing to separately disable. Do not run this alongside the installed `Aaroneous-Devtools-LocalWorker` Windows task against the same config; both would compete for the same `review.lock` and job pool. Pick one.

## Verification

Foundation tests cover exclusive reservations, scope exclusions, changed/unchanged inputs, backpressure, failure retention, protocol handling, job-selection fairness (a job whose input changes every tick can no longer starve every other eligible job — selection is oldest-last-attempted-first, not alphabetical), and `local-worker-serve`'s quiet/active pacing logic. Live scheduled execution evidence is recorded in the coordination queue. This service does not certify Aaroneous correctness and is not a framework dependency.
