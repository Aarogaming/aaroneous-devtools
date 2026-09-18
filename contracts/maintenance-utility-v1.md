# Maintenance utility v1

Producer: the standalone Rust `devtools` binary in `foundation/crates/maintenance_ui/`.

Inputs: explicit Foundation worker TOML, explicit Foundation binary, separate absolute runtime directory and loopback port. `--allow-job-build` enables job publication, not product source changes. The old `--allow-verify` spelling is accepted as a compatibility alias only; it no longer exposes a verification execution button.

## Interface

- `GET /`: static packet interface; no source or runtime data embedded.
- `GET /api/status`: authorized packet summaries, job-authoring criteria, research information, service observations and retained run evidence.
- `POST /api/packet`: a single supported packet selection: `readiness`, `research`, `blueprint`, `refine-blueprint`, or `build-job`.
- `POST /api/mode`: job-authoring `plan` or `build`. Build requires all server-side publication criteria, independently of UI disabled buttons.
- `POST /api/shutdown`: controlled utility shutdown while idle; native OS tasks are unchanged.
- `POST /api/result`: inspect one pending result by filename, restricted to jobs in this utility's inbox.
- `POST /api/review`: archive that result with a required note and `accepted-for-follow-up` or `rejected` disposition. Active execution, reservation conflicts and archive collisions block review. Neither disposition verifies or changes product code.

Private API requests require the startup session token, exact loopback Host and matching Origin when present. JSON selections deny unknown fields and have a 1 KiB body limit. Raw jobs, arbitrary commands, target files and cloud dispatch are not user API routes. Result filenames are validated; other workers' jobs cannot be reviewed through this utility. Unsupported `Expect` headers (e.g. `100-continue`) and chunked/unsupported framing are rejected immediately (417/400) before body read attempts to prevent blocking the serial event loop. Server responses specify `Connection: close` and strict security headers.

The browser validates status before enabling controls, bounds requests to six seconds and response bodies to 2 MiB, and cancels outstanding requests when leaving the page. Failed observations mark retained information stale and disable action controls. Mutations with a lost/invalid response have an unknown outcome and are never automatically retried; reconnect is read-only.

## Artifacts

`blueprint.json` is a version-1 authored job definition with purpose, checkout revision, evidence digest, exact inputs, scope, steps/dependencies, acceptance evidence, rollback/retirement, open questions research depth and creation time. Refinement revises the draft in Plan. Build publishes a schema-version-1 analysis manifest into `packet-jobs/`. The published manifest is immutable; duplicate publication is idempotent. The job identifier hashes the source digest and trusted instruction; refinement publishes a new version while retaining prior definitions and run history.

`last-run.json` and `run-*.log` retain actual command outcomes. A completed worker tick can mean idle, quiet or deferred work, and never means generated code passed verification. The worker retains complete results in its configured external result/archive directories. Source evidence and generated outputs are private runtime artifacts, not Git-tracked state.

On restart, authoring remains Plan, but the published blueprint's job (or a retained inbox job) is restored for research display. Archived results remain available to that display. Review preserves the full JSON record and a separate `.review` decision containing `product_verified: false`; it does not delete the published manifest or its input history.

`utility.lock` is an exclusive process-lifetime runtime marker containing the owner and PID. Startup validates boundaries and acquires it, then binds the port before rewriting session configuration. Normal exit releases it; crashes require explicit maintainer inspection rather than automatic takeover.

## Authority

Plan/Build apply only to job authoring. Existing published jobs execute through independent orchestration. Research is read-only; generated assertions cannot authorize actions. Product code is never automatically written, rolled back, committed or pushed by this utility. Aaroneous remains independent of the utility.

Future incompatible artifact/API changes increment the contract version and provide migration behavior. New packet kinds require producer validation and regression tests before exposure.
