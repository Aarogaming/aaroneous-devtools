# Review Conditions

## Purpose

`CHANGE_VERIFICATION_TIERS.md` sets the evidence bar per risk tier but leaves tier
assignment to judgment ("Tier 1 is code that does not touch unsafe, FFI..."). This
document replaces that judgment call with a fixed, checkable condition table:
given what a change actually touches, it states exactly which tier and which
specific checks apply — deterministically, the same way every time, for every
reviewer (human, cloud agent, or local worker's evaluator stage). This is the
"dynamic but strict" split: what applies to a given change varies with what the
change touches (dynamic), but the rule mapping a condition to a requirement is
fixed and does not vary by who is reviewing or how busy the queue is (strict).

Every condition below cites the concrete rule or check it comes from — nothing
here is invented; it collects existing requirements from `CONTRIBUTING.md` (both
repositories), `xtask/src/gate.rs`, and `benches/README.md` into one lookup table
so a reviewer (or an automated evaluator) can apply them mechanically instead of
re-deriving them per change.

## Aaroneous conditions

| Condition (what the change touches) | Tier | Required check | Source |
| --- | --- | --- | --- |
| Any change under `core/`, `crates/`, `dev/emulator_harness/` | 1 (minimum) | Full `cargo xtask gate` (11 stages: encoding, fmt, clippy `-D warnings`, workspace check, workspace test, `ast_auditor`, zero-stub/unsafe-impl-Pod scan, emulator harness tests, release binary check, optional-feature compile, iroh compile) | `xtask/src/gate.rs`, `CONTRIBUTING.md` |
| Touches a `kernel`-profile crate (`core/hypervisor`, `ipc_bus`, `compute`, `wire`, `si_format`, `si_ir`, `platform_bridge`, `runtime_monitor`, `dev/emulator_harness`) | 1, heightened | No `String`/`Vec`/`Box`/`format!` allocation in `#[hot_path]` code; scan-loop code is marked; `#[repr(C)]` + derived `Pod` boundary types | Aaroneous `docs/CRATIFY_SPEC.md` sections 2-4 |
| Touches any Aaroneous crate | 1 | Universal floor: no ambient env/clock reads, no self-spawned threads/tasks, no `.unwrap()`/`.expect()`/`panic!` on runtime input outside tests/bootstrap; crate declares `[package.metadata.cratify] profile`; new dependencies carry an admission verdict | Aaroneous `docs/CRATIFY_SPEC.md` sections 1, 5 |
| Graduates devtools code into Aaroneous | 1, heightened | Follows the graduation gate: behind an Aaroneous trait, inert or shadow-mode, devtools copy deleted in the same cycle | Aaroneous `docs/CRATIFY_SPEC.md` section 6, `FOUNDATION_BOUNDARY.md` |
| Adds or edits `unsafe` code, `unsafe impl Pod`/`Zeroable`, or an FFI/ABI boundary | 2 | Second independent review pass per `CHANGE_VERIFICATION_TIERS.md`; gate 7 (zero-stub/unsafe-impl-Pod scan) must stay clean — manual `unsafe impl` is banned outright, not just reviewed | `CONTRIBUTING.md`, `xtask/src/gate.rs` gate 7 |
| Touches a surface named in `SECURITY.md`'s Security Architecture section (workspace path containment, the sandboxed micro-VM, constant-time auth, named pipe client restriction, HID failsafe, desktop sandbox, integrity monitoring) | 2 | Second independent review pass; explicit sign-off the named boundary is not weakened, or `SECURITY.md` is updated in the same change | `SECURITY.md`, `CHANGE_VERIFICATION_TIERS.md` |
| Adds a new crate, type, or module name | 1 | No `aaroneous_`/`aaroneous-` prefix stutter; matches Ring placement (`crates/governance` docs the canonical Ring 0–4 layout; a lower ring must not import a higher one) | `CONTRIBUTING.md` Rules table, "Architecture at a glance" |
| Adds or edits a type with a name also defined in `crates/governance` | 1 | Use the canonical name from `crates/governance`; do not introduce or extend a legacy alias | `CONTRIBUTING.md` Rules table |
| Adds or edits a test | 1 | Use `tempfile::tempdir()`; never touch the ambient filesystem; no `std::env::var`/`.canonicalize()`/ambient reads outside bootstrap | `CONTRIBUTING.md` Rules table |
| Adds, removes, or could change the behavior of a benchmarked component (`compute` SSM inference, `autonomic_adaptation` error steering, `capabilities` MCP dispatch, `si_format` mount, `ipc_bus` throughput) | 1 | Run the matching `cargo bench` target and record machine/build/date/result in `benches/README.md`'s Recording Results format before any latency/throughput claim about it is stated as fact anywhere (docs, commit message, roadmap) | `benches/README.md` |
| Adds a new CI-relevant check of any kind | 1 | A matching gate stage is added to `xtask/src/gate.rs` so local `cargo xtask gate` and CI stay mechanically identical (this is M18's own acceptance criterion) | `CONTRIBUTING.md`, `governance/MULTITHREAD_PROGRAM.md` M18 |

## Devtools conditions

Devtools follows its own, lighter standard per `governance/OPERATING_MODEL.md` —
these conditions do not import Aaroneous's zero-allocation constraints.

| Condition | Tier | Required check | Source |
| --- | --- | --- | --- |
| Any Rust change under `foundation/` | 1 (minimum) | `cargo fmt`, `cargo test`, strict Clippy for `foundation/` | `CONTRIBUTING.md` (devtools) |
| Adds or edits a `foundation_cli` command | 1 | Explicit input/output paths only; no ambient path discovery; no automatic Aaroneous source mutation. Depending on Aaroneous crates is allowed and preferred over re-implementation; nothing may make Aaroneous depend on devtools | `CONTRIBUTING.md`, `FOUNDATION_BOUNDARY.md` |
| Adds a `worker-jobs/*.toml` job template or changes `mode` handling | 1, heightened | Must stay within `LOCAL_AGENT_CONTROL_PLANE.md`'s bounds (proposal-only output; no queue-claim, write, or shell-invocation authority granted to the local worker) regardless of template content | `governance/LOCAL_AGENT_CONTROL_PLANE.md` |
| A devtools-drafted patch is proposed for flow into an Aaroneous branch | 2 | Must pass Aaroneous's own native gates (the Aaroneous conditions table above applies in full once the patch targets Aaroneous); no autonomous writer treats its own generated output as pre-verified | `FOUNDATION_BOUNDARY.md` |
| Adds or edits a UI change under `gui/` | 1 | Build/type checks plus meaningful behavior verification (not just a build pass) | `CONTRIBUTING.md` (devtools) |
| A documentation-only change | 0 | Valid links, clean encoding/diffs; no test required to mirror a prose edit | `CONTRIBUTING.md` (devtools) |
| A draft is verified via `dogfood::ComplianceProfile` (`VerifyDraftWithProfile`, M53) | 1 | `AaroneousStrict` and `DevtoolsOperatingModel` are not interchangeable: use `AaroneousStrict` (`cratify audit`) only for a draft targeting the Aaroneous repository; use `DevtoolsOperatingModel` (`cargo clippy --manifest-path foundation/Cargo.toml -- -D warnings`) only for a draft targeting devtools' own `foundation/`. Running `AaroneousStrict` against devtools code checks it against a stricter ruleset than devtools actually follows and produces false-positive findings, not a real compliance signal | `foundation/crates/foundation_cli/src/dogfood.rs` (M53), `governance/OPERATING_MODEL.md` |

## Ambiguity and gaps

If a change matches no row above, default to the nearest broader Aaroneous or
devtools condition (`core/`/`crates/`/`dev/` for Aaroneous, `foundation/` for
devtools) rather than treating an unmatched change as exempt. If a change matches
multiple rows with different tiers, the highest tier applies. Per
`CHANGE_VERIFICATION_TIERS.md`, unresolved ambiguity itself defaults to Tier 2.

This table is expected to grow as new conditions are identified; add a row with
its concrete source rather than a paraphrase, so the table stays traceable to the
rule it enforces instead of drifting into restated opinion.
