# Aaroneous Operational Worklist

See `MAINTENANCE_SYSTEM.md` for operating rules and `COORDINATION_QUEUE.md` for live ownership.

## Active

### A1 — Merge the text-encoding and boundary-audit branch

**Status:** Waiting for GitHub CI result  
**Branch:** `codex/text-encoding-integration`

**Done when:** Linux and Windows CI pass, the pull request is reviewed, and the branch merges to
`main`.

## Ready

### A2 — Establish a golden bounded reducer path

Define the smallest reusable execution-host contract from `dev/emulator_harness`, with fixed trace
replay, an allocator measurement, execution budget, and Linux/Windows CI evidence.

### S1 — Revalidate legacy security findings

Reproduce historical findings against current source before fixing confirmed memory safety, path,
authorization, deadlock, or numeric-ordering defects.

### S2 — Establish reproducible dependency vulnerability review

Use a lockfile-aware scanner and record advisory paths, fixed versions, and mitigations before
updating dependencies.

### B6 — Complete non-Rust and legacy reduction

Port the encoding checker into `ast_auditor`, classify WIT/WGSL/deployment interfaces, and perform
compatibility-led terminology reduction. See Aaroneous
`docs/architecture/LEGACY_REDUCTION_PLAN.md`.
