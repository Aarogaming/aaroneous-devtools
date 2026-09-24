# Controlled production-pipeline trial: C56

Date: 2026-09-23. Product checkout HEAD:
`4e7f71fdb1cb6a7268364191a67159f7ff0484b7`. Exact source contents are
retained in each request; HEAD alone does not certify a clean checkout.

## Boundaries and reproducibility

Existing packets were copied into `artifacts/controlled-pipeline-c56/`, with
separate analysis/fixture/code jobs, results and archives. A fourth `context`
configuration repeats the path analysis with focused excerpts. Each config uses
the real coordination queue, existing shared reservation, explicit product
checkout, `qwen3.5:9b-q6`, and `refill_jobs = false`. One tick per configuration;
at most one correction per tick. No service installation, shared-packet changes,
product writes, or automatic integration. No historical results were deleted.

Worker entrypoint: `cargo run --manifest-path foundation/Cargo.toml -p
foundation_cli -- local-worker-tick --config <absolute trial worker.toml>`.
The trial used the C56 working tree, not a replaced installed service binary.

## Observed outcomes (not completion claims)

| Existing task | Model calls | Inspection / executed check | Disposition |
| --- | --- | --- | --- |
| PATHS-REVIEW, whole file | 1 | Claimed recursion lacked depth protection; supplied source explicitly checks `max_depth == 0` and decrements on recursion | Needs changes; source-contradicted claim |
| ROLLBACK-JOURNAL-FIXTURES | 2 | Initial response falsely claimed missing empty-journal guard; automatic correction wrapped tests but introduced malformed function signatures. `rustfmt --edition 2024 --check .../fixture/draft.rs` reports unexpected closing delimiter | Needs changes; actual requested source exists but cannot parse |
| FIX-C40-WORKSPACE-VERSION | 1 | Reviewed draft executed unchanged in isolated Cargo crate: three model-supplied tests ran, two passed, explicit member version failed with `Expected table for version, found String("4.5.6")` | Needs changes; fixes inheritance while breaking explicit versions |
| PATHS-REVIEW, focused context | 1 | Same instruction, excerpts 119-169 and 512-610. Claimed `CurDir` handling missing despite explicit matching arm in supplied excerpt | Needs changes; narrowing alone did not establish correctness |

The focused input also selected the adapter's smaller context window (16384
versus 32768), so this is a practical package comparison, not a clean causal
experiment isolating only excerpt length. One sample per package is not a model
quality benchmark. These are evaluator findings; worker records remain
`pending-owner-review`, illustrating the distinction between delivery and
acceptance. No accept verdict or product integration was recorded.

Code check: `cargo test --offline --manifest-path
D:/aaroneous-devtools/artifacts/controlled-pipeline-c56/code/Cargo.toml`.
The draft and dependency lock are retained there. Executed code was inspected
first; fixture writes occur in temporary test directories. This is a focused
scratch test, not Aaroneous's full native gate.

## Implemented C56 controls

- Each dispatch gets a unique `results/attempts/<uuid>/` journal containing task
  identity, exact input, model/endpoint, system prompt, request, response,
  validation diagnostics, and final result snapshot when available.
- Request intent is synced before dispatch. Returned proposals/errors are synced
  before correction. Both first and corrected responses survive. Events are
  immutable; partial writes are retained, not silently replaced.
- Storage errors stop processing rather than being classified as provider errors.
  A request without a response denotes unknown/interrupted execution, not success.
- `refill_jobs = false` disables automatic producers for controlled runs; default
  remains true for existing configurations. Corrections now honor temperature 0.0.
- Unit tests inject interruption and request/response storage failures, verify
  both responses survive, and replay three existing task contracts with an outage.

## Remaining production gates

This is not a complete lossless task engine. Journal files are retained evidence,
not an automatic restart-reconciliation or retry-budget controller. OS/filesystem
power-loss guarantees, bounded retention/compaction, raw oversized/invalid-byte
transport retention, and recovery after storage exhaustion need separate work.
No task may be called complete merely because a journal has a final event.

The live trial exposed two gate weaknesses: fixture snippets placed inside an
existing test module can be falsely rejected by the textual panic check, and
malformed Rust can pass the current textual checks. Use declared placement and
syntax/compile checks, not a global string heuristic. Analysis findings need
source-grounded acceptance assertions. The adapter's model-size label also called
the configured 9B model "14B+"; this label is not evidence of the actual model.

Next increment: reusable context packages with source identity/digests, explicit
acceptance and placement contracts, and evaluator diagnostics fed into bounded
rework. Preserve these failed samples as test cases; do not automatically convert
them into new product bugs or repeatedly dispatch unchanged inputs.
