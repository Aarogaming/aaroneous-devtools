# Aaroneous Rust Component Framework Primer

Use this primer before selecting product work. The authoritative rules remain Aaroneous `AGENTS.md` and `docs/ARCHITECTURE.md`; this is the concise context for incoming agents.

## Identity and scan model

Aaroneous is a Rust component framework for interchangeable execution blocks. It is not a monolithic application or a general automation service. Components have narrow traits, typed configuration, explicit binary boundaries, and independently testable behavior.

The core execution model is a deterministic scan:

```text
acquire explicit input -> reduce state -> emit bounded output
                         S(next) = reduce(S(current), input)
```

Acquisition may interact with I/O. Reduction is pure: no filesystem, network, clock, environment, or hidden task launch, and in `kernel`-profile crates no allocation. Output performs telemetry or actuation through declared, bounded interfaces.

## Design laws

| Law | Consequence |
| --- | --- |
| Explicit authority | Pass paths, buffers, handles, endpoints, and configuration through constructors or tick inputs. Do not perform ambient reads in components. |
| Deterministic reduction | Keep state transitions reproducible from supplied state and input. Put side effects in acquisition/output phases. |
| Compliance profiles | Every crate declares `kernel`, `control`, `presentation`, or `tooling` in `[package.metadata.cratify]`. A universal floor (no ambient env/clock reads, no self-spawned threads, no panics on runtime input) applies to all; the profile adds the rest (`Aaroneous/docs/CRATIFY_SPEC.md`). |
| Bounded memory | `kernel`-profile hot paths (`#[hot_path]`) use preallocated buffers and bounded structures. Avoid heap allocation, locks, and unbounded collections there. `control` crates may allocate off the scan loop. |
| Fault tolerance | A violated runtime precondition takes a degraded path or safe-hold, not a panic. Mode changes use deadband thresholds. |
| Admission and graduation | New dependencies are scored on compliance distance and admitted, conditioned, pattern-extracted, or rejected. Outside code graduates behind an Aaroneous trait, inert or in shadow mode, with its origin copy deleted. |
| Ratchet | Rules only tighten; fixed defect classes become types, assertions, or `ast_auditor` rules. |
| Stable boundaries | IPC and external binary messages use deliberate geometry: `repr(C)`, derive-based `Pod`/`Zeroable`, and explicit padding where needed. |
| Component isolation | A crate owns one role behind narrow traits. Do not create global services or hidden dependencies inside subcomponents. |
| Evidence before retirement | Remove interfaces, config, WIT, or compatibility shims only after runtime, package, consumer, and migration evidence proves they are unused or superseded. |

## Topology

The hypervisor hosts the execution loop. `orchestrator` schedules work, `ipc_bus` provides bounded transport, `compute` owns performance-sensitive engines, `platform_bridge` isolates OS adapters, `capabilities` exposes domain tools, `governance` enforces safety policy, and `api` / `studio_hud` provide presentation. Preserve the distinction between host policy, transport, domain reduction, and presentation.

## Verification and devtools

Compilation does not prove component conformance. Product work follows the complete verification sequence in `AGENTS.md`, including workspace compilation/tests, AST audit, forbidden-pattern inspection, emulator harness, and the full agent-check gate.

Devtools supports planning and evidence collection. It must never become a product runtime, build, CI, or crate dependency. Local models may analyze explicit evidence or draft a scratch proposal, but cannot prove a component safe or alter its framework obligations.
