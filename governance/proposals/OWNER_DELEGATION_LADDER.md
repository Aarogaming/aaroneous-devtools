# Owner-to-user delegation ladder (proposal)

notation: v1
purpose: Define the mechanism that lets the owner delegate the full operator role and be demoted to user
authority: propose
status: proposal; owner-directed requirement recorded in DECISIONS.md 2026-10-11
relations:
  - amends: doc:AUTOMATED_MERGE_POLICY.md (Scope, phase framing)
  - depends-on: doc:IDENTITY_SETUP.md, doc:SEAT_APPS.md
  - informs: doc:LOCAL_MODEL_AUTONOMY_POLICY.md
checked-by: none yet

Contribution-ID: C128/mimo-owner-delegation-ladder-v1. Author seat: MiMo (OpenCode on Windows).
Status: proposal for review. Nothing here changes operation until landed and separately activated per rung.

## Problem

Owner, 2026-10-11: devtools "should not require owner anywhere"; Aaroneous owner-gating is a
development-phase constraint; the owner's preferred end state is **demotion to user** — blocked only
because "we haven't built the solution yet for me to delegate the full role and operation to yet."

The owner role today is a bundle of powers: product merges, claim arbitration, policy edits, trust
admissions (seats, keys, `--require-signatures`), circuit-breaker clearance, scope exceptions, and
Tier-3 escalations. Demotion is safe only when each power has a named mechanical successor and the
owner keeps one fail-safe.

## The ladder (order matters: each rung is a precondition of the next)

| Rung | Mechanism | State |
| --- | --- | --- |
| R1. Seat identity | Per-seat GitHub Apps posting as `aaroneous-*-seat[bot]`; ed25519 attestation keys; signed head-bound verdicts | **Done** (2026-10-10/11) |
| R2. Signature enforcement | Signed independent accepts are the only accepts the merge gate counts; the steward/report `--require-signatures` flag exists for its own outputs | **On at the merge gate** (correction 2026-10-11: the gate already refuses unsigned accepts; only the steward-side report flag was ever "unflipped") |
| R3. Mechanical quorum | N-of-M signed independent accepts + `auto_merge.py` as the only button-presser (devtools) | **Live** for devtools |
| R4. Scoped delegation record | A signed, owner-held instrument enumerating which owner powers move to which quorum and for how long | **Missing — this is "the solution"** |
| R5. Product-merge graduation | Aaroneous merges graduate to the gate **by category** (test-only first, the trust-building pattern of `AUTOMATED_MERGE_POLICY.md`), owner checkpoint shrinking to the category-graduation PR | Not started |
| R6. Fail-safe | Owner retains root revocation until the quorum is proven over a recorded trust-building period. Demotion means never using the power, not deleting it | Standing rule |

## R4 in detail: the delegation record

A machine-checked document (proposed: `governance/delegation.json`, validated like `identities.json`)
signed by the owner key, carrying per-power grants:

- `power`: one of `devtools-merge` (granted by default, 2026-10-11), `claim-arbitration`,
  `policy-edit`, `seat-admission`, `require-signatures-toggle`, `breaker-clearance`,
  `scope-exception`, `product-merge:<category>`.
- `quorum`: e.g. `2-of-4-seats`, `3-of-5-seats`, or `owner` (default for everything not listed).
- `expires`: every grant renews explicitly; expiry falls back to `owner`, never to a wider quorum.
- `conditions`: mechanical predicates checked by the merge gate (signed accepts, green CI,
  no open threads, evidence bar unchanged).

Rules the record must encode, from the recorded decision and the C14/C66 history:

1. The evidence bar never drops; only who presses the button changes (`AUTOMATED_MERGE_POLICY.md`
   precedent).
2. A quorum is seats with **verified** signatures (R1+R2), never names on an unauthenticated channel.
3. Powers are granted one at a time, each behind its own trust-building observation period.
4. Revocation is owner-held (R6) and **epoch-bounded**, not prose-instant (correction 2026-10-11, Codex review): grants carry an `epoch` and `issued_at`; revocation bumps the epoch, verifiers reject stale-epoch grants and replayed artifacts, and an offline node cannot act past its epoch window — so revocation is bounded by the window, never claimed as instantaneous.
5. Owner-demotion ("user") is declared only when every power in the bundle has a live grant and the
   trust-building periods have closed — recorded as a final owner decision, not assumed.
6. **Activation, not just recording** (correction 2026-10-11, Codex review of #189): a grant is
   `pending` until an explicit activation record with a fresh epoch activates it. Every reserved
   power stays pending — owner-side — until a valid **activated** grant exists. Recording a grant
   activates nothing by itself.
7. **Self-amendment boundary**: the delegation validator, the grant schema, the identity registry and
   R6 itself are trust anchors. No grant may amend, replace or bypass them; a grant whose scope
   touches the validator, the schema or R6 fails closed. The grant system cannot widen itself.
8. **Freshness and replay**: grants carry `epoch` and `issued_at`; verifiers reject stale epochs and
   replayed grant artifacts; a node that was offline re-syncs the epoch before acting on any grant.
   These three rules answer Codex's review points on #189 (activation, self-amendment protection,
   offline-node revocation bounds); the per-power activation matrix from that review is adopted in
   shape: each power's row will name its quorum, its activation precondition and its epoch window
   when R4 is implemented.

## What this proposal is not

- Not an authorization to loosen any current gate. Product merges stay owner-pressed until R5
  lands by its own recorded decision.
- Not a charter for board-text governance: authority changes land here, in reviewed files (the
  2026-10-11 board provenance discussion is the counterexample this follows).

## Review focus

- Is the power bundle complete? Anything the owner does today that is missing from R4's list?
- Is owner-held root revocation (R6) the right fail-safe, and is "never delegated" implementable
  mechanically (e.g. the delegation validator refuses any grant whose quorum can amend R6)?
- Are category graduations at R5 the right unit, and is test-only-first the right first category?

## Not verified

- No validator, schema or quorum mechanism exists yet; all rungs past R3 are design.
- Whether GitHub's branch-protection APIs can express N-of-M signed-accept rules mechanically
  (today `auto_merge.py` enforces them in code — sufficient, but worth checking).
- The trust-building period length for each grant; unmeasured.
