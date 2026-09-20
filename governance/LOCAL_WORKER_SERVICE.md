# Local worker service

This service makes the PC an unattended Rust support-work processor. It polls a directory of approved TOML jobs every two minutes and processes at most one new job per poll with local `qwen3.5:9b-q6`. It uses no cloud-model API and does not spend Codex credits. It is a support lane, not an autonomous product committer.

## Scheduling and isolation

| Service | Schedule | Role |
| --- | --- | --- |
| Native multithreader review | Hourly at :00 | Propose evidence-backed new work |
| Codex audit and Rust reuse review | Hourly at :30 | Audit and research community alternatives |
| Local worker | Poll every two minutes | Analyze approved evidence and draft Rust fixtures/code in artifacts |

Local model dispatch is allowed only during minutes 06–20 and 41–50. Requests have a 120-second timeout, leaving a margin before the hourly reviews. All local worker ticks acquire the same exclusive `review.lock` used by native multithreader review. The Codex audit is instructed to acquire that reservation for its complete deeper review and defer if it cannot. Quiet windows prevent starts around the scheduled slots; the shared reservation also covers longer reviews. Other Ollama clients do not participate in this lock; route cooperating agents through this worker.

The worker skips evidence referenced by Claimed or Review queue rows, including quoted directory/** scopes. This is a conservative textual scope check, not a full Markdown/glob parser. Product source is read-only. All proposed code stays inside JSON artifacts, is explicitly unverified, and needs owner review and Aaroneous's native gates before adoption.

## Installed runtime

The worker runs as an in-process service via `foundation multithreader-serve` (default off) or as a manual tick. There is no Windows scheduled task; execution is governed by a daily 09:00 local maintainer review. The PC must be awake and Ollama available.

Runtime root: `C:\Users\aarog\.codex\worktrees\aaroneous-devtools\artifacts\local-worker-service`. It contains a copied native executable, explicit `worker.toml`, `results/`, and `archive/`. The reservation directory is the existing native review inbox. Runtime files are ignored by Git and never placed in Aaroneous.

Config fields: schema_version=1, absolute checkout, queue, jobs_directory, results_directory, archive_directory, reservation_directory; explicit loopback HTTP endpoint; max_pending from 1 to 16. Results/archive/reservation must be outside the product. Configured max_pending is 8. Bootstrap injects time into dispatch policy; the worker does not infer machine paths.

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

The three initial jobs cover emulator reducer fixtures, path edge cases and gate behavior. Polling discovers added jobs; edited instructions or evidence trigger a new attempt. Exact recorded inputs suppress repeated unchanged work, including unchanged failures. The worker intentionally becomes idle when no eligible new work exists rather than manufacturing work to keep the GPU busy.

## Reviewing results and recovery

The worker uses [Ollama schema-constrained structured output](https://docs.ollama.com/capabilities/structured-outputs) and strict Rust deserialization to reject duplicate or unknown fields. No ambient HTTP proxy is used.

Results are versioned JSON containing job ID, submission time, complete input snapshot, proposed observations/risks/test plan, optional rust_draft, and outcome. No draft is executed, formatted, compiled, applied, merged or pushed automatically. A successful generation means pending-owner-review, not a successful product fix.

Review pending records, then move reviewed records from results/ to archive/ while the task is paused; retain them so unchanged jobs are not regenerated. All complete JSON records in results/ count against the pending limit, including failures. When 8 records await review, dispatch pauses. This bounds review debt.

Inputs and result directories are bounded to 256 files each. Archive older reviewed batches outside the active archive directory with an explicit retention decision; removing the active input history may allow an unchanged job to regenerate. Results publish through synced .partial staging files and rename; interrupted partial files are ignored and retained for inspection. Stale reservation locks (>3600s) are automatically recovered while retaining crash markers (`crash_marker_retained.json`) in the results directory. Active locks write owner/time/PID metadata and return reserved status (`status.txt`).

Model failures are retained and exit nonzero. They are not retried indefinitely: restore Ollama or correct the job, review/archive the failure and explicitly change its instruction (for example, record the retry reason) to request a fresh attempt. This keeps an outage from flooding the inbox.

## Operations

```powershell
# Start in-process worker (default off)
cargo run -p foundation -- multithreader-serve --config ABSOLUTE_WORKER_TOML

# Manual single tick
cargo run -p foundation -- local-worker-tick --config ABSOLUTE_WORKER_TOML
```

Quiet windows and reservations still apply. To update the binary, stop the worker, build foundation, replace the copied executable, then restart. Adding multiple polling services for the same GPU is unnecessary; extend approved job manifests instead.

## Verification

Foundation tests cover quiet windows, exclusive reservations, scope exclusions, changed/unchanged inputs, backpressure, failure retention and protocol handling. Live scheduled execution evidence is recorded in the coordination queue. This service does not certify Aaroneous correctness and is not a framework dependency.
