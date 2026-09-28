> **Recovery note (2026-09-27, Claude):** this file existed only as an untracked file inside a git stash (`0bf9b6ff7cd16bbe1c776ed10e872d1895fab736^3`) from 2026-09-24, referenced by C62/C66 but never committed to any branch — effectively unfindable without that exact commit hash. Recovered here verbatim (no content changed) so it stops being invisible, per the 2026-09-26/27 audit's finding on it. Its own status line below still governs: this is a proposal, not adopted policy.

# System Composition Blueprint

Status: proposed architecture, revision 1, 2026-09-25 UTC. Owner-requested planning deliverable under C62. No implementation authorization or compliance certification is implied. Further construction of this layer is held pending the design gates below. The accompanying [review](SYSTEM_COMPOSITION_REVIEW.md) records the author's second-pass critique, not independent approval.

## 1. Purpose and governing sources

Provide compact, named interfaces and reproducible presets over Aaroneous's existing capabilities. Preserve one maintained implementation of each responsibility, explicit resource ownership, and bounded authority as compositions grow. Develop compositions independently from the base without requiring the base to consume development tooling.

Binding sources remain the [foundation boundary](../FOUNDATION_BOUNDARY.md), [operating model](../governance/OPERATING_MODEL.md), and product constitution. This document is a private design proposal; accepted product rules must be published in Aaroneous before product enforcement. Operational planning remains in devtools. Do not maintain two authoritative copies of the adopted specification.

Product references, read at checkout HEAD `c35917194de3ad6617eec994edc7743474b4e36a` on 2026-09-25, with working-tree content not certified as that commit:

- `docs/CRATIFY_SPEC.md`: sections 1–2 (floor, profiles, composition root), 5–7 (admission, graduation, ratchet), 8 (nomenclature).
- `docs/ARCHITECTURE.md`: constructor injection, reducers, execution separation.
- `docs/architecture/architecture_overview.md`: rings, dependency inversion, generic naming.
- `AGENTS.md`: product verification and contributor obligations.

These are repository-relative product references, not dependencies on a sibling checkout. Integration must record an explicit checkout and verified revision; this observed HEAD is not a selected release baseline. C61's interrupted refactor and earlier partial test results require fresh reconciliation before reuse. C57/C58 documentation ownership must be checked before any amendment.

## 2. Canonical terminology

| Conversational term | Technical term | Meaning |
| --- | --- | --- |
| Fascia | System composition | Named assembly of capabilities, configuration, and a bounded public interface |
| Summary wrapper | Façade | Simplified API delegating to capabilities; may be stateless |
| Transformer layer | Adapter or request mapper | Explicit translation between contracts; not a neural Transformer and not necessarily a compiler |
| System preset | Composition preset | Versioned declarative selection and configuration; does not grant authority |
| Macro-crate | Composition crate | Optional Rust packaging unit implementing a composition; unrelated to procedural macros |
| Bracket ownership | Resource ownership and scoped delegation | One lifecycle owner per resource instance; explicit borrowing or restricted handles |
| Roots/base | Capability substrate | Reusable contracts and implementations; only hardware-facing components are bare-metal operations |
| Rings | Architectural dependency rings | Existing structural classifications; not OS-enforced protection domains |
| Promotion | Admission, activation, or graduation | Separate actions: accept a design, enable an instance, or move proven code into the product |

Use "system composition" in normative rules and "fascia" as an optional explanatory label. Use descriptive Rust package names until a registered-name amendment is adopted. A composition need not receive a new crate: a preset plus a façade over existing APIs may suffice. Create a crate only for a cohesive independently testable responsibility.

Core is a named foundational composition preset, on the same basis as the other systems. It is not Rust's `core`, the `core/` directory, `core-contracts`, a mandatory dependency, or an authority tier. Select its eventual package identifier during registry admission to avoid ambiguity; do not reserve bare `core` as a crate name here.

## 3. Separate architectural models

| Model | Shape and invariant | What it does not establish |
| --- | --- | --- |
| Capability/package dependencies | Directed acyclic graph; shared capabilities may have multiple consumers | Runtime instance sharing or authority |
| Composition expansion | Acyclic inclusion graph, flattened into a bounded resolved plan | Blanket access to all transitive operations |
| Resource lifecycle ownership | Tree or forest rooted at explicit host owners; shared services have one lifecycle owner | Exclusive use by one caller |
| Delegated authority | Explicit grants whose scope can only decrease on delegation | Permission inferred from a name, dependency, or reference count |
| Execution/data flow | May contain supervised feedback cycles with bounded queues and termination/cancellation policy | Permission for dependency cycles or unbounded recursion |

Do not require all five models to resemble a tree. Rings, compliance profiles, system roles, and runtime privileges are independent classifications. Existing profile and ring rules both apply until reconciled; a composition needing incompatible responsibilities must split them. A host composition root is an explicitly reviewed bootstrap location, not a privilege inherited by every façade.

## 4. Proposed rules and enforcement plan

All SC rules below are proposals. "Mechanism" names intended enforcement, not tooling already implemented. Existing Cratify requirements retain their current authority and documented enforcement limitations.

| ID | Proposed requirement | Mechanism / acceptance evidence |
| --- | --- | --- |
| SC-01 | Every named system has a registered ID, technical purpose, owner, and provided/required contracts. Name exceptions apply only to registered system identities. | Registry validation; duplicate and unknown-name rejection |
| SC-02 | A composition delegates capability behavior; no second maintained implementation or hidden fallback clone. | Ownership map and reviewer inspection; structural checks cannot prove semantic uniqueness |
| SC-03 | Dependency and inclusion graphs are acyclic and obey both approved ring and profile rules. | Manifest and Cargo metadata checks, including selected features and target matrix |
| SC-04 | Resolution uses explicit inputs and emits a deterministic plan before resource creation or execution. | Golden plans, bounded parser tests, zero-side-effect resolution tests |
| SC-05 | Each runtime resource instance has one lifecycle owner. Consumers receive explicitly injected handles. | Ownership table, startup/teardown and shared-instance tests |
| SC-06 | Effective grants are bounded by host policy, parent grant, and declared request. Delegation cannot amplify scope. | Typed restricted handles and host-side checks; denial and revocation tests |
| SC-07 | Presets cannot mint grants, load secrets, select undeclared endpoints, or start tasks implicitly. | Schema restrictions; explicit bootstrap binding and negative fixtures |
| SC-08 | Nested compositions and aggregate resource use are bounded before activation. | Depth/node/edge limits, aggregate budget admission, checked arithmetic |
| SC-09 | Every instance has declared startup, cancellation, fault, and teardown behavior. | Partial-start rollback, bounded shutdown, unavailable-provider and safe-hold tests |
| SC-10 | Compatibility is determined by versioned contracts and artifact evidence, not equal display names or lineage strings. | Positive and negative contract matrix, stale-plan rejection |
| SC-11 | All executions retain the resolved plan identity, capability versions, grant decision, and result provenance without secrets. | Structured evidence validation and redaction tests |
| SC-12 | Compliance decisions and execution approval remain separate; a composition cannot self-approve promotion. | Verdict consumer tests and explicit reviewer/host decision |
| SC-13 | Graduation follows Cratify section 6; Aaroneous never depends on devtools. | Product-native gates and removal of the maintained origin implementation at cutover |
| SC-14 | Each package declares one existing Cratify profile; naming and composition add no exemptions. | Profile-aware audit and review of bootstrap exceptions |

Code-level boundaries provide discipline, not isolation against hostile native code. Untrusted plugins require a separately reviewed process or sandbox boundary; Rust privacy and an in-process façade are insufficient. Dynamic loading is outside the first implementation.

## 5. Manifest and resolved-plan contracts

Design the schema before implementing a parser. The first version is data-only and closed to unknown fields. Do not permit embedded scripts, arbitrary shell commands, implicit filesystem discovery, or executable configuration. No runtime resource is constructed merely by parsing a manifest.

| Record | Required information |
| --- | --- |
| System registration | Stable system ID, display name, descriptive purpose, owner, naming decision, status |
| Manifest header | Schema version, composition version, base contract-set requirement |
| Capability binding | Capability ID, required contract version, explicit provider selection, typed configuration reference |
| Public operation | Input/output contract IDs, invoked capabilities, side-effect class, required grant scopes |
| Resource binding | Resource ID, lifecycle owner, shared/exclusive use, isolation domain, capacity, acquisition/release policy |
| Permission request | Operation and resource scopes, endpoint/path restrictions, delegation bounds; never a credential value |
| Budgets | Aggregate memory, concurrency, queue bounds, execution deadline, retry bound, nesting limits |
| Lifecycle | Startup dependencies, degraded behavior, cancellation deadline, compensating actions and non-reversible effects |
| Evidence | Contract tests, dependency admission references, rule-set version, supported feature/target matrix |
| Resolved plan | Exact providers and versions, flattened graph, complete effective configuration, resource bindings, required grants, input digest |
| Activation record | Host-issued grant binding, plan digest, artifact digest, instance identity, decision and lifecycle events |

Canonicalization must define field ordering, defaults, encoding, and digest domain separation. Reject duplicate IDs, cycles, unsupported versions, missing providers, unknown operations, conflicting bindings, and budget overflow. Distinguish Invalid, Unavailable, Denied, Incompatible, and ResourceExhausted outcomes. No silent provider substitution. Approved alternatives require explicit selection and a newly resolved plan.

Compose permissions by intersection of structured scopes, not string matching or set union. Parent budgets include descendants; shared services charge reserved capacity once and per-client demand against explicit quotas. A single process-wide singleton is not mandatory: separate tenant/security domains can require separate instances.

Initial prototype limits are design test parameters, not claimed optimal scale: maximum inclusion depth 8, 256 expanded nodes, and 1,024 edges; reject overflow before allocation beyond the configured bound. Tune only with measured workloads and an approved policy update. Execution memory/deadlines depend on the selected scenario and must be specified before its pilot.

## 6. Identity, Base36, and dependency reuse

Keep distinct types and namespaces for system ID, capability ID, contract version, preset version, artifact digest, source revision, build provenance, and runtime instance ID. Base36 is an encoding choice, not identity semantics, compatibility proof, authentication, or a replacement for Git.

Specify canonical case, width, padding, range, decoding errors, collision behavior, and registry ownership separately for each encoded ID. A truncated digest needs a documented collision budget and full-digest comparison on ambiguity. Existing 24-character lineage work is provisional input to review; do not assign its format to every identity. A message-derived hash does not identify source contents or provide a trustworthy build attestation.

Cargo can share compilation of compatible resolved packages, but versions, features, targets, profiles, and separate artifacts affect reuse. Statically linked executables may contain separate copies. Rust ABI compatibility is not promised between arbitrary builds. Inspect the resolved dependency graph and artifact size; do not promise that a dependency appears once everywhere.

Runtime reuse is a separate host obligation: inject a shared model service, pool, store, or executor where appropriate. Presets must not create 32 instances merely because 32 façades reference one capability. Measure instance count, peak memory, startup cost, and per-client isolation against a direct-capability baseline.

## 7. Named-system disposition

| Name | Proposed disposition | Evidence required before implementation |
| --- | --- | --- |
| Core | Foundational composition preset; ordinary registration and grants | Minimal useful composition of existing public contracts; no compulsory base dependency |
| Cratify | Compliance façade over reusable evaluators | Map current `cratify_core`, `cratify`, `ast_auditor`, and other auditors; establish actual overlaps and consumer migration |
| Chimera | Historical name candidate | Recover source-backed purpose, contracts, and current consumers; no invented responsibility |
| Marionette | Historical name candidate | Same admission evidence |
| Merlin | Historical name candidate | Same admission evidence |
| Foundry | Future general software-construction composition | Proven capabilities and section-6 graduation; owner-specific operations stay in devtools |

Do not create six empty crates. Prefer one small read-only Cratify-oriented composition as the first candidate, subject to public-contract inventory. Core may be the second proof of reuse once its purpose is concrete. Names are not a delivery roadmap or assurance claim.

`cratify_core` remains a candidate reusable rule engine; `cratify` a candidate primary façade; `xtask` coordinates repository gates and propagates failures. `ast_auditor` remains supported until consumer evidence permits retirement. Reconcile the spec's existing `control` classification for `cratify_core` before deciding which layers can depend on it. The façade must not create a duplicate rules engine or embed private queue policy.

## 8. Parallel-development arrangement

Use an isolated experiment workspace under devtools only after design admission. It consumes an explicitly supplied, immutable, verified Aaroneous checkout. A local path dependency is acceptable only when that checkout is pinned and read-only to the composition lane; a live sibling path to the active base branch is not a stable integration boundary. Record source revision, clean-tree status, lockfile digest, toolchain, target, features, and contract inventory.

| Lane | Owns | Cannot silently change | Handoff |
| --- | --- | --- | --- |
| Base | Existing capability implementations and product contracts under exact claims | Composition assumptions or consumer compatibility | Reviewed contract set, verified revision, migration notes |
| Composition | Experiment manifests, adapters, façades, and contract-consumer tests | Product source or underlying business logic | Candidate plan, contract gaps, test and resource evidence |
| Review/integration | A bounded compatibility evaluation against a candidate base | Either lane's shared files outside its claim | Accepted pin update or rejection with evidence |

These are proposed work lanes, not agents launched or active claims. Follow queue capacity before allocating them. Use separate branches/worktrees and build directories. Test reports bind to exact revisions. A base change never silently updates the consumer pin.

For a missing contract, file a bounded proposal containing the use case, existing API limitation, proposed signature/schema, compatibility impact, target profile, and acceptance test. Keep the consumer blocked on that operation; an adapter may translate an existing contract but cannot implement missing capability behavior. Base work remains possible while the proposal is reviewed.

## 9. Delivery sequence and gates

| Stage | Deliverable | Exit condition / stop condition |
| --- | --- | --- |
| 0 — Preserve | Record C61 provisional state and current ownership; no construction | No assumption that prior partial gates certify current HEAD |
| 1 — Reconcile rules | Product amendment proposal for naming, composition, profile/ring precedence, and bootstrap ownership | All conflicting normative statements have explicit proposed replacements and reviewer decisions |
| 2 — Inventory contracts | Existing APIs, named-system history, capability ownership and resource map | One viable scenario using current public APIs; missing contracts recorded separately |
| 3 — Specify contracts | Manifest, resolved plan, grants, budgets, lifecycle, identity semantics | Valid/invalid examples cover SC-01 through SC-14; policy decisions approved |
| 4 — Implement resolver | Data-only resolver/validator in isolated devtools experiment | Deterministic bounded plans, negative fixtures, no resource creation or product edits |
| 5 — Read-only pilot | One façade using injected existing services | Same results as direct capability calls, denied operations fail, resource count verified |
| 6 — Reuse and scale | Second composition shares a service under separate grants | No duplicated implementation; budget, contention, cancellation, isolation tests pass |
| 7 — Integrate candidate base | Explicit pin update and compatibility matrix | Both lanes' gates pass on recorded revisions; rollback to prior pin proven |
| 8 — Graduate | General code rewritten behind product-defined contracts and landed inert | Native product gates, admission records, reviewed activation, origin switches to product implementation |
| 9 — Operate and ratchet | Bounded rollout, audit records, deprecation and regression policy | Observed behavior meets predeclared budgets; new violations become enforceable rules |

Stage 1–3 are the next work, not permission to skip directly to code. Completion of this blueprint is distinct from approval of all policy choices. First resolve naming/ring amendments, public API readiness, and the execution-host authority contract. Numeric scale targets are accepted for the concrete pilot before performance claims.

## 10. Verification, rollout, and rollback

Resolution tests cover cycles, diamond dependencies, repeated instances, version conflicts, unsupported schemas, resource conflicts, and limits. Authorization tests cover privilege escalation through nesting, confused-deputy calls, revocation, stale grants, and cross-tenant access. Runtime tests cover partial startup, provider loss, queue pressure, cancellation, teardown while another consumer remains active, and retries with idempotency keys where effects require them.

Compile-time Cargo features are not runtime authorization. Re-resolve on changes to providers, configuration, grants, features, or contracts. Bind activation to the evaluated artifact and plan to prevent a checked plan being replaced before execution. Record provenance and structured failures; logging never substitutes for enforcement.

The pilot is read-only and inert by default. Promotion and source mutation require their own workflow authority and evidence. For stateful upgrades, agree schema migration and shadow parity criteria before rollout; do not run duplicate side effects in shadow mode. Rollback must account for data compatibility and irreversible external effects, not just restore a Git pin. Unsupported rollback prevents activation until recovery is designed.

## 11. Required policy decisions and document amendments

1. Amend generic-naming rules to recognize registered system names while preserving descriptive internals and the prefix-stutter prohibition.
2. Reconcile architectural rings with Cratify v2 profiles. Until explicit precedence is adopted, satisfy both and flag contradictions; do not invent a new privileged ring for fascias.
3. Specify the host authority/grant issuer and how scoped handles are enforced and revoked. Core is not implicitly that issuer.
4. Approve one read-only pilot and measurable budgets after inventory; historical system names remain unassigned until evidence is recovered.
5. Decide product publication paths and reviewer ownership for the approved specification and registry. This proposal then becomes a planning/history pointer, not competing product policy.

The current topology document prohibits dynamic scripting wrappers and monolithic coupling. A typed façade over injected capabilities is the proposed mechanism; an accepted amendment must make that distinction explicit. The current spec still describes hypervisor binaries as the composition root, while the interrupted refactor moved binaries: reconcile the actual reviewed topology before assigning exemptions.

No implementation is certified by this blueprint. Acceptance requires a reviewed specification, explicit contract/budget decisions, and recorded enforcement status. Independent review remains required before the queue item can be marked Done.

## 12. Cross-repository applicability

Owner-directed planning addendum, 2026-09-25. The principles extend beyond compositions, but their implementation requirements remain proportional to the repository and execution context. This addendum does not adopt new product laws or certify existing code.

| Scope | Requirements | Authoritative home after adoption |
| --- | --- | --- |
| Shared engineering principles | Explicit ownership and authority, controlled resource sharing, bounded execution, distinct identity/compatibility/authenticity, reproducible evidence, deliberate integration, recovery planning, one maintained implementation per responsibility | Existing foundation boundary and linked product/companion rules; designate one canonical source per shared rule |
| Aaroneous product | Cratify floor and profiles, product dependency constraints, deterministic reducers, kernel hot-path requirements, native admission and verification | Aaroneous product governance and architecture documents |
| Devtools operations | Explicit operational inputs, schedules, queues, review coordination, experiments and companion verification; ordinary allocation, async I/O and synchronization allowed | Devtools operating model and operational governance |
| Composition-specific | Registered names, preset expansion, composition manifests, nested grant and budget accounting | Product composition specification after adoption; experiment contract before graduation |

Do not impose composition packaging on every utility or kernel constraints on all maintenance code. Devtools flexibility does not relax ownership, credentials, spending, evidence, or review boundaries. Devtools may consume Aaroneous; Aaroneous must build, test, release and run without devtools.

Existing explicit-input, ownership, evidence, dependency-direction and graduation obligations remain binding. Common grant schemas, composition manifests and new naming provisions remain proposed. Audit code before labeling missing proposed machinery as an existing compliance violation.

## 13. Correction program

This six-phase program groups the detailed delivery stages in section 9; it does not introduce a competing roadmap. Some review findings are confirmed documentation conflicts, while others are design gaps or risks requiring inspection. Record that distinction for each item.

| Phase | Correction and scope | Exit evidence | Section 9 alignment |
| --- | --- | --- | --- |
| 1. Reconcile governing documents | Separate shared principles, product requirements and companion requirements. Resolve naming, rings, profiles and permissions; propose exact amendments and one authoritative home per rule. | Reviewed amendment packet and explicit decisions for R1, R2 and R12; no unresolved contradictory instructions for the pilot | Stages 1–3 |
| 2. Audit implementation | Inventory capability ownership, public contracts, resource creation, execution hosts and permission checks in both repositories. Classify confirmed defects, missing enforcement, and future requirements separately. | Source- and revision-backed findings; bounded queue candidates with exact ownership; concrete pilot API inventory | Stages 0–3 |
| 3. Correct existing boundary violations | Prioritize unintended authority, hidden resource creation, unsafe lifecycle behavior and unreliable verification. Reconcile the interrupted refactor while preserving behavior and test coverage. | Regression evidence for confirmed defects, applicable native gates, verified base revision and recovery notes | Stage 0 baseline completion, prerequisite to stages 4–7 |
| 4. Specify and enforce composition contracts | Define manifests, resolved plans, ownership, grants, identity and lifecycle contracts. Place reusable evaluation in appropriate Cratify components; keep xtask coordinating gates. Introduce measured baselines where immediate enforcement is infeasible. | Approved contracts, negative fixtures, recorded enforcement status, non-increasing violation baselines and explicit host enforcement points | Stages 3–4 |
| 5. Prove with read-only compositions | Run one small composition against an immutable verified base, then a second consumer sharing an existing service under separate grants. | Correct results, denied unauthorized operations, bounded resources, deterministic resolution, cancellation, provenance, sharing and isolation evidence | Stages 5–7 |
| 6. Graduate and roll out | Graduate proven general mechanisms to Aaroneous; devtools consumes them and retains operational configuration. Require migration and recovery evidence for stateful or mutating workflows. | Admission records, product-native gates, reviewed activation, origin implementation retirement, rollout and recovery results | Stages 8–9 |

Phases 1 and 2 may proceed concurrently under non-overlapping documentation and inventory claims. Confirmed safety defects may receive separately authorized remediation claims before the entire design is finished; this is not permission to construct the new layer. Phase 4 implementation waits for accepted contracts, and phase 5 waits for the verified base and relevant enforcement. No new implementation lane is opened by this documentation update.

### Correction record and prioritization

Each correction record must contain: finding ID; classification (confirmed defect, enforcement gap, or future requirement); source rule and current enforcement status; concrete failure case and source location; inspected revision and reproduction evidence; severity; named owner and reviewer; exact paths; dependencies; acceptance test; migration/rollback approach; and closure evidence. An unverified risk becomes an investigation item, not a claimed implementation defect.

Prioritize authority violations and unsafe execution first, followed by lifecycle/resource defects and untrustworthy verification. Resolve documentation contradictions before they guide new construction. Address naming, packaging and convenience changes after their semantics are settled. Do not weaken a rule simply to obtain a passing gate; record existing violations and a ratchet where permitted by governing policy.

Completion means the applicable acceptance tests pass on the recorded revision and review is recorded. A drafted rule, successful compile, or author self-review alone does not close a correction.

### Immediate next packet

Produce a policy-amendment packet plus a current-state inventory: exact proposed normative replacements and their publication paths; source-backed ownership/resource/authority maps for both repositories; one read-only scenario using existing public contracts; separate missing-contract proposals; and bounded correction candidates using the record above. Check queue ownership before proposing exact edit scopes. This packet is the next planning deliverable, not an implemented registry, resolver, grant service or composition runtime.
