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

The connection indicator shows the last successful observation. Failed, invalid or timed-out refreshes retain the display but mark it stale and disable actions. Reconnect/refresh reads current state; it does not repeat a mutation. Requests have a six-second deadline and a 2 MiB response limit. If a mutation loses its response, inspect retained evidence before repeating it: a timeout does not prove the server did nothing.

The current template reads the wire component to develop hardening ideas. It does not author arbitrary product changes, consolidate all Windows services, or implement cloud provider routing. Those require separate reviewed profiles/adapters rather than more low-level user controls.

## Operations

The utility's runtime must be outside the product and must not overlap existing worker/config storage. It has its own job inbox but shares the configured result/archive and reservation contracts. Existing OS workers do not automatically ingest its inbox. Scheduling stays in native software; no Codex automation is created.

A launch holds an exclusive `utility.lock` in its declared runtime and binds its loopback port before rewriting session configuration. Duplicate launches fail without replacing the running instance's configuration. Controlled shutdown releases the lock. A crash can leave a marker: inspect its PID and confirm no process owns that runtime before manually removing that exact file. There is no automatic stale-lock takeover.

Status distinguishes real observations from stale/missing data. The existing worker does not record every quiet/reserved early return, so a last-poll timestamp can be stale while the task still exists. Do not label the service healthy solely from that timestamp.

Retention is bounded: at most 256 artifacts per worker directory, pending-result backpressure from its configuration, and a utility launch limit of 512 operation logs or 64 MiB of retained logs. A maintainer reviews/archives logs; no automatic recursive deletion occurs. Preserve result/attempt history, which suppresses duplicate inference. Model errors are retained and are not retried unchanged.

## Security & Authentication

The maintenance utility enforces same-origin token authentication on all HTTP endpoints:
- **Token Protection**: Web requests must supply a matching `X-Devtools-Token` header. Header values are validated using constant-time byte comparisons (`constant_time_eq`) to eliminate side-channel timing leaks.
- **Header & Origin Binding**: Requests must present an exact `Host` matching the server loopback address and a matching `Origin` header (no wildcard CORS permitted).
- **Token-File Config**: `WorkerConfig` supports an optional `http_token_file` path. When present, `load_http_token` validates that the token file contains exactly 64 hexadecimal characters derived from 32 random bytes.
- **Fail-Closed Admission**: Unsupported framing (`Transfer-Encoding`), non-standard HTTP methods, and `Expect: 100-continue` header stalls are rejected immediately with 400/417 error codes before reading body bytes.

Browser-client regression tests need Node only for development, not deployment:

```text
node --test foundation/crates/maintenance_ui/tests/client.test.mjs
```

## Native Aaroneous Hypervisor MCP Integration

Devtools can launch and manage Aaroneous's native `hypervisor mcp` server out-of-tree without adding Cargo path dependencies:

```text
foundation hypervisor-mcp --checkout d:\Aaroneous --host 127.0.0.1 --port 8766
```

- **Out-of-Tree Resolution**: `foundation` resolves `hypervisor.exe` from Cargo build caches via `find_prebuilt_binary` or builds it out-of-tree on target checkout if absent.
- **MCP Capabilities Exposing**: Launches loopback MCP server (`127.0.0.1:8766`) exposing native `ast_auditor`, `adaptation_engine`, and `capabilities` tools to external agents and software.

