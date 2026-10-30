# Intelligence broker: report-only Rust boundary

Status: implemented first slice, pending independent review. C178. Rust API,
not a wire schema or deployed service. No dispatch path calls it yet.
Entry point: `steward::intelligence_broker::assess(&Job, &[Provider])`.

Steward matches jobs to models, people, deterministic tools or future providers.
Guild retains operation ownership and aggregate budgets. Existing model-hub
adapters supply resource-fit decisions. This adds evidence-based recommendations
without replacing `model_hub::choose` or building another provider gateway.

## Inputs and trust

Principal identifiers must be nonempty canonical ASCII lowercase IDs (at most 128 bytes; letters, digits, `-`, `_`, `.`, `:`). Both provider and excluded principal IDs are validated; malformed IDs fail the whole report. The trusted identity adapter must resolve distinct aliases to the same canonical principal before calling the broker; syntax validation cannot establish identity or reviewer independence.

The caller supplies job identity, immutable subject revision, time, deadline,
remaining aggregate budget, interface, requirements and excluded principals.
The authorized profile list comes from trusted policy evaluation for this exact
job, subject, role and data scope, never from provider self-description. This API
does not authenticate signatures, authorize data export or evaluate reviewer-family
quorum. It cannot turn a caller's untrusted assertion into trusted evidence.

Profile identity includes an immutable configuration revision covering the
model/tool version, settings and adapter; human qualifications are versioned too.
Evidence is nested under that profile: capability, suite revision, retained
reference, timestamp, passes and samples. Ingestion must verify provenance and
configuration binding; relabeling old evidence as a new profile is invalid even
though the pure function cannot detect a dishonest caller. Missing evidence means
unknown, not incapable. Raw histories remain retained outside this report.

## Eligibility and ranking

All hard checks must pass: exact profile authorization, role independence,
availability, resource fit, interface match, known upper-bound cost/duration
within remaining budget/deadline and fresh sufficient evidence for every required
capability/suite. Future observations and conflicting evaluation summaries hold.
One canonical summary per capability/suite avoids cherry-picking a favorable run.
Minimum sample count and pass rate are independent requirements.

Rank by worst observed pass rate across requirements (basis points, descending),
then cost, duration and profile identity ascending. Provider kind grants no
preference. These observed rates are not confidence intervals, predictions or
proof; representative evaluation and independent acceptance remain necessary.
Cost uses one caller-defined accounting unit. Duration/time use a common unit and
time domain; an adapter must normalize cross-host clocks. Equality at bounds is
allowed. Reports include job, subject, evaluation time, evidence references and
typed exclusion reasons. Empty ranked candidates means hold, including an empty
inventory. Ambiguous identities or invalid jobs fail the report rather than
returning a partial recommendation.

Input collection limits: 256 providers, evidence entries per provider, authorized
profiles and excluded principals; 64 requirements. Checked identity/reference
strings must be nonempty, control-free and at most 1024 bytes. The caller owns
input allocation and transport-size limits; this is not a hostile-byte parser.

## Integration and recovery

Pure function: no I/O, calls, dispatch, prompt rewriting, reservation, persistent
learning or input mutation. Existing services continue without this report.
This does not implement the proposed Guild operation state machine.

Next adapter must retain an immutable input snapshot/digest with each report,
authenticate evidence/policy, re-check eligibility at dispatch and atomically
reserve aggregate operation budget. Recommendations are not reusable grants.
Retries cannot restore spending limits. Adaptation may clarify and structure
intent, not conceal it to bypass provider safeguards. Refusal, unsupported task,
malformed output, timeout and failed correctness checks stay distinct outcomes.
Learned adapter changes need versioned evaluation and independent promotion.

Validation: `foundation/crates/steward/tests/intelligence_broker.rs` exercises
hard gates, stale/configuration/suite evidence, overflow, insufficient samples,
missing capabilities, deterministic ranking, retry budget and revoked eligibility.
Run Foundation formatting, tests and strict Clippy. Before integration, rollback
is reverting this additive module/export/tests/contract; no runtime state or
receipt format is migrated. Live dispatch, provider adapters and LAN operation
remain unimplemented by this slice.
