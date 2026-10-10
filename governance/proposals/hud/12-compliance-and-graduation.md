---
notation: v1
purpose: Build devtools modularly and Aaroneous-compliant by construction, with a thin stable base and a thick frontier, so proven components can graduate into Aaroneous
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:01-settings-and-permissions
  - relates-to: doc:AGENTS.md (product repository, sections 0, 1, 4, 5, 6)
  - relates-to: doc:docs/CRATIFY_SPEC.md (product repository)
checked-by: partial: the product's AST auditor was run on the devtools crates (see "Measured today"); nothing else
---

# Compliance with Aaroneous, a thin base and a thick frontier (proposal 12)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 12 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## The request

Owner direction in chat, 2026-10-10: build modularly; use Aaroneous as the base and as inspiration; stay as compliant with
Aaroneous as possible, to make it easy to fold devtools into Aaroneous later and to make devtools thinner at the base while
staying thick at the frontier of operations. The toolkit expands from forward progress.

## Principle: three rings

| Ring | What lives there | Rules | Where it goes |
|---|---|---|---|
| **Frontier** | experiments, adapters, integrations, model-specific prompts, UI, scripts, prototypes | may be rough and change daily, but always behind a workspace trait, always within the universal floor (no ambient authority in library code, no stubs, no panics on runtime input) | proven pieces move inward; unproven ones are deleted |
| **Base** | small, stable, mostly pure components with contract tests or golden vectors: the settings and permission engine, the election and handoff reducer, the `can_take` gate, the validators, the steward port | the full compliance profile for its role (below); every change reviewed | candidates to graduate |
| **Graduated** | components now living in Aaroneous | owned by Aaroneous's gate; devtools depends on them by a pinned revision and **deletes its own copy** | the permanent library |

Direction of flow: frontier to base to Aaroneous. Direction of dependency: devtools depends on Aaroneous, never the reverse
(separation of church and state). The base stays thin because anything that proves out leaves for Aaroneous; the frontier
stays thick because that is where new operations are tried. The library grows by what survives the frontier.

## Compliance by construction

Checklist for every new devtools crate, taken from the product `AGENTS.md` and `docs/CRATIFY_SPEC.md`:

| Rule | What it means for a devtools crate |
|---|---|
| Declare one profile in the manifest (`[package.metadata.cratify] profile = ...`) | `control` for reducers, engines and validators with I/O in adapters; `presentation` for the native application; `tooling` for scripts and auditors. `kernel` is not used by devtools |
| Dependency direction by profile | a crate depends only on crates of the same or a stricter profile (`kernel` > `control` > `presentation`/`tooling`) |
| Zero ambient authority | no `std::env::var`, no `current_dir`, no `.canonicalize()`, no ambient clock, no self-started threads in library code; everything arrives through constructors and typed configs; adapters and entry points may do I/O |
| No panics on runtime input | no `unwrap`, `expect`, `panic!` or `assert!` on values from I/O, config or model output, except tests, entry points and a documented `// INFALLIBLE:` site |
| No stubs; no unsafe `Pod`/`Zeroable`; no unchecked transmutes | as written in the product rules |
| Naming | generic systems names, no `aaroneous_` prefix on crates, types, modules or channels; documented exceptions only |
| Fault tolerance | a violated precondition is an operating condition: primary, degraded and safe-hold paths, with separate trip and recovery thresholds |
| Dependency admission | any new third-party crate is scored on `alloc`, `ambient`, `abi` and `safety` and the verdict is recorded; `control` crates need `ambient = 0` and `safety <= 1` after conditions |
| Ratchet | counts of violations may only fall; a fixed defect class becomes a type, a rule or a test |

The rules marked for hot paths and memory geometry apply to `kernel` crates only and are not imposed on devtools.

## Use the toolbelt on ourselves

The product's operating principle is to use the components while building them. So:

- run the product's auditors on devtools code (measured below);
- keep a **component inventory** for the library, using the inventory tool and schema that already exist in devtools
  (`component_inventory.rs`, `contracts/component-inventory-v1.schema.json`), adding the ring, the profile, the owner,
  the tests, and the graduation status for each component;
- apply the ratchet: a baseline of known findings per crate that may only shrink, advisory in CI first.

## Measured today

Run on 2026-10-10: the product's `ast_auditor` (version 0.3.2, built from a shallow clone of the product repository at commit
`09ed595b` on `main`, in a scratch directory; the product's working checkout was not touched) with `ast_auditor audit <crate path>`
on the devtools crates as they stand on `origin/main` plus the open C120 branch (which adds `hud_core`).

| Crate | Files scanned | Violations | Detail |
|---|---:|---:|---|
| `hud_core` (new, lane C120) | 8 | 0 | passes |
| `maintenance_ui` | 4 | 0 | passes |
| `steward` | 17 | 2 | two ambient-authority findings, `std::env::var` in `tests/differential.rs` (test code) |
| `foundation_cli` | 26 | 3 | two `.canonicalize()` calls in `src/virtual_patch.rs`; one prefix-stutter finding for the string `"aaroneous-attestation"` in `src/guild_collector.rs` |

Reading it: the auditor covers the syntax rules it implements (ambient authority, prefix stutter, soundness, hot-path
allocation, memory geometry, safety comments); passing it is **not** a full compliance proof, and the product's own gate
also covers formatting, clippy, the unwrap ratchet and the profile dependency check, none of which I ran against devtools.
The prefix-stutter finding is the namespace string of the signed attestations; changing it would invalidate existing
signatures, so it should be a recorded exception rather than a rename. Neither tool run changed any file.

`hud_core` now declares `profile = "control"` in its manifest (lane C120), as the next step the findings above call for.

## Graduation

The product's rule for code arriving from outside (section 4 of its `AGENTS.md`), applied to a devtools component:

1. **Proven in its origin:** real use, with retained evidence, in devtools.
2. **Classified:** a new component, or an upgrade of an existing Aaroneous one, never a second implementation. Check the
   product's crates first (for example `governance`, `capabilities`, `orchestrator`, `paths`, `llm_gateway`) for the home.
3. **Behind a workspace trait with contract tests.**
4. **Landed inert** (a feature flag) **or in shadow mode.**
5. **Origin copy deleted** once the Aaroneous version is live, and devtools depends on the pinned revision.

Candidates, in my order of readiness, with no claim that the product wants them: the steward port (its golden vectors are
already the contract, and the owner approved the Guild name earlier); the settings and permission engine; the election and
handoff reducer; the `can_take` gate and the validators. Which product crate would host each one is an open question for the
product owner and is not decided here.

## Modular layout

Small crates with one trait each, named generically, in dependency order:

| Crate (proposed) | Profile | Role |
|---|---|---|
| `hud_core` | control | settings, permissions (exists, lane C120) |
| `hall_handoff` | control | election and handoff reducer (see [11](11-hall-host-election-and-handoff.md)) |
| `hall_gate` | control | `can_take`, ratings, qualification (see [08](08-roles-skills-and-trust.md)) |
| `worker_validators` | control | the checks that grade worker output (see [07](07-guild-workers-staff-and-inspection.md)) |
| adapters (MCP, loopback HTTP, model backends, log store, signer) | control or tooling | the I/O edges; the frontier lives here |
| `hud_native` | presentation | the native application (see [10](10-native-app-operations-and-reuse.md)) |

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| C1 | profile metadata on every devtools crate; baseline of the auditor findings recorded per crate | 01 |
| C2 | ring, profile and graduation status added to the component inventory | C1 |
| C3 | the auditor run as an advisory CI check in devtools, with the ratchet baseline | C1 |
| C4 | choose the first graduation candidate and its Aaroneous home with the product owner | C2 |

## Decisions for this feature

1. OPEN: confirm the three rings and their rules.
2. OPEN: whether devtools CI may depend on the product's auditors (allowed in direction; adds a build dependency on a pinned product revision).
3. OPEN: which component graduates first and into which crate.
4. OPEN: whether `"aaroneous-attestation"` is recorded as a permanent exception.

## Review focus

- Are the rings and the flow (frontier, base, Aaroneous) the right structure for "thin base, thick frontier"?
- Is the checklist a faithful reading of the product rules, and is it right to leave hot-path and memory-geometry rules to `kernel` crates?
- Is the measured table honest about what the auditor does and does not prove?
- Is running the product's tools on devtools in CI acceptable with respect to the dependency direction?

## Not verified

- I ran only `ast_auditor`. I did not run `cratify`, `compliance_auditor`, the unwrap ratchet or the profile dependency check against devtools.
- The scan covered the crate directories as checked out on `origin/main` plus the C120 branch; findings elsewhere may differ.
- Whether any devtools component is welcome in Aaroneous, and which crate would host it, is for the product owner.
- That the existing component inventory tool can carry the ring and graduation fields without change.
- That the `control` profile is the right one for every crate listed in the layout table.
