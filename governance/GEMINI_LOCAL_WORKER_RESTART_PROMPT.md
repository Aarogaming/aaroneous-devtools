# Gemini Local Worker Restart Prompt

A ready-to-paste prompt asking Gemini/Antigravity (currently hosting the
local worker) to rebuild and restart it so the M61/M62/outbox commands
merged this session actually become available. Not executed by Claude —
Gemini is the one actively responsible for this process right now, and
restarting something it's hosting is Gemini's call, not something to do
out from under it.

---

```text
You're currently hosting the local worker (foundation.exe, PID 124336 at
last check, adaptive local-worker-serve loop). New code has been merged
into integration/local-cloud-orchestration in this same checkout since you
started it -- the outbox (C42: outbox-list/show/verdict), M61
(outbox-promote), and M62 (local staging worktrees: stage-draft/
list-staged/prune-staged). See governance/COORDINATION_QUEUE.md rows
C42/C43 and governance/ROUND_TABLE_PING.md for the full design context if
you want it.

None of this touches local_worker.rs's own tick/generation logic -- what
it generates and how it decides when to tick again is unchanged. The only
reason a restart matters is that the currently-running binary predates all
of the above, so none of the new subcommands exist in it yet. This is a
binary refresh, not a data or config migration -- worker.toml, results/,
and archive/ are all untouched by any of this and don't need anything done
to them.

Please:

1. Confirm how you're currently running the process (a terminal running
   `foundation local-worker-serve --config <worker.toml>` directly, the
   installed Windows task, or something else) -- you know this, I don't,
   and the right stop/rebuild/restart steps depend on which it is.
2. Stop it cleanly for whichever mechanism that is. If it's the installed
   Windows task, governance/LOCAL_WORKER_SERVICE.md documents the exact
   procedure (pause the task, verify it is not running, build foundation,
   replace the copied executable, then resume) -- follow that rather than
   improvising if that's your setup.
3. Rebuild from the current integration/local-cloud-orchestration branch:
   cargo build --release -p foundation_cli --manifest-path foundation/Cargo.toml
   (or whatever build invocation your hosting setup actually uses -- match
   your existing convention, don't introduce a new one).
4. Before restarting the service, verify the new commands are actually
   present in what you just built:
   foundation --help
   should list outbox-list, outbox-show, outbox-verdict, outbox-promote,
   stage-draft, list-staged, and prune-staged alongside the existing
   commands. If any are missing, something didn't rebuild from the branch
   you think it did -- stop and check before restarting the service on a
   stale binary.
5. Restart local-worker-serve the same way you had it running before.
6. Confirm it's actually ticking again -- check status.txt or the next
   couple of worker-jobs/results/ entries land as expected, the same way
   you'd verify after any restart.

Report back what you found for step 1 (how it was actually running) and
confirmation that step 4's --help check passed before you restarted it.
```
