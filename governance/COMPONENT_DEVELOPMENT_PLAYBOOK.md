# Component Development Playbook

Use devtools as the external workshop for Aaroneous. It prepares contributors, gathers evidence,
and runs reproducible analysis without becoming a framework dependency or bypassing product
verification.

## Development loop

```text
framework primer -> component profile -> bounded change -> product verification -> evidence record -> reusable guard
```

1. **Orient:** read root `AGENTS.md`, the component-framework primer, Aaroneous `AGENTS.md`, and
   the component's public contract.
2. **Profile:** run read-only devtools commands against an explicit checkout. Produce a versioned
   report on dependencies, API shape, hot-path rule coverage, test topology, configuration reach,
   and existing measurements.
3. **Plan:** turn one confirmed concern into a small worklist item with paths, invariant,
   acceptance evidence, rollback, and a named owner.
4. **Change:** claim the exact product files, make the smallest viable Rust change, and keep any
   generated draft or model output in devtools scratch space.
5. **Verify:** run Aaroneous's native gates. Devtools can summarize their results but cannot
   substitute for them.
6. **Learn:** preserve a report, benchmark, fixture, or static check that makes the same defect
   harder to reintroduce.

## Component profile card

Every mature component should eventually have an evidence-backed profile containing:

| Area | Questions |
| --- | --- |
| Role and boundary | Which narrow responsibility and traits does it own? Which crates may depend on it? |
| State and authority | What state transition occurs? Which dependencies, paths, buffers, and handles are injected? |
| Hot-path geometry | Which functions must avoid allocation, locks, I/O, and ambient reads? Which fixed bounds apply? |
| Wire and ABI | Which messages cross process, thread, FFI, or persistence boundaries? Are their layouts checked? |
| Behavior | Which fixtures establish determinism, recovery, rejection, and compatibility behavior? |
| Cost | What allocation, latency, throughput, memory, or binary-size evidence exists? What is only a target? |
| Supply chain | Which dependencies carry risk, are stale, or are broader than the component needs? |
| Retirement | Which compatibility aliases, configs, WIT interfaces, or adapters remain, and what evidence supports them? |

## First tooling sequence

Build these as standalone Rust commands in devtools, each consuming explicit paths and producing a
versioned report:

1. **Component inventory:** crate dependencies, public symbols, configuration files, generated
   interfaces, and test coverage map.
2. **Invariant profile:** the component's declared Cratify compliance profile (`kernel` / `control` /
   `presentation` / `tooling`) plus static hot-path, ambient-authority, memory-geometry, unsafe, and
   compatibility findings against that profile, reusing Aaroneous's native audit semantics where
   possible. For a graduation candidate, also report its compliance-distance scores and target
   profile (`Aaroneous/docs/CRATIFY_SPEC.md` sections 2, 5, 6).
3. **Measurement harness:** reproducible benchmark metadata and result capture; it reports actual
   measurements and never treats design targets as results.
4. **Change impact report:** compares a selected revision or branch, mapping modified product files
   to component profiles, tests, contracts, and required gates.
5. **Release evidence bundle:** composes native CI, audit, dependency, benchmark, and migration
   evidence for a human review.

Do not build a second compiler, a second product CI, or an autonomous patching system. Prefer
calling product-owned checks and storing their explicit inputs/results.

## Onboarding use

An incoming contributor starts with the primer and this playbook, then receives one component
profile card and one bounded queue claim. This gives enough theory to reason about the framework
without burying the contributor in every historical design document.

## Local model use

Qwen can summarize a declared component profile, identify missing evidence, or draft a scratch
test plan. It cannot discover undisclosed files, make a claim, change a component, or judge the
verification gate. Follow `governance/LOCAL_AGENT_CONTROL_PLANE.md`.
