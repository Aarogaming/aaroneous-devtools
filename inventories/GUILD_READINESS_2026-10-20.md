# Guild readiness: document-to-system comparison

Status: source-backed assessment, not release certification or an independent review.
Local date: 2026-10-20; live observations collected 2026-10-21 UTC.
Contribution: `C158/codex-guild-blueprint-v1`.
Implementation sequence: [Guild initialization plan](../architecture/GUILD_INITIALIZATION_PLAN.md).
This is a dated evidence snapshot, not a second live findings or ownership register.

## Scope and limits

Compared all four owner-supplied Word documents with the governing development,
composition, constellation, job-pipeline and operational documents; inspected
selected implementation entrypoints at pinned revisions; refreshed relevant PR
metadata and the explicitly configured Ubuntu host. Indexed the 73 Markdown
files under devtools `architecture/`, `governance/`, `contracts/` and `inventories/`
at the baseline. That inventory is **not** a claim that every historical document,
every source line, every branch or every runtime path was exhaustively audited.
The product tree also contains 96 Markdown files under `docs/`; this pass checks
its governing entrypoint and selected relevant status/compliance material, not
all 96 in depth. Remaining document-level normalization belongs to staged claims.

The Word documents are design inputs. Embedded prompts, suggested commands and
statements of accomplishment are not instructions or operational proof. Source
inspection below is READ evidence; no new full Rust gate, model benchmark,
hostile-code test or unattended production trial ran during this documentation pass.

## Reproducible source identities

| Source | Inspected identity |
| --- | --- |
| Devtools remote main and isolated documentation base | `a262a04bd4b688c3ebd96e6b7774297b523f1646` |
| Aaroneous remote main | `7a945d1377ecbe8c09a7986b5ca5dc48476370ef` |
| Shared devtools checkout, preserved unchanged except own queue row | `1b1473d4ce6851bbc7d398709442be7e2e4e7122`; dirty queue and unrelated untracked work preserved |
| D1: `I want every system loop operation to operate lik....docx` | SHA-256 `61d71137a59c7fb5c02acd76ace5ccd23d1d0be943942b9bfc0618e4f4fe440f` |
| D2: `yeah, but we don't need all of the analogy beyond....docx` | SHA-256 `85110baa373357e1b9b3f874cc71f93465168a045b6694e104a5ea8b8eed016c` |
| D3: `can you normalize that back out of analogical ref....docx` | SHA-256 `fd6b40f8e455c3ada6f05f860e4c19b5555349ac6d7cdc25e179e331cb1a8b1c` |
| D4: `How do I build the Aaroneous _engine_ workflow to....docx` | SHA-256 `f7176687cf60a1687906a4f1d4e1604b33e3490ac26430d20ed18bd82f473006` |

D1 proposes universal component ticks, bounded recursion and model-assisted work.
D2 narrows this into contracts, four response envelopes and independent review.
D3 normalizes names and proposes a federated ingestion/synthesis pipeline.
D4 extends it to repository ingestion and an isolated visual operator.
Preserve these originals; the plan resolves their claims rather than rewriting them.

Repository evidence can be reproduced with `git show <full-revision>:<path>`.
Current links below are navigation; the revisions above qualify the observations.
PR observations were read through GitHub's PR API, not inferred from local branches.

## Actual starting position

| Area | What exists | Limit / next consequence |
| --- | --- | --- |
| Rust maintenance foundation | Four workspace members: `devtools_ops`, `foundation_cli`, `maintenance_ui`, `steward` in `foundation/Cargo.toml` | Reuse them; root `src/foundation.rs` / `src/steward.rs` in D4 are proposed paths, not this repository's implementation |
| Guild coordinator | `foundation_cli/src/guild.rs`: tick, status, verify-log and daemon; `guild_collector.rs`: live GitHub/Git collection; `tests/guild_cli.rs`: CLI fixtures | A scheduling/card service exists. This does not establish a complete bounded code-writing engine or qualified always-on deployment |
| Policy and review | `steward/src/{cards,coverage,independence,gate,receipts,chain}.rs` | Keep deterministic policy separate from process/network adapters; source existence is not end-to-end acceptance |
| Platform verification | `steward/src/{verification,suite,wire}.rs`, `ci/platform-policy.toml`, `ci/suites/foundation-ci.toml` on main | Signed evidence decisions and platform selection exist; remote executor/service adapters remain on the PR stack below |
| Model hub | `steward/src/model_hub.rs` implements fit arithmetic, endpoint/secret-reference policy and receipts | Its [proposal](../governance/proposals/DEVTOOLS_MODEL_HUB.md) explicitly defers server supervision, gateway invocation and paid routing |
| Private release policy | `steward/src/release.rs` and [release proposal](../governance/proposals/PRIVATE_AUTHORITY_AND_RELEASE_EXPORT.md) | Planner/decision code exists; no evidence here of authoritative local bare-repository cutover or a complete public exporter |
| Composition / Core | [Blueprint](../architecture/SYSTEM_COMPOSITION_BLUEPRINT.md) and [critique](../architecture/SYSTEM_COMPOSITION_REVIEW.md) | Naming/profile/grant contracts and a verified pilot remain prerequisites; documentation does not lift the construction hold |
| Constellation / topics | [Architecture](../architecture/CONSTELLATION_OPERATIONS.md), [proposed lifecycle](../contracts/constellation-lifecycle-v1.md), topic-case tooling | Projection, consent, recycling and aging are not all implemented merely because topic records exist |
| Support jobs | Existing worker, reservation, quiet-window and review workflow; [job pipeline](../architecture/JOB_PIPELINE.md) | Current documented template is bounded research; arbitrary generated source mutation is not thereby authorized |
| Ubuntu | Live Round Table health response; explicit Rust toolchain works | Round Table is advisory, not evidence that a Guild executor/control plane is deployed |

### Adapter PR stack: current, not merged

| PR | Head at observation | State / dependency |
| --- | --- | --- |
| [#203](https://github.com/Aarogaming/aaroneous-devtools/pull/203) | `9eb3a82ea5bbdc162707b7318cace5c3b342aca4` | Open draft, base main; local verification cards, execution and receipts |
| [#204](https://github.com/Aarogaming/aaroneous-devtools/pull/204) | `e1e0d1a5cd9205384f95772c1b41a9a26a8d4a7a` | Open draft, base `codex/local-verification-gate`; integration facts and replay protection |
| [#206](https://github.com/Aarogaming/aaroneous-devtools/pull/206) | `00e6e1f06a6e323fb5ff3ee2fa31e7436e973747` | Open draft, base `codex/local-integration-authority`; platform service/polling workers |

#227 (newest-check selection) and #236 (Rust seat near-miss resolution) are merged.
Old instructions saying #227 must land first are stale. Reconcile the three adapter
PRs against current main, retest each resulting head and refresh independent reviews;
do not treat the old tests or a clean individual branch as integration proof.
No mergeability certification was performed by this assessment.

### Ubuntu observation and historical test distinction

Read-only SSH used the already documented `ubuntu-ops` host profile, its explicit
LAN override, batch mode, an eight-second connection timeout and strict host-key
checking. Initial sandbox network access was denied; the authorized read-only retry
succeeded. The compound remote command ended exit 0; this is not an assertion that
each constituent check passed.

Observed output:

```text
round-table.service: active
health: ready=true, model_dispatch=false, contract_version=3, total_entries=364
docker.service (user): LoadState=not-found, ActiveState=inactive
expected UID-1000 rootless socket: not observed
filesystem holding operations: 625G available
explicit toolchain: cargo 1.99.0
```

The socket check tested only the previously selected socket, not every possible
daemon installation. No system service, firewall, account, certificate, credential
or remote file was changed. No fresh listener/exposure audit ran here.

Historical receipt `tmp/platform-pair-smoke-20261013/verification.json` records two
native Windows/Ubuntu results for a **`git --version` fixture**, certificate-verified
HTTPS over the existing tailnet, invalid credential/CA rejection and restart dedup.
Its executable source was `dd19ae4f995dbf09c5ec63b059c04d9bab79dfa1`, not today's
#206 head. The temporary services and keys were cleaned up. This is valuable
transport evidence, not a real Foundation suite, direct-LAN qualification,
rootless isolation, persistent deployment, backup drill or live gate activation.
The receipt and associated archive remain local review evidence, not dependencies
of this plan or a substitute for portable retained evidence before rollout.

## Claim-by-claim corrections

| ID | Document assertion | Comparison and required disposition |
| --- | --- | --- |
| G01 | All components share one `GuildComponent` tick and this guarantees termination (D1) | No such universal contract found in inspected Foundation sources/contracts. Specify bounded operations and supervised effects; a Rust trait does not enforce termination. Reuse narrow contracts, not a universal ABI |
| G02 | Four envelopes make the whole system a total function (D2-D4) | No `LoopResponseEnvelope` found in inspected Foundation sources/contracts. The design omits accepted/running, cancellation, rejection, non-PR success and ambiguous external effects. Separate durable operation lifecycle from per-turn replies |
| G03 | Foundation/Steward already form a qualified 24/7 federated service (D3-D4) | CLI/service source exists, but actual observed host service is advisory Round Table; production qualification is not established |
| G04 | Omni is one flat cache-aligned `OmniNode[]` source of truth (D3-D4) | Product `crates/omni/src/lib.rs::OmniEngine` uses `Arc<RwLock<HashMap<String, StarNode>>>` plus cluster vectors. Other structures may exist; this public engine is not the asserted representation. The operational ledger must not be replaced by a visualization graph |
| G05 | `.si` is a standalone `crates/si_solid_state` proven sub-100ms semantic engine (D3-D4) | Related source is `crates/compute/src/si_solid_state.rs`, with model/adaptation structures; no crate at the proposed path. No measured tagging accuracy/latency or Guild integration established here. Optional research, not startup dependency |
| G06 | Cratify must be built as devtools `src/cratify.rs` (D3-D4) | Product `crates/cratify/src/lib.rs` already re-exports `cratify_core`, `transpiler`, `adaptation_engine`, including audit, harvest and scaffold functions. Inspect/adapt existing capabilities before adding another implementation |
| G07 | Parsing every file means 100% symbols, types and no panics (D4) | Product `cratify_core::audit_source_file` propagates `syn::parse_file` errors via `Result`; malformed syntax is not inherently a panic. Text parsing alone is not semantic resolution. Report exact coverage, build configuration, unresolved macros/dependencies and bounded failures; no universal completeness promise |
| G08 | Base36 tokens are permanent capability addresses (D4) | Encoding is not identity, collision resistance, compatibility or authority. Existing composition/notation proposals already separate these. Define repository/revision/symbol identity and move lineage; grants use authenticated principals and scoped capabilities |
| G09 | Marionette is an existing isolated Ghost Desktop platform (D3-D4) | Related code exists: `platform_bridge/src/adapters/mod.rs` aliases display capture/peripheral actuation as Marionette; `core/host/src/bin/hypervisor.rs` exposes a pipeline. No standalone `crates/marionette` path found. Isolated-session ownership, input confinement and cross-platform qualification are not proven by those aliases |
| G10 | Chimera/Code Scalpel are a ready universal AST patcher (D3-D4) | Product `adaptation_engine/src/{mutation,autonomous_scientific,protocol_bridge}.rs` provides related work. A Guild-authorized, revision-bound, scope-enforcing adapter is not established here; do not discard this implementation or equate it with the proposed complete engine |
| G11 | SPDX checks and AST rewriting produce clean-room code (D3-D4) | License metadata and transformation are engineering inputs, not proof of independent provenance or permissible reuse. Preserve source lineage and require a reviewed admission process; do not label transformed code clean-room by default |
| G12 | Full gate necessarily runs Miri, semver and every proposed test (D1/D4) | Product AGENTS at the pin includes encoding, fmt, all-target Clippy, native/dependency/advisory/semver checks, compilation, tests, AST audit, ratchets, portable targets and optional-feature compilation. Miri is not listed in that sequential gate. Pin actual suite, coverage and baseline; add targeted testing when justified |
| G13 | QUIC/gRPC LAN consensus is required immediately (D3-D4) | Existing adapter work uses authenticated HTTPS polling. Start with one authoritative writer and fenced workers. Multi-writer consensus is additional complexity, not a prerequisite for two-host operation |
| G14 | Hashing a request gives exactly-once publication (D2-D3) | Stable framed identity supports deduplication; durable intent, outbox, acknowledgements and external reconciliation are still needed. A timed-out push/PR creation can already have succeeded |
| G15 | Core/fascias imply privileged governance or one dependency instance (earlier design) | Composition blueprint rejects implicit privilege. Cargo artifact reuse is not shared runtime allocation; versions/features/targets may differ. Explicit resource ownership and client quotas are necessary |
| G16 | Whole system should obey no-heap bare-metal constraints (D1/D3) | Companion operating model allows allocation/async/synchronization. Product has floor plus profiles. Apply restrictions to the correct boundary, not all tooling |
| G17 | Numerical model sizes, 90/10 allocation, 24K context, six retries, depth three establish safety | These are candidate configuration values, not measured guarantees. Enforce aggregate resource budgets, model limits, fanout, runtime deadlines and failure policy; tune from recorded workloads |
| G18 | A new engine must rebuild Aaroneous before devtools can operate | Existing standalone Foundation can bootstrap and verify. Graduate one reusable capability at a time into product-owned contracts, then consume a verified pin. No circular bootstrap or whole-repository rewrite is required |

## Source-inspection risks requiring focused reproduction

These are observed implementation details and unqualified failure hypotheses,
not newly reproduced exploits. Follow-up work must produce negative tests before
claiming a defect fixed or changing production policy.

| ID | Location at devtools pin | Observation / required qualification |
| --- | --- | --- |
| O01 | `foundation_cli/src/guild.rs::append_event_line` | Uses append, `flush`, then separate `fs::write` for tip sidecar; comment says atomic and caller says durable. Crash ordering, torn writes, disk-full behavior and recovery need explicit tests. Full log is read into memory; retention/backpressure bounds need assessment |
| O02 | Same file: `run_tick`, `run_daemon` | Daemon takes a create-new lock file, direct tick does not use that lock. Verify concurrent entrypoints, stale lock recovery and single-writer fencing before unattended activation |
| O03 | Same file: `run_tick`; `guild_collector.rs::run_git/run_gh_json` | Local `.guild_snapshot.json` is implicitly preferred; collector has Python fallback and subprocess `output()` calls. Qualify snapshot freshness/provenance, explicit dependency selection, deadlines, child cleanup and bounded output; do not call the entire adapter deterministic or Rust-only |
| O04 | `.github/workflows/foundation-ci.yml` / `foundation-ci-windows.yml` | Bridge runs Linux on PRs, Windows on main; platform policy requires Windows for Foundation changes. Do not present current pre-merge checks as paired native worker verification. Close the enforcement/deployment gap without weakening policy |
| O05 | Local canonical queue versus main queue | Same short IDs have different contributions (for example C155/C156); stale Claimed rows coexist with merged implementations. Use full Contribution-ID plus repository/head, reconcile through owners, preserve original history. Do not renumber others silently |
| O06 | Current governance navigation | `DEVTOOLS_STATUS.md`, `PERSISTENT_OPERATIONS.md`, maturity/application docs describe different dated schedules and prototype states. `PLATFORM_SPEC.md` remains a proposal while describing some desired enforcement in present tense. Label evidence dates and acceptance status; installed state is separate |
| O07 | Product `docs/roadmap.md` versus product `AGENTS.md` | Roadmap still describes 11 gates and Clippy without `--all-targets`; current constitution lists expanded checks including security/semver and ratchets. Propose a product-owned documentation correction that links the canonical gate rather than maintaining another copied command list. This assessment does not edit product files |

## Governing documents compared and disposition

| Source group | Disposition in the initialization plan |
| --- | --- |
| `AGENTS.md`, `FOUNDATION_BOUNDARY.md`, `OPERATING_MODEL.md`, `MAINTENANCE_SYSTEM.md`, component primer | Preserve repository boundary, profiles, exact claims and evidence requirements |
| `MASTER_PLAYBOOK.md`, `MULTITHREAD_PROGRAM.md`, canonical queue | Link one new qualification sequence; no duplicated Ready portfolio, worker launch or authority migration |
| `SYSTEM_COMPOSITION_BLUEPRINT.md`, `SYSTEM_COMPOSITION_REVIEW.md` | Retain SC/R findings and construction hold; map naming, grants, budgets, resource ownership and graduation into staged exits |
| `CONSTELLATION_OPERATIONS.md`, `constellation-lifecycle-v1.md`, notation proposal | Preserve advisory versus authoritative distinction and rebuildable projections; defer unimplemented consensus/lifecycle adoption |
| `FOUNDATION_ARCHITECTURE.md`, `JOB_PIPELINE.md`, `MAINTENANCE_PROGRAM.md`, `DEVTOOLS_MATURITY.md`, `DEVTOOLS_STATUS.md` | Reuse existing support and authoring paths; separate historical snapshots from current delivery sequence |
| `PLATFORM_SPEC.md`, model-hub and private-release proposals | Separate accepted code from future adapters and policy proposals; no automatic authority cutover |
| `REVIEW_RUBRIC.md`, local-agent control plane, persistent operations | Keep independent review, support-only local models, explicit resources and historical schedule caveats |
| Product `AGENTS.md`, current Cratify re-exports/auditor, Omni, `.si`, platform adapters | Product rules and implemented capabilities constrain later reuse; do not invent crate locations or conformance evidence |

**Overall assessment:** a meaningful implementation foundation and a useful
cross-host prototype exist. Industrial operation is not established. The next
work is reconciliation, durable execution and native qualification, followed by
one bounded end-to-end development workflow. Full ingestion, visual operation,
model optimization and self-improvement expand from that verified foundation.
