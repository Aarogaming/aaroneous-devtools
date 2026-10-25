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
