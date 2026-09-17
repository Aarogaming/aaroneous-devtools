# Aaroneous Operational Worklist

See `MAINTENANCE_SYSTEM.md` for operating rules and `COORDINATION_QUEUE.md` for live ownership.

## Status of this document

These are historical task briefs, retained to preserve acceptance context. Their original Ready/Active labels are not current certification. Consult [the priority portfolio](MULTITHREAD_PROGRAM.md) and [the coordination queue](COORDINATION_QUEUE.md) for current selection and ownership. Application work follows [the maintenance program delivery map](../architecture/MAINTENANCE_PROGRAM.md). Do not duplicate new tasks here.

## Active

### A1 — Merge the text-encoding and boundary-audit branch

**Status:** CI passed; integration pull request is not currently open
**Branch:** `codex/text-encoding-integration`

**Done when:** The current integration branch has an open pull request, its Linux and Windows CI
passes, the pull request is reviewed, and the branch merges to `main`.

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

### B1 — Enforce execution-host dependency policy

After the bounded reducer path is proven, create a package-level policy that prevents the execution
host from acquiring browser, GUI, database, model, filesystem-watch, GPU, network, or async-runtime
dependencies.

### B2 — Narrow the hypervisor public surface

Keep hypervisor binaries as composition roots and replace broad library re-exports with narrow
traits and typed frames owned by lower layers.

### B3 — Extract bounded primitives from mixed crates

Move only proven scan-path primitives from `compute` and `ipc_bus`; leave persistence, model
loading, OS integration, and scheduling outside the execution-host dependency closure.

### B4 — Define the plugin lifecycle contract

Write an RFC covering the distinction between the current in-process Rust trait and any future
dynamic ABI or process boundary, including ownership, startup, shutdown, compatibility, and
failure isolation.

### B5 — Publish measured assurance claims

Replace unmeasured latency and verification claims with reproducible benchmark reports covering
machine, profile, inputs, warmup, allocation observation, and latency distribution.
### B6 — Complete non-Rust and legacy reduction

Port the encoding checker into `ast_auditor`, classify WIT/WGSL/deployment interfaces, and perform
compatibility-led terminology reduction. See Aaroneous
`docs/architecture/LEGACY_REDUCTION_PLAN.md`.
## Foundation buildout

### F1 — Bootstrap the standalone Rust foundation workspace

Create the `foundation/` Cargo workspace and its first explicit-input CLI. It must have no
Aaroneous path, crate, build, or runtime dependency.

### F2 — Define versioned evidence contracts

Create schemas and fixtures for command results, audit inventories, and revision metadata under
`contracts/`.

### F3 — Implement stack-manifest validation

Parse and validate `stacks/*.toml` in Rust before any command runs. Reject undeclared commands,
ambient paths, and unspecified outputs.

### F4 — Add read-only dogfood harness

Run a declared Aaroneous verification command against a caller-supplied checkout and write a
versioned report without mutating the checkout.

### F5 — Build platform adapters last

Add thin Windows/Linux adapters only after F1–F4 have stable Rust commands to invoke.
## Multithread portfolio

The complete prioritized portfolio and capacity rules are in
[MULTITHREAD_PROGRAM.md](MULTITHREAD_PROGRAM.md). Ready work enters the coordination queue only
when its dependencies and exact file scope are known.
