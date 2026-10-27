# Guild initialization and industrial qualification plan

Status: proposed execution blueprint for independent review, 2026-10-20 local
date (evidence captured 2026-10-21 UTC). Contribution `C158/codex-guild-blueprint-v1`.
No runtime, product rule, service installation, merge authority or autonomous
write permission is adopted by this document.

Evidence: [document/source/deployment comparison](../inventories/GUILD_READINESS_2026-10-20.md).
Existing direction: [constellation operations](CONSTELLATION_OPERATIONS.md),
[composition blueprint](SYSTEM_COMPOSITION_BLUEPRINT.md),
[job pipeline](JOB_PIPELINE.md), [companion operating model](../governance/OPERATING_MODEL.md).
The [queue](../governance/COORDINATION_QUEUE.md) alone records live claims.

## 1. Decision in plain terms

### Owner priority and source reconciliation 2026-10-27

The owner directs this work ahead of other planned expansion, followed by
Devtools alignment on the current Aaroneous operational base. This section
updates sequencing; the existing stages and contract decisions remain the
qualification authority. No model-authored implementation sketch is adopted
merely because it appears in the supplied documents.

#### Source provenance and fresh baseline

Three owner-supplied responses were compared with current source. Their original
attachment IDs and SHA-256 digests identify the exact inputs without copying
private operational details into the repository:

| Source | Attachment ID | SHA-256 | Evidence limit |
| --- | --- | --- | --- |
| Access & Evidence Declaration | c55d0aab-c3fd-4fc9-ba0a-8e972db7ffd6 | b9cfba03caeaa6c3d0cdbc5453bb2407c362357401abb5fd2aa9ade48ecafaa4 | Design only; declared no repository access |
| Evidence & Inspection Status Update | 32f8c9a9-52e6-424d-89bf-1b8dd5bd88ca | 4ea999b851431a6746e956f693cea938e3fadd7e981ee5673e1d4a96dbeb16ca | Archive path inventory; explicitly did not inspect source bodies |
| Dossier critique | 2e006afe-0f6b-4955-b06a-35b7d1883414 | d1534bfa375b388459eb83d78c763f138e552dd98d5835bb849fa866b1f76ce4 | Proposed corrections; runtime and PR assertions need independent refresh |

GitHub branch/PR reads on 2026-10-27 UTC confirm product main
`175e93966bec40b119048143054ad8a38b34e861` and Devtools main
`f553f1f25a7e795ce796840e9669bf198e9d626b`. PR242, PR243 and PR246
are merged. PR243 merge `c4c7fd95866e4330c937c22719c71728bf243048`
does not convert its explicitly proposed contract into implemented behavior.
The older baseline in this plan remains dated evidence, not current PR status.

READ evidence at the Devtools pin: `foundation_cli/src/guild.rs` exposes Tick,
VerifyLog, Status and Daemon. The operation contract still says PROPOSED, NOT
IMPLEMENTED, NOT ADOPTED. Its 26 cases and 160 phase/event cells remain specified
qualification targets, not a test result. Paths use prefix `foundation/crates/`.
READ evidence in the clean product files at the product pin: `compute`'s
`automata.rs` uses Vec/HashMap/HashSet; `si_macro.rs` performs filesystem work and
uses Instant; `compute/Cargo.toml` includes substantial compute/JIT dependencies.
These are reuse candidates requiring boundary qualification, not a ready-made
bounded, allocation-free reducer. Product paths use prefix `crates/`.
No Rust tests, host health checks or performance measurements were run for this
documentation increment.

#### Disposition of the supplied recommendations

| Recommendation | Disposition and necessary correction |
| --- | --- |
| Shared operating spine, specialized engines | Retain: shared intent/evidence/review relationships, with domain-specific execution and recovery. No universal runtime or AST. |
| Reuse Guild, Steward, Cratify and existing automata | Retain as the first inspection route. Existence is not behavioral qualification; neither a new guild_core nor foundation_macro crate is justified yet. |
| Four independent status dimensions | Preserve the existing operation contract, including per-effect records and review references. The supplied single effect/status sketch loses multi-effect history; its nine-state enum also fails to represent the advertised ten-state matrix. |
| Core and named facades | Core remains a minimal contract candidate with no implicit authority. Reuse core-contracts/wire where appropriate; do not introduce the prohibited aaroneous_core prefix or promote UI-specific ASTs into shared contracts. |
| Fuel-bounded synthesis | Qualify node, edge, loop, call-depth, input, output and aggregate descendant budgets. Flat storage alone proves neither Copy, panic freedom nor termination. Validate cycles by graph semantics, not numeric index direction. |
| Single writer and lease recovery | Retain the goal; do not equate scheduler seat rotation with effect fencing. Expiry alone cannot prove an old worker stopped. SQLite/WAL and epoch fields alone do not establish cross-host fencing. |
| Eliminate duplicate orchestrators | Investigate callers, re-exports, features, contracts and consumers first. Similar filenames do not authorize deletion or moving product policy into Devtools. |
| Model critic as verifier | Reject. Model output is a proposal; deterministic checks and independently authorized acceptance remain separate. A Rust signature, AST lint, cargo --offline or a Git worktree is not a security sandbox. |
| Simplified report-only recovery | Narrow: builds can execute build scripts/procedural macros and write caches or artifacts. Explicit confinement, output scope and surviving-child reconciliation are still required. |
| Rollback by deleting ledger/sidecar state | Reject for maintained operation evidence. Preserve journals, associations and recovery history; disable execution and restore a qualified version through an explicit migration. |
| Desktop zero-leak guarantees | Defer until isolated-target qualification. Focus checks have races, timestamps need host-local semantics, and cleanup inputs are effects too. No hard timing or no-leak promise follows from filenames. |
| C150 and old PR merge order | Do not reuse C150: it identifies an existing attribution repair. Refresh each PR and claim before scheduling; the dossier is not merge authority. |

One supplied source includes a plaintext credential. Do not reproduce it in
prompts, commits or handoffs. Its validity and containment status are unknown;
the responsible service operator must assess revocation/rotation without
distributing the literal. No credential or service was changed here.

#### Ordered execution and exit criteria

1. **Reconcile and qualify the operating contract first.** Resolve section 8
   of [guild-operation-v1](../contracts/guild-operation-v1.md) with independent
   review; map each proposed transition and case to current Guild/Steward code
   or an explicit missing capability. Preserve fixed-field legacy wire records.
   Exit: accepted decisions, exact implementation scope, reproducible fixtures,
   and no claims that scheduler ticks implement operation admission/cancellation.
2. **Qualify one bounded execution component.** Trace existing automata/si_ir
   callers and loading/evaluation boundaries. Select a narrow existing home;
   add malformed-index, cycle, fuel exhaustion, output-capacity and cancellation
   counterexamples under product-native gates. Keep domain storage and I/O out
   of the reducer. Exit: independently reviewed contract and measured bounded
   behavior; a new crate requires evidence that an existing home cannot serve it.
3. **Run one report-only operation end to end.** Reuse existing local verification
   and job-service paths. Bind intent, attempt, exact subject, evidence and
   independent judgment; test restart, corrupt state, late results, revocation
   and unchanged aggregate budgets. Exit: retained success and negative-case
   evidence without repository publication or desktop actuation.
4. **Align Devtools on qualified Aaroneous capabilities.** For each selected
   component record product commit/package/API, enabled features, transitive
   dependencies, license/admission result, target support and update policy.
   Prefer existing explicit CLI boundaries where a stable library boundary is
   not available. Do not import compute or a host wholesale just to reuse one
   function. Product fixes land and qualify independently before the consumer
   pin advances; migrate one consumer at a time, preserving rollback evidence.
5. **Qualify Ubuntu-primary and Windows operation.** Verify the same contracts
   on both platforms with explicit endpoints, credentials, storage and output
   roots. Exercise disconnect, duplicate delivery, stale writer, crash recovery
   and operator stop. Exit: Ubuntu coordinates and Windows remains a supported
   worker/client, with no automatic authority promotion after lost contact.

For step 4, inspect Cargo metadata/tree for normal, build, dev and target-specific
edges, including feature activation and duplicate versions. Verify product build,
tests and release without a Devtools checkout or service. Check workflow scripts,
generated artifacts and runtime calls too: an acyclic Cargo graph alone is not
the complete boundary. Devtools may depend on qualified Aaroneous components;
Aaroneous must not acquire a dependency on Devtools. Shared package versions do
not create shared runtime resources: composition roots explicitly own and inject
handles. Git rebases, if needed, stay within each repository's own history.

The next source-work packet is step 1's mapping and decision closure, followed
by the smallest approved implementation. These steps are proposed work, not new
Ready claims. Confirm reviewer capacity and current ownership first. Route any
new verified gaps through the existing Foundry gap register, not this source
comparison. No fleet service, model dispatch, dependency or live effect is enabled
by this priority update.

### Execution work packets and acceptance gates - 2026-10-27

This continuation turns the owner priority above into sequenced work packets;
it does not grant claims, settle the Guild proposal, or authorize deployment.
The [queue](../governance/COORDINATION_QUEUE.md) remains the source of live
ownership and the [gap register](../governance/FOUNDRY_GAP_REGISTER.md) routes
verified gaps. All packet owners refresh both before editing. Product main was
`1f8c4ed59ee580c7e5ac94df6ceed96029e8818f` and devtools main was
`400e304279b9aefcae7461cee3c2129057b60d31` at this comparison; these are
dated evidence, not moving branch identifiers.

| Order | Work packet and existing lane | Required exit evidence |
| --- | --- | --- |
| 0 | Reconcile current main, worktrees, claims and reviews; preserve dirty work and qualify every lane ID. | Pinned heads, active ownership, scoped handoffs and reviewer capacity; never infer adoption from a merged documentation PR. |
| 1 | Independently decide Guild contract D01-D05 and its four counterexamples. Compare existing Steward codec and authority boundaries before choosing formats. | Per-decision disposition at the exact contract revision; signer/trust anchor, review authority and unresolved fields recorded. Generic PR acceptance is not a schema decision. |
| 2 | Complete product trust and truth prerequisites: C79 capability manifest, C80 caller-bound MCP authority and C85 cost-layered verification. | C79 binds explicit root, revision, capabilities and gates. C80 rejects absent, expired or over-broad authority. C85 preserves the full final native gate. Each requires its own claim, native verification and independent review; serialize overlapping `xtask` paths. |
| 3 | After contract decisions, claim language-neutral fixtures and a pure operation reducer. Evaluate an existing bounded product component for one concrete need; specify Base36 only for its actual record. | Matrix and guard coverage, malformed inputs, concurrent allocation, cancellation, late results and revocation. Product component work passes native gates. Do not presume `scan_core` makes arbitrary reducers pure or bounded. |
| 4 | Replay retained verification evidence in shadow mode, then separately authorize one report-only pilot with explicit identity, subject, suite and resource budgets. | Retained successful and negative cases, independent report review, crash/restart and stale-subject handling. Existing receipt formats remain intact. Report-only does not imply sandboxed execution. |
| 5 | Integrate one qualified product capability into a concrete Devtools consumer. | Pin product revision and interface; inspect feature, license, target and transitive normal/build/dev dependencies. Verify product build, test and release without Devtools checkout or service; check scripts and runtime edges as well as Cargo. Record compatibility and rollback. |
| 6 | Qualify Ubuntu-primary coordination and Windows worker/client behavior. | Both native platforms pass relevant tests; demonstrate authenticated handoff, stop, restart, partition, duplicate delivery, stale writer and unknown-effect reconciliation. Expired leases do not prove old workers stopped. Installation or authority cutover remains separately authorized. |
| 7 | Expand to bounded patches, model proposals and isolated desktop actuation only after their prerequisites. | Explicit scopes, independent evidence, reversible rollout, measured cancellation and applicable effect-boundary checks. No model output grants itself authority. |

At the comparison above, the remote queue records C79 claimed by Gemini for a
revision-bound capability manifest. Check its current ownership and handoff
before touching its product paths; this document does not assign another worker.
C80 addresses the product MCP service's self-issued broad dispatch token, so an
external controller must not assume that endpoint already provides caller-bound
least privilege. C79's manifest is information, not an authorization grant.

For each packet, retain the exact subject revision, actual commands and exit
codes, logs or reproducible worktree, reviewer, unresolved limits and rollback.
Product and Foundation gates are distinct. Windows-only tests, filenames,
signature shapes or a green documentation check cannot establish industrial
readiness or cross-host safety. Route verified new gaps through the existing
register and claim each bounded change separately; do not duplicate C79-C85.

### Prepared downstream PR packets - 2026-10-27

Owner direction: prepare the full delivery chain now rather than serially
rediscovering each step. These are **proposed packets, not Ready claims**.
An owner may prepare design and fixtures in isolated scratch immediately; a
maintained edit or draft PR needs the queue's exact Ready claim, available
review capacity, current source and applicable native pre-edit gate. Preserve
existing C79-C89 identities. Record unknowns instead of inventing interfaces.

| Packet / proposed PR boundary | Preload now | Dependency and merge exit |
| --- | --- | --- |
| P01 C79 capability export (product; existing Gemini claim) | Review checklist: root/revision mismatch, deterministic ordering and digest, unknown gate, schema version, path bounds, no Devtools edge. | Gemini's existing lane and product gate; independent exact-head review. Do not edit claimed `xtask` paths. |
| P02 C80 caller authority (product) | Map HTTP, direct and batch/notification callers; draft scope and principal/revocation tests. `mcp_service/auth.rs` token verification checks key/expiry but its use as an authorization context needs caller and scope binding review. | Existing C80 Ready lane after queue/capacity reconciliation; product native gates and independent review. Preserve local compatibility through an explicitly authorized adapter, not a wildcard fallback. |
| P03 C81 explicit inputs (product) | Identify every strict-versus-bootstrap constructor and prepare missing root, endpoint, model and path negative cases. | Existing C81 claim; no ambient fallback in the strict path; native gates, migration notes and review. |
| P04 C85 verification tiers (product) | Design stop-on-first-failure table, typed result and fixture cases; map each tier to the existing gate. | Coordinate C79 ownership of `xtask`; no final-gate weakening; product gate and CI-parity evidence. |
| P05 Guild decisions (devtools, review only) | Request separate exact-revision D01-D05 and four counterexample dispositions; prepare signer/anchor, codec alternatives and report-reviewer questions. | Independent family and policy decisions as required in [Guild contract section 8](../contracts/guild-operation-v1.md#8-review-decisions-and-adversarial-critique). No self-acceptance or schema adoption. |
| P06 Guild semantic fixtures (devtools) | Draft adversarial examples for OCV01-26 and all 160 phase/event pairs in scratch; distinguish guard cases from adapter faults. | P05 accepted; separately claim existing contract/fixture homes. Review explicit version, byte limits and golden vectors before parser/signature implementation. |
| P07 Guild reducer (devtools) | Sketch pure state-transition responsibilities and test inputs, retaining separate attempt, review and per-effect facts. | P06 accepted; one unfinished attempt, shared allocator, cancellation/resume/revocation and unknown-effect cases pass. No service/effect activation. |
| P08 Guild persistence/recovery (devtools) | Draft crash-point, writer-exclusion, orphan-association and reservation-settlement fault matrix; compare existing spool/checkpoint mechanisms. | P07 and independently decided D04; explicit commit and fencing protocol, both native host fault tests. No automatic lease takeover from elapsed time alone. |
| P09 C82 Foundry adapter (devtools) | Prepare consumer questions and provisional mock manifest; distinguish observed data from authorization, receipt and effect authority. | Existing C82 remains blocked until C79 format and revision binding are accepted. Bind exact subject/fingerprint and preserve legacy receipts; no scheduler or product writes. |
| P10 C85/C86 gate routing (product then devtools) | Draft comparison of gate declarations, inputs, result references, platform matrix and full-gate evidence. | C79, C82 and C85 accepted; C86 consumes declared tiers and never substitutes a partial diagnostic for the final native gate. |
| P11 C88/C87 notation and build sheets (product) | Draft canonical Base36 edge cases and a build-sheet data inventory; do not rewrite historical IDs. | C88 semantics accepted before C87 consumes them; C87 also depends on accepted product capability/profile/verification contracts and its own native gate. |
| P12 report-only vertical slice (devtools) | Select candidate retained-evidence examples; draft budget, signer, stale-subject, self-review, crash and replay scenarios without dispatch. | P05-P08 and P09 as applicable; offline shadow first. Policy owner must specify and authorize exact live recipe, paths, principals and numeric budgets before execution. |
| P13 selected component consumer (devtools/product) | Inventory a real consumer and compare qualified `scan_core`/other product interfaces with CLI boundaries, versioning and transitive feature costs. | Accepted component and concrete need; pin revision; test both platforms and product independence; one consumer per reviewable change. Prior three-test Windows scan result is not product certification. |
| P14 Ubuntu/Windows coordination (devtools) | Prepare deployment-free test design: lost connectivity, stale worker, duplicate upload, cancellation, disk failure, operator handoff and restart. | P08/P12 accepted; prove actual fencing and effect reconciliation on both hosts. Explicit installation, credentials, endpoint and authority decisions precede a live pilot. |
| P15 Chimera proposal workflow (devtools) | Draft isolated worktree, provenance, bounded model output, deterministic validation and independent review cases. | P12/P13 and authorized mutation policy; no generated patch self-acceptance, shared-worktree writes or autonomous merge. |
| P16 Marionette isolated use (product/devtools) | Draft target identity, observation freshness, focus-race, stuck-input, emergency stop and unknown-effect tests against an isolated application. | Qualified platform components, P14 and separately authorized actuation. Measure real cancellation and leakage limits; do not claim hard real-time or guaranteed zero leakage from a focus check. |
| P17 graduation and release (both repositories) | Prepare source/license review, contract tests, feature/shadow rollout, migration and origin-copy retirement checklist per product graduation rules. | Each capability independently qualified, integrated and reviewed; release/deployment authority and rollback are separate. Keep Aaroneous build, test, release and runtime independent of Devtools. |

For each later draft PR, record its exact parent commit and prerequisite PRs,
the evidence available **now**, the missing acceptance evidence, and rollback.
After a parent lands, refresh the diff and rerun affected checks before review;
an old acceptance does not automatically transfer to a changed head. Do not
create a chain of code PRs against invented schemas or use a draft to reserve
another owner's files. These packets enable parallel preparation, not a claim
that downstream implementation or industrial qualification is complete.

#### Cold-handoff procedure for every packet

1. Read the applicable `AGENTS.md`, the [master playbook](../governance/MASTER_PLAYBOOK.md),
   the canonical local queue named there, current remote queue, and the packet's
   cited sources. Record repository, full base/head, dirty paths, input hashes,
   existing PR and owner. Do not assume this dated plan is the live queue.
2. Run [schedule review](../governance/SCHEDULER_MAINTAINER.md#command) with
   explicit absolute queue/program/output paths. Resolve stale rows and check
   the current owner before using its capacity result; it is advisory, not a
   claim. A blocked packet can be analyzed in scratch but cannot be claimed for
   maintained edits until its dependencies and review capacity are available.
3. Convert a packet to one exact Ready scope, then record owner, branch,
   worktree, UTC time, concrete paths, evidence and gate in the queue before
   editing. Respect other claims and the product pre-edit native gate. Do not
   create placeholder implementations or undocumented authorities.
4. Capture a failing or missing-behavior case before changing code where
   practical. Implement one boundary, preserve existing data and verify the
   relevant positive and negative cases. Use an explicit scratch build target;
   coordinate expensive builds. Product: `cargo xtask gate` before editing and
   before completion per product `AGENTS.md`. Foundation: relevant workspace
   tests, formatting, strict Clippy and any contract/CI checks. Documentation:
   UTF-8/LF, links, fence and diff checks.
5. Commit scoped paths only. PR text names base/head, prerequisite PRs, exact
   commands/results, platform, retained logs, known limits and rollback.
   Release the claim to Review; an independent reviewer assesses it. If a
   prerequisite or gate fails, preserve evidence and hand off the exact blocker.
   Updating a parent requires rechecking a dependent PR at its new head.

#### Packet implementation briefs

**P01 — product capability export (existing C79).** Start with the current
C79 owner/branch and `xtask` workspace metadata, not a competing exporter.
Inputs must identify the selected repository root and resolved full commit;
output describes only declared product capabilities, profiles and allowed gate
identifiers with a version and fingerprint. Review fixtures for wrong root,
moved revision, duplicate or unknown gates, stable sorting and changed inputs.
The export is descriptive, never a grant. Handoff its accepted schema, exact
revision, examples and compatibility policy to P09; a draft is not an API freeze.

**P02 — caller authorization (C80).** Trace `mcp_service/http_api.rs`,
`mcp_service/service.rs`, `mcp_service/auth.rs` and `capability_broker.rs` at
the selected product revision, including direct, batch and notification paths.
Define a trusted context derived from authenticated transport credentials,
with current principal, scope, revocation and injected time; do not trust
caller-supplied principal/scope fields. Test missing/bad credentials, modified
identity and scope, expiry, revocation, wildcard escalation, cross-principal
reuse and read-only versus mutating dispatch. Assert rejected calls perform no
effect and thermal checks still run. Review compatibility and credential
handling without printing secrets. C80's full scope and product gate govern.

**P03 — strict explicit inputs (C81).** Trace product `paths` and
`llm_gateway` constructors, configuration and current callers. Define an
explicit strict construction route with named missing-input errors and a
separately named legacy/bootstrap route where compatibility is required. Test
missing root, endpoint, model and model path independently, supplied values,
invalid values and absence of ambient fallback on the strict route. Record a
caller migration table before changing defaults; do not claim the legacy route
is strict.

**P04 — product gate tiers (C85).** Read product `xtask/src/gate.rs`, its CI
drift tests, native/dependency audits and current CI workflows. Inventory each
full-gate step with prerequisites, output and failure behavior. Define a
machine-readable preflight as a strict subset with explicit revision/root and
stable ordering. Test first failure, omitted or unknown gates, bad root/revision
and full-gate parity. The full gate remains mandatory; coordinate the `xtask`
scope with C79 and don't launch simultaneous full builds without a reservation.

**P05 — operation decisions.** Read the complete proposed
`contracts/guild-operation-v1.md`, especially sections 3-8, and independent
review history referenced there. For each D01-D05 record accept/revise/defer,
reason, reviewer identity and exact revision; separately resolve resume,
overlapping allocation, pre-allocation cancel and post-acceptance revocation.
Name the association signer and trust anchor and the report acceptance owner.
Return open codec, numerical and storage choices as explicit blockers. Review
is not authority to publish or mutate code.

**P06 — semantic fixtures.** After P05, take the transition table and
phase/event matrix as the reviewed oracle, then define language-neutral cases
for every phase/event pair plus OCV01-26 guards and conditional destinations.
Each fixture names prior state, event, supplied authority/time/subject, expected
state or rejection, attempt history, effect set and budget changes. Include
truncated, duplicate-key, unknown-field, oversize and integer-boundary cases at
the byte parser layer; a parsed map cannot detect an already discarded duplicate.
Select exact schema paths only in the later Ready claim. Preserve existing wire
and receipt bytes and provide old/new-reader compatibility vectors.

**P07 — operation reducer.** Inspect existing Steward request/admission/outcome
and Guild scheduler code first. Implement only accepted pure decisions in the
existing approved home; use P06 fixtures without I/O or trust claims from workers.
Prove a shared T04/T13 allocator prevents two unfinished attempts, T07 never
allocates, pre-allocation cancellation creates no attempt, and acceptance does
not authorize a later effect without current eligibility. Keep multiple effects
and historical review records separate. Fault/host tests belong to adapters.

**P08 — durability and recovery.** Map existing `local_verification::Store`,
Guild checkpoint and `contracts/platform-jobs-v1.md` before selecting storage.
Draw each persisted-write/dispatch/acknowledgement boundary; enumerate crash
before and after every boundary and an orphan association. Test full disk,
corrupt/truncated log, lost checkpoint, stale revision, surviving child, duplicate
delivery and lost acknowledgement. A retry stays blocked until the old executor
is excluded and effects are known. Record storage assumptions and rollback;
do not infer atomic cross-store commit from separate writes or a lease TTL.

**P09 — Foundry adapter (C82).** Await C79's accepted schema; read
`architecture/FOUNDRY_CONTROL_TRANSITION.md` and G-2026-04. Prepare a mock only
in scratch, clearly marked provisional. The claimed adapter binds explicit
product root, full revision, manifest fingerprint, declared gate, result
reference and actor attribution; it must reject mismatch, unknown fields/gates,
stale reports, modified manifest and absent inputs. Keep status read-only and
transport data separate from authority. Do not write a scheduler or product code.

**P10 — tier consumption (C86).** Compare accepted C79 manifest, P09 adapter
and C85 tier contract. Pass selected explicit inputs to the product-owned gate;
record tier, exact command, target revision, platform, result and log digest.
Reject undeclared/skipped gates, stale or missing receipts and substituted
partial checks. A fast preflight may guide iteration; only the declared final
gate can satisfy final product verification. Test native Windows and Ubuntu
handling before claiming platform equivalence.

**P11 — Base36 and build sheet (C88/C87).** C88 decides alphabet, case,
width, ordering, reserved values, invalid forms and versioning for its real
consumer. Cover zero, maximum, overflow, leading zero, bad case and round trip;
keep operation identity distinct from a display alias. C87 then binds each
product crate's accepted manifest identity/profile, dependencies, lifecycle,
owner, capabilities and required verification to a revision. Reject stale or
missing facts and do not fabricate owners or retrofit historical identifiers.

**P12 — report-only pilot.** Begin with retained signed evidence and a selected
immutable subject; replay it offline and compare outcomes to existing Steward
rules without dispatch. Choose the trusted suite, issuer, independent reviewer,
checkout, spool, outputs, time/memory/process limits and network policy only
through the authorized owner. Then test exact-subject success, failing receipt,
self-review, revocation, cancellation, crash, stale attempt and replay. A
successful process exit is not acceptance; report-only execution may run build
scripts. Keep publication and model calls disabled for the first pilot.

**P13 — product capability consumer.** Pick one Devtools workflow with a
measurable need and compare existing product APIs, including `scan_core`, to an
explicit CLI boundary. Record API/semver stability, owner, product commit,
features, normal/build/dev/target dependencies, license, platforms and removal
strategy. Test current and unsupported revisions, absent capability, malformed
response and rollback. Verify product gates without the Devtools checkout or
service. No use is justified solely by a small dependency count.

**P14 — cross-host coordination.** Read `contracts/platform-jobs-v1.md`,
existing service/worker tests and the actual approved deployment runbooks.
Use explicit host endpoints, identity files, spool and output paths; collect no
credentials in reports. Exercise wrong role/platform, duplicate or late result,
coordinator restart, partition with a live old worker, disk failure, operator
stop and independently witnessed recovery on Windows and Ubuntu. Verify an
unknown effect cannot be repeated and no second writer takes over merely
because time passed. A live deployment requires its own authorization.

**P15 — Chimera proposal route.** Trace `virtual_patch` and `local_proposal`
contracts plus the accepted verification adapter. Define immutable source
revision and file digests, allowed paths, proposal provenance and resource
limits. Test malformed output, stale base, path escape, patch outside scope,
failed gates and a passing proposal awaiting independent judgment. Apply only
to an isolated claimed worktree after admission; never execute model text as
authority, write to shared product paths or claim clean-room provenance from a
rewrite. Retain evidence and the rollback path.

**P16 — Marionette.** Trace actual platform capture, target selection, input
and stop paths before composing a test. Define a disposable isolated target,
window/process identity, approved actions, observation-age limit and local stop.
Test lost focus, stale frame, process exit, delayed input, stuck key, worker
disconnect and crash; measure observation age and stop/release latency. A
focus check cannot make the operating-system input queue atomic; unknown target
state requires reconciliation. No live actuation is granted by this packet.

**P17 — graduation and release.** For each candidate record origin/license,
owner, existing equivalent, accepted public contract, profile, tests, feature
flag/shadow plan, migration and origin-copy retirement. Product graduation is
its own change under product-native gates; verify independent product build,
test, release and runtime without Devtools. Separately record release authority,
artifact provenance, support runbook, rollback and independently reviewed
evidence. Do not equate a tested component with an industrial certification.

### Existing operating direction

Build Guild as a **supervised software-development operations system**, not a new
operating-system kernel and not one enormous autonomous agent. Use the existing
Rust Foundation and Steward implementation. Ubuntu is the proposed primary
coordination/execution host; Windows remains a native verification worker and
operator client. Both communicate through versioned, authenticated devtools
interfaces. Loss of Ubuntu must produce an honest unavailable state, not a second
unfenced coordinator on Windows.

The central unit is a durable, bounded, authorized operation with inspectable
evidence. Models propose; policy admits; isolated workers execute; deterministic
checks verify; independent reviewers accept; separately authorized actors publish,
integrate and deploy. Names and interfaces may simplify the experience without
concealing those boundaries.

Do not wait for all of Aaroneous to be rewritten. Use the existing standalone
bootstrap, select one suitable product capability, verify it, consume its pinned
contract, and grow that dependency deliberately. Aaroneous remains independently
buildable, testable, releasable and runnable throughout.

For this plan, industrial qualification means reliable software-development
operations under a declared workload and threat model. It does **not** claim
certification for machinery, medical systems, transport, weapons or hard real-time
control. Those uses require separately defined assurance obligations and review.

## 2. Terminology and boundaries

| Term | Precise meaning / restriction |
| --- | --- |
| Guild | The operational product/service surface coordinating authorized work; not a grant issuer merely because it schedules |
| Foundation | Existing devtools Rust workspace and maintenance services; not a new universal product dependency |
| Steward | Existing deterministic policy/review/scheduling modules, invoked by effectful adapters |
| Façade / composition preset | User-facing selection of existing capabilities and defaults; the earlier "fascia" concept. A preset cannot union or manufacture permissions |
| Core preset | Optional minimum coherent capability composition; no implicit privilege and no requirement to import every component; distinct from Rust `core` and existing `core/` directories |
| Cratify | Existing product auditing/harvesting/scaffolding entrypoints plus compliance specification; devtools orchestrates selected capabilities instead of recreating them |
| Chimera, Marionette, Merlin, Foundry | Descriptive program/preset identities where admitted; implementation paths and APIs remain subject to product naming rules |
| Operation | Versioned intent and acceptance contract spanning attempts; not synonymous with a chat turn, queue claim or process |
| Attempt | One execution against pinned inputs, a resource reservation and a fenced lease; failures do not erase earlier attempts |
| Gate evidence | A recorded check on an exact subject and environment; not independent acceptance, merge, publication or deployment |
| Projection | Rebuildable status, topic graph, search index or dashboard; never a competing authority ledger |

Architectural tiers describe dependency placement. Cratify profiles describe
implementation obligations. Authorization describes what a principal may do.
Keep these three axes separate. Higher-level presets do not acquire higher
privilege by their position or name. An adopted naming exception needs a product
amendment; this private plan cannot silently create one.

Do not put everything in `xtask`, or create a crate for each brand name. `xtask`
coordinates product checks. Reusable audit logic belongs behind the existing
Cratify/auditor boundaries; operations policy belongs in devtools; OS/network
effects belong in adapters. Split crates only for independently useful contracts,
ownership, dependency/profile isolation or release needs. A shared dependency
graph does not guarantee one runtime service or one memory allocation.

## 3. Non-negotiable system invariants

1. **Explicit authority:** selected repositories, refs, endpoints, executables,
   support directories, identities and budgets; no permissions inferred from text.
2. **One writer per authoritative responsibility:** lease/operation state, review
   acceptance and Git ref updates have identified writers. Existing authorities
   remain in place until migration is approved and tested.
3. **Deterministic decisions, supervised effects:** recorded events and supplied
   time drive reducers. Networks, compilers, models and GUI actions remain external
   effects; replay does not re-execute them automatically.
4. **Bounded everything:** admission queue, recursion/fanout, subprocess trees,
   memory, disk, logs, tokens, cost, wall time and retained history. Exhaustion
   yields a visible blocked/paused result, not unlimited retry.
5. **Immutable evidence subject:** repository identity, base/head, suite/policy,
   toolchain/environment and artifact hashes accompany every verification record.
6. **Least authority at every hop:** child grants are no broader than parent and
   host grants; capability possession is not proven by a readable Base36 ID.
7. **Isolation before mutation:** reserve scope and create an isolated candidate
   before any generated skeleton or patch write. Never capture a shared Git index.
8. **No self-certification:** implementation authors cannot supply independent
   acceptance. Model family/seat attribution and contribution history are checked.
9. **Safe uncertainty:** incomplete parsing, stale health, lost acknowledgements,
   unknown cost and missing verification block the relevant action explicitly.
10. **Independent recovery:** another authorized operator can restore, inspect and
    continue using canonical records, without the original chat or model.

These requirements are proposed qualification criteria, not assertions that every
existing path already enforces them. Existing product and companion policies win
until reviewed amendments are adopted.

## 4. Operational architecture and state ownership

| Plane | Existing starting point | Target owner / boundary |
| --- | --- | --- |
| Operator interface | Maintenance UI, packet utility, status commands | Windows/Linux clients read projections and submit authenticated requests; no browser policy engine |
| Intake and contracts | Intake/job/topic contracts | Validate bounded intent, scope, acceptance, source provenance and approval class before scheduling |
| Policy and coordination | Steward reducers and Guild CLI | Single coordinator admits work and leases; evaluation remains testable without I/O |
| Execution | Existing local worker; PR203/204/206 adapters | Native workers execute approved recipes, not arbitrary commands supplied by a model or network caller |
| Evidence and acceptance | Signed receipts, coverage/independence/gate modules | Verification results and independent judgments remain separate; credentials remain outside candidate execution |
| Model support | Model-fit/endpoint policy, support manifests | Proposal-only by default; supervised calls through selected provider contract and reservations |
| Product capability library | Aaroneous public contracts and native tools | Version-pinned downstream reuse; no devtools dependency in product |
| Persistence and publication | Existing event chain/spools; release decision | Explicit durable state/outbox, scoped Git publisher, recovery and export adapters to qualify |

Initial topology is one Ubuntu coordinator and independently authenticated native
workers. Retain HTTPS polling from the existing adapter stack unless measured
requirements justify another transport. Do not introduce P2P consensus, a shared
mutable network filesystem or a second scheduler to connect two machines.

Primary hosting is not automatically Git authority. Keep existing Git/review
authority until the separately proposed private-authority migration passes its
own import, fencing, reconciliation and rollback gates. The advisory Round Table
cannot authorize jobs, sign verification, approve merges or replace that migration.

### Canonical records and compatibility

Define one versioned contract family around existing records rather than parallel
schemas. The future contract packet must cover:

- Operation ID, schema version, requester, repository, pinned input digests,
  desired behavior, acceptance tests, prohibited effects and authorized scope.
- Attempt ID, parent relationship, expected state revision, lease epoch/token,
  worker identity, platform, deadline and aggregate resource reservation.
- Effects requested, dispatched, acknowledged and reconciled; artifact manifest,
  stdout/stderr bounds, diagnostic fingerprint and resume/retry eligibility.
- Verification subject/suite/policy/environment, actual command exits and retained
  log digests; review scope, implementers, independent reviewers and verdicts.
- Publication/integration/deployment references, unknown external outcomes,
  compensation requirements, retention class and final disposition.

Unknown versions fail with a useful compatibility error. Specify canonical
serialization or length-framed hashing; test field-boundary collisions. Migrate
state with backups and backward-read/rollback rules before changing writers.
Base36 may encode IDs for display but does not replace repository namespaces,
revision anchors, collision handling or cryptographic authorization.

## 5. Lifecycle: bounded does not mean magically total

The four responses in the supplied documents are useful user-facing categories,
not a complete durable lifecycle. An HTTP response, agent reply and multi-hour
operation must not be treated as the same state machine.

| Category | Required behavior |
| --- | --- |
| Intake rejected / clarification required | Preserve the reason and request identity; no candidate mutation; revised intent creates a recorded revision |
| Accepted / queued / running | Return a durable handle and current phase; allow status and authorized cancellation without pretending work finished |
| Preflight blocked | No candidate-repository changes; audit/intake bookkeeping may still be written and must be stated explicitly |
| Paused / execution halted | Preserve evidence and candidate; record what can resume, prerequisites and spent budget |
| Cancelled / failed | Stop or fence descendants, reconcile effects, release resources only when safe, retain diagnostic evidence |
| Artifact ready / awaiting independent review | Work may succeed without a PR; output is not yet accepted or integrated |
| Published / integrated / deployed | Distinct evidence-bound transitions, each with its own authority; publishing a draft PR is not the universal success condition |
| Reconciliation required | External action may have completed despite timeout; inspect existing result before retrying |

Prove transition validity, not merely enum serialization. Terminal disposition
must be durable and unique for an attempt; external-effect outcomes may require
explicit reconciliation. Do not promise mathematically total execution under
arbitrary hardware failure. Require bounded supervised steps and recoverable
operation state under the declared failure model.

Recursion is optional. Begin with single-level jobs. Before enabling decomposition,
bound depth, fanout, total descendants and aggregate budgets; children debit the
same parent reservation. Child file scopes may equal a permitted single-file
scope but cannot expand it. Serialize overlapping writers and prevent stale
children from publishing after parent cancellation. A new task ID cannot reset
budget, rejection history or review obligations.

## 6. Qualification stages and exits

Stages are dependency gates, not calendar promises or automatically Ready claims.
The near-term critical path is S0 -> S1 -> S2 -> S3 -> S4 -> S5. Later capabilities
attach only after their prerequisites; full-depth operation includes S6-S9 at
the approved scope. Existing open work is reconciled before new implementation.

### S0 — Reconcile facts, authorities and document ownership

Deliverables:

- Accept/revise this comparison and plan; distinguish source, tested, accepted,
  merged and deployed status for the selected operational path.
- Resolve queue/main divergence using full contribution identities, not short
  C-numbers alone; preserve dirty work, historical evidence and original authors.
- Reconcile PR203 -> PR204 -> PR206 against current main. Review actual merge
  candidates and refresh head-bound evidence after dependency changes.
- Inventory the selected services and their real entrypoints, owners, storage,
  credentials-by-reference, endpoint policy, stop procedures and deployed hashes.
- Agree pilot trust class and workload; classify unrelated prototypes as retained,
  retired-after-migration or separate research, not casually delete them.
- Route the stale product roadmap gate list (O07) to a separately claimed product
  documentation correction; do not let the companion plan become product law.

Exit: independently reviewed baseline and document precedence; no conflicting
writer/claim for the pilot; a source/receipt location another operator can inspect.
Rollback: documentation changes only; no service migration occurs in this stage.

### S1 — Freeze minimum operation, authority and failure contracts

The [operation contract reconciliation proposal](../contracts/guild-operation-v1.md)
maps existing records to missing operation-level rules, state ownership and
22 qualification cases. It is specified only, not adopted or behavior-verified.

Deliverables:

- Map existing job authoring, queue claims, runtime leases, review and publication
  states into the lifecycle above without merging their meanings.
- Specify principal roles and allowed transitions, including inspect/propose,
  verify, mutate, publish, integrate and deploy; define who may grant/revoke each.
- Pin schema/version negotiation, idempotency framing, evidence freshness,
  cancellation, deadline accounting, ambiguous outcomes and support-data privacy.
- Reconcile model route differences between product and companion rules; no live
  call until the selected route and its authority are explicit.
- Document threat model: malicious repository text, generated code/build scripts,
  stale/compromised worker, forged receipt, path alias, stolen credential, outage.

Exit: positive/negative language-neutral fixtures; transition matrix; every
mutable field has a writer and stale-write rule; independent contract acceptance.
No universal tick trait, new workflow language or broad crate hierarchy is needed.

### S2 — Qualify durable coordination and bounded adapters

Start from Guild/Steward and the current local-verification/integration/service
stack. Do not write a second scheduler or event store merely for naming consistency.

Required work:

- Reproduce O01-O03 from the comparison: append/tip crash ordering, concurrent
  tick/daemon writers, stale locks, implicit snapshot selection and unbounded
  subprocess paths. Fix only confirmed defects in exact-path claims.
- Choose and test a durable transaction/outbox boundary. If current files can
  satisfy it with a bounded protocol, retain them; otherwise justify storage
  replacement with migration and failure evidence. A hash chain alone is not
  fsync, authorization, atomicity, immutable retention or backup.
- Persist full operation/attempt/lease identity; reject stale epochs at every
  effect boundary, not only when scheduling. Quarantine ambiguous interrupted jobs.
- Bound reads, queues, log bytes, child process trees and shutdown latency. Preserve
  diagnostic tails and digests when truncating output; signal truncation clearly.
- Provide manual pause, admission stop, drain, cancel, safe shutdown and recovery;
  separate liveness from readiness, worker health from task success.

Exit: crash injection before/after durable commit and effect acknowledgement;
concurrent/stale writer, disk-full, corrupted-tail, clock-jump and duplicate-message
tests; native Windows/Linux results; reconstructed state matches retained events.
Rollback: stop admission, preserve logs/state, restore only through documented
compatible recovery; never overwrite newer history with an old snapshot.

### S3 — Qualify Ubuntu-primary and Windows-native execution

Required work:

- Explicit host provisioning manifest: binaries/toolchains and hashes, immutable
  SDK/image, checkout/object source, storage quotas, service account, endpoints,
  certificate trust, node identity, secret references and allowed recipe set.
- Qualify the dedicated rootless Linux executor and Windows containment separately.
  Rootless is a useful boundary, not proof of hostile-code safety. Containers share
  a kernel; Windows process controls are not automatically equivalent isolation.
- Candidate code gets no signing/publishing keys, model keys, host-control sockets,
  coordinator storage or unrestricted network. Approved dependency acquisition
  happens through an explicit controlled stage; caches cannot cross trust classes
  without validation. Set CPU/memory/process/disk/time limits and reap descendants.
- Repeat authenticated pair smoke at the integrated head over the intended LAN
  route, then execute the real pinned Foundation suite on both physical hosts.
  Admit node keys to platform roles through reviewed policy, not self-reported labels.
- Prove boot/logout/restart behavior, certificate/key rotation and revocation,
  offline reconnect, stale worker rejection and result replay protection.
- Keep the previous gate path until native evidence meets all required platforms;
  do not remove a required Windows check while its replacement is unqualified.

Exit: two independently attributable native receipts for the exact candidate,
negative auth/routing/isolation fixtures, no remaining child after cancellation,
restart recovery and operator-tested install/pause/uninstall. Untrusted external
code stays disabled until its separate isolation profile passes.
Rollback: stop new admissions/pollers; preserve accepted receipts and candidate
state; use the prior verification route without silently transferring authority.

### S4 — First useful end-to-end vertical slice

Pilot: an authorized read-only maintenance request on a pinned Aaroneous checkout
produces a bounded report, native verification evidence where required and an
independently reviewed disposition. No model is required for this first pass.

Exercise intake -> policy -> lease -> worker -> artifacts -> validation -> review
-> operator-visible result. Include unavailable worker, malformed input, cancellation,
duplicate request and coordinator restart. Status must show observed revision,
age, actual gate coverage and unverified areas. No simulated "Verified" state.

Exit: an operator who did not implement it completes and recovers the workflow
using the runbook; unauthorized requests produce no candidate mutation; all
accepted work reaches a disposition or visible actionable hold. Benchmark the
manual path and record actual improvement before claiming efficiency.

### S5 — Bounded, independently reviewed development jobs

Pilot: one small defect correction with a predeclared regression, exact file scope
and isolated worktree. Use existing patch/scaffold mechanisms only after adapter
review; generated output is untrusted. Do not begin with a whole-repository rewrite.

Required controls:

- Scope reservation before worktree/skeleton writes; content/base validation before
  applying patches; path containment on Windows/Linux including aliases and links.
- Immutable protected acceptance tests or independently assessed test changes;
  a candidate cannot weaken its own suite, policy, signatures or safety limits.
- Bounded propose/validate/apply/test loop. Compiler success and fewer errors are
  useful signals, not semantic correctness. Fingerprints include the relevant
  source/configuration and diagnostic context, not error code alone.
- Full product-native gate for product code, selected independent security and
  behavior checks, complete diff/command receipts and independent review.
- Separate publication action: inspect whether a prior push/PR creation succeeded
  before retry. Draft PR or local review artifact is permitted by configured policy;
  automatic merge/release remains a separate accepted authority path.
- Resume checks base/intent/lease/toolchain changes and charges prior costs. Unsafe
  cleanup cannot delete shared or user-owned worktrees.

Exit: successful correction plus deliberate regression, scope escape, poisoned
instruction, timeout, stale base, review conflict and ambiguous-publication cases;
two operators can reproduce the evidence. Candidate mutation remains disabled
outside the qualified recipe and repository scope.

### S6 — Reliable repository ingestion and capability mapping

Depends on S1-S4; mutation/decomposition requires S5. Reuse existing Cratify,
adaptation, parser and Omni capabilities after evidence-backed contract inspection.

- Build a bounded repository manifest: source revision, files/bytes, excludes,
  submodules, generated code, languages and build configurations. Exclusions are
  visible. Do not run arbitrary build scripts just to index text.
- Stage syntax recovery and semantic resolution separately. Report parsed,
  partial, unsupported, unresolved and failed entries with reasons. Symbol coverage
  has an explicit denominator; macros, feature sets and generated sources limit it.
- Define symbol IDs, collision handling and revision/rename lineage; retain source
  spans and hashes. Graphs/search are disposable projections, not authority.
- Record provenance, dependency/admission decisions, license metadata and permitted
  reuse. Escalate unclear provenance; AST transformation does not erase obligations.
- Evaluate `.si`, semantic tags or models against a deterministic baseline only
  if they improve a measured task. Report accuracy, uncertainty, resource cost and
  fallback behavior; none is a correctness oracle or required bootstrap service.

Exit: fixed mixed-quality repository corpus; deterministic manifest/replay;
malformed/huge/unsupported file containment; explicit unresolved coverage; no
silent drops, no authority gained through indexed text; independently reviewed
component-boundary proposals before any automated decomposition.

### S7 — Isolated visual and interactive verification

Depends on S3 and S5. Inspect existing Marionette/display/input code before creating
a new crate or driver. Prefer application APIs and structured accessibility evidence
where available; pixels supplement behavior, not replace it.

Qualification requires a dedicated test session/display, application/window allowlist,
bounded capture retention, input ownership, process supervision, a kill switch,
and proof that the physical operator desktop and unrelated applications cannot
receive input. A virtual framebuffer or Windows desktop handle alone is not a
security boundary. Test Linux and Windows separately, including loss of display,
focus changes, crashes, hung rendering and cleanup after disconnect.

Exit: one useful GUI acceptance scenario with action/observation receipts, structured
behavior assertions and calibrated image thresholds; adversarial wrong-window
tests; no credential capture or uncontrolled host input. If isolation cannot be
proved, keep GUI automation disabled and continue CLI/API-only operation.

### S8 — Product-backed presets and incremental self-improvement

Depends on stable S4/S5 evidence and accepted composition contracts. Address the
existing blueprint findings R1-R14 rather than silently superseding them.

- Inventory existing public contracts and resource/lifecycle owners. Choose one
  read-only composition, then a second consumer sharing an explicitly owned service.
- Specify manifest resolution, compatibility, grants/revocation and aggregate
  budgets; test cycles, diamond dependencies, duplicate resources and cancellation.
- Register Core as a minimal optional preset only after product naming/profile
  amendments. Other named façades follow the same rules; Foundry is a later preset,
  not a privileged escape hatch or a giant repackaged library.
- Graduate one stable general mechanism into Aaroneous with product-owned contracts,
  native gates, shadow/inert adoption and rollback. Move the consumer to the verified
  pin; retire the origin implementation only after migration evidence.
- Treat changes to the running Guild, validators, identity, policy or updater as
  high-risk separate changes. A running agent cannot approve, weaken or install its
  own successor. Maintain known-good binaries and out-of-band recovery controls.

Exit: two consumers demonstrate bounded sharing and isolation; product builds/runs
without devtools; upgrade and compatible rollback both work; no duplicate hidden
implementation, implicit privilege or unreviewed self-modification.

### S9 — Operational service acceptance and controlled expansion

Before an "industrial" label, qualify the enabled feature set under a declared
workload, then publish its support limits. GUI/model/research features may remain
disabled in an initial industrial **CLI-only** service profile; that is not a
claim of full-depth capability until their own stages pass.

Required evidence:

- Load/soak tests with concurrency, repository sizes, event volume and failure
  injection; bounded storage and predictable backpressure under overload.
- Backup/restore to a clean host, credential revocation, loss of primary host,
  corrupted state, upgrade/schema migration and recovery after partial external
  publication. Replication alone is not a backup.
- Supply-chain inventory, tool/image/model provenance and signatures where used,
  dependency vulnerability response, secret redaction and access audit.
- Runbooks, owner/on-call responsibilities, incident severity/communication,
  emergency admission stop, support matrix, release/change controls and retention.
- Honest dashboards and alerts for stale evidence, queue age, failed verification,
  resource saturation, repeated failures and unavailable reviewers; idle is healthy.
- Independent acceptance of the actual deployment revision and configuration;
  scope exceptions have owners, expiry and compensating controls, not silent waivers.

Exit: reviewed service acceptance record and approved expansion boundary. Start
with one repository/one active execution slot, expand by measured headroom and
review capacity; do not increase parallel work solely because more models exist.

## 7. Measurable acceptance profile

The following are **proposed pilot thresholds to approve at S0**, not present
measurements or universal production guarantees. Change them with a reason before
running the acceptance campaign, not afterward to hide a failed result.

| Property | Proposed pilot target / evidence |
| --- | --- |
| Authority | All unauthorized-effect fixtures denied; zero observed effects outside declared scope; independent security review before widening trust |
| Traceability | Every admitted attempt links intent, input revision, lease, artifact and disposition; missing evidence blocks acceptance |
| Cancellation | Admission stops immediately in reducer tests; workers stop/reap or are quarantined within 30 seconds on the declared pilot workload |
| Crash recovery | No loss of durably acknowledged local operation events in tested crash windows; ambiguous external outcomes enter reconciliation, never blind success/retry |
| Host recovery | Restore coordinator and retained evidence to a clean test host within 60 minutes; initial backup recovery-point objective at most 15 minutes, subject to approved backup cadence |
| Duplicate delivery | Replaying 100 copies of a request/receipt yields one logical accepted transition; no duplicate publication in ambiguity tests |
| Boundedness | Test at configured limits and one above each limit; rejection/backpressure is visible and does not corrupt existing work |
| Soak | Seven-day two-host trial covering idle/load/outage/restart; zero lost accepted jobs or unauthorized effects; every halt has a diagnosable disposition |
| Usefulness | Compare at least ten representative pilot requests with recorded manual baseline; report reviewed outcomes, elapsed time, operator effort, failures and cost, not token volume |
| Availability | Measure first; choose service-hours/availability objectives before wider release. One Ubuntu host is a single point of failure, not an HA deployment |

Resource numbers (memory, concurrency, time, tokens, disk, provider spend) must be
explicit per recipe/host and aggregate across children. Paid calls remain disabled
until data permission, model route, price version and spend reservation are approved.
Unknown usage or ambiguous billing cannot reset the reservation. No specific model
latency, VRAM size or 90/10 split is assumed from the Word documents.

## 8. Documentation realignment without another competing constitution

| Authoritative home | Required treatment |
| --- | --- |
| Product `AGENTS.md` / Cratify specification / canonical product architecture | Product rules, floor/profiles, naming amendments and native gates; changes require product-local claims/review |
| `FOUNDATION_BOUNDARY.md` / `OPERATING_MODEL.md` | Companion scope, dependency direction and implementation standards |
| `contracts/` | Adopted versioned schemas, transition invariants and fixtures; label proposals explicitly |
| This plan | Cross-system qualification order and exit criteria, not live status or executable permission |
| Dated readiness comparison | What was inspected and what it supports; refresh only with a new revision-qualified assessment |
| `MAINTENANCE_PROGRAM.md` / `MULTITHREAD_PROGRAM.md` | Link the qualification sequence and retain useful historical milestones; do not copy all tasks |
| `COORDINATION_QUEUE.md` and accepted review records | Exact active ownership, contributions, evidence and independent dispositions |
| Service runbooks and private deployment receipts | Actual installed revision/configuration, access by reference, start/stop/restore; never infer from a design doc |

This change adds navigation in the master playbook, maintenance delivery map and
portfolio. It does not edit other owners' historical documents or rewrite the
Word originals. Follow-up document work should:

1. Mark old status/schedule statements as dated observations; route readers to
   the installed-state receipt rather than reproducing account-specific schedules.
2. Reconcile proposal language with accepted implementation precisely, including
   model-hub stages, local Git authority and the event-store migration.
3. Replace invented paths with verified interfaces or explicit proposed paths.
4. Link each invariant to an enforcing component, negative test and gate; separate
   reference resolution from actual supporting evidence.
5. Retire duplicate guidance only with redirects/provenance and independent review.
   Preserve historical claims for audit without advertising them as current truth.

Do not mass-edit all 73 documents. Correct the operational entrypoints and the
documents needed for the next stage first. A single indexed decision with a
reproducible receipt is better than many repeated descriptions.

## 9. First dispatch packets and parallel work

These are bounded recommendations, **not active claims or assignments**. Refresh
the queue, author availability and independent reviewer capacity before opening
each. Maximum three primary lanes plus one review lane remains the default.

| Order | Packet | Exact responsibility / exit |
| --- | --- | --- |
| A | Independent review of this blueprint | Challenge G01-G18/O01-O07, contract priorities and threat scope; classify accepted/revised/deferred. Author cannot self-accept |
| B | Existing adapter stack integration | One integration owner reconciles #203, then #204, then #206 against current main; retain native gates and fresh independent head-bound reviews; no deployment |
| C | Guild durability reproducer | Narrow claim in `guild.rs` and its tests for one of O01/O02; reproduce crash/concurrency failure before proposing storage redesign |
| D | Host qualification packet | Explicit Ubuntu rootless/toolchain/isolation design and Windows counterpart; no provisioning until scope and permissions are approved; begin with disposable fixtures |
| E | Operation-contract reconciliation | Compare current request/receipt/job/claim contracts, specify only missing fields and transitions; freeze interfaces before dependent code lanes |

B and C can proceed in parallel only if their actual file scopes are disjoint;
E must not race shared `steward/src/lib.rs` or schemas. D's read-only/design work can
proceed alongside them; heavy builds and local model resources remain serialized
under the shared reservation. Prioritize finishing reviews over starting all five.

Use Codex/Gemini/Claude or another admitted seat according to capability and genuine
independence, not a predetermined author/reviewer rotation that certifies its own
work. Qwen remains a bounded proposal-support worker under its existing contract;
no command execution, queue ownership or verification authority. No agent sessions,
messages, jobs or recurring automations are started by this plan.

## 10. Second-pass critique and remaining decisions

This is the author's adversarial second pass, **not independent acceptance**.

| Challenge | Adjustment / remaining gate |
| --- | --- |
| The plan could become an excuse for a rewrite | S2 reuses existing service stack; S4 proves a small path; S6-S8 require measured reuse and graduation |
| Safety controls could depend on the system they protect | Keep bootstrap tools, signing/review credentials and recovery controls outside candidate authority; exercise clean-host restore |
| A healthy board could be mistaken for a healthy executor | Separate service identities, readiness endpoints, deployed hashes and proof of actual job completion |
| A single writer fails availability expectations | Acknowledge the single point of failure; recover with fencing. Add HA only after explicit availability requirement and split-brain tests |
| Passing tests may reward a candidate that changed its tests | Protect suite/policy provenance and independently assess acceptance-test changes; candidate never decides its own evidence standard |
| Whole-repository ingestion is an unbounded promise | Explicit corpus/limits/coverage and unresolved states; no guaranteed 100% semantic extraction |
| GUI control expands host risk dramatically | Optional S7 profile; explicit session/input isolation, wrong-window tests and safe fallback to CLI/API |
| Metadata proliferation could create another ledger | One authoritative writer per fact; readiness inventory is dated; graphs/dashboards are projections; queue owns dispatch |
| Full autonomy could erase independence | Preserve implementer attribution, separate reviewing roles, quorum rules and explicitly scoped authority at every promotion |
| Retrying could disguise failure or overspend | Aggregate budgets survive retries/children; repeated failure needs a changed hypothesis; unknown effects require reconciliation |

Decisions still needed before activation: pilot repository/workload/trust class;
approved service-hours and recovery targets; deployment identities and retention;
which mutation/publication actions are authorized; model/data/spend policy; named
independent reviewers; eventual local-Git authority migration; product naming and
composition contract amendments. These do not block reviewing this plan or
reproducing defects; they do block the associated operational expansion.

**Recommended immediate outcome:** accept a corrected operational vocabulary and
one source-backed baseline, reconcile the existing adapter stack, and qualify one
recoverable two-host workflow. That is the shortest defensible route to the larger
Guild system, and the foundation from which full-depth capability can safely grow.
