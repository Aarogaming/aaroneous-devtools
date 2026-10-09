# Maintenance restart recovery v1

C112 corrects persistent state/control reads in the existing native maintenance pipeline.
The Rust pipeline was merged in PR #138; this change extends it rather than introducing another controller.

## Behavior

Only an absent `pipeline/state.json` permits initialization. An existing malformed, unreadable,
oversized (over 64 KiB), or non-regular artifact (including symlinks) reports its path and stops processing, retaining its bytes.
Schema version must be 1, attempt must be 0 or 1, and stage must be one of the existing pipeline stages.
These checks do not grant acceptance or authenticate a producer.

An absent `loop-control.json` retains the existing daemon run default. An existing file must parse
as loop control with state `run`, `pause`, or `restart`. Invalid input no longer silently resumes work.
The daemon checks saved pipeline state before refreshing evidence or calling the worker, and rechecks
control before dispatch and during its interruptible wait. Errors release its in-process lease.

`maintenance status` reports state/control errors instead of concealing them as JSON null.
`maintenance housekeeping` also refuses corrupt pipeline state instead of presenting an unknown stage.
Fresh runtime status still reports absent artifacts as null. Existing valid run/pause/restart files remain compatible.

## Recovery

The tool does not repair, delete, quarantine, or overwrite invalid artifacts. Stop the host, preserve
and inspect the named file and its retained receipts, then explicitly restore a verified state/control
record. Removing state intentionally starts fresh work; this is an operator decision, not an automatic retry.

## Scope and verification

CLI regressions cover truncated JSON, missing control fields, unknown control values, directories in
place of files, unsupported state version/stage, exhausted retry bounds, valid controls, absent state,
and daemon refusal before refresh/dispatch. All test inputs are isolated temporary artifacts; no model
call or deployed runtime is needed. Run Foundation formatting, all-target workspace tests and strict Clippy.

This increment does not implement automatic restart reconciliation, stale-lease reclamation, or semantic
state authentication. Lease and last-cycle status parsing still follows the previous behavior. These are separate follow-ups.
No model authority, automatic acceptance/merge, credential, schedule, deployment, or product behavior changes.
Rollback reverts this increment while preserving runtime evidence.

## Atomic replacement and explicit inputs (C114)

C114 builds upon C112 by implementing atomic file replacement and explicit checkout parameters:
- `save_json_atomic` and `write_atomic` write to a temporary file in the same directory and perform an atomic `fs::rename` over the target, ensuring power-loss durability and eliminating partially-written state or job files.
- `maintenance daemon` accepts an explicit `--checkout <PATH>` parameter, eliminating hardcoded host paths (`D:\Aaroneous`) for zero ambient authority across operating systems. When `--checkout` is omitted, it logs an explicit notice and continues without failing.
