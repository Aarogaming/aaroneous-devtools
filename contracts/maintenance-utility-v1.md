# Maintenance utility v1

Producer: the standalone Rust `devtools` binary in `foundation/crates/maintenance_ui/`.

Inputs: explicit Foundation worker TOML, explicit Foundation binary, separate absolute runtime directory and loopback port. `--allow-job-build` enables job publication, not product source changes. The old `--allow-verify` spelling is accepted as a compatibility alias only; it no longer exposes a verification execution button.

## Interface

- `GET /`: static packet interface; no source or runtime data embedded.
- `GET /api/status`: authorized packet summaries, job-authoring criteria, research information, service observations and retained run evidence.
- `POST /api/packet`: a single supported packet selection: `readiness`, `research`, `blueprint`, `refine-blueprint`, or `build-job`.
- `POST /api/mode`: job-authoring `plan` or `build`. Build requires all server-side publication criteria, independently of UI disabled buttons.
- `POST /api/shutdown`: controlled utility shutdown while idle; native OS tasks are unchanged.

Private API requests require the startup session token, exact loopback Host and matching Origin when present. JSON selections deny unknown fields and have a 1 KiB body limit. Raw jobs, arbitrary commands, target files, result-disposition operations and cloud dispatch are not user API routes.

## Artifacts

`blueprint.json` is a version-1 authored job definition with purpose, checkout revision, evidence digest, exact inputs, scope, steps/dependencies, acceptance evidence, rollback/retirement, open questions research depth and creation time. Refinement revises the draft in Plan. Build publishes a schema-version-1 analysis manifest into `packet-jobs/`. The published manifest is immutable; duplicate publication is idempotent. The job identifier hashes the source digest and trusted instruction; refinement publishes a new version while retaining prior definitions and run history.

`last-run.json` and `run-*.log` retain actual command outcomes. A completed worker tick can mean idle, quiet or deferred work, and never means generated code passed verification. The worker retains complete results in its configured external result/archive directories. Source evidence and generated outputs are private runtime artifacts, not Git-tracked state.

## Authority

Plan/Build apply only to job authoring. Existing published jobs execute through independent orchestration. Research is read-only; generated assertions cannot authorize actions. Product code is never automatically written, rolled back, committed or pushed by this utility. Aaroneous remains independent of the utility.

Future incompatible artifact/API changes increment the contract version and provide migration behavior. New packet kinds require producer validation and regression tests before exposure.
