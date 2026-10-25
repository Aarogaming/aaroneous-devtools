# Multithreader review service

The native `foundation multithreader-review --config ABSOLUTE_FILE` command reviews explicitly selected product files with local Qwen and writes an immutable candidate inbox record. It does not implement work, claim lanes, change the canonical queue, check GitHub CI, or certify the framework. Owners promote useful suggestions through the existing queue protocol after checking the evidence.

## Configuration

A version-1 TOML config requires absolute `checkout`, `queue`, `program` and `output_directory` paths, a loopback HTTP `endpoint`, and 1–8 checkout-relative `evidence` file paths. The output directory must exist outside the product checkout. Use the explicit existing devtools checkout, not an assumed `D:\aaroneous-devtools` location.

```toml
schema_version = 1
checkout = 'D:\Aaroneous'
queue = 'C:\Users\aarog\.codex\worktrees\aaroneous-devtools\governance\COORDINATION_QUEUE.md'
program = 'C:\Users\aarog\.codex\worktrees\aaroneous-devtools\governance\MULTITHREAD_PROGRAM.md'
evidence = ['README.md', 'docs/ARCHITECTURE.md']
endpoint = 'http://localhost:11434'
output_directory = 'C:\Users\aarog\.codex\worktrees\aaroneous-devtools\artifacts\multithreader-service\inbox'
```

Each evidence file is bounded to 32 KiB and the complete prompt to 64 KiB. Queue input is bounded to 256 KiB. Responses are bounded to 64 KiB. The generator uses `qwen3.5:9b-q6`, temperature 0, 4096 predicted tokens, JSON output, and `think:false`; it rejects incomplete/truncated responses and HTTP redirects. Evidence remains untrusted data.

## Outcomes and ownership

- `no-new-work`: model returned a null proposal; this is not proof the repository has no defects.
- `pending-owner-review` (status name retained verbatim): structural, declared-source, existing-file, dependency, portfolio ID and queue checks passed. A seat independent of the submitter must still reproduce the defect and approve its priority and acceptance criterion.
- `rejected`: proposal overlaps a queue scope or fails contextual checks. Completed queue references also require explicit reconciliation before repeat work.
- `duplicate-inbox-candidate`: an existing pending record has the same ID or exact ordered file scope. Semantically similar proposals with different scopes still require independent review.
- `failed`: model/protocol failure; an inbox failure record is written and the executable returns nonzero. Invalid configs or input acquisition failures return nonzero before creating a report.

Records are `review-NNNNNN.json`; they retain the exact evidence bundle and queue diagnostics. A `review.lock` prevents simultaneous service runs. After an interrupted process, inspect activity before removing a stale lock. Review and archive records weekly; automatic retention, unchanged-input caching, claim dispatch, and semantic duplicate detection are future work. Do not delete pending records before their disposition is recorded.

`generate-intake` writes a standalone proposal, or JSON null when no work is found. `validate-intake` validates only the standalone proposal structure, reporting `structurally_valid` and `requires_owner_review`; contextual validation belongs to `multithreader-review`. A parseable response is never an approved task.

## Persistent mode: `multithreader-serve` (2026-09-17)

`foundation multithreader-serve --config ABSOLUTE_FILE [--interval-minutes N]` (default 30) runs review cycles on an interval **inside the process** — no OS Task Scheduler entry, no installed service, nothing to separately enable or forget to disable. Starting the command is "on"; stopping it (Ctrl+C, or just not running it) is "off". Each cycle is the same `multithreader-review` logic, including its own `review.lock` acquisition and release, so a bad or interrupted cycle can't wedge the next one; a cycle that fails is logged to stderr and the loop continues rather than exiting, matching what a long-running service should do (a transient failure shouldn't require restarting it).

```powershell
& '<path>\foundation.exe' multithreader-serve --config '<path>\review.toml' --interval-minutes 30
```

Owner decision (2026-09-17): the previously-installed `Aaroneous-Devtools-Multithreader` Windows Task Scheduler task (below, historical) was removed — the repository "wasn't ready for the schedule yet, there were still too many bugs for it to run all day every day" unattended. Persistence now defaults to off (this is an explicit subcommand you choose to run, not something installed to auto-start) precisely so the decision to run it — and for how long — stays in the owner's hands each time, rather than living in OS-level configuration that's easy to forget is still active.

## Historical: installed Windows service (2026-09-16, removed 2026-09-17)

Task: `Aaroneous-Devtools-Multithreader`, every 30 minutes while the user is logged in, limited privileges, overlapping launches ignored, five-minute execution limit. This is Windows Task Scheduler software, separate from the paused Codex heartbeat. Ollama must already be running; the service does not launch it or install models.

Executable and config live under the private checkout's ignored `artifacts/multithreader-service/`. The initial evidence selection is README and ARCHITECTURE only. Extend the declared files deliberately within the bounded budget; this is not a whole-repository audit.

Manual invocation of the underlying one-shot command (still available, unchanged, and still what `multithreader-serve` calls each cycle):

```powershell
& 'C:\Users\aarog\.codex\worktrees\aaroneous-devtools\artifacts\multithreader-service\foundation.exe' multithreader-review --config 'C:\Users\aarog\.codex\worktrees\aaroneous-devtools\artifacts\multithreader-service\review.toml'
```

## Verification evidence

Local foundation tests: 53 passed (includes 2 new tests for `multithreader-serve`'s interval bounds and its non-panicking failure handling); formatting and all-target clippy with warnings denied passed. The full mocked pipeline proves pending candidate creation, duplicate suppression and preservation of the original queue. Protocol tests prove no-work handling, truncation rejection and loopback-only transport.

Historical (2026-09-16): a direct local-Qwen run created `review-000000.json` with `no-new-work`. A Windows Task Scheduler launch created `review-000001.json` with the same outcome. LastTaskResult was 0 at 2026-09-16 16:33:14 EDT; next launch was scheduled for 17:03:14 EDT. Both runs reported the existing duplicate M21 queue records without modifying them. Product code was not changed by this task.
