# Guild operation v1: contract reconciliation proposal

Status: **PROPOSED, NOT IMPLEMENTED, NOT ADOPTED**. Semantic design revision 2,
2026-10-23 UTC; continuation of `C158/codex-guild-blueprint-v1`.
This is not a wire schema, authority grant, new queue, or deployment instruction.
Independent review must settle the decisions in section 8 before implementation.

Purpose: carry an owner's bounded intent through attempts, evidence, review and
recovery without relying on the original chat, model or machine. Reuse the
existing records below; do not replace them with a universal component trait.
The [initialization plan](../architecture/GUILD_INITIALIZATION_PLAN.md) owns
qualification order; the [dated comparison](../inventories/GUILD_READINESS_2026-10-20.md)
retains the four supplied documents and their source hashes. This contract
specifies only the missing operation-level relationships and guard conditions.

## 1. Existing records and precise gaps

READ evidence at devtools main `a262a04bd4b688c3ebd96e6b7774297b523f1646`.
Adapter sources are separately pinned: PR203
`9eb3a82ea5bbdc162707b7318cace5c3b342aca4`, PR204
`e1e0d1a5cd9205384f95772c1b41a9a26a8d4a7a`, PR206
`00e6e1f06a6e323fb5ff3ee2fa31e7436e973747`. All three were open at inspection;
their existence is not evidence of integration, deployment or new test execution.

| Existing source / owner | Preserve | Missing operation-level relationship |
| --- | --- | --- |
| [Intake proposal](intake-proposal-v1.schema.json) | Source, bounded file scope, dependencies, proposed checks and decision text | Admission principal, immutable intent revision, authorized effects and aggregate budget; intake text is not a grant |
| [Contribution envelope](contribution-envelope-v1.schema.json) | Full contribution identity, producer, origin, target, evidence key, commands and review references | Operation/attempt association; a claimed review field is not authenticated acceptance |
| [Virtual patch set](virtual-patch-set-v1.schema.json) | Task and patch identity, base/file digests, provenance; original immutable bytes | Bind an artifact to the admitted intent and attempt without rewriting signed or hashed content |
| [Verification core](../foundation/crates/steward/src/verification.rs), `Request`, `Admission`, `Outcome`, `evidence` | Request ID binds repo/head/base/platform/suite/digest; admitted node/attempt; conflict and stale-result rejection | Request is one verification task, not an owner's whole operation; result eligibility is not effect fencing |
| [Wire codec](../foundation/crates/steward/src/wire.rs) | Strict version/field validation; signature trust supplied by verifying adapter, never the packet | Preserve v1 bytes and identifiers; adding operation fields directly would break strict old readers |
| PR203, `steward/src/local_verification.rs`, `Store::claim/recover`, receipt validation | Per-request attempts, exclusive claim, explicit recovery, pinned suite and signed log evidence | No aggregate operation budget or complete descendant cancellation; a stored receipt can report failure even when the command exits 0 |
| PR204, `steward/src/local_authority.rs`, `collect` | Trusted-base roles/policy, independent signed review, immutable candidate context, replay high-water state | Integration eligibility remains separate from an operation's progress; collector does not perform integration |
| PR206, `steward/src/job_service.rs` and `contracts/platform-jobs-v1.md` | Existing spool authority, authenticated claim/result transport, retained acknowledgement and idempotent re-upload | No automatic lease expiry/failover or general operation lifecycle; successful upload is not successful work |
| [Maintenance recovery](maintenance-recovery-v1.md) and [topic lifecycle proposal](constellation-lifecycle-v1.md) | Corrupt-state holds and explicit controls; proposed topic refinement retains history | Pipeline stage, topic review window, queue claim and operational attempt are different clocks/state machines |

The maintenance recovery document's C114 statement that rename ensures power-loss
durability is not an accepted premise here. Atomic replacement and durable commit
need separate failure-model evidence. Route correction through its own claim;
this proposal does not silently edit that existing contract.

Reproduce adapter references with `git show <pinned-head>:<path>`, using prefix
`foundation/crates/` for the Rust paths above. Their governing documents are
`governance/LOCAL_VERIFICATION_GATE.md` and
`governance/LOCAL_INTEGRATION_AUTHORITY.md` at the corresponding pins.

## 2. Identity and ownership, not another authority ledger

An operation links existing authoritative facts. It does not copy a reviewer
verdict into a writable boolean or make a dashboard authoritative. An operation
coordinator owns operation admission/progression; existing spool, review and Git
authorities continue to own their facts. These are responsibilities, not newly
enrolled service accounts. Host selection confers no integration authority.

| Proposed fact | Writer / rule |
| --- | --- |
| Operation identity | Coordinator-issued ID namespaced by authority and repository; never just a short queue C-number or Base36 display label |
| Intent revision | Requester proposes; authorized admission freezes desired outcome, acceptance, source references, read/write scope, prohibited effects, trust class and budget; changed meaning requires a new admitted revision |
| Admission reference | Authorized issuer supplies authenticated grant/policy identity and digest; coordinator verifies it independently of requester prose |
| State revision | Single logical operation writer advances by compare-and-swap against expected revision; stale or conflicting updates reject without replacement |
| Operation attempt | Coordinator allocates a monotonic attempt identity within the operation; previous history/cost never resets |
| Verification association | Coordinator records immutable association of operation/attempt to existing request ID and spool attempt; these counters are not interchangeable |
| Artifact/evidence references | Producers submit immutable digest-bound artifacts; trusted adapters validate them; evidence describes its subject, not global success |
| Review and effect references | Existing acceptance/publication authorities own signed decisions and observed outcomes; coordinator records references, not substitute verdicts |

One operation attempt may require several platform requests. A successful earlier
attempt remains historical evidence; it cannot satisfy a changed head, policy,
suite, required platform set or newer admitted verification attempt. Sharing one
verification result between operations requires explicit compatibility and
fresh authority evaluation; it does not grant either operation execution rights.

For the first pilot, prohibit nested operations and cross-operation reservation
sharing. Later child operations must link the parent's reservation, have no
broader authority, and retain aggregate costs and ancestry across retries.

Existing strict signed records remain byte-for-byte intact. The initial proposed
adapter uses an authenticated, versioned association record outside candidate
storage; its serialization/signature namespace must be reviewed before coding.
An operation ID supplied by a worker cannot create that association.

The proposed association signer is a registered coordinator principal. Its key,
role, allowed repositories and revocation state must come from the policy-owner
approved trust registry supplied to the verifier, never from candidate storage
or the association itself. Actual enrollment, key identity and signature
namespace remain unapproved deployment/schema inputs; no identity is created
by this proposal.

## 3. Independent state dimensions

Avoid a single enum that treats verified, approved, merged and deployed as
successive synonyms. Track these dimensions with references to their authorities:

- Execution phase: `proposed`, `admitted`, `active`, `blocked`, `paused`,
  `cancelling`, `reconciling`, `finished` or `rejected`.
- Attempt disposition: absent while unresolved; exactly one final
  `succeeded`, `failed`, `cancelled` or `abandoned` record. A correction appends
  a disputed/superseding record; it never overwrites historical disposition.
- Acceptance: `not_requested`, `pending`, `accepted` or `rejected`, qualified by
  candidate/intent/policy and actual independent-review evidence.
- External effects: separate intended/dispatched/confirmed/failed/unknown records
  for publication, integration or deployment; each has its own authorization.

A report-only operation can finish without a PR. An artifact-ready attempt may
succeed while operation acceptance remains pending. A failed attempt need not
finish its operation; a separately authorized retry creates a new attempt.
Overall success requires the admitted acceptance contract and all required
effects confirmed, not merely `execution=finished`. Show the relevant pending
or unknown dimension in the UI instead of a misleading green summary.

Current effect eligibility is a pure function of the exact subject (including
intent, head/base, suite and platforms), policy, current authority set and
explicit time, using authenticated evidence references as supplied inputs.
It is evaluated at every effect dispatch and never persisted as an eligibility
boolean on an acceptance record. Historical acceptance remains immutable.
The dispatch adapter must bind the evaluated revisions to its dispatch boundary;
an intervening revocation/change requires another evaluation or a hold.
If that ordering cannot be established, it must not dispatch.

For the report-only pilot, admission must name a policy-owner-authorized report
reviewer principal, independent of the implementer/producer under existing review
rules, and the exact report/evidence criteria. That reviewer signs a decision
bound to operation, intent revision, artifact digest and policy; the coordinator
only records its reference. Missing identity, signature format or criteria blocks
admission. No PR is required, but neither the worker nor an exit code accepts its
own report. Selecting the actual principal/format remains a review prerequisite.

### Transition guards

These proposed transitions require authenticated actors, expected state revision,
valid references and current authority. Otherwise reject the event; do not
silently coerce it into another transition. Observational evidence may still be
retained without granting permission to act on it.

| ID | Event / source phase | Guard and result |
| --- | --- | --- |
| T01 | Submit / no operation | Validate bounded intent and deduplication key; enter proposed, no execution |
| T02 | Admit / proposed | Existing issuer authority, scope, named acceptance authority, policy and budget valid; freeze intent, enter admitted |
| T03 | Reject / proposed | Record reasons and requester identity; enter rejected without candidate mutation |
| T04 | Begin attempt / admitted or blocked | Initial attempt only; blocker resolved, no previous attempt or unresolved writer/effect, reservations available. Use the sole allocation operation below; enter active |
| T05 | Block / admitted or active | Enter blocked, retain cause/evidence and reservations, stop new dispatch; active effects must be stopped/fenced before reservation release |
| T06 | Pause request / admitted or active | Stop new dispatch and record pause pending. If no open attempt/effect/reservation, or retained executor is provably quiescent and resumable with all effects known, enter paused; otherwise enter reconciling without releasing unknown resources |
| T07 | Resume / paused or blocked | Require no cancel pending; revalidate subject, policy, authority, budget and resolved blocker. Successful resume clears pause pending. No open attempt or unresolved effect/reservation: enter admitted. Retained open attempt with proven resumable quiescence/ownership: resume same attempt and enter active. Otherwise reject; T11 is the recovery route. Never allocate an attempt here |
| T08 | Cancel request / proposed, admitted, active, blocked, paused, cancelling or reconciling | Stop dispatch; record cancel pending (takes precedence over pause). No open attempt/effect/reservation: enter finished with operation cancellation reason, no fabricated attempt. Unknown effects: enter reconciling. Otherwise enter cancelling and retain resources |
| T09 | Cancel acknowledged / cancelling | Unknown outcome: enter reconciling. Otherwise require all workers stopped/fenced and effects reconciled; settle reservations, finalize any open attempt cancelled, enter finished with operation cancellation reason. Preserve already-final dispositions |
| T10 | Worker result / active | Authenticate full subject and current request/node/attempt; retain evidence, remain active. Finalize attempt only when all required work/effects are resolved; T16 separately finishes execution |
| T11 | Timeout/disconnect/crash / active, blocked, paused or cancelling | Enter reconciling, preserve evidence and reservations, block conflicting work. Lack of acknowledgement never proves failure |
| T12 | Reconcile / reconciling | Require proved outcomes and writer exclusion, or proved resumable quiescence under retained ownership. Append resolution and settle only known resources. Cancel pending: enter cancelling. Otherwise pause pending: enter paused only with no open attempt or a proven resumable one and no unknown effects. Otherwise enter blocked. Nonresumable open attempts require an evidence-bound final disposition before leaving reconciliation |
| T13 | Retry / admitted or blocked | Prior attempt final failed/abandoned; no open attempt or unresolved writer/effect; changed hypothesis or eligible transient cause, remaining aggregate budget, fresh authority checks. Use T04's sole allocation operation and enter active; no reset of history/cost |
| T14 | Independent judgment / active, blocked, paused or finished | Require artifact-ready authenticated evidence; named review authority evaluates exact subject and independence; append acceptance reference, retain source execution phase. Never auto-publish or cache eligibility on the acceptance record |
| T15 | Publish/integrate/deploy / active | Scoped grant, unchanged subject, required gates, reserved effect resources and concurrency control; persist intent, remain active. Immediately before dispatch re-evaluate pure eligibility and current execution/cancel state. Denial withdraws an undispatched intent without calling target. Unknown already-dispatched outcome uses T11/T12 |
| T16 | Finish / active or blocked | Require at least one attempt, all attempts final, effects resolved, reservations settled and no live descendants; enter finished with operation-level reason and separate acceptance references. Missing required confirmation cannot produce success |

### Invariants and complete phase/event coverage

Every committed transition must preserve at most one attempt without a final
disposition per operation, monotonically increasing state/attempt revisions,
retained settled cost and outstanding reservations, and no dispatch after a
pause/cancel hold. Failed guards leave authoritative state unchanged. T04 and
T13 invoke one allocation operation: atomically check expected revision, absence
of an open attempt, writer/effect exclusion and available budget, then reserve
and allocate exactly one attempt. T13 adds retry guards, not another allocator.
Admission creates no execution reservation; acquisition starts at allocation.

An operation finalization reason is separate from attempt disposition. Cancelling
before any attempt/reservation/effect exists finishes directly; it creates no
attempt, receipt or imaginary resource release. Cancellation of completed work
does not rewrite that work's historical result. Pause/cancel intent and retained
executor ownership must be explicit state, not inferred from the phase label.

The matrix covers all 160 pairs of ten source states (including no operation)
and sixteen events. G means apply that transition's guards and named destination;
R means explicit rejection without state change. A failed G guard also rejects.
Unknown events/phases reject. Duplicate submission is resolved by intake
deduplication before this matrix: identical intent returns the existing handle
without emitting T01 again; conflicting intent rejects. Late evidence outside
T10's active phase may be retained as historical evidence but cannot mutate
execution state. T14 in finished changes review references only, not final work.

| Phase | T01 | T02 | T03 | T04 | T05 | T06 | T07 | T08 | T09 | T10 | T11 | T12 | T13 | T14 | T15 | T16 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| none | G | R | R | R | R | R | R | R | R | R | R | R | R | R | R | R |
| proposed | R | G | G | R | R | R | R | G | R | R | R | R | R | R | R | R |
| admitted | R | R | R | G | G | G | R | G | R | R | R | R | G | R | R | R |
| active | R | R | R | R | G | G | R | G | R | G | G | R | R | G | G | G |
| blocked | R | R | R | G | R | R | G | G | R | R | G | R | G | G | R | G |
| paused | R | R | R | R | R | R | G | G | R | R | G | R | R | G | R | R |
| cancelling | R | R | R | R | R | R | R | G | G | R | G | R | R | R | R | R |
| reconciling | R | R | R | R | R | R | R | G | R | R | R | G | R | R | R | R |
| finished | R | R | R | R | R | R | R | R | R | R | R | R | R | G | R | R |
| rejected | R | R | R | R | R | R | R | R | R | R | R | R | R | R | R | R |

This is a proposed specification matrix, not an executed reducer test. After
independent acceptance, generate the first reducer fixture from all pairs, then
test both passing and failing guards and every conditional destination. Matrix
coverage alone does not prove guard exclusivity, runtime totality or safety.

Scope conflicts are checked on actual canonical resources, not just spelling of
paths. Aliases/symlinks, shared output directories and branch/ref writers must not
create two supposedly independent owners. Do not add automatic takeover to the
current spool: it presently requires explicit recovery after stopping the old
worker. A future lease epoch is useful only if every affected adapter enforces it.

## 4. Budgets, cancellation and ambiguous effects

Budgets are explicit per operation and host: elapsed-time deadline, attempts,
concurrency, CPU/memory, process count, disk/log bytes, and model tokens/cost when
model use is admitted. Disabled capabilities have zero authorization, not an
unlimited or default budget. Units, limits, accounting owner and uncertainty
policy must be fixed in the admitted recipe; no arbitrary global numbers here.

Reserve before dispatch. Outstanding reservations plus settled consumption may
not exceed the authorized total. Retries and eventual children debit the same
total. Unknown usage remains reserved until reconciled; new IDs cannot erase it.
Release requires evidence that the corresponding effect ended or was contained.
Overflow, missing accounting, stale grant or backward/ambiguous time causes a
hold, not free capacity. Clock acquisition belongs in adapters; reducers receive
explicit time. Restart retains original deadlines rather than extending them.

Cancellation is a protocol, not a status string. Stop admission of descendants,
signal existing work, confirm termination or effective fencing, reconcile remote
effects, then settle resources and disposition. A late valid receipt remains
historical evidence but cannot restart a cancelled operation or publish a result.
If a local executor cannot prove descendant cleanup, retain the hold and require
operator inspection; do not advertise automatic failover.

For publication, persist an intent ID bound to target, subject and authorized
effect before making the call. On a lost reply, inspect that target and retained
receipt before retrying. A matching observed effect confirms the existing intent;
a conflicting or unknowable result stays in reconciliation. This is not a claim
of exactly-once delivery. Replaying recorded events rebuilds state but must never
repeat external calls automatically.

Revoking acceptance authority after intent persistence but before dispatch must
withdraw the undispatched intent and deny the call. If dispatch may already have
occurred, retain an unknown outcome and reconcile through T11/T12 rather than
claiming withdrawal undid it. Execution-grant revocation and acceptance-authority
revocation are distinct inputs to the same dispatch eligibility decision.

## 5. Enforcement ownership and qualification cases

Proposed implementation responsibility: pure admission/state/budget decisions in
the existing Steward policy boundary; persistence, process/network control and
resource checks in Foundation/adapters. Reuse existing verification, wire,
receipt and independent-review validators. `xtask` remains a product gate entry
point; no new crate, universal trait or alternate scheduler is required here.

All cases below are **specified, NOT RUN as operation-contract tests**. Existing
unit tests may cover constituent checks; that does not establish the composition.
Future implementations must convert these cases into executable fixtures and
retain exact command/head/environment evidence. Text checks in this PR validate
the specification's structure, not any behavior below.

| Case | Input or fault | Required observable result | Owning boundary |
| --- | --- | --- | --- |
| OCV01 | Valid report-only intent and authorized admission | Durable handle; no PR requirement; evidence-bound final result | Admission/reducer |
| OCV02 | Missing acceptance, unknown version, oversized request or forged grant | Rejection with reason; zero candidate effects | Intake/admission |
| OCV03 | Same request key and identical intent delivered twice | One operation and one reservation; return existing handle | Admission/store |
| OCV04 | Same key with different intent bytes | Conflict; preserve first intent; no overwrite | Admission/store |
| OCV05 | Stale expected revision or two simultaneous begin events | At most one committed transition/reservation; stale update refused | Coordinator/store |
| OCV06 | Base/head/suite/policy changes after evidence | Old evidence retained but insufficient for changed subject | Verification/authority |
| OCV07 | Worker supplies signature_verified, operation ownership, or publisher role | Reject self-assigned trust; no privilege increase | Wire/auth adapter |
| OCV08 | Prior-attempt pass arrives after a newer attempt starts | Historical receipt cannot make current attempt pass | Existing verification plus association |
| OCV09 | Native run exits 0 but receipt verdict is fail/error | Operation never reports verification success from process exit alone | Receipt consumer |
| OCV10 | Implementer signs own acceptance, or required reviewer is silent | Acceptance remains withheld under existing independence rules | Review authority |
| OCV11 | Retry or new task ID tries to reset spent/unknown cost | Same ancestry accounting; exhausted budget blocks dispatch | Budget/admission |
| OCV12 | Grant revoked after queue admission | Next effect denied; in-flight effects stopped/fenced or reconciled | Effect adapter |
| OCV13 | Cancellation races successful worker result | One deterministic recorded resolution; no post-cancel publication | Reducer/effect adapter |
| OCV14 | Disconnected worker may still be running | No overlapping reassignment before exclusion proof; visible hold | Execution/recovery |
| OCV15 | PR creation succeeds but acknowledgement is lost | Reconcile existing effect, never blindly create a duplicate | Publisher/outbox |
| OCV16 | Crash before/after intent commit, dispatch and acknowledgement | Replay distinguishes absent, pending and unknown effects; no silent success | Store/outbox |
| OCV17 | Disk full, partial checkpoint, corrupt history, deleted high-water state | Evidence-preserving recovery hold; no automatic fresh history | Store/recovery |
| OCV18 | Symlink/path alias or shared ref disguises overlap | Conflict before mutation despite different input spelling | Scope/OS adapter |
| OCV19 | Restart, clock rollback or budget arithmetic overflow | Original deadline/accounting retained or explicit hold; no extra budget | Time/budget adapter |
| OCV20 | Legacy receipt with no authenticated operation association | Inspect/import as historical evidence only; no new authorization | Migration adapter |
| OCV21 | Replay after cancellation or projection deletion | Rebuilt state matches authoritative events; zero external redispatch | Replay/projection |
| OCV22 | Two authorized operators hand off on different hosts | Successor can identify exact state, evidence, budget and next permitted step without chat history | End-to-end handoff |
| OCV23 | Pause/resume and block/resume, with no attempt, resumable attempt or uncertain executor; reconciliation with pause/cancel pending | Explicit T07/T12 destination; same attempt resumes only with ownership/quiescence proof; unknown work never implicitly re-executes | Reducer/recovery |
| OCV24 | T04/T13 compete while an attempt is open, then retry after final disposition | Every committed transition preserves at most one unfinished attempt; one allocation operation, monotonic identity and retained cost | Reducer/store |
| OCV25 | Cancel proposed/admitted work before allocation, and cancel with reserved or unknown work | Empty work finishes directly with operation reason and no fabricated attempt; reserved work cancels/reconciles before release | Reducer/accounting |
| OCV26 | Acceptance authority revoked between persisted intent and dispatch, or after dispatch with lost acknowledgement | Re-evaluate eligibility with explicit time/current authority; deny and withdraw only undispatched intent; otherwise reconcile; retain historical acceptance | Pure eligibility/effect adapter |

## 6. Compatibility and recovery

First adoption is observational: compare proposed operation decisions against the
existing authority without dispatching effects. Retain original schemas, request
IDs, attempt numbers, signed bytes, namespaces, artifact digests and directories.
Do not broaden v1 parsers, renumber existing attempts or regenerate signatures.
Unknown or conflicting associations must fail closed, not guess from filenames.

Legacy history is importable only with provenance and explicit unknown fields.
Missing original authorization, cost or subject cannot be invented retrospectively.
An authorized new admission can reference historical evidence after revalidation;
it cannot turn that evidence into proof of previously authorized execution.

Before changing any writer: back up state, prove clean-host restore, record format
compatibility and old-reader behavior, compare shadow decisions, then obtain a
reviewed cutover. Stop admission for rollback; preserve newer attempts/effects and
high-water marks. If the old reader cannot understand new state, stay stopped or
use a reviewed forward recovery; never restore an old snapshot over newer history.

One physical Ubuntu coordinator is initially a single point of failure. Windows
reports unavailable while it is down; becoming a second writer requires explicit
fencing/recovery evidence and authority. Neither a dashboard nor a healthy advisory
Round Table is an alternate coordinator.

## 7. Smallest implementation sequence after review

1. Resolve section 8 and freeze language-neutral association/intent/event fixtures.
   Preserve current signed protocols; choose serialization and version negotiation
   deliberately before a parser or a signing namespace becomes public behavior.
2. Implement a pure operation reducer and bounded reservation rules in the existing
   policy boundary with OCV01-13/19/23-26 fixtures; begin with all 160 phase/event
   pairs, guard failures and conditional destinations; no file, process or
   network effects. Adapter fault qualification for these cases remains separate.
3. Bind one report-only operation to existing request/receipt evidence in shadow
   mode. Reject legacy ambiguity and altered subjects; do not fork receipt writers.
4. Qualify durable single-writer storage, cancellation and outbox adapters with
   OCV14-21 fault injection on native Windows and Ubuntu before effectful use.
5. Demonstrate OCV22 with a different authorized operator; then propose bounded
   mutation/publication separately. Policy, enrollment and running-agent upgrades
   remain separately reviewed changes, never self-approved improvements.

These are dependency-ordered acceptance slices, not Ready claims or assigned work.
The existing queue controls capacity. No new ongoing monitor/model loop is created.

## 8. Review decisions and adversarial critique

Review boundary (2026-10-22):
[Claude's acceptance of PR243 at acc8769d](https://github.com/Aarogaming/aaroneous-devtools/pull/243#issuecomment-6331860889)
explicitly covers documentation/proposal status. Its reported contract reading
covers sections 1 and 2; it does not record decisions on the items below or a
deep review of sections 3-8. Preserve that acceptance as evidence for its stated
scope, not as a contract freeze, runtime adoption or approval of a later head.
Subsequent [Claude delta review at c83161bd](https://github.com/Aarogaming/aaroneous-devtools/pull/243#issuecomment-6345051894)
read sections 3-8, accepted documentation, and supplied design dispositions:
D01 accepted in principle; D02 sequencing accepted, codec deferred; D03 shadow
accepted, execution deferred; D04 direction accepted, engine/protocol deferred;
D05 requires revision. One independent family is not final design approval.
This revision responds with explicit transition destinations, global invariants,
the 160-pair matrix, OCV23-26, association trust and report-review ownership,
and dispatch-time pure eligibility. These changes need fresh independent review
and a second independent family before the schema slice is claimed.

D02 exit evidence must compare reuse/extension of the existing strict
`steward/src/wire.rs` codec against a separate versioned association codec;
show why the existing codec cannot carry the required facts unchanged before
choosing another. Do not broaden existing v1 readers or signatures.
D04's storage design must assess reuse of PR242's checkpoint validation and
negative cases within the proper ownership boundary, not invent a parallel
validation policy or equate its current tests with power-loss qualification.

The owner has authorized preparing this groundwork, not bypassing independent
review or enabling its runtime. Reviewers must record accept/revise/defer for:

- Whether an authenticated association outside the existing receipt is the smallest
  compatible extension; compare a formally versioned packet migration before choice.
- Exact intent/association/event schema, ID collision rules, framed canonical
  encoding, signature namespace and maximum field sizes. None is silently frozen here.
- Pilot repository, allowed recipe, trust class, budget units/limits and policy
  authority. Public-fork execution and recursive decomposition remain disabled.
- Persistence ownership and durable commit/recovery protocol; avoid introducing a
  second spool or granting the operation coordinator control of acceptance records.
- Review freshness and operation completion semantics for report-only versus
  mutation requests; do not let stored accepted status survive a revoked authority.

Author critique (not independent acceptance): an all-in-one lifecycle could become
a new monolith, so this proposal separates dimensions and reuses existing facts;
an operation ID could become a permission token, so association and grants remain
authenticated separately; retry could hide cost, so ancestry survives new attempts;
cancellation could be dishonest, so it requires containment evidence; a shadow
feature could silently take authority, so live effects stay disabled until a
separate reviewed cutover. If the association adds no demonstrated value to the
report-only pilot, revise or drop it rather than retaining redundant machinery.

### Decision packet for the next independent review

Author recommendations, 2026-10-22; **final decisions pending**. These are proposed
answers to the five questions above, not accepted policy or a schema freeze.
Review each against the pinned sources in section 1 and the full transitions and
cases, not just the document's structure. A recommendation may be accepted in
principle while its implementation gate remains closed.

| Decision | Recommended direction and reason | Alternative / required exit evidence |
| --- | --- | --- |
| D01: association | Prefer a separate authenticated association of operation, intent revision, operation attempt, verification request, spool attempt and evidence digest. Preserve existing signed packet bytes and keep the coordinator outside candidate-controlled storage | Compare a versioned packet extension with an old/new-reader compatibility matrix. Reject a second mutable receipt. Require swapped-operation, swapped-attempt and altered-digest negative cases (OCV06-08/20); drop the association if it adds no demonstrated pilot value |
| D02: representation | Freeze the semantic fields before selecting a signing codec. Require explicit version and domain separation; reject duplicate/unknown fields, ambiguous identifiers, integer overflow and oversize input before admission. Keep Base36 a display alias | Do not choose JSON formatting, canonical byte framing, namespace or numerical limits by implication. Reviewer must accept exact schema, byte-level golden vectors and field/total bounds, including cross-language integer behavior, before parser or signature implementation. Those artifacts are not supplied or approved here |
| D03: pilot | Use private devtools, an explicit immutable revision and an existing trusted verification recipe as the candidate pilot. First consume retained evidence offline in shadow mode; later allow a separately approved report-only execution. Ubuntu is the proposed sole coordinator; Windows is a worker/client, never automatic fallback writer | No product rewrite, public-fork execution, model calls, recursive children, publication, integration or deployment. Propose one active operation and one attempt for the first execution trial. Policy owner must approve the exact recipe, principals, paths and numerical time/memory/process/output budgets; unset limits block dispatch. Report-only still executes code when a recipe runs and is not a sandbox guarantee |
| D04: persistence | Keep verification claims/receipts in the existing spool. Keep operation-only facts under one logical writer with expected-revision checks; rebuild projections from retained authoritative facts. A cross-store association must not pretend to be one atomic transaction | Defer storage engine and commit protocol until a scoped design demonstrates crash boundaries, deduplication, writer exclusion and recoverable orphan associations. Require OCV05/14/16/17/21 on both native hosts before live effects. Neither hash chaining nor rename alone qualifies durability; no automatic lease takeover |
| D05: completion | Preserve historical review evidence but derive current eligibility from exact intent, candidate, policy, suite, platform set and current authority. Report-only completion needs its admitted report/evidence criteria, not a PR; execution finished is not equivalent to accepted | A changed subject or revoked grant removes current eligibility without deleting the old review. Unknown required effects prevent successful completion. Reviewer must specify freshness/revocation behavior and the report-only acceptance requirement; mutation remains separately gated (OCV06/09/10/12/15) |

### Counterexamples to resolve before reducer fixtures

These are historical author-discovered specification gaps, not reproduced
implementation failures. Revision 2 incorporates proposed resolutions and
Claude's refinements into sections 2-4 and OCV23-26. The traces below preserve
the review rationale; the revised transition table is the single proposed rule
source, not this historical summary. Neither is independently frozen yet.

| Gap | Minimal trace exposing ambiguity | Proposed resolution for review |
| --- | --- | --- |
| Pause/resume destination | T04 starts an attempt, T06 pauses it after quiescence, then T07 revalidates; T07 does not say whether the attempt resumes or a new one starts | Record the suspended attempt and whether its executor is resumable. Resume that same attempt to active only with proof of retained ownership and quiescence. Otherwise reconcile and close it before a fresh attempt; no implicit re-execution. Paused-before-attempt returns to admitted |
| Block/retry ownership | T05 blocks an active attempt; T04 permits beginning from blocked, while T13 also allocates retry attempts | A blocked phase alone must never authorize a second unresolved attempt. Require final disposition and writer/effect exclusion first. Have one attempt-allocation transition used by both initial start and retry, with T13 supplying retry-specific guards and an explicit destination |
| Cancel before execution | T08 permits cancelling proposed/admitted work with no attempt; T09 says finalize cancelled without naming which record owns that disposition | Finish the operation with an operation-level cancellation reason and no fabricated attempt/receipt. Keep attempt disposition absent when none existed; separately specify operation finalization fields in D02 |
| Revocation after acceptance | T14 records accepted; T16 finishes execution; later the accepting authority is revoked before any publication request | Historical acceptance remains auditable, but current eligibility becomes withheld on re-evaluation. No execution restart and no effect based on the old accepted label. Specify a revision-bound eligibility query or invalidation event rather than silently overwriting the signed decision |

Review handoff: record the reviewed full head, reviewer independence, a separate
accept/revise/defer for D01-D05, and a disposition for each counterexample. Name
remaining evidence and its owner rather than treating silence as acceptance.
A defer keeps the affected implementation gate closed. Once these dispositions
are reconciled and independently accepted, claim the schema/golden-fixture slice
under section 7; until then, no parser, reducer or service activation is cleared.
