# Maintenance Schedule

## Daily automated check

Review Aaroneous CI and `governance/WORKLIST.md`. Update only when evidence changes: CI result,
blocked item, completed item, or newly reproduced defect. Do not issue routine status noise.

## Weekly manual review

- Reconcile completed queue claims and expired ownership.
- Run the dependency/advisory review when its lane is active.
- Review the hardening matrix for evidence gaps introduced by recent changes.

## Monthly architecture review

- Reassess the execution-host boundary and public APIs.
- Classify one bounded legacy/interface/terminology area.
- Retire obsolete foundation artifacts and verify no product dependency points at devtools.
## Per-claim lifecycle

At claim time, record priority, dependency, exact paths, and expected evidence. At review time,
verify the required gate and release paths. At weekly review, keep no more than three primary
implementation claims open unless an additional scope has independent files and a named reviewer.

## Native candidate review (installed 2026-09-16)

Windows task `Aaroneous-Devtools-Multithreader` reviews the declared evidence every 30 minutes through the Rust foundation executable. Suggestions and failures go to an external devtools inbox for owner review. See [MULTITHREADER_SERVICE.md](MULTITHREADER_SERVICE.md) for inputs, limits, verification and operating commands. This service does not check CI or automatically promote/implement suggestions. The Codex scheduled heartbeat remains paused.
