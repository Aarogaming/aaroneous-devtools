# Guild broker preview v1

Status: implemented preview adapter, pending independent review; C180, stacked
on broker PR263. This command does not activate job dispatch or adopt Guild's
proposed operation lifecycle. It exposes the existing
[report-only broker](intelligence-broker-v1.md) through the Guild CLI.

```text
foundation guild broker-preview --input <absolute-snapshot.json>
```

Input is one UTF-8 JSON regular file, at most 1 MiB. Reading is bounded even if
the file grows. The parser rejects duplicate struct keys, unknown fields,
unsupported versions, wrong types and missing required fields. Missing or null
cost/duration bounds are unknown quotes, producing holds. JSON recursion limits
remain enabled. Semantic collection limits come from the broker contract.
Principal IDs follow the broker's canonical ASCII grammar; invalid provider or
exclusion IDs fail the entire preview rather than producing candidates.
File type is checked before open and again on the opened handle. The caller must
keep the path and its parent directories stable: this is not protection against
a hostile concurrent replacement between metadata inspection and open.

Minimal illustrative snapshot (identities and evidence references are examples,
not credentials or qualified providers):

```json
{
  "v": 1,
  "job": {
    "id": "job-1", "subject_revision": "subject-1",
    "now": 100, "deadline": 120, "max_evidence_age": 10,
    "remaining_budget": 10, "interface": "task-v1",
    "requirements": [{"capability": "review", "suite_revision": "suite-1",
      "minimum_samples": 10, "minimum_pass_bps": 9000}],
    "authorized": [{"provider": "worker", "revision": "config-1"}],
    "excluded_principals": []
  },
  "providers": [{
    "key": {"provider": "worker", "revision": "config-1"},
    "principal": "principal-1", "kind": "model",
    "available": true, "resources_fit": true, "interface": "task-v1",
    "cost_bound": 10, "duration_bound": 20,
    "evidence": [{"capability": "review", "suite_revision": "suite-1",
      "reference": "evaluation-1", "observed_at": 90, "passed": 9, "samples": 10}]
  }]
}
```

Provider kind is `model`, `human`, `tool` or `other`. All policy, identity,
resource and evidence assertions here are unverified input. The adapter never
reads credential stores or treats a snapshot's authorized list as an actual
grant. Use sanitized metadata only: the report retains the full snapshot.
Reports are as sensitive as their input, including policy exclusions; keep them
within the same authorized audience as the snapshot.

## Output and replay

One JSON report is printed to stdout, without creating a log, outbox, spool or
claim. Exit 0 means evaluation completed, including a hold; it never means work
was authorized or executed. Invalid input exits nonzero without a report.

Output fields: `v:1`, `kind:"guild.broker-preview"`, `decision` (`candidates` or
`hold`), `job_id`, `subject_revision`, `evaluated_at`, `ranked`, `excluded`,
`input_sha256` and `input_utf8`. `dispatch_authorized`, `policy_verified` and
`evidence_verified` are always false. Rank and cost fields follow the broker;
excluded reasons have a stable `code` and nullable `capability`. The codes are
unauthorized, principal_excluded, unavailable, resources, interface, unknown_quote,
budget, deadline, missing_evidence, invalid_evidence, stale_evidence,
insufficient_evidence and below_threshold.

The digest is SHA-256 of the exact UTF-8 input bytes, including whitespace.
`input_utf8` preserves those bytes as a JSON string for replay. This binds the
report to what was read, not to a signature or trusted provenance. The caller
retains output with the executable/commit identity and appropriate data controls.
Time is entirely supplied; replay never substitutes the wall clock.

## Path to dispatch

Current `guild::run_tick` invokes `steward::cards::build_cards`, then records
and publishes advisory cards. Actual platform jobs use authenticated issuer and
worker roles through `steward::job_service::Service::handle` and durable Store
claims. This preview deliberately modifies neither path.

Before a separate dispatch adapter can use broker results, independent review
must accept PR263 and this contract; the adapter must obtain verified capability
evidence and role/data authorization, bind provider identity to worker identity,
re-evaluate the current subject and revocation state, and atomically reserve the
operation's remaining budget with a durable claim. Cancellation, unknown effects,
late results and restart recovery require integration tests. Existing job-service
claims do not by themselves implement aggregate operation budget reservations.
Never submit this preview report to a claim endpoint as an authorization token.

### Next implementation boundary (proposed, not implemented)

The existing `/v1/claim` branch in `job_service::Service::handle` authenticates a
worker, filters platform cards, checks trusted base policy and calls
`local_verification::Store::claim`. Integrate admission immediately before a new
claim, and validate resumed claims against their retained admission. Do not route
LLM jobs through platform verification requests: those execute pinned suites,
not arbitrary provider tasks.

| Increment | Required behavior and evidence |
| --- | --- |
| Trusted admission input | Bind exact request/subject, profile revision, enrolled worker principal, policy revision, evidence digest, quote and expiry. Resolve aliases through enrollment, not display names. Verify provenance before calling the broker; test forged binding, stale evidence and revoked roles. |
| Durable reservation | One serialized operation transition binds the selected candidate, attempt and upper-bound cost before execution can become visible. Concurrent claims cannot overspend; replay of the same attempt cannot debit twice. Test contention and crashes at each publication boundary. |
| Claim and resume adapter | Recheck subject, policy and expiry at claim time; persisted admission must match the authenticated worker and exact request. Recovery retains the reservation when effects are unknown; test reconnect, restart and mismatched worker/attempt. |
| Settlement | Bind signed result and actual cost to the reservation. Duplicate or late results cannot release budget twice; cancellation alone cannot refund uncertain external effects. Test late completion, cancellation and reconciliation. |

Use the existing service and spool ownership boundary. A separate budget file
written after `Store::claim` is insufficient: a crash could expose executable
work without reserving its cost. Specify the recoverable transaction and failure
states before enabling the endpoint. Each increment needs an exact queue claim,
tests and independent review; this table grants no runtime authority.

Rollback: remove the additive preview action/module, tests and contract. No
persistent runtime state or existing wire format changes. No dependency on
Devtools is added to Aaroneous. Linux/Windows CLI tests live in
`foundation/crates/foundation_cli/tests/guild_broker_cli.rs`.
