# Aaroneous Foundation Maintenance System

This repository owns operational planning, multithread coordination, maintenance records, and
private foundation tooling for Aaroneous. The Aaroneous repository owns product code, product
contracts, verification commands, and product documentation.

## Sources of truth

| Artifact | Purpose |
| --- | --- |
| `governance/WORKLIST.md` | Priorities, acceptance criteria, and evidence-backed status. |
| `governance/COORDINATION_QUEUE.md` | Live ownership and file scopes for concurrent work. |
| `governance/SCHEDULE.md` | Recurring maintenance cadence and evidence required per run. |
| Aaroneous `docs/HARDENING_AUDIT_MATRIX.md` | Technical audit lenses and acceptance criteria. |

## Operating rules

- A work item may enter the worklist only with a source path, concrete risk or failure mode, and
  a bounded acceptance criterion.
- A thread claims a queue item before editing its scoped paths. One owner holds a file scope at a
  time.
- Maintenance updates use repository and CI evidence. They never invent completion or overwrite
  another thread's uncommitted work.
- Product changes must pass Aaroneous's native verification gates. Foundation tooling must not
  become an Aaroneous build, CI, release, or runtime dependency.
- Qwen is used only for a narrow, owner-reviewed subtask in a scratch location; its output is
  never applied directly to a shared branch.
