# Local Worker Runtime Reconciliation

## Status

This is the evidence record for M74 as of 2026-09-23. It describes observed
state; it does not start, schedule, promote, or authorize the local worker.

## Authoritative configuration selection

Two explicit configurations were found. The active devtools checkout is
`D:\aaroneous-devtools` at revision `50d106b`. Its
`artifacts/local-worker-service/worker.toml` names that same checkout, queue,
job directory, runtime results/archive directories, reservation directory, and
the declared `qwen3.5:9b-q6` loopback endpoint. This is the configuration to
use for the next deliberately requested run.

An older configuration under
`C:\Users\aarog\.codex\worktrees\aaroneous-devtools\artifacts\local-worker-service`
points at a different checkout and an older copied binary. It is historical
runtime evidence, not a fallback target. Do not start it merely because it
exists.

## Observed liveness and backlog

At the sweep time, no `foundation` process was running. The declared D runtime
had 108 files in `results/` and 678 in `archive/`, while its configured
`max_pending` is 64. The most recent result file was
`FIX-C40-SI-PACKER-HEADER-0000.json`; the most recent archive file was
`status.txt`. Recent source job manifests were created on 2026-09-22, so the
worker did run against this checkout before stopping.

The retained verdict ledger contains 19 `accept`, 17 `needs-changes`, and 2
`uncertain` verdicts. A fresh read-only `outbox-list` scan of the selected
runtime's results plus sibling archive found 731 retained records: 693 have no
recorded verdict, 38 are reviewed, 726 are `pending-owner-review`, and 5 are
failed. The older 17-entry review-batch snapshot is therefore a partial stale
view, not a valid backlog measure. Review and disposition capacity, not job
generation, is the immediate constraint.

## Operating decision

Do not restart generation until the pending results are reviewed or a bounded
retention/disposition policy is implemented and verified. Restarting at this
point would either remain backpressured or add noise to a backlog that already
exceeds its configured pending limit.

Before any restart, an integration owner must explicitly choose the D runtime
configuration above, rebuild from its declared revision if source changed, and
record the process start, binary revision, health outcome, result count, and
review count. A restart never grants permission to promote a draft or modify
the Aaroneous checkout.

## Next bounded work

1. Produce a bounded, current review batch from the 693 unreviewed records,
   ordered by risk, age, and duplicate-pass suppression; do not rely on the
   stale 17-entry snapshot.
2. Recompute results, archive, and unreviewed counts from the declared D
   runtime after each review batch.
3. Reproduce any claim that the worker fails to recover after pending results
   drop below `max_pending`; only then change `local_worker.rs` or its tests.
4. If no recovery defect is reproduced, start one explicitly approved,
   bounded run with the D configuration and inspect the resulting health record.
