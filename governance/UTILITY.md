# Simple maintenance utility

The packet interface uses Rust Foundation and has no Node or browser-extension installation requirement. The existing TypeScript prototype remains preserved in its original checkout; this utility does not certify or overwrite it.

Read [the job pipeline](../architecture/JOB_PIPELINE.md) for authoring/orchestration separation and [the interface contract](../contracts/maintenance-utility-v1.md) for authority and artifact details.

## Build

From the devtools checkout:

```text
cargo build --manifest-path foundation/Cargo.toml --workspace --bins
```

Place `devtools` and `foundation` in a caller-selected binary directory, or use the executables emitted by Cargo. Do not assume a default Cargo target directory: host configuration may select an external build cache.

## Launch

```text
devtools --worker-config <absolute-existing-worker.toml> --runtime <absolute-separate-runtime-directory> --foundation <absolute-foundation-binary> --allow-job-build --open
```

Without `--allow-job-build`, the utility supports inspection and shipped research packets but cannot publish a new authored job. The default port is 8787 on 127.0.0.1. Open the complete link printed at startup. Its token stays in the browser session, not in HTTP query strings or API logs. Do not share that link.

Windows also has a thin `platform/windows/start-utility.ps1` launcher with explicit binary, config and runtime arguments. It contains no maintenance policy. Close an idle utility through its controlled shutdown API or stop its foreground process; existing native local tasks stay installed. Forcefully stopping an active operation can leave a worker reservation requiring maintainer inspection.

## Use

1. Check readiness to receive a packet of known facts and blockers.
2. Research now to run the shipped bounded information-gathering profile. It may wait for a quiet window or review capacity; no verification or implementation is implied.
3. Prepare the supported job blueprint. Inspect its scope, dependencies, acceptance criteria and retirement strategy. Use Refine blueprint to deepen the fixed research profile with alternatives, dependency assessment and falsifiable tests.
4. Switch authoring to Build only after every publication criterion passes, then publish the prepared job. Refining a published definition requires returning to Plan and creates a new version; previous definitions remain intact.
5. Leave the utility process running for its local pipeline to poll published packet jobs. Authoring may return to Plan while existing jobs continue.
6. Open a pending utility result in Review research results, inspect its complete input/proposal/failure evidence, enter a note, and accept for follow-up or reject. Both choices preserve the record and decision in the archive; neither implements suggestions or verifies product code. Other workers' results cannot be disposed through this interface.

Restarting with the same explicit runtime restores research display from published jobs and retained results, including archived decisions' result evidence. Authoring starts in Plan. Worker input history continues to suppress unchanged inference; no fresh model call is requested simply by viewing or reviewing a result.

The current template reads the wire component to develop hardening ideas. It does not author arbitrary product changes, consolidate all Windows services, or implement cloud provider routing. Those require separate reviewed profiles/adapters rather than more low-level user controls.

## Operations

The utility's runtime must be outside the product and must not overlap existing worker/config storage. It has its own job inbox but shares the configured result/archive and reservation contracts. Existing OS workers do not automatically ingest its inbox. Scheduling stays in native software; no Codex automation is created.

Status distinguishes real observations from stale/missing data. The existing worker does not record every quiet/reserved early return, so a last-poll timestamp can be stale while the task still exists. Do not label the service healthy solely from that timestamp.

Retention is bounded: at most 256 artifacts per worker directory, pending-result backpressure from its configuration, and a utility launch limit of 512 operation logs or 64 MiB of retained logs. A maintainer reviews/archives logs; no automatic recursive deletion occurs. Preserve result/attempt history, which suppresses duplicate inference. Model errors are retained and are not retried unchanged.

Run Foundation formatting, tests and strict Clippy for changes. Verify the live packet flow and negative entry criteria before shipping; a successful compile alone is insufficient.
