# Guild admission and reservation v1

Status: candidate implementation, pending independent review and qualification.
C182 adds `local_verification::Store::claim_admitted` at the existing durable
claim boundary. No HTTP route or provider executor invokes it. The parent broker
and preview delta reviews remain separate acceptance gates.

## Inputs and authority

The caller supplies an existing absolute private ledger directory separate from
the verification spool, an immutable operation ID, total budget, policy revision,
verified evidence digest, selected profile, current broker job/provider inputs,
the exact platform verification request and the authenticated worker principal.
The exact request card must already have been issued into the spool. Admission
does not create a new job or leave an empty job directory when it holds work.
Operation/policy/evidence IDs are lowercase 64-character hexadecimal identifiers.
The operation metadata fixes budget, policy revision and canonical spool path.
Changing any of these on replay fails. One operation must always use the same
ledger; selecting a new directory or operation ID is not a budget reset mechanism.

These are trusted in-process inputs, not authenticated merely because they are
typed or contain hashes. Before endpoint integration, the caller must verify
provenance, enrollment, family independence, role/data permission, clock, freshness
and policy revocation. Provider aliases must resolve to the enrolled principal.
The selected profile must belong to the authenticated worker. Job identity and
subject must match the request ID and head. Broker eligibility is reevaluated on
every call, including resume. The ledger-derived remaining budget replaces the
job's supplied remaining-budget field; snapshot preview reports are never grants.

This API handles existing pinned-suite verification jobs only. It is not a new
general model-job schema or a provider gateway. The caller still performs the
job service's trusted-base/suite/platform checks before using this API.

## Transaction and failure behavior

1. Acquire an exclusive create-new operation marker. Contention fails promptly;
   there is no automatic stale-lock reclamation or unbounded retry.
2. Validate immutable operation metadata and every existing reservation. Reject
   malformed, noncanonical, oversized or ambiguous history. Sum costs with checked
   arithmetic; at most 10,000 reservations are admitted per operation.
3. Reevaluate broker eligibility using total budget less all retained charges.
   A matching replay restores only its own charge for this eligibility check;
   it does not append another debit. Bind request, attempt, worker, profile,
   policy, evidence, cost and deadline in one immutable reservation file.
4. Write and sync that reservation before creating any executable claim. Acquire
   the existing per-job exclusive claim and fence the reserved attempt against
   the latest attempt history. Sync the claim, then write/sync the attempt record.
   Only then return the existing `Claim` type to the caller.
5. An existing complete claim is refused, including calls from the same worker.
   Never issue a second executable handle for that attempt. After stopping the
   old worker, explicit `Store::recover` allows a new attempt with a new charge.
   A matching incomplete publication may repair a missing attempt record only
   when the claim and reservation agree; subsequent calls are refused. A legacy
   claim without a reservation is refused.

All charges are retained, including failed claim publication, cancelled jobs and
abandoned attempts. The first version provides no settlement or refund API.
Over-reservation is preferable to an unaccounted external effect. Retry attempts
consume additional budget. Quote and deadline are immutable replay bindings;
changes require explicit reconciliation. An empty claim left by a failed attempt
fence is intentionally retained for operator inspection, never auto-unlocked.

A process crash can leave `transaction.lock`, an incomplete file, or a reservation
without a claim. They cannot authorize new execution. After stopping every user
of the operation, an operator may inspect the retained metadata, reservations,
claims and attempt history and remove only the stale operation marker. No marker
TTL proves that the worker stopped. A complete reservation can be replayed through
the API; incomplete/corrupt records require manual reconciliation, not deletion
to reclaim spending. Existing `Store::recover` retains every admission charge.

The filesystem and ledger are private, single-host and trusted against concurrent
external edits. File writes use `sync_all`; host power-loss/filesystem durability,
directory-entry persistence, network filesystems and multi-host shared storage
are **not qualified**. The contract is process-crash recovery on a retained local
filesystem, not a distributed transaction guarantee. Relocation requires an
explicit migration because the spool path is bound in operation metadata.

## Acceptance and remaining integration

Focused tests cover aggregate contention, restart/idempotent replay, reservation
and claim publication prefixes, corrupt records, stale locks, attempt recovery,
unchanged spending after abandonment, revoked roles, deadline, identity and
evidence binding. Foundation formatting, workspace tests and strict Clippy apply.
Fixtures simulate persisted crash prefixes; they do not establish power-loss
durability or an industrial deployment qualification.

Before routing HTTP claims here, independently accept the revised broker and
preview plus this transaction, implement trusted admission acquisition, ensure
all jobs in the opted-in policy use this boundary (including resumed claims),
and test the actual authenticated service with restart and revocation. The old
claim endpoint remains unchanged and does not enforce operation budgets. Do not
enable mixed budgeted and legacy writers for the same operation. Signed-result
settlement and cancellation reconciliation need a separately claimed increment.

Rollback: remove the additive module/export/tests/contract before activation.
Never delete ledger or claim history as rollback. Existing HTTP behavior and the
Aaroneous product dependency direction are unchanged.
